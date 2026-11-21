#!/usr/bin/env bash
# TASK-023 RT rows under local-pc-v1 (TASK-022 RT request, 2026-11-11).
# Runs the same rows on whichever kernel is booted, so a generic run and a
# PREEMPT_RT run of this script compare directly. The loops run as
# SCHED_FIFO control 70 / safety 80 with all memory locked
# (DOT_LOGGER_PERF_RT=fifo); the applied policy is read back per run.
#   1. Plan, calibration (FIFO, logger off), kernel facts.
#   2. cyclictest on the control/safety CPUs, idle (diagnostic only).
#   3. Control off, `reps` times.
#   4. Control on: P1 32 producers × 16 files (256 B × 100k/s) and P3 normal
#      load (min(1 GB/s, 0.6 × S) of 8 MiB + 100k/s × 256 B), logger only,
#      `seconds` after a `warmup_s` warm-up, once each.
# S defaults to the S3 sustained rate (EV-064). Raw slots follow
# `Preset::sensor` like run-perf-s3.sh. Each run executes in its own cgroup
# with memory.max = 8 GiB; each run's data under <parent> is removed when it
# ends. One JSON line per fact goes to <out>/runs.jsonl.
#
# usage: RT_EXPECT=realtime|generic scripts/run-perf-rt.sh <parent> <out> [seconds] [warmup_s] [reps]
set -euo pipefail

parent=${1:?parent}
out=${2:?out}
seconds=${3:-60}
warmup=${4:-10}
reps=${5:-3}
# Default: the built example under this repository's target directory
# (CARGO_TARGET_DIR when set). Override with PERF_BASELINE (REV-071 RV-183).
repo=$(cd "$(dirname "$0")/.." && pwd)
bin=${PERF_BASELINE:-${CARGO_TARGET_DIR:-$repo/target}/release/examples/perf_baseline}
s=${S_BYTES:-1543573777}
mem=${RT_MEM:-8G}
export DOT_LOGGER_PERF_RT=fifo
# The kernel this run is meant for; a mismatch stops before any measurement.
expect=${RT_EXPECT:?RT_EXPECT=realtime or RT_EXPECT=generic}
config="/boot/config-$(uname -r)"
is_rt=$([ "$(cat /sys/kernel/realtime 2>/dev/null)" = 1 ] && grep -q '^CONFIG_PREEMPT_RT=y' "$config" && echo realtime || echo generic)
if [ "$is_rt" != "$expect" ]; then
  echo "running kernel $(uname -r) is $is_rt, expected $expect" >&2
  exit 1
fi
if [ "$(ulimit -r)" -lt 80 ] || [ "$(ulimit -l)" != unlimited ]; then
  echo "rtprio $(ulimit -r) / memlock $(ulimit -l): FIFO 80 and mlockall need rtprio >= 80, memlock unlimited" >&2
  exit 1
fi

mkdir "$out"
log="$out/runs.jsonl"
part=$(basename "$(findmnt -no SOURCE --target "$parent")")
if [ -e "/sys/block/$part" ]; then disk=$part; else disk=$(basename "$(readlink -f "/sys/class/block/$part/..")"); fi
jq -cn --arg started "$(date -Iseconds)" --arg git "$(git rev-parse HEAD)" \
  --argjson dirty "$([ -z "$(git status --porcelain -- crates scripts)" ] && echo false || echo true)" \
  --arg kernel "$(uname -r)" --arg cmdline "$(cat /proc/cmdline)" \
  --arg realtime "$(cat /sys/kernel/realtime 2>/dev/null || echo absent)" \
  --arg preempt_rt "$(grep -E '^CONFIG_PREEMPT_RT=' "$config" 2>/dev/null || echo unset)" \
  --arg rtprio "$(ulimit -r)" --arg memlock "$(ulimit -l)" \
  --arg rustc "$(rustc --version)" --arg device "$disk $(cat "/sys/block/$disk/device/model" 2>/dev/null | xargs)" \
  --argjson seconds "$seconds" --argjson warmup "$warmup" --argjson reps "$reps" --argjson s "$s" \
  --arg expect "$expect" \
  '{scope:"RTRun",expect:$expect,started:$started,git:$git,dirty:$dirty,kernel:$kernel,cmdline:$cmdline,
    realtime:$realtime,preempt_rt:$preempt_rt,rtprio_limit:$rtprio,memlock_limit:$memlock,
    rustc:$rustc,device:$device,seconds:$seconds,warmup_s:$warmup,reps:$reps,
    sustained_bytes_per_second:$s,policy:"SCHED_FIFO control 70 / safety 80, mlockall"}' >>"$log"

pb() {
  systemd-run --user --scope -q -p MemoryMax="$mem" "$bin" "$@"
}
run() {
  local label=$1 rep=$2
  shift 2
  echo "[$(date +%T)] rep $rep $label" >&2
  if ! pb "$@" | jq -c --arg label "$label" --argjson rep "$rep" '. + {label: $label, rep: $rep}' >>"$log"; then
    printf '{"scope":"RTFailure","label":"%s","rep":%s}\n' "$label" "$rep" >>"$log"
  fi
}

pb --plan >>"$log"
pb --calibrate | tee -a "$log" >"$out/calibration.json"
control=$(jq -r '.control.result.iterations' "$out/calibration.json")
safety=$(jq -r '.safety.result.iterations' "$out/calibration.json")
loops=("$control" "$safety")

# Diagnostic only: scheduler latency on the loop CPUs with nothing else running.
echo "[$(date +%T)] cyclictest" >&2
cyclictest -m -q -t 2 -a 2,3 -p 80 -i 1000 -d 0 -D "$seconds" >"$out/cyclictest.txt" 2>&1 || true

mib8=8388608
rate=$(python3 -c "print(min(10**9, int(0.6 * $s)))")
large=$(python3 -c "print(max(1, $rate // $mib8))")
raw=$(python3 -c "import math; print(math.ceil(math.ceil($large / 4) * 5 / 4))")
p3_mem=$((2048 + 4 * (raw - 8) * 8))
printf '{"scope":"RTPlan","normal_bytes_per_second":%s,"large_per_second":%s,"raw_slots":%s}\n' \
  "$rate" "$large" "$raw" >>"$log"

for rep in $(seq 1 "$reps"); do
  run control-off "$rep" --control "$seconds" "${loops[@]}"
done
run p1-32x16-logger 1 --p1 "$parent" "$seconds" "$warmup" 256 $((256 * 100000)) 16 8 \
  32 128 4096 2048 0 "${loops[@]}"
run p3-normal-logger 1 --p1-mix "$parent" "$seconds" "$warmup" 256 100000 "$mib8" "$large" 4 16 8 \
  32 128 4096 "$raw" "$p3_mem" 0 "${loops[@]}"
echo "[$(date +%T)] done" >&2
