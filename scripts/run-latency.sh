#!/usr/bin/env bash
# TASK-042 S2: `try_log` latency rows (H, C1, C2, R, J, M) on whichever
# kernel and CPU governor are active, so runs under RT/generic and
# powersave/performance compare directly. RT_EXPECT and GOVERNOR_EXPECT
# must match the machine, or nothing runs. The measuring threads are
# SCHED_FIFO 70 with all memory locked (DOT_LOGGER_PERF_RT=fifo).
#
# Rows write only to a tmpfs ring under <tmpfs_parent> (checked by the
# harness). J's IO load writes 64 KiB records 100 times a second to
# <load_parent> (an SSD): about 390 MB per 60 s IO row.
# One JSON line per fact goes to <out>/runs.jsonl.
#
# usage: RT_EXPECT=realtime|generic GOVERNOR_EXPECT=powersave|performance \
#   scripts/run-latency.sh <tmpfs_parent> <load_parent> <out> [reps] [jitter_s]
set -euo pipefail

tmpfs=${1:?tmpfs_parent}
load=${2:?load_parent}
out=${3:?out}
reps=${4:-3}
jitter_s=${5:-60}
repo=$(cd "$(dirname "$0")/.." && pwd)
bin=${PERF_BASELINE:-${CARGO_TARGET_DIR:-$repo/target}/release/examples/perf_baseline}
export DOT_LOGGER_PERF_RT=fifo

expect=${RT_EXPECT:?RT_EXPECT=realtime or RT_EXPECT=generic}
config="/boot/config-$(uname -r)"
is_rt=$([ "$(cat /sys/kernel/realtime 2>/dev/null)" = 1 ] && grep -q '^CONFIG_PREEMPT_RT=y' "$config" && echo realtime || echo generic)
if [ "$is_rt" != "$expect" ]; then
  echo "running kernel $(uname -r) is $is_rt, expected $expect" >&2
  exit 1
fi
governor=$(cat /sys/devices/system/cpu/cpu2/cpufreq/scaling_governor)
if [ "$governor" != "${GOVERNOR_EXPECT:?GOVERNOR_EXPECT=powersave or performance}" ]; then
  echo "CPU 2 governor is $governor, expected $GOVERNOR_EXPECT" >&2
  exit 1
fi
if [ "$(ulimit -r)" -lt 70 ] || [ "$(ulimit -l)" != unlimited ]; then
  echo "rtprio $(ulimit -r) / memlock $(ulimit -l): FIFO 70 and mlockall need rtprio >= 70, memlock unlimited" >&2
  exit 1
fi
[ "$(stat -f -c %T "$tmpfs")" = tmpfs ] || { echo "$tmpfs is not tmpfs" >&2; exit 1; }
[ -d "$load" ] || { echo "$load is not a directory" >&2; exit 1; }

mkdir "$out"
log="$out/runs.jsonl"
jq -cn --arg started "$(date -Iseconds)" --arg git "$(git -C "$repo" rev-parse HEAD)" \
  --argjson dirty "$([ -z "$(git -C "$repo" status --porcelain -- crates scripts)" ] && echo false || echo true)" \
  --arg kernel "$(uname -r)" --arg cmdline "$(cat /proc/cmdline)" --arg expect "$expect" \
  --arg governors "$(cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor | sort | uniq -c | xargs)" \
  --arg epp "$(cat /sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference 2>/dev/null | sort | uniq -c | xargs)" \
  --arg boost "$(cat /sys/devices/system/cpu/cpufreq/boost 2>/dev/null || echo unknown)" \
  --arg smt "$(cat /sys/devices/system/cpu/smt/control 2>/dev/null || echo unknown)" \
  --arg cpu "$(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | xargs)" \
  --arg rustc "$(rustc --version)" --argjson reps "$reps" --argjson jitter_s "$jitter_s" \
  '{scope:"LatencyRun",started:$started,git:$git,dirty:$dirty,kernel:$kernel,expect:$expect,
    cmdline:$cmdline,governors:$governors,epp:$epp,boost:$boost,smt:$smt,cpu:$cpu,rustc:$rustc,
    reps:$reps,jitter_s:$jitter_s}' >>"$log"

row() {
  echo "$(date +%T) $*" >&2
  "$bin" --latency "$tmpfs" "$@" >>"$log"
}
for rep in $(seq 1 "$reps"); do
  echo "rep $rep/$reps" >&2
  for size in 16 64 256; do row warm "$size" 10000000; done
  row cold 256 20000
  row sparse 256 1000 60 262144
  row reject 256 1000000
  for writers in 1 4 8; do row multi 256 1000000 "$writers"; done
  # local-pc-v1 control loop (10 ms, 500 µs work) and a 1 kHz loop (50 µs).
  for calls in 0 50; do row jitter 256 10000 "$jitter_s" 500 "$calls"; done
  row jitter 256 10000 "$jitter_s" 500 50 "$load" 65536 100
  for calls in 0 50; do row jitter 256 1000 "$jitter_s" 50 "$calls"; done
  row jitter 256 1000 "$jitter_s" 50 50 "$load" 65536 100
done
echo "$(date +%T) done: $log" >&2
