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
import time
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


mixed = load("soak_mixed", "soak-mixed.py")


def fake_cli(directory, body):
    """A stand-in for the `dot-logger` CLI: an sh script with `body`."""
    path = Path(directory) / "fake-cli"
    path.write_text("#!/bin/sh\n" + body + "\n")
    path.chmod(0o755)
    return path


class FakeArgs:
    def __init__(self, cli):
        self.cli, self.port = cli, 1


class MixedWorkers(unittest.TestCase):
    """RV-205: a worker that dies or hangs can no longer pass."""

    def test_status_timeout_is_recorded_and_fails(self):
        with tempfile.TemporaryDirectory(dir=SCRATCH, prefix="rv205-") as tmp:
            # First call answers, every later call hangs past the timeout.
            cli = fake_cli(tmp, 'if [ -f "$0.done" ]; then sleep 5; '
                                'else touch "$0.done"; fi')
            status = mixed.Status(FakeArgs(cli), 0.05, timeout=0.3)
            status.start()
            deadline = time.monotonic() + 10
            while len(status.calls) < 3 and time.monotonic() < deadline:
                time.sleep(0.05)
            status.finish()
            self.assertGreaterEqual(len(status.calls), 3, status.calls)
            self.assertEqual(status.calls[0]["code"], 0)
            self.assertIn("timeout", status.calls[1]["error"])
            ok, _ = mixed.judge_status(status, expected=3)
            self.assertFalse(ok)

    def test_status_too_few_calls_fails(self):
        status = mixed.Status(FakeArgs("unused"), 60)
        status.calls = [{"code": 0, "ms": 1.0}]
        self.assertFalse(mixed.judge_status(status, expected=10)[0])
        status.calls = [{"code": 0, "ms": 1.0}] * 10
        self.assertTrue(mixed.judge_status(status, expected=10)[0])

    def test_finish_stops_a_quiet_subscriber(self):
        with tempfile.TemporaryDirectory(dir=SCRATCH, prefix="rv205-") as tmp:
            cli = fake_cli(tmp, "exec sleep 60")
            sub = mixed.Subscriber(FakeArgs(cli), Path(tmp), "control", [])
            sub.start()
            time.sleep(0.5)
            started = time.monotonic()
            sub.finish()
            self.assertLess(time.monotonic() - started, 5)
            self.assertFalse(sub.is_alive())
            self.assertFalse(sub.join_timed_out)

    def test_subscriber_error_is_recorded(self):
        with tempfile.TemporaryDirectory(dir=SCRATCH, prefix="rv205-") as tmp:
            # A record line without `data` breaks the reader loop.
            cli = fake_cli(tmp, 'echo \'{"kind":"record"}\'; exec sleep 60')
            sub = mixed.Subscriber(FakeArgs(cli), Path(tmp), "control", [])
            sub.start()
            deadline = time.monotonic() + 5
            while not sub.errors and time.monotonic() < deadline:
                time.sleep(0.05)
            sub.finish()
            self.assertTrue(sub.errors)


class KeptRecords(unittest.TestCase):
    """RV-206: both channels' suffix and live protection windows."""

    side = mixed.EVENT_SIDE_S * mixed.CONTROL_HZ

    def test_review_counterexample_fails(self):
        ok, _ = mixed.judge_kept({"control": [[4999, 4999]], "frames": []},
                                 {"control": 5000, "frames": 10},
                                 [{"ok": True, "control": 2500, "n": 1, "t_ms": 1}], [],
                                 hold_s=3600, seconds=10)
        self.assertFalse(ok)

    def test_frame_suffix_required(self):
        ok, note = mixed.judge_kept({"control": [[0, 4999]], "frames": [[0, 7]]},
                                    {"control": 5000, "frames": 10}, [], [],
                                    hold_s=60, seconds=10)
        self.assertFalse(ok)
        self.assertIn("frames", note)

    def test_active_window_must_be_whole(self):
        n = 1_000_000
        mark = {"ok": True, "control": 600_000, "n": 2, "t_ms": 120_000}
        runs = {"control": [[600_000 - self.side, 600_000 + 10], [700_000, n - 1]],
                "frames": [[0, 99]]}
        counts = {"control": n, "frames": 100}
        # Even marks are held: still protected at the end of a 200 s run.
        ok, note = mixed.judge_kept(runs, counts, [mark], [], hold_s=600, seconds=200)
        self.assertFalse(ok, note)
        runs["control"][0] = [600_000 - self.side, 600_000 + self.side]
        ok, note = mixed.judge_kept(runs, counts, [mark], [], hold_s=600, seconds=200)
        self.assertTrue(ok, note)
        # The same partial window is fine once the hold has expired.
        runs["control"][0] = [600_000 - self.side, 600_000 + 10]
        ok, note = mixed.judge_kept(runs, counts, [mark], [], hold_s=30, seconds=200)
        self.assertTrue(ok, note)

    def test_reader_coverage(self):
        good = {"passes": 10, "failed_passes": 0,
                "total": {"files": 30, "control": 10, "frames": 5, "loss_errors": 0}}
        self.assertTrue(mixed.judge_reader(0, good, expected=10)[0])
        for bad in ({**good, "passes": 3},
                    {**good, "total": {**good["total"], "files": 0}},
                    {**good, "total": {**good["total"], "control": 0}},
                    {**good, "total": {**good["total"], "loss_errors": 1}}):
            self.assertFalse(mixed.judge_reader(0, bad, expected=10)[0], bad)
        self.assertFalse(mixed.judge_reader(1, good, expected=10)[0])


def batch(start, end, at, outcome="Acknowledged", ms=5):
    return {"from": start, "to": end, "outcome": outcome, "at_unix_ms": at, "ms": ms}


class InfluxOutage(unittest.TestCase):
    """TASK-047 f6: DB outage judgment from the writer's batch log."""

    FAULT = (10_000, 20_000)  # unix ms: fault start, fault end

    def clean(self):
        return [batch(0, 10, 9_000), batch(10, 20, 9_100),
                batch(20, 30, 11_100, "Unknown", 4_200), batch(30, 40, 15_300, "Unknown", 4_200),
                {"from": 40, "to": 50, "error": "Busy", "at_unix_ms": 18_600, "ms": 0},
                batch(50, 60, 21_000), batch(60, 70, 21_100), {"end": True}]

    def window(self, batches):
        return faults.f6_window(batches, *self.FAULT, recover_s=10)

    def test_clean_outage_passes(self):
        checks = self.window(self.clean())
        self.assertTrue(all(v for k, v in checks.items() if isinstance(v, bool)), checks)
        self.assertEqual(checks["fault_outcomes"], {"Unknown": 2, "Busy": 1})

    def test_acknowledged_during_fault_fails(self):
        batches = self.clean()
        batches[3]["outcome"] = "Acknowledged"
        self.assertFalse(self.window(batches)["fault_no_ack"])

    def test_retry_answered_after_resume_is_not_a_fault_ack(self):
        # f6a 2026-11-27: a batch begun during the pause retried, and the
        # resumed DB answered it 45 ms after the fault ended.
        batches = self.clean()
        batches[4] = batch(40, 50, 17_700, "Acknowledged", 2_345)
        checks = self.window(batches)
        self.assertTrue(checks["fault_no_ack"], checks)
        self.assertEqual(checks["fault_outcomes"], {"Unknown": 2})
        self.assertEqual(checks["straddling_outcomes"], {"Acknowledged": 1})

    def test_no_recovery_fails(self):
        batches = [b for b in self.clean() if b.get("at_unix_ms", 0) < 20_000] + [{"end": True}]
        self.assertFalse(self.window(batches)["recovered"])

    def test_slow_recovery_fails(self):
        batches = self.clean()
        batches[5]["at_unix_ms"] = batches[6]["at_unix_ms"] = 31_000
        self.assertFalse(self.window(batches)["recovered"])

    def test_unknown_after_recovery_fails(self):
        batches = self.clean()
        batches[6]["outcome"] = "Unknown"
        self.assertFalse(self.window(batches)["post_all_acknowledged"])

    def test_worker_error_other_than_busy_fails(self):
        batches = self.clean()
        batches[4]["error"] = "Collision"
        self.assertFalse(self.window(batches)["worker_errors_0"])

    def test_nothing_before_fault_fails(self):
        batches = [b for b in self.clean() if b.get("at_unix_ms", 99_999) >= 10_000]
        self.assertFalse(self.window(batches)["pre_acknowledged"])

    def test_db_matches_reported_attempts(self):
        checks = faults.f6_db_checks(self.clean(), set(range(0, 20)) | {25, 26} | set(range(50, 70)))
        self.assertTrue(checks["db_acked_present"] and checks["db_only_attempted"], checks)
        self.assertEqual(checks["db_unknown_stored"], 2)

    def test_db_missing_acknowledged_point_fails(self):
        db = set(range(0, 20)) | set(range(50, 69))
        self.assertFalse(faults.f6_db_checks(self.clean(), db)["db_acked_present"])

    def test_db_point_from_skipped_or_failed_batch_fails(self):
        db = set(range(0, 20)) | set(range(50, 70)) | {45}
        self.assertFalse(faults.f6_db_checks(self.clean(), db)["db_only_attempted"])

    def test_flux_csv_values(self):
        csv = ("\r\n,result,table,_value\r\n,_result,0,3\r\n,_result,0,4\r\n\r\n"
               ",result,table,_value\r\n,_result,1,7\r\n")
        self.assertEqual(faults.flux_values(csv), {3, 4, 7})


if __name__ == "__main__":
    unittest.main()
