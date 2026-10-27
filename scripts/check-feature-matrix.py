#!/usr/bin/env python3
"""Run contracts across optional features and the isolated RT allocation guard."""

import argparse
import itertools
import json
import subprocess

FEATURES = ("remote", "influxdb", "tracing-adapter", "compression-zstd")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--toolchain", default="1.96.0")
    args = parser.parse_args()
    cargo = ["cargo", f"+{args.toolchain}"]
    for enabled in itertools.product((False, True), repeat=len(FEATURES)):
        features = [name for name, active in zip(FEATURES, enabled) if active]
        print(f"TC-001: {args.toolchain}: {','.join(features) or 'default/core'}", flush=True)
        command = cargo + ["test", "-p", "dot-logger", "--locked", "--no-default-features"]
        if features:
            command += ["--features", ",".join(features)]
        subprocess.run(command, check=True)

    # Resolve the consumer alone: optional network/runtime and compression crates
    # must not become mandatory dependencies through feature unification.
    metadata = json.loads(subprocess.check_output(
        cargo + ["metadata", "--locked", "--format-version", "1", "--no-default-features"],
        text=True,
    ))
    packages = {package["id"]: package["name"] for package in metadata["packages"]}
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    consumer = next(key for key, name in packages.items() if name == "dot-logger-api-consumer")
    reached, pending = set(), [consumer]
    while pending:
        package = pending.pop()
        if package not in reached:
            reached.add(package)
            pending.extend(nodes[package]["dependencies"])
    forbidden = {"tokio", "tonic", "reqwest", "hyper", "zstd", "zstd-sys", "tracing"}
    unexpected = forbidden.intersection(packages[key] for key in reached)
    if unexpected:
        raise SystemExit(f"core consumer unexpectedly requires: {sorted(unexpected)}")
    subprocess.run(cargo + ["run", "--locked", "-p", "dot-logger-api-consumer"], check=True)
    subprocess.run(cargo + ["test", "--locked", "-p", "dot-logger-rt-allocation"], check=True)
    subprocess.run(cargo + ["run", "--locked", "-p", "dot-logger", "--example", "scoped_capture"], check=True)
    print("TC-001: all 16 feature combinations and independent core consumer passed", flush=True)


if __name__ == "__main__":
    main()
