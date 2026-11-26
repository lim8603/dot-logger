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


if __name__ == "__main__":
    unittest.main()
