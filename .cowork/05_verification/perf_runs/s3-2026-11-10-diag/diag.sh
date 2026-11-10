#!/usr/bin/env bash
set -uo pipefail
B=/data/build/target/release/examples/perf_baseline
P=/home/lim8603/s3-diag; mkdir -p $P
O=$1; : > $O
cal=/home/lim8603/src/dot-logger/.cowork/05_verification/perf_runs/s3-2026-11-10/calibration.json
c=$(jq -r '.control.result.iterations' $cal); s=$(jq -r '.safety.result.iterations' $cal)
T=$((16<<30)); M=8388608
run() { local tag=$1; shift; echo "[$(date +%T)] $tag" >&2
  systemd-run --user --scope -q -p MemoryMax=8G $B "$@" | jq -c --arg tag "$tag" '. + {tag: $tag}' >> $O || echo "{\"tag\":\"$tag\",\"fail\":true}" >> $O; }
p0()  { run p3-p0 --p0-mix-max $P $T 256 100000 $M 120 4 16 8; }
lg()  { run p3-raw$1 --p1-mix-max $P $T 256 100000 $M 120 4 16 8 32 128 4096 $1 $2 0; }
p0; lg 8 2048; lg 35 2560; lg 35 2560; lg 8 2048; p0
run p2n-raw8  --p1-mix $P 20 5 256 0 $M 110 4 4 4 4 4 1024 8 1024 0 $c $s
run p2n-raw35 --p1-mix $P 20 5 256 0 $M 110 4 4 4 4 4 1024 35 1536 0 $c $s
