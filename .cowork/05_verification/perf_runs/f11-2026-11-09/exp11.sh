#!/usr/bin/env bash
# Finding 11 A/B in the SLC-depleted state. Diagnostic only.
set -uo pipefail
S=$(cd "$(dirname "$0")" && pwd); O=$S/exp; P=/home/lim8603/perf-tmp; mkdir -p $P
DEV=nvme1n1; T=$((12<<30)); CG="systemd-run --user --scope -q -p MemoryMax=4G"
sec() { awk '{print $7}' /sys/block/$DEV/stat; }
log() { echo "[$(date +%T)] $*" | tee -a $O/progress.txt; }
sample() { # pid outfile: count (comm,state,wchan) every 50 ms
  local pid=$1 f=$2; : >"$f"
  while kill -0 "$pid" 2>/dev/null; do
    for t in /proc/$pid/task/*; do
      c=$(cat $t/comm 2>/dev/null) || continue
      st=$(awk '{print $3}' $t/stat 2>/dev/null); w=$(cat $t/wchan 2>/dev/null)
      echo "$c $st ${w:-0}" >>"$f"
    done; sleep 0.05
  done
}
run() { # label binary args...
  local label=$1 bin=$2; shift 2
  log "start $label"
  sync; local s0=$(sec) t0=$(date +%s.%N)
  $CG $S/$bin "$@" >$O/$label.json 2>$O/$label.err &
  local spid=$!; sleep 0.3
  local pid=$(pgrep -n -f "$S/$bin") ; sample "$pid" $O/$label.wchan & local wp=$!
  wait $spid; local rc=$?; wait $wp
  sync; local s1=$(sec) t1=$(date +%s.%N)
  jq -c --arg l "$label" --argjson rc $rc --argjson dev $(( (s1-s0)*512 )) --argjson wall "$(echo "$t1-$t0"|bc)" \
    '{label:$l, rc:$rc, mbps:(.throughput.bytes_per_second/1e6|floor), payload:.payload_bytes, dev_bytes:$dev, wall:$wall, files:.files_created}' $O/$label.json 2>/dev/null \
    | tee -a $O/summary.jsonl || echo "{\"label\":\"$label\",\"rc\":$rc}" | tee -a $O/summary.jsonl
}
P0="--p0-mix-max $P $T 256 0 8388608 120 4 4 4"
L="--p1-mix-max $P $T 256 0 8388608 120 4 4 4 4 4 1024 8 1024 0"
log "deplete"; s0=$(sec)
$CG $S/pb-default --sustained $P $((500<<30)) 4 4 60 >$O/sustained.json 2>$O/sustained.err
log "deplete done dev_bytes=$(( ($(sec)-s0)*512 )) $(jq -c '{stable, drop_at_s, peak:(.peak_bytes_per_second|tonumber/1e6|floor), S:(.bytes_per_second|tonumber/1e6|floor), total:(.total_bytes|tonumber/1e9|floor)}' $O/sustained.json)"
for rep in 1 2; do
  run p0-$rep pb-default $P0
  run default-$rep pb-default $L
  run nosync-$rep pb-nosync $L
  run 1gib-$rep pb-1gib $L
done
log "done total_dev_since_deplete=$(( ($(sec)-s0)*512 ))"
