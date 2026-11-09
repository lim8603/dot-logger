#!/usr/bin/env bash
# TASK-023 S3 under the long-run rule (Human 2026-11-08, requirement §12.4):
#   1. S  = device write rate after its SLC cache ran out (P0 pattern).
#   2. Normal load: P2/P3 offered at min(1 GB/s, 0.6 × S) for
#      max(seconds, 2 × RAM worth) after the warm-up; P1 at 100k × 256 B/s.
#      Lossless, no extra control miss, periodic syncs on time.
#   3. Efficiency: unpaced P2/P3 of 2 × RAM payload, logger ≥ 0.8 × P0; an
#      overdue periodic sync is reported, not a failure. Instrumentation cost
#      = unpaced P3 logger with telemetry against without.
# P0 and logger alternate order per repetition. One JSON line per fact goes
# to <out>/runs.jsonl; each run's data under <parent> is removed when it ends.
# P4/P5 are out of S3 (Human 2026-11-08).
#
# usage: scripts/run-perf-s3.sh <parent> <out> [seconds] [warmup_s] [reps]
set -euo pipefail

parent=${1:?parent}
out=${2:?out}
seconds=${3:-120}
warmup=${4:-10}
reps=${5:-3}
bin=${PERF_BASELINE:-/data/build/target/release/examples/perf_baseline}
sustained_cap=${SUSTAINED_CAP:-$((600 << 30))}

mkdir "$out"
log="$out/runs.jsonl"
# S3_RAM_BYTES / S3_DRY only shorten a dry run; a real run uses neither.
ram=${S3_RAM_BYTES:-$(($(awk '/MemTotal/{print $2}' /proc/meminfo) * 1024))}
# The device that holds <parent>, not a fixed name: partition -> its disk.
part=$(basename "$(findmnt -no SOURCE --target "$parent")")
if [ -e "/sys/block/$part" ]; then disk=$part; else disk=$(basename "$(readlink -f "/sys/class/block/$part/..")"); fi
printf '{"scope":"S3Run","started":"%s","git":"%s","dirty":%s,"dry":%s,"seconds":%s,"warmup_s":%s,"reps":%s,"kernel":"%s","rustc":"%s","ram_bytes":%s,"device":"%s"}\n' \
  "$(date -Iseconds)" "$(git rev-parse HEAD)" \
  "$([ -z "$(git status --porcelain -- crates scripts)" ] && echo false || echo true)" \
  "$([ -n "${S3_DRY:-}" ] && echo true || echo false)" \
  "$seconds" "$warmup" "$reps" "$(uname -r)" "$(rustc --version)" "$ram" \
  "$disk $(cat "/sys/block/$disk/device/model" 2>/dev/null | xargs)" >>"$log"

run() {
  local label=$1 rep=$2
  shift 2
  echo "[$(date +%T)] rep $rep $label" >&2
  if ! "$bin" "$@" | jq -c --arg label "$label" --argjson rep "$rep" '. + {label: $label, rep: $rep}' >>"$log"; then
    printf '{"scope":"S3Failure","label":"%s","rep":%s}\n' "$label" "$rep" >>"$log"
  fi
}
# Run a P0/logger pair in the repetition's order.
pair() {
  local rep=$1 p0_label=$2 logger_label=$3
  shift 3
  local -a p0_args logger_args
  local split=0
  for a in "$@"; do
    if [ "$a" = "--" ]; then split=1; continue; fi
    if [ $split = 0 ]; then p0_args+=("$a"); else logger_args+=("$a"); fi
  done
  if [ $((rep % 2)) = 1 ]; then
    run "$p0_label" "$rep" "${p0_args[@]}"
    run "$logger_label" "$rep" "${logger_args[@]}"
  else
    run "$logger_label" "$rep" "${logger_args[@]}"
    run "$p0_label" "$rep" "${p0_args[@]}"
  fi
}

"$bin" --plan >>"$log"
"$bin" --calibrate | tee -a "$log" >"$out/calibration.json"
control=$(jq -r '.control.result.iterations' "$out/calibration.json")
safety=$(jq -r '.safety.result.iterations' "$out/calibration.json")
loops=("$control" "$safety")

# 1. Long-run device rate.
echo "[$(date +%T)] sustained" >&2
"$bin" --sustained "$parent" "$sustained_cap" 4 4 120 | jq -c '. + {label: "sustained", rep: 0}' | tee -a "$log" >"$out/sustained.json"
if [ "$(jq -r '.stable' "$out/sustained.json")" != true ] && [ -z "${S3_DRY:-}" ]; then
  echo "no SLC cache drop within the cap; S is not a long-run rate" >&2
  printf '{"scope":"S3Failure","label":"sustained","rep":0}\n' >>"$log"
  exit 1
fi
s=$(jq -r '.bytes_per_second' "$out/sustained.json")
mib8=8388608
rate=$(python3 -c "print(min(10**9, int(0.6 * $s)))")
large=$(python3 -c "print(max(1, $rate // $mib8))")
heavy=$(python3 -c "import math; print(max($seconds, math.ceil(2 * $ram / ($large * $mib8))))")
total=$((2 * ram))
printf '{"scope":"S3Plan","sustained_bytes_per_second":%s,"normal_bytes_per_second":%s,"large_per_second":%s,"heavy_seconds":%s,"max_total_bytes":%s}\n' \
  "$s" "$rate" "$large" "$heavy" "$total" >>"$log"

small_rate=$((256 * 100000))
for rep in $(seq 1 "$reps"); do
  run control-off "$rep" --control "$seconds" "${loops[@]}"
  # P1 normal load: 256 B × 100k/s; (producers, files) representative set.
  for shape in "1 1 1 8" "8 4 4 32" "32 16 8 128"; do
    read -r producers files writers channels <<<"$shape"
    pair "$rep" "p1-${producers}x${files}-p0" "p1-${producers}x${files}-logger" \
      --p0 "$parent" "$seconds" "$warmup" 256 "$small_rate" "$files" "$writers" "${loops[@]}" -- \
      --p1 "$parent" "$seconds" "$warmup" 256 "$small_rate" "$files" "$writers" \
      "$producers" "$channels" 4096 2048 0 "${loops[@]}"
  done
  # P2/P3 normal load at 0.6 × S.
  pair "$rep" p2-normal-p0 p2-normal-logger \
    --p0-mix "$parent" "$heavy" "$warmup" 256 0 "$mib8" "$large" 4 4 4 "${loops[@]}" -- \
    --p1-mix "$parent" "$heavy" "$warmup" 256 0 "$mib8" "$large" 4 4 4 4 4 1024 8 1024 0 "${loops[@]}"
  pair "$rep" p3-normal-p0 p3-normal-logger \
    --p0-mix "$parent" "$heavy" "$warmup" 256 100000 "$mib8" "$large" 4 16 8 "${loops[@]}" -- \
    --p1-mix "$parent" "$heavy" "$warmup" 256 100000 "$mib8" "$large" 4 16 8 \
    32 128 4096 8 2048 0 "${loops[@]}"
  # Efficiency: unpaced, 2 × RAM payload.
  pair "$rep" p2-max-p0 p2-max-logger \
    --p0-mix-max "$parent" "$total" 256 0 "$mib8" 120 4 4 4 -- \
    --p1-mix-max "$parent" "$total" 256 0 "$mib8" 120 4 4 4 4 4 1024 8 1024 0
  pair "$rep" p3-max-p0 p3-max-logger \
    --p0-mix-max "$parent" "$total" 256 100000 "$mib8" 120 4 16 8 -- \
    --p1-mix-max "$parent" "$total" 256 100000 "$mib8" 120 4 16 8 32 128 4096 8 2048 0
  run p3-max-logger-telemetry "$rep" --p1-mix-max "$parent" "$total" 256 100000 "$mib8" 120 4 16 8 \
    32 128 4096 8 2048 1
  # T4 input: try_log split at 256 B.
  run p1-cost "$rep" --p1-cost "$parent" 256 1000000
done
printf '{"scope":"S3Done","finished":"%s"}\n' "$(date -Iseconds)" >>"$log"
