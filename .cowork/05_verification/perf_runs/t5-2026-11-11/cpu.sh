#!/usr/bin/env bash
# usage: cpu.sh <out_prefix> <perf_baseline args...>  — runs in a fresh dir under /dev/shm, samples per-thread CPU every 0.5 s
set -u
pre=$1; shift
bin=/data/build/target-diag/release/examples/perf_baseline
d=$(mktemp -d /dev/shm/t5.XXXX)
args=("$@"); args[1]=$d
"$bin" "${args[@]}" > "$pre.json" 2> "$pre.err" &
pid=$!
: > "$pre.samples"
while kill -0 $pid 2>/dev/null; do
  t=$(date +%s.%N)
  awk -v t=$t '{c=$0; tid=$1; sub(/^[0-9]+ \(/,"",c); n=substr(c,1,index(c,") ")-1); r=substr(c,index(c,") ")+2); split(r,a," "); gsub(/ /,"_",n); print t, tid, n, a[1], a[12]+a[13], a[12], a[13]}' /proc/$pid/task/*/stat 2>/dev/null >> "$pre.samples"
  sleep 0.2
done
wait $pid; echo "exit $?" >> "$pre.err"
rm -rf "$d"
