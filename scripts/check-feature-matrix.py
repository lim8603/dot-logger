#!/usr/bin/env python3
"""Run contracts across optional features and the isolated RT allocation guard."""

import argparse
import itertools
import subprocess
import sys

FEATURES = ("remote", "influxdb", "tracing-adapter", "compression-zstd")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--toolchain", default="1.96.0")
    args = parser.parse_args()
    cargo = ["cargo", f"+{args.toolchain}"]
    subprocess.run([sys.executable, "scripts/check-remote-protocol.py"], check=True)
    for enabled in itertools.product((False, True), repeat=len(FEATURES)):
        features = [name for name, active in zip(FEATURES, enabled) if active]
        print(f"TC-001: {args.toolchain}: {','.join(features) or 'default/core'}", flush=True)
        command = cargo + ["test", "-p", "dot-logger", "--locked", "--no-default-features"]
        if features:
            command += ["--features", ",".join(features)]
        subprocess.run(command, check=True)

    # Resolve the consumer alone: optional network/runtime and compression crates
    # must not become mandatory dependencies through feature unification.
    forbidden = {"tokio", "tonic", "reqwest", "hyper", "zstd", "zstd-sys", "tracing", "subtle"}
    # metadata resolves all workspace members together, including the intentional
    # remote CLI. Inspect each selected package graph to test isolation accurately.
    for consumer in ["dot-logger-api-consumer", "dot-logger-client"]:
        tree = subprocess.check_output(cargo + [
            "tree", "--locked", "-p", consumer, "--no-default-features",
            "--edges", "normal,build", "--prefix", "none", "--format", "{p}"
        ], text=True)
        reached = {line.split()[0] for line in tree.splitlines() if line.strip()}
        unexpected = forbidden.intersection(reached)
        if unexpected:
            raise SystemExit(f"{consumer} unexpectedly requires: {sorted(unexpected)}")
    subprocess.run(cargo + ["run", "--locked", "-p", "dot-logger-api-consumer"], check=True)
    subprocess.run(cargo + ["test", "--locked", "-p", "dot-logger-rt-allocation"], check=True)
    subprocess.run(cargo + ["run", "--locked", "-p", "dot-logger", "--example", "scoped_capture"], check=True)
    subprocess.run(cargo + ["run", "--locked", "-p", "dot-logger-api-consumer", "--features", "remote"], check=True)
    subprocess.run(cargo + ["test", "--locked", "-p", "dot-logger-rt-allocation", "--features", "remote"], check=True)
    print("TC-001: all 16 feature combinations and independent core consumer passed", flush=True)


if __name__ == "__main__":
    main()
