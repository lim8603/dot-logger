#!/usr/bin/env python3
"""TASK-040: the representative soak load with reading and transfer mixed in.

Measurement harness only. One `soak run` records the TASK-024 load
(control 5,000 records/s, frames 10/s) into a ring with the remote server
on and periodic event marks, while this driver runs beside it:

  reader       `soak reader`: another process reads every closed file of the
               ring, newest first, every --reader-s, while the ring deletes
  subscribers  two CLI `tail` subscriptions (control metadata, frame
               originals), restarted whenever a bounded `tail` ends
  status       CLI `status` (GetStatus) every --status-s

At the end it inspects the ring and judges: the TASK-024 S4 criteria (no
loss, rejection or missed deadline, RT threads off the heap, disk within the
ring), every reader pass intact, subscribers always reconnected, every
status answered, every mark and release accepted, and the kept records a
gapless suffix plus islands that each overlap a marked event window.
Results go to `<out>/` (`*.jsonl`, `result.json`); exit status 1 on Fail.

Build first:
  cargo +1.96.0 build --release --locked -p dot-logger --example soak --features remote
  cargo +1.96.0 build --release --locked -p dot-logger-cli
"""

import argparse
import json
import os
import pathlib
import shutil
import subprocess
import sys
import threading
import time

ROOT = pathlib.Path(__file__).resolve().parent.parent
TARGET = pathlib.Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
CONTROL_HZ = 5000
EVENT_SIDE_S = 30
MIB = 1 << 20
# Channel 0 is the reserved .logger topic, 1 soak/control, 2 soak/frame.
# `--limit` keeps each bounded `tail` below its 256 MiB output cap, so it
# ends normally (exit 0) and is restarted: control lines carry metadata
# only, frame lines a 64 KiB original.
# The CLI's output cap ends a session with outcome `limit` (exit 3). Gap lines
# are not bounded per record, so no record limit rules the cap out; a session
# that ends at the cap is a restart, any other `limit` end is unexpected.
OUTPUT_CAP = 256 * MIB
SUBSCRIBERS = {"control": ["--channel", "1", "--limit", "200000"],
               "frame": ["--channel", "2", "--originals", "--limit", "1500"]}


def now():
    return time.monotonic()


class Child:
    """A child process whose stdout JSON lines are stored and kept."""

    def __init__(self, command, out, name, env=None):
        self.name = name
        self.process = subprocess.Popen(command, stdout=subprocess.PIPE,
                                        stderr=open(out / f"{name}.stderr", "w"), env=env)
        self.items = []
        self.lock = threading.Lock()
        self.file = open(out / f"{name}.jsonl", "w")
        self.thread = threading.Thread(target=self._read, daemon=True)
        self.thread.start()

    def _read(self):
        for raw in self.process.stdout:
            line = raw.decode(errors="replace").rstrip("\n")
            self.file.write(line + "\n")
            self.file.flush()
            try:
                value = json.loads(line)
            except ValueError:
                value = {"raw": line}
            with self.lock:
                self.items.append(value)

    def events(self, name):
        with self.lock:
            return [v for v in self.items if v.get("event") == name]

    def wait_for(self, name, timeout):
        end = now() + timeout
        while now() < end:
            found = self.events(name)
            if found:
                return found[-1]
            time.sleep(0.1)
        return None

    def wait(self, timeout):
        try:
            code = self.process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            self.process.kill()
            code = self.process.wait()
        self.thread.join(timeout=30)
        self.file.close()
        return code


class Subscriber(threading.Thread):
    """Keep one CLI `tail` subscription running until `stop`.

    Record lines are only counted (5,000/s would fill the disk); every other
    line is kept with its receive time. Delivery delay is receive wall time
    minus the record's real time.
    """

    def __init__(self, args, out, name, flags):
        super().__init__(daemon=True)
        self.args, self.out, self.name, self.flags = args, out, name, flags
        self.stop = threading.Event()
        self.records = 0
        self.gaps = 0
        self.ends = []
        self.capped = []
        self.delays_ms = []
        self.reconnect_ms = []
        self.every = 1000 if name == "control" else 2
        self.lines = open(out / f"tail-{name}.jsonl", "w")

    def run(self):
        ended_at = None
        while not self.stop.is_set():
            command = [str(self.args.cli), "tail", "--json", "--endpoint",
                       f"http://127.0.0.1:{self.args.port}", *self.flags,
                       "--records", "4", "--record-bytes", str(MIB),
                       "--max-output-bytes", str(OUTPUT_CAP),
                       "--timeout-ms", "3600000"]
            process = subprocess.Popen(command, stdout=subprocess.PIPE,
                                       stderr=subprocess.DEVNULL)
            first = True
            last = None
            written = 0
            for raw in process.stdout:
                written += len(raw)
                if self.stop.is_set():
                    break
                try:
                    value = json.loads(raw)
                except ValueError:
                    continue
                kind = value.get("kind")
                if kind == "record":
                    self.records += 1
                    if first and ended_at is not None:
                        self.reconnect_ms.append((now() - ended_at) * 1000)
                    first = False
                    # About one delay sample per 0.2 s keeps memory flat.
                    if self.records % self.every == 1 or self.every == 1:
                        real = int(value["data"].get("real_ns", 0))
                        self.delays_ms.append((time.time_ns() - real) / 1e6)
                    continue
                if kind == "gap":
                    self.gaps += 1
                last = value
                self.lines.write(json.dumps({"t": time.time(), **value}) + "\n")
                self.lines.flush()
            if self.stop.is_set():
                process.terminate()
            code = process.wait()
            ended_at = now()
            if not self.stop.is_set():
                end = {"code": code, "last": last, "t": time.time(), "bytes": written}
                summary = (last or {}).get("data", {})
                at_cap = written >= OUTPUT_CAP - MIB
                if code == 3 and summary.get("outcome") == "limit" and at_cap:
                    self.capped.append(end)
                else:
                    self.ends.append(end)
            time.sleep(0.2)

    def finish(self):
        self.stop.set()
        self.join(timeout=30)
        self.lines.close()


class Status(threading.Thread):
    """CLI `status` (GetStatus) every `every` seconds."""

    def __init__(self, args, every):
        super().__init__(daemon=True)
        self.args, self.every = args, every
        self.stop = threading.Event()
        self.calls = []

    def run(self):
        while not self.stop.wait(self.every):
            started = now()
            result = subprocess.run(
                [str(self.args.cli), "status", "--json", "--endpoint",
                 f"http://127.0.0.1:{self.args.port}"],
                capture_output=True, text=True, timeout=60)
            self.calls.append({"code": result.returncode,
                               "ms": round((now() - started) * 1000, 1)})

    def finish(self):
        self.stop.set()
        self.join(timeout=90)


def percentile(values, p):
    if not values:
        return None
    ordered = sorted(values)
    return round(ordered[min(len(ordered) - 1, int(len(ordered) * p / 100))], 2)


def judge_runs(runs, final_control, marks):
    """Kept control records: a suffix up to the last record, and islands
    before it that each overlap a marked window (protection is per file,
    so an island may reach beyond the window to its file's edges)."""
    if not runs:
        return False, "no records kept"
    if runs[-1][1] != final_control - 1:
        return False, f"suffix ends at {runs[-1][1]}, last record {final_control - 1}"
    side = EVENT_SIDE_S * CONTROL_HZ
    windows = [(m["control"] - side, m["control"] + side) for m in marks if m.get("ok")]
    for first, last in runs[:-1]:
        if not any(first <= hi and last >= lo for lo, hi in windows):
            return False, f"island {first}..{last} overlaps no marked window"
    return True, f"{len(runs) - 1} islands, suffix from {runs[-1][0]}"


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("parent", type=pathlib.Path, help="new, empty output directory")
    parser.add_argument("out", type=pathlib.Path, help="result directory")
    parser.add_argument("--seconds", type=int, required=True)
    parser.add_argument("--ring-mib", type=int, required=True)
    parser.add_argument("--sample-s", type=int, default=60)
    parser.add_argument("--events", required=True, help="every_s,hold_s,release_s")
    parser.add_argument("--reader-s", type=int, default=300)
    parser.add_argument("--status-s", type=int, default=60)
    parser.add_argument("--port", type=int, default=50551)
    parser.add_argument("--rt", action="store_true", help="DOT_LOGGER_PERF_RT=fifo")
    parser.add_argument("--soak", type=pathlib.Path, default=TARGET / "release/examples/soak")
    parser.add_argument("--cli", type=pathlib.Path, default=TARGET / "release/dot-logger")
    args = parser.parse_args()

    if args.parent.exists() and any(args.parent.iterdir()):
        sys.exit(f"{args.parent} is not empty")
    args.parent.mkdir(parents=True, exist_ok=True)
    free = shutil.disk_usage(args.parent).free
    if free < (args.ring_mib + 4096) * MIB:
        sys.exit(f"only {free // MIB} MiB free below {args.parent}")
    args.out.mkdir(parents=True, exist_ok=True)

    env = dict(os.environ)
    if args.rt:
        env["DOT_LOGGER_PERF_RT"] = "fifo"
    soak = Child([str(args.soak), "run", str(args.parent), str(args.seconds),
                  str(args.sample_s), str(args.ring_mib), "--remote", str(args.port),
                  "--events", args.events], args.out, "soak", env)
    start = soak.wait_for("start", 60)
    if start is None:
        soak.process.kill()
        sys.exit("soak did not start")
    directory = pathlib.Path(start["dirs"][0])
    reader = Child([str(args.soak), "reader", str(directory), str(args.reader_s),
                    str(args.seconds)], args.out, "reader")
    subscribers = {name: Subscriber(args, args.out, name, flags)
                   for name, flags in SUBSCRIBERS.items()}
    for s in subscribers.values():
        s.start()
    status = Status(args, args.status_s)
    status.start()

    load_ends = time.time() + args.seconds
    soak_code = soak.wait(args.seconds + 600)
    for s in subscribers.values():
        s.finish()
    status.finish()
    reader_code = reader.wait(args.seconds + 1200)

    end = (soak.events("end") or [{}])[-1]
    samples = soak.events("sample")
    marks = soak.events("mark")
    releases = soak.events("release")
    reader_end = (reader.events("reader_end") or [{}])[-1]
    counts = end.get("counts", {})
    control, frames = counts.get("control", 0), counts.get("frames", 0)
    inspected = json.loads(subprocess.run(
        [str(args.soak), "inspect", str(directory), str(control), str(frames)],
        check=True, capture_output=True, text=True).stdout.strip().splitlines()[-1])
    found = inspected["found"]
    runs_ok, runs_note = judge_runs(inspected["runs"]["control"], control, marks)
    ring_bytes = args.ring_mib * MIB
    disk_max = max((s.get("disk_bytes", 0) for s in samples), default=0)
    subs = {}
    for name, s in subscribers.items():
        # A `tail` the shutting-down server ends is the run's end, not a fault.
        unexpected = [e for e in s.ends if e["code"] != 0 and e["t"] < load_ends - 1]
        subs[name] = {"records": s.records, "gaps": s.gaps,
                      "restarts": len(s.ends) + len(s.capped),
                      "output_cap_ends": s.capped[:20],
                      "ends": [{k: e[k] for k in ("code", "bytes", "t")} | {
                          "summary": (e["last"] or {}).get("data")} for e in s.ends[:20]],
                      "unexpected_ends": unexpected[:20],
                      "delay_ms": {"p50": percentile(s.delays_ms, 50),
                                   "p99": percentile(s.delays_ms, 99),
                                   "max": percentile(s.delays_ms, 100),
                                   "samples": len(s.delays_ms)},
                      "reconnect_ms": {"p50": percentile(s.reconnect_ms, 50),
                                       "max": percentile(s.reconnect_ms, 100)}}
    status_failed = [c for c in status.calls if c["code"] != 0]
    checks = {
        "soak_success": soak_code == 0 and end.get("success") is True,
        "no_rejection": counts.get("control_rejected") == 0 and counts.get("frames_rejected") == 0,
        "no_deadline_miss": counts.get("misses") == 0,
        "rt_threads_off_heap": end.get("rt_heap", {}).get("allocs") == 0
        and end.get("rt_heap", {}).get("frees") == 0,
        "disk_within_ring": disk_max <= ring_bytes,
        "reader_all_passes_intact": reader_code == 0 and reader_end.get("failed_passes") == 0,
        "subscribers_received": all(v["records"] > 0 for v in subs.values()),
        "subscribers_ended_cleanly": all(not v["unexpected_ends"] for v in subs.values()),
        "status_all_answered": bool(status.calls) and not status_failed,
        "marks_accepted": bool(marks) and all(m.get("ok") for m in marks),
        "releases_accepted": all(r.get("ok") for r in releases),
        "files_intact": found["unreadable_files"] == 0 and found["torn_files"] == 0
        and found["foreign"] == 0 and found["duplicates"] == 0
        and found["corrupt_payloads"] == 0 and found["nonzero_loss_rows"] == 0,
        "kept_suffix_and_event_islands": runs_ok,
    }
    result = {
        "pass": all(checks.values()), "checks": checks, "runs_note": runs_note,
        "kernel": start.get("kernel"), "rt_mode": start.get("rt_mode"),
        "seconds": args.seconds, "ring_mib": args.ring_mib, "events": args.events,
        "counts": counts, "worst_response_us": counts.get("worst_response_us"),
        "rt_heap": end.get("rt_heap"), "disk_max_bytes": disk_max,
        "rss_kib": {"first": samples[0].get("rss_kib") if samples else None,
                    "last": samples[-1].get("rss_kib") if samples else None},
        "reader": reader_end, "subscribers": subs,
        "status": {"calls": len(status.calls), "failed": status_failed[:20],
                   "ms_p50": percentile([c["ms"] for c in status.calls], 50),
                   "ms_max": percentile([c["ms"] for c in status.calls], 100)},
        "marks": len(marks), "releases": len(releases),
        "found": found, "control_runs": inspected["runs"]["control"][:20],
    }
    (args.out / "result.json").write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps({"pass": result["pass"],
                      "failed": [k for k, v in checks.items() if not v]}))
    sys.exit(0 if result["pass"] else 1)


if __name__ == "__main__":
    main()
