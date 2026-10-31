#!/usr/bin/env python3
"""Check TASK-022 declarations and local prerequisites without running a load."""
import argparse
import json
import os
from pathlib import Path
import shutil
import sys
import tomllib


def inspect(profile):
    """Separate invalid declarations, prerequisites, and unexecuted results."""
    errors = []
    pending = []
    storage = profile["storage"]
    memory = profile["memory"]
    measure = profile["measurement"]
    control = profile["control"]
    p3 = profile["p3"]
    partitions = ["input", "raw", "output", "external", "control", "worker"]
    if sum(memory[k + "_mib"] for k in partitions) != memory["total_mib"]:
        errors.append("memory partitions do not sum to total_mib")
    if any(memory[k + "_mib"] <= 0 for k in partitions):
        errors.append("memory partitions must be positive")
    if measure["minimum_ram_multiples"] < 2 or measure["minimum_p3_seconds"] < 60:
        errors.append("P3 minimums cannot weaken the approved 2 * RAM / 60s requirements")
    rate = (p3["raw_channels"] * p3["raw_payload_bytes"]
            * p3["raw_records_per_channel_second"]
            + p3["small_payload_bytes"] * p3["small_records_per_second"])
    payload = rate * measure["measured_seconds"]
    memtotal = next(int(line.split()[1]) * 1024 for line in
                    Path("/proc/meminfo").read_text().splitlines()
                    if line.startswith("MemTotal:"))
    minimum = max(measure["minimum_recorded_bytes"],
                  memtotal * measure["minimum_ram_multiples"])
    if measure["measured_seconds"] < measure["minimum_p3_seconds"] or payload < minimum:
        errors.append("P3 must meet both minimum duration and recorded bytes >= 2 * RAM")
    if rate <= 0 or measure["warmup_seconds"] < 10 or measure["repetitions"] < 3:
        errors.append("positive rate, >=10s warmup and >=3 repetitions are required")
    # Warmup and measured data plus metadata and all concurrently active files.
    worst = (rate * (measure["measured_seconds"] + measure["warmup_seconds"])
             + measure["metadata_headroom_bytes"] + p3["files"] * storage["file_limit_bytes"])
    if worst > storage["run_limit_bytes"]:
        errors.append("payload/warmup/metadata/active-file allowance exceeds run limit")
    if not storage["final_file_and_directory_sync"] or storage["reserve_bytes"] <= 0:
        errors.append("final file/directory sync and positive disk reserve are required")
    roles = [[control["control_cpu"]], [control["safety_cpu"]],
             control["reserved_smt_cpus"], control["worker_cpus"],
             control["housekeeping_cpus"]]
    cpus = [cpu for group in roles for cpu in group]
    if len(cpus) != len(set(cpus)):
        errors.append("CPU roles overlap")
    allowed = set(os.sched_getaffinity(0))
    if not set(cpus).issubset(allowed):
        pending.append("some planned CPUs are outside current process affinity")
    if control["deadline_us"] > control["period_us"] or control["deadline_us"] <= 0:
        errors.append("control deadline must be positive and <= period")
    if not 0 < control["safety_deadline_us"] <= control["safety_period_us"]:
        errors.append("safety deadline must be positive and <= period")
    ups = profile["ups"]
    if (ups["enabled_by_default"] or ups["simulation_window_ms"] != 3000
            or min(ups["signal_delays_ms"]) < 0
            or max(ups["signal_delays_ms"]) + ups["safety_margin_ms"] >= 3000
            or not 0 < ups["safety_margin_ms"] < ups["earlier_deadline_ms"] < 3000):
        errors.append("UPS timing must preserve default-off and the original 3000ms deadline")
    if sum([ups["rank1_max_bytes"], ups["rank2_max_bytes"]]) != max(ups["protection_bytes"]):
        errors.append("UPS rank quotas must sum to maximum protected bytes")
    rt_cpus = {control["control_cpu"], control["safety_cpu"]}
    for cpu in rt_cpus:
        path = Path(f"/sys/devices/system/cpu/cpu{cpu}/topology/thread_siblings_list")
        if path.exists():
            siblings = set()
            for part in path.read_text().strip().split(","):
                ends = list(map(int, part.split("-")))
                siblings.update(range(ends[0], ends[-1] + 1))
            if not (siblings - {cpu}).issubset(set(control["reserved_smt_cpus"])):
                errors.append(f"CPU {cpu} SMT siblings are not reserved")
    parent = Path(storage["parent"])
    if parent.is_dir():
        free = shutil.disk_usage(parent).free
        if free < storage["run_limit_bytes"] + storage["reserve_bytes"]:
            pending.append("free space is below run limit plus reserve")
        if not os.access(parent, os.W_OK):
            pending.append("storage parent is not writable")
    else:
        free = None
        pending.append("storage parent does not exist")
    realtime = Path("/sys/kernel/realtime")
    rt = realtime.exists() and realtime.read_text().strip() == "1"
    if not rt:
        pending.append("PREEMPT_RT boot required for RT rows")
    if profile["approval"] != "Approved":
        pending.append("profile acceptance criteria await Human approval")
    for name in ["performance_harness", "actual_dot_probe_control", "ups_adapter"]:
        if profile["readiness"][name] not in ["Implemented", "Integrated"]:
            pending.append(f"{name}: {profile['readiness'][name]}")
    return {"profile": profile["id"], "declarations": "Invalid" if errors else "Valid",
            "performance_result": "NotRun", "errors": errors, "pending": pending,
            "kernel": os.uname().release, "preempt_rt": rt,
            "p3_payload_bytes_per_second": rate, "measured_payload_bytes": payload,
            "minimum_measured_bytes": minimum, "reserved_run_bytes": worst,
            "storage_free_bytes": free,
            "tools": {name: shutil.which(name) for name in
                      ["cyclictest", "perf", "iostat", "fio", "iperf3"]}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("profile", type=Path)
    parser.add_argument("--require-ready", action="store_true",
                        help="fail if any execution prerequisite remains")
    args = parser.parse_args()
    try:
        with args.profile.open("rb") as stream:
            report = inspect(tomllib.load(stream))
    except (OSError, ValueError, KeyError, TypeError, StopIteration) as error:
        print(json.dumps({"declarations": "Invalid", "error": str(error)}))
        return 1
    print(json.dumps(report, indent=2))
    return 1 if report["errors"] else (2 if args.require_ready and report["pending"] else 0)


if __name__ == "__main__":
    sys.exit(main())
