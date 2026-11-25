#!/usr/bin/env python3
"""Run contracts across optional features and the isolated RT allocation guard.

`--shard K/N` runs every N-th combination starting at K, so slow runners can
split the matrix across parallel jobs without dropping any combination: the
protocol check runs in shard 0 and the consumer checks in the last shard.
"""

import argparse
import itertools
import subprocess
import sys

FEATURES = ("remote", "influxdb", "tracing-adapter", "compression-zstd")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--toolchain", default="1.96.0")
    parser.add_argument("--shard", default="0/1", help="K/N: run combinations K, K+N, ...")
    args = parser.parse_args()
    shard, shards = (int(n) for n in args.shard.split("/"))
    if not 0 <= shard < shards:
        raise SystemExit(f"invalid shard {args.shard}")
    cargo = ["cargo", f"+{args.toolchain}"]
    combinations = [
        [name for name, active in zip(FEATURES, enabled) if active]
        for enabled in itertools.product((False, True), repeat=len(FEATURES))
    ]
    # `calibrate` (TASK-028 S2) only adds a module and `Sizing::profile`; it is
    # independent of the others, so check it alone and with all of them.
    combinations += [["calibrate"], [*FEATURES, "calibrate"]]
    if shard == 0:
        subprocess.run([sys.executable, "scripts/check-remote-protocol.py"], check=True)
    for features in combinations[shard::shards]:
        print(f"TC-001: {args.toolchain}: {','.join(features) or 'default/core'}", flush=True)
        command = cargo + ["test", "-p", "dot-logger", "--locked", "--no-default-features"]
        if features:
            command += ["--features", ",".join(features)]
        subprocess.run(command, check=True)
    if shard != shards - 1:
        print(f"TC-001: shard {args.shard}: {len(combinations[shard::shards])} of "
              f"{len(combinations)} combinations passed", flush=True)
        return

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
    print(f"TC-001: shard {args.shard}: {len(combinations[shard::shards])} of {len(combinations)} "
          "combinations (16 feature combinations, calibrate alone and with all) and the "
          "independent core consumer passed", flush=True)


if __name__ == "__main__":
    main()
