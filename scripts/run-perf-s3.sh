#!/usr/bin/env bash
# TASK-023 S3: formal local-pc-v1 runs of P0~P3, control off/on and the
# instrumentation cost, interleaved over repetitions. One JSON line per fact
# goes to <out>/runs.jsonl; each run's data directory under <parent> is
# removed when that run ends. P4/P5 are out of S3 (Human 2026-11-08).
#
# usage: scripts/run-perf-s3.sh <parent> <out> [seconds] [warmup_s] [reps]
#   parent   exclusive scratch parent on the measured file system
#   out      result directory (created; must not exist)
#   seconds  measured window per run (default 120)
#   warmup_s warm-up left out of the results (default 10)
#   reps     repetitions per condition (default 3)
set -euo pipefail

parent=${1:?parent}
out=${2:?out}
seconds=${3:-120}
warmup=${4:-10}
reps=${5:-3}
bin=${PERF_BASELINE:-/data/build/target/release/examples/perf_baseline}

mkdir "$out"
log="$out/runs.jsonl"
{
  printf '{"scope":"S3Run","started":"%s","git":"%s","dirty":%s,"seconds":%s,"warmup_s":%s,"reps":%s,"kernel":"%s","rustc":"%s","mem_total_kb":%s}\n' \
    "$(date -Iseconds)" "$(git rev-parse HEAD)" \
    "$([ -z "$(git status --porcelain -- crates)" ] && echo false || echo true)" \
    "$seconds" "$warmup" "$reps" "$(uname -r)" "$(rustc --version)" \
    "$(awk '/MemTotal/{print $2}' /proc/meminfo)"
} >>"$log"

# Each fact line gets the condition label and repetition.
run() {
  local label=$1 rep=$2
  shift 2
  echo "[$(date +%T)] rep $rep $label" >&2
  if ! "$bin" "$@" | jq -c --arg label "$label" --argjson rep "$rep" '. + {label: $label, rep: $rep}' >>"$log"; then
    printf '{"scope":"S3Failure","label":"%s","rep":%s}\n' "$label" "$rep" >>"$log"
  fi
}

"$bin" --plan >>"$log"
"$bin" --calibrate | tee -a "$log" >"$out/calibration.json"
control=$(jq -r '.control.result.iterations' "$out/calibration.json")
safety=$(jq -r '.safety.result.iterations' "$out/calibration.json")
loops=("$control" "$safety")

small_rate=$((256 * 100000))
for rep in $(seq 1 "$reps"); do
  run control-off "$rep" --control "$seconds" "${loops[@]}"
  # P1: 256 B × 100k/s; (producers, files) representative set.
  for shape in "1 1 1 8" "8 4 4 32" "32 16 8 128"; do
    read -r producers files writers channels <<<"$shape"
    run "p1-p0-${producers}x${files}" "$rep" --p0 "$parent" "$seconds" "$warmup" 256 "$small_rate" \
      "$files" "$writers" "${loops[@]}"
    run "p1-logger-${producers}x${files}" "$rep" --p1 "$parent" "$seconds" "$warmup" 256 "$small_rate" \
      "$files" "$writers" "$producers" "$channels" 4096 2048 0 "${loops[@]}"
  done
  # P2: 4 originals × 8 MiB × 30/s.
  run p2-p0 "$rep" --p0-mix "$parent" "$seconds" "$warmup" 256 0 8388608 120 4 4 4 "${loops[@]}"
  run p2-logger "$rep" --p1-mix "$parent" "$seconds" "$warmup" 256 0 8388608 120 4 4 4 \
    4 4 1024 8 1024 0 "${loops[@]}"
  # P3: P2 + 256 B × 100k/s, 32 producers, 16 files, 128 channels.
  run p3-p0 "$rep" --p0-mix "$parent" "$seconds" "$warmup" 256 100000 8388608 120 4 16 8 "${loops[@]}"
  run p3-logger "$rep" --p1-mix "$parent" "$seconds" "$warmup" 256 100000 8388608 120 4 16 8 \
    32 128 4096 8 2048 0 "${loops[@]}"
  # Instrumentation cost: the same P3 run with telemetry on.
  run p3-logger-telemetry "$rep" --p1-mix "$parent" "$seconds" "$warmup" 256 100000 8388608 120 4 16 8 \
    32 128 4096 8 2048 1 "${loops[@]}"
  # T4 input: try_log split at 256 B.
  run p1-cost "$rep" --p1-cost "$parent" 256 1000000
done
printf '{"scope":"S3Done","finished":"%s"}\n' "$(date -Iseconds)" >>"$log"
