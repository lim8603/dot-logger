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
# Evidence (REV-073 RV-208): an undeterminable kernel, a CPU on another
# governor, a failed or empty row and a missing row all stop the run with a
# non-zero exit. The runner builds perf_baseline itself from this checkout
# (PERF_BASELINE=<path> uses a prebuilt one, recorded as such) and records
# the binary's sha256 and whether crates, scripts or the lockfile differ
# from HEAD.
#
# usage: RT_EXPECT=realtime|generic GOVERNOR_EXPECT=powersave|performance \
#   scripts/run-latency.sh <tmpfs_parent> <load_parent> <out> [reps] [jitter_s]
set -euo pipefail

tmpfs=${1:?tmpfs_parent}
load=${2:?load_parent}
out=${3:?out}
reps=${4:-3}
jitter_s=${5:-60}
positive='^[1-9][0-9]*$'
[[ $reps =~ $positive ]] || { echo "reps must be a positive integer, got '$reps'" >&2; exit 1; }
[[ $jitter_s =~ $positive ]] || { echo "jitter_s must be a positive integer, got '$jitter_s'" >&2; exit 1; }
repo=$(cd "$(dirname "$0")/.." && pwd)
export DOT_LOGGER_PERF_RT=fifo

expect=${RT_EXPECT:?RT_EXPECT=realtime or RT_EXPECT=generic}
# realtime: /sys/kernel/realtime is 1 and the config has PREEMPT_RT.
# generic: no /sys/kernel/realtime (or 0) and a readable config without it.
# Anything else (unreadable config, the two disagreeing) is unknown: stop.
config="/boot/config-$(uname -r)"
sys_rt=$(cat /sys/kernel/realtime 2>/dev/null || echo absent)
if [ ! -r "$config" ]; then
  is_rt=unknown
elif grep -q '^CONFIG_PREEMPT_RT=y' "$config"; then
  [ "$sys_rt" = 1 ] && is_rt=realtime || is_rt=unknown
else
  case $sys_rt in absent|0) is_rt=generic ;; *) is_rt=unknown ;; esac
fi
if [ "$is_rt" != "$expect" ]; then
  echo "running kernel $(uname -r) is $is_rt (config readable: $([ -r "$config" ] && echo yes || echo no), /sys/kernel/realtime: $sys_rt), expected $expect" >&2
  exit 1
fi
# Rows pin to CPU 2 and, for M, to further CPUs: every CPU must match.
want=${GOVERNOR_EXPECT:?GOVERNOR_EXPECT=powersave or performance}
for g in /sys/devices/system/cpu/cpu[0-9]*/cpufreq/scaling_governor; do
  if [ "$(cat "$g")" != "$want" ]; then
    echo "$g is $(cat "$g"), expected $want on every CPU" >&2
    exit 1
  fi
done
if [ "$(ulimit -r)" -lt 70 ] || [ "$(ulimit -l)" != unlimited ]; then
  echo "rtprio $(ulimit -r) / memlock $(ulimit -l): FIFO 70 and mlockall need rtprio >= 70, memlock unlimited" >&2
  exit 1
fi
[ "$(stat -f -c %T "$tmpfs")" = tmpfs ] || { echo "$tmpfs is not tmpfs" >&2; exit 1; }
[ -d "$load" ] || { echo "$load is not a directory" >&2; exit 1; }

if [ -n "${PERF_BASELINE:-}" ]; then
  bin=$PERF_BASELINE
  built=prebuilt
else
  (cd "$repo" && cargo build --release --locked -p dot-logger --example perf_baseline >&2)
  bin=${CARGO_TARGET_DIR:-$repo/target}/release/examples/perf_baseline
  built=runner
fi
[ -x "$bin" ] || { echo "$bin is not an executable" >&2; exit 1; }

mkdir "$out"
log="$out/runs.jsonl"
jq -cn --arg started "$(date -Iseconds)" --arg git "$(git -C "$repo" rev-parse HEAD)" \
  --argjson dirty "$([ -z "$(git -C "$repo" status --porcelain -- crates scripts Cargo.lock Cargo.toml)" ] && echo false || echo true)" \
  --arg bin "$bin" --arg bin_sha256 "$(sha256sum "$bin" | cut -d' ' -f1)" --arg built "$built" \
  --arg kernel "$(uname -r)" --arg cmdline "$(cat /proc/cmdline)" --arg expect "$expect" \
  --arg governors "$(cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor | sort | uniq -c | xargs)" \
  --arg epp "$(cat /sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference 2>/dev/null | sort | uniq -c | xargs)" \
  --arg boost "$(cat /sys/devices/system/cpu/cpufreq/boost 2>/dev/null || echo unknown)" \
  --arg smt "$(cat /sys/devices/system/cpu/smt/control 2>/dev/null || echo unknown)" \
  --arg cpu "$(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | xargs)" \
  --arg rustc "$(rustc --version)" --argjson reps "$reps" --argjson jitter_s "$jitter_s" \
  '{scope:"LatencyRun",started:$started,git:$git,dirty:$dirty,kernel:$kernel,expect:$expect,
    cmdline:$cmdline,governors:$governors,epp:$epp,boost:$boost,smt:$smt,cpu:$cpu,rustc:$rustc,
    bin:$bin,bin_sha256:$bin_sha256,built:$built,reps:$reps,jitter_s:$jitter_s}' >>"$log"

# A row passes when every line it printed reports success and some
# histogram holds samples.
ROW_OK='length > 0 and all(.[]; .success == true
  and ([.. | objects | select(has("count")) | .count | tonumber] | max // 0) > 0)'
rows=0
row() {
  echo "$(date +%T) $*" >&2
  local lines code=0
  lines=$("$bin" --latency "$tmpfs" "$@") || code=$?
  printf '%s\n' "$lines" >>"$log"
  if [ "$code" -ne 0 ]; then
    echo "row '$*' exited $code" >&2
    exit 1
  fi
  if ! jq -se "$ROW_OK" <<<"$lines" >/dev/null; then
    echo "row '$*' failed or recorded no samples" >&2
    exit 1
  fi
  rows=$((rows + 1))
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
expected=$((reps * 15))
jq -cn --argjson rows "$rows" --argjson expected "$expected" \
  '{scope:"LatencyRunEnd",rows:$rows,expected:$expected}' >>"$log"
if [ "$rows" -ne "$expected" ]; then
  echo "$rows rows, expected $expected" >&2
  exit 1
fi
echo "$(date +%T) done: $log" >&2
