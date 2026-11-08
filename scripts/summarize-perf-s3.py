#!/usr/bin/env python3
"""Summarize scripts/run-perf-s3.sh results (TASK-023 S3).

usage: scripts/summarize-perf-s3.py <out>/runs.jsonl

Per condition and repetition: durable throughput, logger/P0 efficiency of the
same repetition, durability tail, control/safety misses, load-window
judgment, call latencies, rejections and read-back. A rate is shown only
when the run's own judgment gave one; read-back failures are printed.
"""
import json
import statistics
import sys
from collections import defaultdict


def load(path):
    facts = defaultdict(dict)
    for line in open(path):
        d = json.loads(line)
        if "label" not in d:
            continue
        key = (d["label"], d["rep"])
        scope = d["scope"]
        if scope == "P0Control":
            facts[key][d["loop"]] = d
        else:
            facts[key][scope] = d
    return facts


def rate(storage):
    t = storage.get("throughput", {})
    return t.get("bytes_per_second")


def main(path):
    facts = load(path)
    order = []
    for (label, _rep) in facts:
        if label not in order:
            order.append(label)
    rows = []
    for label in order:
        reps = sorted(r for (l, r) in facts if l == label)
        for rep in reps:
            f = facts[(label, rep)]
            storage = f.get("LoggerStorage") or f.get("P0Storage")
            row = {"label": label, "rep": rep}
            for loop in ("control", "safety"):
                if loop in f:
                    row[f"{loop}_miss"] = f[loop]["misses"]
                    row[f"{loop}_max_ms"] = int(f[loop]["response"]["max_ns"]) / 1e6
            if "LoadWindow" in f:
                w = f["LoadWindow"]
                row["concurrent"] = w["control"]["concurrent"] is True and w["safety"]["concurrent"] is True
                row["tail_s"] = int(w["tail_ns"]) / 1e9 if w["tail_ns"] else None
            if storage:
                row["MBps"] = (rate(storage) or 0) / 1e6 or None
                row["rejected"] = storage["throughput"].get("rejected")
                row["verified"] = storage.get("verified_records")
                row["verify_error"] = storage.get("verify_error")
                if "accepted_calls" in storage:
                    a = storage["accepted_calls"]
                    row["p99_ns"], row["p999_ns"] = a["p99_ns"], a["p999_ns"]
                    r = storage["raw_accepted_calls"]
                    row["raw_p99_ns"] = r["p99_ns"]
                    row["rejections"] = storage["rejections"]
                    row["sync_failed"] = "sync_failed: true" in storage["report"]
            if "P1CallCost" in f:
                c = f["P1CallCost"]
                row["try_log_p99"] = c["try_log"]["p99_ns"]
                row["copy_p50"] = c["copy"]["p50_ns"]
                row["clock_p50"] = c["clock"]["p50_ns"]
            rows.append(row)
    # Efficiency against the P0 run of the same workload and repetition.
    by = {(r["label"], r["rep"]): r for r in rows}
    pairs = {"p2-logger": "p2-p0", "p3-logger": "p3-p0", "p3-logger-telemetry": "p3-p0"}
    for r in rows:
        base = pairs.get(r["label"])
        if r["label"].startswith("p1-logger-"):
            base = "p1-p0-" + r["label"].split("-")[-1]
        if base and (base, r["rep"]) in by and r.get("MBps") and by[(base, r["rep"])].get("MBps"):
            r["eff_%"] = 100 * r["MBps"] / by[(base, r["rep"])]["MBps"]
    # Instrumentation cost: telemetry-on throughput relative to off, same rep.
    for r in rows:
        if r["label"] == "p3-logger-telemetry":
            off = by.get(("p3-logger", r["rep"]))
            if off and off.get("MBps") and r.get("MBps"):
                r["telemetry_cost_%"] = 100 * (1 - r["MBps"] / off["MBps"])
    keys = []
    for r in rows:
        for k in r:
            if k not in keys:
                keys.append(k)
    print("\t".join(keys))
    for r in rows:
        print("\t".join(fmt(r.get(k)) for k in keys))
    # Means per label for the numeric columns.
    print()
    print("label\tn\tMBps_mean\teff_%_mean\ttail_s_mean\tcontrol_miss\tsafety_miss")
    for label in order:
        rs = [r for r in rows if r["label"] == label]
        def mean(k):
            v = [r[k] for r in rs if isinstance(r.get(k), (int, float))]
            return statistics.mean(v) if v else None
        def total(k):
            v = [r[k] for r in rs if isinstance(r.get(k), int)]
            return sum(v) if v else None
        print("\t".join([label, str(len(rs)), fmt(mean("MBps")), fmt(mean("eff_%")),
                         fmt(mean("tail_s")), fmt(total("control_miss")), fmt(total("safety_miss"))]))


def fmt(v):
    if v is None:
        return "-"
    if isinstance(v, float):
        return f"{v:.2f}"
    return str(v)


if __name__ == "__main__":
    main(sys.argv[1])
