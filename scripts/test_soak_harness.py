#!/usr/bin/env python3
"""Self-tests for the soak drivers (REV-073).

RV-202: the F3 fill and cleanup scripts get the tmpfs-checked directory as
"$1" only, so a path with spaces or glob characters cannot widen or move
the target. Runs `sh` locally on a throwaway directory (no Docker, no fill
of any file system: at most 1,000 empty files).

Run: python3 scripts/test_soak_harness.py
"""

import importlib.util
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


faults = load("soak_faults", "soak-faults.py")
# Throwaway files go to memory when the platform has it.
SCRATCH = "/dev/shm" if os.path.isdir("/dev/shm") else None


class FillPaths(unittest.TestCase):
    def run_script(self, script, parent):
        subprocess.run(faults.fill_argv(script, parent), check=True)

    def test_awkward_path_stays_one_argument(self):
        with tempfile.TemporaryDirectory(dir=SCRATCH, prefix="rv202-") as tmp:
            root = Path(tmp)
            # "decoy" is what an unquoted path's first word would name.
            decoy = root / "decoy"
            decoy.write_text("keep")
            parent = root / "decoy dir [x]*"
            parent.mkdir()
            outside = root / "fill-outside"
            outside.write_text("keep")

            self.run_script(faults.TAKE_INODES, parent)
            made = sorted(p.name for p in parent.iterdir())
            self.assertEqual(len(made), 1000)
            self.assertTrue(all(n.startswith("fill-") for n in made))
            self.assertEqual(sorted(p.name for p in root.iterdir()),
                             ["decoy", "decoy dir [x]*", "fill-outside"])

            (parent / "segment.mcap").write_text("keep")
            self.run_script(faults.REMOVE_FILLS, parent)
            self.assertEqual([p.name for p in parent.iterdir()], ["segment.mcap"])
            self.assertEqual(decoy.read_text(), "keep")
            self.assertEqual(outside.read_text(), "keep")

    def test_no_fill_file_is_not_an_error(self):
        with tempfile.TemporaryDirectory(dir=SCRATCH, prefix="rv202-") as tmp:
            self.run_script(faults.REMOVE_FILLS, Path(tmp))

    def test_scripts_do_not_embed_a_path(self):
        for script in (faults.TAKE_INODES, faults.REMOVE_FILLS):
            self.assertIn('"$1', script)
            self.assertNotIn("{", script)


class FakeSoak:
    def __init__(self, samples):
        self._samples = samples

    def samples(self):
        return self._samples


def sample(t_ms, live=1000, since_sync_ms=None, worker_failed=False, control=0):
    return {"event": "sample", "t_ms": t_ms, "heap": {"live_bytes": live},
            "worker_failed": worker_failed, "counts": {"control": control, "frames": 0},
            "outputs": [{"since_sync_ms": since_sync_ms}]}


CLEAN_END = {"counts": {"misses": 0}, "failures": [], "success": True}


class Judgment(unittest.TestCase):
    """RV-203: a required failure can no longer pass."""

    def setUp(self):
        faults.args_global = type("A", (), {"heap_slack_mib": 16})()

    def common(self, samples, end, code, clean=True):
        return faults.common_checks(FakeSoak(samples), end, {}, code, clean)

    def test_clean_run_passes(self):
        checks = self.common([sample(i) for i in range(30)], CLEAN_END, 0)
        self.assertTrue(faults.judge("f1", checks), checks)

    def test_missing_heap_samples_fail(self):
        samples = [dict(sample(i), heap=None) for i in range(30)]
        checks = self.common(samples, CLEAN_END, 0)
        self.assertFalse(checks["heap_samples"])
        self.assertFalse(faults.judge("f1", checks))

    def test_producer_or_rt_heap_failure_fails(self):
        end = dict(CLEAN_END, failures=["RT threads used the heap under load: {}"])
        self.assertFalse(faults.judge("f1", self.common([sample(1)] * 30, end, 1)))

    def test_exit_code_must_match_expectation(self):
        samples = [sample(i) for i in range(30)]
        self.assertFalse(faults.judge("f2", self.common(samples, CLEAN_END, 1)))
        failed_end = dict(CLEAN_END, success=False)
        # F3: the output fault makes the shutdown unsuccessful, exit 1.
        self.assertTrue(faults.judge("f3a", self.common(samples, failed_end, 1, clean=False)))
        self.assertFalse(faults.judge("f3a", self.common(samples, failed_end, 0, clean=False)))

    def test_missing_end_or_failed_worker_fails(self):
        samples = [sample(i) for i in range(30)]
        self.assertFalse(faults.judge("f1", self.common(samples, None, 0)))
        bad = samples + [sample(31, worker_failed=True)]
        self.assertFalse(faults.judge("f1", self.common(bad, CLEAN_END, 0)))

    def test_required_key_absent_fails(self):
        checks = self.common([sample(i) for i in range(30)], CLEAN_END, 0)
        del checks["failures_none"]
        self.assertFalse(faults.judge("f1", checks))

    def test_f2a_gap_count_zero_fails(self):
        checks = self.common([sample(i) for i in range(30)], CLEAN_END, 0)
        self.assertNotIn("f2a_gap_notified", faults.OBSERVED)
        self.assertFalse(faults.judge("f2", dict(checks, f2a_gap_notified=False)))
        self.assertTrue(faults.judge("f2", dict(checks, f2a_gap_notified=True)))


class SyncCompletions(unittest.TestCase):
    """RV-204: sync times come from the soak's own clock, not stdout arrival."""

    def test_one_completion_seen_by_many_samples_counts_once(self):
        # A sync completed at t=1000 ms, seen by samples 200 ms apart.
        samples = [sample(1000 + d, since_sync_ms=d) for d in (5, 205, 405, 605)]
        self.assertEqual(faults.sync_completions(samples), [1.0])

    def test_distinct_completions(self):
        samples = [sample(1100, since_sync_ms=100), sample(2150, since_sync_ms=150),
                   sample(2350, since_sync_ms=350), sample(3020, since_sync_ms=20)]
        self.assertEqual(faults.sync_completions(samples), [1.0, 2.0, 3.0])

    def test_no_sync_yet(self):
        self.assertEqual(faults.sync_completions([sample(100), sample(300)]), [])


if __name__ == "__main__":
    unittest.main()
