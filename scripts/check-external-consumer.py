#!/usr/bin/env python3
"""Build and run the API consumer as a project outside this workspace.

Workspace-only settings such as `[patch]` do not reach a downstream project,
so a crate that builds here can still fail for its users (TASK-036). The
consumer gets no lockfile: dependencies resolve the way a new user's would.
A git dependency resolves this repository the same way as the path used here.
"""

import argparse
import os
import pathlib
import shutil
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent

MANIFEST = """[package]
name = "dot-logger-external-consumer"
version = "0.0.0"
edition = "2024"
rust-version = "1.85"
publish = false

[features]
remote = ["dot-logger/remote"]
full = [
    "remote",
    "dot-logger/influxdb",
    "dot-logger/tracing-adapter",
    "dot-logger/calibrate",
    "dot-logger/compression-zstd",
    "dep:dot-logger-client",
]

[dependencies]
dot-logger = {{ path = "{core}", default-features = false }}
dot-logger-client = {{ path = "{client}", features = ["remote"], optional = true }}
"""


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--toolchain", default="1.96.0")
    parser.add_argument("--offline", action="store_true", help="resolve from the local registry cache")
    args = parser.parse_args()
    cargo = ["cargo", f"+{args.toolchain}"]
    if args.offline:
        cargo.append("--offline")
    with tempfile.TemporaryDirectory(prefix="dot-logger-consumer-") as temp:
        project = pathlib.Path(temp)
        (project / "src").mkdir()
        shutil.copy(ROOT / "tests/api-consumer/src/main.rs", project / "src/main.rs")
        (project / "Cargo.toml").write_text(MANIFEST.format(
            core=(ROOT / "crates/dot-logger").as_posix(),
            client=(ROOT / "crates/dot-logger-client").as_posix(),
        ))
        env = dict(os.environ)
        target = env.get("CARGO_TARGET_DIR")
        env["CARGO_TARGET_DIR"] = (
            str(pathlib.Path(target) / "external-consumer") if target else str(project / "target")
        )
        steps = [
            ["run"],
            ["run", "--features", "remote"],
            ["check", "--features", "full"],
        ]
        for step in steps:
            print(f"TC-001: {args.toolchain}: external consumer: {' '.join(step)}", flush=True)
            subprocess.run(cargo + step, cwd=project, env=env, check=True)
    print("TC-001: a project outside the workspace builds and runs against dot-logger", flush=True)


if __name__ == "__main__":
    main()
