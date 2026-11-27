#!/usr/bin/env python3
"""TASK-024 S2: inject faults into the soak harness and judge each scenario.

Measurement harness only. Every scenario runs the representative load of
`examples/soak.rs` (control 5,000 records/s, frames 10/s) and splits the run
into before / during / after the fault. Results go to `<out>/<scenario>/`
(the soak JSON lines, the subscriber lines and `result.json`); this script
prints one summary line per scenario and exits non-zero if any failed.

Scenarios (design: .cowork/04_implementation/tasks/TASK-024.md, S2):
  f1   slow file: Docker container, write bandwidth cut on the disk (D1)
  f2   slow subscriber: short stops below max_age (F2a) and a 2 min stop (F2b)
  f3a  free space below reserve + segment (probe path), then freed again
  f3b  a real ENOSPC on write (IoPoisoned), recorded as current behavior (D3)
  f4   network: a socat proxy stopped silently, then killed
  f5   SIGKILL without warning, unfinished files read back, restart
  f6a  InfluxDB paused (silent: requests hang), then resumed (TASK-047)
  f6b  InfluxDB stopped (connections refused), then started again
  f6c  InfluxDB's network disconnected, then connected again

f6 runs a digest-pinned InfluxDB 2.9.1 container (the image of
scripts/check-influxdb.py) on a fixed loopback port with its data in a
/dev/shm directory, so it survives stop/start without SSD writes, and the soak
`--influx` writer thread beside the load. After recovery the stored points are
read back with Flux and compared with the writer's batch log.

f1, f3a and f3b run the harness in a Docker container that chroots into the
host root (`--image` only provides the container; the host binaries run).
f1 changes the container's io.max during the run through a privileged helper
container, because `--device-write-bps` is fixed at start. f2, f4 and f5 run
on the host with output in /dev/shm.

Build first:
  cargo +1.96.0 build --release --locked -p dot-logger --example soak --features remote,influxdb
  cargo +1.96.0 build --release --locked -p dot-logger-cli
"""

import argparse
import importlib.util
import json
import os
import pathlib
import re
import shutil
import signal
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
TARGET = pathlib.Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
CONTROL_HZ = 5000
FRAME_HZ = 10
MIB = 1 << 20
# Frames reach a subscriber only as originals; channel 2 is soak/frame
# (0 is the reserved .logger topic, 1 soak/control).
FRAME_CHANNEL = "2"
# Receiver window: 4 x 1 MiB fits the facade's 4 MiB subscription limit.
TAIL_WINDOW = ["--records", "4", "--record-bytes", str(MIB)]


def now():
    return time.monotonic()


class Lines:
    """Collect a child's stdout JSON lines with the driver's receive time."""

    def __init__(self, stream, path):
        self.items = []
        self.lock = threading.Lock()
        self.file = open(path, "w")
        self.thread = threading.Thread(target=self._read, args=(stream,), daemon=True)
        self.thread.start()

    def _read(self, stream):
        for raw in stream:
            line = raw.decode(errors="replace").rstrip("\n")
            at = now()
            self.file.write(line + "\n")
            try:
                value = json.loads(line)
            except ValueError:
                value = {"raw": line}
            with self.lock:
                self.items.append((at, value))
        self.file.flush()

    def snapshot(self):
        with self.lock:
            return list(self.items)

    def events(self, name):
        return [v for _, v in self.snapshot() if v.get("event") == name]

    def wait_for(self, name, timeout):
        end = now() + timeout
        while now() < end:
            found = self.events(name)
            if found:
                return found[-1]
            time.sleep(0.05)
        return None

    def close(self):
        self.thread.join(timeout=10)
        self.file.close()


class Soak:
    """One `soak run`, on the host or inside a chrooting container."""

    def __init__(self, args, out, name, parent, seconds, ring_mib, extra=(),
                 docker=None, env_extra=None):
        self.name = name
        self.docker = docker
        self.container = f"dot-logger-s2-{name}-{os.getpid()}" if docker is not None else None
        soak = str(args.soak)
        run = [soak, "run", str(parent), str(seconds), "1", str(ring_mib),
               "--sample-ms", str(args.sample_ms), *extra]
        env = dict(os.environ)
        env.update(env_extra or {})
        if args.rt:
            env["DOT_LOGGER_PERF_RT"] = "fifo"
        else:
            env.pop("DOT_LOGGER_PERF_RT", None)
        if docker is None:
            command = run
        else:
            # The container idles; the soak and later `inspect` run in it
            # with `docker exec`, so a tmpfs mounted in it outlives the soak.
            subprocess.run(["docker", "run", "-d", "--rm", "--name", self.container,
                            "--network", "host", "--cap-add", "SYS_NICE",
                            "--ulimit", "rtprio=99", "--ulimit", "memlock=-1",
                            "-v", "/:/host", *docker, "--entrypoint", "sleep", args.image,
                            "infinity"], check=True, capture_output=True)
            command = ["docker", "exec"]
            if args.rt:
                command += ["-e", "DOT_LOGGER_PERF_RT=fifo"]
            command += [self.container, "chroot",
                        f"--userspec={os.getuid()}:{os.getgid()}", "/host", *run]
        self.started = now()
        self.process = subprocess.Popen(command, stdout=subprocess.PIPE,
                                        stderr=open(out / f"{name}.stderr", "w"), env=env)
        self.lines = Lines(self.process.stdout, out / f"{name}.jsonl")

    def start_event(self, timeout=60):
        event = self.lines.wait_for("start", timeout)
        if event is None:
            raise RuntimeError(f"{self.name}: soak did not start")
        return event

    def exec(self, *command):
        """Run a command inside the container as the host user."""
        return subprocess.run(
            ["docker", "exec", self.container, "chroot",
             f"--userspec={os.getuid()}:{os.getgid()}", "/host", *command],
            check=True, capture_output=True, text=True).stdout

    def samples(self):
        return self.lines.events("sample")

    def last_sample(self):
        samples = self.samples()
        return samples[-1] if samples else None

    def wait(self, timeout):
        try:
            code = self.process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            self.kill()
            code = self.process.wait()
        self.lines.close()
        return code

    def kill(self):
        self.remove()
        self.process.kill()

    def remove(self):
        if self.container:
            subprocess.run(["docker", "rm", "-f", self.container], capture_output=True)


class Tail:
    """CLI `tail` on the frame channel; counts records, gaps and the outcome."""

    def __init__(self, args, out, name, port, disconnect=False):
        command = [str(args.cli), "tail", "--json", "--endpoint", f"http://127.0.0.1:{port}",
                   "--channel", FRAME_CHANNEL, "--originals", *TAIL_WINDOW,
                   "--limit", "1000000", "--max-output-bytes", str(256 * MIB),
                   "--timeout-ms", "3600000"]
        if disconnect:
            command.append("--disconnect-on-overflow")
        self.name = name
        self.started = now()
        self.process = subprocess.Popen(command, stdout=subprocess.PIPE,
                                        stderr=open(out / f"{name}.stderr", "w"))
        self.lines = Lines(self.process.stdout, out / f"{name}.jsonl")

    def kinds(self, after=0.0):
        counts = {}
        for at, v in self.lines.snapshot():
            if at >= after:
                counts[v.get("kind", "?")] = counts.get(v.get("kind", "?"), 0) + 1
        return counts

    def ended(self):
        return self.process.poll() is not None

    def outcome(self):
        for _, v in reversed(self.lines.snapshot()):
            if v.get("kind") in ("error", "summary", "end", "result"):
                return v
        return None

    def stop(self):
        if not self.ended():
            self.process.send_signal(signal.SIGCONT)
            self.process.terminate()
        try:
            self.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait()
        self.lines.close()

    def pause(self):
        self.process.send_signal(signal.SIGSTOP)

    def resume(self):
        self.process.send_signal(signal.SIGCONT)


def inspect(args, directory, control, frames, active=False, soak=None):
    command = [str(args.soak), "inspect", str(directory), str(control), str(frames)]
    if active:
        command.append("--active")
    if soak is not None and soak.container:
        out = soak.exec(*command)
    else:
        out = subprocess.run(command, check=True, capture_output=True, text=True).stdout
    return json.loads(out.strip().splitlines()[-1])


def counts(event):
    c = event["counts"]
    return c["control"], c["frames"]


# Present in every finished scenario but F5; a missing one fails (RV-203).
REQUIRED = ("deadline_misses_0", "end_line", "failures_none", "exit_expected",
            "worker_alive", "heap_samples", "heap_bounded", "heap_returns")


def common_checks(soak, end, checks, code, clean=True):
    """Checks every scenario shares: RT loop never missed, heap bounded, no
    RT heap use or producer error, the expected exit. `clean` is False when
    the fault itself makes the shutdown unsuccessful (F3: exit 1)."""
    samples = soak.samples()
    final = end or (samples[-1] if samples else {})
    checks["deadline_misses_0"] = final.get("counts", {}).get("misses") == 0
    checks["end_line"] = end is not None
    # RT heap use and producer errors go to `failures`, not to `success`.
    checks["failures_none"] = end is not None and end.get("failures") == []
    checks["exit_expected"] = code == (0 if clean else 1)
    checks["worker_alive"] = bool(samples) and not any(s.get("worker_failed") for s in samples)
    lives = [s["heap"]["live_bytes"] for s in samples if s.get("heap")]
    checks["heap_samples"] = bool(lives)
    checks["heap_bounded"] = checks["heap_returns"] = False
    if lives:
        baseline = lives[min(len(lives) - 1, max(1, len(lives) // 20))]
        growth = max(lives) - baseline
        checks["heap_growth_bytes"] = growth
        checks["heap_bounded"] = growth <= args_global.heap_slack_mib * MIB
        # Buffered backlog is released again: the end is back near the start.
        checks["heap_returns"] = lives[-1] <= baseline + MIB
    return checks


def loss_sum(found, prefix):
    total = 0
    for observer in found["losses"].values():
        for key, value in observer["counts"].items():
            if key.startswith(prefix):
                total += value
    return total


def holes(found):
    return found["control"]["holes"] + found["frames"]["holes"]


def phase(marks, name):
    marks.append({"phase": name, "t": round(now() - marks_start[0], 3)})


def wait_samples(soak, seconds):
    end = now() + seconds
    while now() < end:
        if soak.process.poll() is not None:
            raise RuntimeError(f"{soak.name}: soak exited early")
        time.sleep(0.2)


def sample_at(soak, after):
    for at, v in soak.lines.snapshot():
        if at >= after and v.get("event") == "sample":
            return v
    return None


def output(sample):
    return (sample or {}).get("outputs", [{}])[0]


# --- scenarios --------------------------------------------------------------

def f1(args, out):
    """Slow file: cut the container's write bandwidth below the load."""
    parent = pathlib.Path(args.disk_dir) / "f1"
    parent.mkdir(parents=True)
    pre, fault, post = args.pre, args.f1_fault, args.post
    soak = Soak(args, out, "soak", parent, pre + fault + post, 1024, docker=[])
    marks = []
    soak.start_event()
    phase(marks, "pre")
    wait_samples(soak, pre)
    cid = subprocess.run(["docker", "inspect", "-f", "{{.Id}}", soak.container],
                         check=True, capture_output=True, text=True).stdout.strip()
    io_max = f"/sys/fs/cgroup/system.slice/docker-{cid}.scope/io.max"
    device = args.device

    def throttle(limit):
        subprocess.run(["docker", "run", "--rm", "--privileged", "-v",
                        "/sys/fs/cgroup:/sys/fs/cgroup", "--entrypoint", "sh", args.image,
                        "-c", f"echo '{device} wbps={limit}' > {io_max}"],
                       check=True, capture_output=True)

    before = sample_at(soak, now() - 1)
    throttle(args.f1_bps)
    fault_at = now()
    phase(marks, f"throttle {args.f1_bps} B/s")
    wait_samples(soak, fault)
    during = soak.last_sample()
    throttle("max")
    release_at = now()
    phase(marks, "release")
    # Let the buffered backlog drain before judging that rejections stopped.
    wait_samples(soak, min(30, post // 2))
    settled = soak.last_sample()
    code = soak.wait(post + 120)
    end = soak.lines.wait_for("end", 1)
    control, frames = counts(end)
    found = inspect(args, parent, control, frames)
    soak.remove()
    rejected = end["counts"]["control_rejected"] + end["counts"]["frames_rejected"]
    late = [s for at, s in soak.lines.snapshot()
            if fault_at <= at <= release_at and s.get("event") == "sample"]
    checks = {
        "rejected_before_fault_0": (before["counts"]["control_rejected"]
                                    + before["counts"]["frames_rejected"]) == 0,
        "rejected_during_fault": (during["counts"]["control_rejected"]
                                  + during["counts"]["frames_rejected"]),
        "rejections_stop_after_drain": (settled["counts"]["control_rejected"]
                                        + settled["counts"]["frames_rejected"]) == rejected,
        "sync_overdue_seen": any(s.get("health") == "Degraded(SyncOverdue)" for s in late)
        or max((output(s).get("since_sync_ms") or 0) for s in late) > 4000,
        "loss_report_matches_rejections": loss_sum(found, "SubmissionRejected/")
        + loss_sum(found, "AcquireRejected/") == rejected,
        "failed_records_0": end["outputs"][0]["failed_records"] == 0,
        "files_gapless": holes(found) == 0 and found["found"]["torn_files"] == 0
        and found["found"]["unreadable_files"] == 0,
        "shutdown_success": end["success"] is True,
    }
    common_checks(soak, end, checks, code)
    if not args.keep:
        shutil.rmtree(parent, ignore_errors=True)
    return marks, checks, {"exit": code, "end": end, "inspect": found}


def f2(args, out):
    """Slow subscriber: F2a stops below max_age, then F2b a long stop."""
    parent = pathlib.Path(args.shm_dir) / "f2"
    parent.mkdir(parents=True)
    port = args.port
    stops = args.f2_stops
    total = args.pre + stops * (args.f2_stop + args.f2_gap) + args.f2_long + 30 + args.post
    soak = Soak(args, out, "soak", parent, total, 256, ["--remote", str(port)])
    marks = []
    soak.start_event()
    phase(marks, "pre")
    gap = Tail(args, out, "tail-gap", port)
    disc = Tail(args, out, "tail-disconnect", port, disconnect=True)
    wait_samples(soak, args.pre)
    checks = {"subscribers_receive_before": gap.kinds().get("record", 0) > 0
              and disc.kinds().get("record", 0) > 0}
    gap_events = 0
    for i in range(stops):
        phase(marks, f"F2a stop {i} ({args.f2_stop}s)")
        stopped = now()
        gap.pause()
        if not disc.ended():
            disc.pause()
        time.sleep(args.f2_stop)
        gap.resume()
        if not disc.ended():
            disc.resume()
        wait_samples(soak, args.f2_gap)
        gap_events += gap.kinds(after=stopped).get("gap", 0)
    checks["f2a_gap_events_after_stops"] = gap_events
    # Stops below max_age must reach the Gap subscriber as Gap events (RV-203).
    checks["f2a_gap_notified"] = gap_events > 0
    checks["f2a_gap_stream_alive"] = not gap.ended()
    checks["f2a_gap_receives_after_last_resume"] = gap.kinds(after=now() - args.f2_gap + 2).get("record", 0) > 0
    checks["f2a_disconnect_ended"] = disc.ended()
    checks["f2a_disconnect_outcome"] = disc.outcome()
    disc.stop()
    # F2b: both policies, a stop far beyond max_age (10 s).
    disc = Tail(args, out, "tail-disconnect-2", port, disconnect=True)
    time.sleep(5)
    phase(marks, f"F2b stop ({args.f2_long}s)")
    gap.pause()
    disc.pause()
    time.sleep(args.f2_long)
    gap.resume()
    disc.resume()
    resumed = now()
    time.sleep(15)
    checks["f2b_gap_ended"] = gap.ended()
    checks["f2b_gap_outcome"] = gap.outcome()
    checks["f2b_disconnect_ended"] = disc.ended()
    checks["f2b_disconnect_outcome"] = disc.outcome()
    gap.stop()
    disc.stop()
    phase(marks, "reconnect")
    again = Tail(args, out, "tail-reconnect", port)
    wait_samples(soak, 15)
    checks["f2b_reconnect_receives"] = again.kinds().get("record", 0) > 0
    again.stop()
    code = soak.wait(args.post + 120)
    end = soak.lines.wait_for("end", 1)
    control, frames = counts(end)
    found = inspect(args, parent, control, frames)
    checks.update({
        "file_failed_records_0": end["outputs"][0]["failed_records"] == 0,
        "file_rejected_0": end["counts"]["control_rejected"] + end["counts"]["frames_rejected"] == 0,
        "files_gapless": holes(found) == 0 and found["found"]["torn_files"] == 0,
        "shutdown_success": end["success"] is True,
    })
    del resumed
    common_checks(soak, end, checks, code)
    shutil.rmtree(parent, ignore_errors=True)
    return marks, checks, {"exit": code, "end": end, "inspect": found}


def tmpfs_soak(args, out, name, inodes=None):
    parent = pathlib.Path(args.disk_dir) / name
    parent.mkdir(parents=True)
    limit = f",nr_inodes={inodes}" if inodes else ""
    # Mounted below /host: the soak and every fill run chrooted into /host,
    # so a mount at the plain path would leave them on the host disk.
    mount = ["--tmpfs", f"/host{parent}:size={args.tmpfs_mib}m,uid={os.getuid()},"
             f"gid={os.getgid()},mode=0755{limit}"]
    return parent, mount


def assert_tmpfs(args, soak, parent):
    """Refuse to fill anything but the small tmpfs, checked where the fill
    runs (same container, same chroot): never the host disk."""
    kind, blocks, size = soak.exec("stat", "-f", "-c", "%T %b %S", str(parent)).split()
    total = int(blocks) * int(size)
    if kind != "tmpfs" or total > args.tmpfs_mib * MIB:
        raise RuntimeError(f"refusing to fill {parent}: {kind}, {total} bytes")


# F3 fill and cleanup scripts: the tmpfs-checked directory is "$1", never
# text in the script, so spaces or glob characters cannot move the target
# (REV-073 RV-202). Both only touch `fill-*` directly below it.
TAKE_INODES = ('i=0; while [ "$i" -lt 1000 ] && touch -- "$1/fill-$i" 2>/dev/null; '
               'do i=$((i+1)); done')
REMOVE_FILLS = 'rm -f -- "$1"/fill-*'


def fill_argv(script, parent):
    """`sh` running `script` with `parent` as its single argument."""
    return ["sh", "-c", script, "sh", str(parent)]


def tmpfs_free(soak, parent):
    df = soak.exec("df", "-B1", "--output=avail", str(parent)).split()
    return int(df[-1])


def closed_bytes(soak, parent):
    out = soak.exec("find", str(parent), "-name", "*.mcap", "-printf", "%s\n")
    return sum(int(x) for x in out.split())


def f3a(args, out):
    """Free space below reserve + segment: the ring deletes its own closed
    files, then fails records (Cause::Adapter), and recovers once freed."""
    parent, mount = tmpfs_soak(args, out, "f3a")
    total = args.pre + args.f3_fault + args.post
    soak = Soak(args, out, "soak", parent, total, 256, docker=mount)
    marks = []
    soak.start_event()
    assert_tmpfs(args, soak, parent)
    phase(marks, "pre")
    wait_samples(soak, args.pre)
    fill = parent / "fill"
    phase(marks, "fill")
    fault_at = now()
    # Fill to 16 MiB free (below the 32 MiB segment); the ring then deletes
    # its closed files, so top up until none are left.
    part = 0
    while now() - fault_at < args.f3_fault:
        assert_tmpfs(args, soak, parent)
        free = tmpfs_free(soak, parent)
        if free > 24 * MIB:
            soak.exec("fallocate", "-l", str(free - 16 * MIB), f"{fill}-{part}")
            part += 1
        time.sleep(1)
    during = soak.last_sample()
    deleted_all = closed_bytes(soak, parent) == 0
    phase(marks, "free")
    assert_tmpfs(args, soak, parent)
    soak.exec(*fill_argv(REMOVE_FILLS, parent))
    freed_at = now()
    wait_samples(soak, 15)
    settled = soak.last_sample()
    code = soak.wait(args.post + 120)
    end = soak.lines.wait_for("end", 1)
    control, frames = counts(end)
    found = inspect(args, parent, control, frames, soak=soak)
    soak.remove()
    failed = end["outputs"][0]["failed_records"]
    checks = {
        "closed_files_deleted_under_pressure": deleted_all,
        "records_failed_during_fault": output(during).get("failed_records", 0) > 0,
        "failed_stop_after_free": output(settled).get("failed_records") == failed,
        "output_not_failed": end["outputs"][0]["failed"] is False,
        "health_records_failed": end.get("health") == "Degraded(RecordsFailed)",
        "rejected_0": end["counts"]["control_rejected"] + end["counts"]["frames_rejected"] == 0,
        "shutdown_not_success_expected": end["success"] is False,
        "failed_records": failed,
        "loss_report_matches_failed": loss_sum(found, "OutputFailed/Adapter") == failed,
        "file_holes_match_failed": holes(found) == failed,
        "files_intact": found["found"]["torn_files"] == 0
        and found["found"]["unreadable_files"] == 0,
    }
    del freed_at
    common_checks(soak, end, checks, code, clean=False)
    return marks, checks, {"exit": code, "end": end, "inspect": found}


def f3b(args, out):
    """A real ENOSPC the free-space probe cannot predict (D3: record it).

    The tmpfs has few inodes; at the fault every remaining inode is taken, so
    the next segment the ring creates fails with ENOSPC while plenty of bytes
    are free (the probe reads bytes only). Racing a byte fill against the
    probe almost never lands between probe and write."""
    parent, mount = tmpfs_soak(args, out, "f3b", inodes=64)
    total = args.pre + args.f3_fault + args.post + 60
    soak = Soak(args, out, "soak", parent, total, 256, docker=mount)
    marks = []
    soak.start_event()
    assert_tmpfs(args, soak, parent)
    phase(marks, "pre")
    wait_samples(soak, args.pre)
    phase(marks, "take all inodes")
    assert_tmpfs(args, soak, parent)
    soak.exec(*fill_argv(TAKE_INODES, parent))
    free_bytes = tmpfs_free(soak, parent)
    poisoned_at = None
    end_wait = now() + args.f3_fault
    while now() < end_wait:
        if output(soak.last_sample()).get("failed"):
            poisoned_at = now()
            break
        time.sleep(0.1)
    phase(marks, "poisoned" if poisoned_at else "not poisoned in time")
    at_poison = soak.last_sample()
    # Give the inodes back: the output stays failed for the session.
    assert_tmpfs(args, soak, parent)
    soak.exec(*fill_argv(REMOVE_FILLS, parent))
    phase(marks, "inodes freed")
    wait_samples(soak, 20)
    later = soak.last_sample()
    code = soak.wait(total + 120)
    end = soak.lines.wait_for("end", 1)
    soak.remove()
    report = end.get("report", "")
    report_failed = [int(x) for x in re.findall(r"failed: (\d+)", report)]
    failed = end["outputs"][0]["failed_records"]
    checks = {
        "bytes_free_at_fault": free_bytes,
        "io_poisoned": poisoned_at is not None and end.get("health") == "Failed(IoPoisoned)",
        "output_failed_sticks": end["outputs"][0]["failed"] is True,
        "rt_keeps_accepting": later["counts"]["control"] > at_poison["counts"]["control"],
        "failures_keep_counting": output(later).get("failed_records", 0)
        > output(at_poison).get("failed_records", 0),
        "failed_records": failed,
        "report_failed_counts": report_failed,
        "memory_failed_matches_report": failed in report_failed,
        "shutdown_not_success_expected": end["success"] is False,
    }
    common_checks(soak, end, checks, code, clean=False)
    return marks, checks, {"exit": code, "end": end}


def f4(args, out):
    """Network: a proxy between subscriber and logger stopped, then killed."""
    parent = pathlib.Path(args.shm_dir) / "f4"
    parent.mkdir(parents=True)
    port, proxy_port = args.port, args.port + 1
    total = args.pre + 2 * args.f4_fault + 90 + args.post
    soak = Soak(args, out, "soak", parent, total, 256, ["--remote", str(port)])
    marks = []

    def proxy():
        return subprocess.Popen(["socat", f"TCP-LISTEN:{proxy_port},reuseaddr,fork",
                                 f"TCP:127.0.0.1:{port}"], stderr=subprocess.DEVNULL)

    soak.start_event()
    socat = proxy()
    time.sleep(0.5)
    phase(marks, "pre")
    tail = Tail(args, out, "tail-1", proxy_port)
    wait_samples(soak, args.pre)
    checks = {"receives_through_proxy": tail.kinds().get("record", 0) > 0}
    phase(marks, "proxy stopped (silent)")
    # SIGSTOP the whole socat process group: no packets, no FIN.
    socat.send_signal(signal.SIGSTOP)
    for child in children(socat.pid):
        os.kill(child, signal.SIGSTOP)
    time.sleep(args.f4_fault)
    checks["silent_stop_detected"] = tail.ended()
    checks["silent_stop_outcome"] = tail.outcome()
    for child in children(socat.pid):
        os.kill(child, signal.SIGCONT)
    socat.send_signal(signal.SIGCONT)
    # The server closed the stalled subscription after max_age; once packets
    # flow again the subscriber should learn that.
    time.sleep(10)
    checks["after_resume_ended"] = tail.ended()
    checks["after_resume_outcome"] = tail.outcome()
    tail.stop()
    phase(marks, "reconnect")
    tail = Tail(args, out, "tail-2", proxy_port)
    wait_samples(soak, 30)
    checks["receives_after_silent_stop"] = tail.kinds().get("record", 0) > 0
    phase(marks, "proxy killed")
    for child in children(socat.pid):
        os.kill(child, signal.SIGKILL)
    socat.kill()
    socat.wait()
    time.sleep(args.f4_fault)
    checks["kill_detected"] = tail.ended()
    checks["kill_outcome"] = tail.outcome()
    tail.stop()
    phase(marks, "proxy back, reconnect")
    socat = proxy()
    time.sleep(0.5)
    tail = Tail(args, out, "tail-3", proxy_port)
    wait_samples(soak, 30)
    checks["receives_after_kill"] = tail.kinds().get("record", 0) > 0
    tail.stop()
    socat.kill()
    socat.wait()
    code = soak.wait(args.post + 120)
    end = soak.lines.wait_for("end", 1)
    control, frames = counts(end)
    found = inspect(args, parent, control, frames)
    checks.update({
        "file_failed_records_0": end["outputs"][0]["failed_records"] == 0,
        "file_rejected_0": end["counts"]["control_rejected"] + end["counts"]["frames_rejected"] == 0,
        "files_gapless": holes(found) == 0 and found["found"]["torn_files"] == 0,
        "shutdown_success": end["success"] is True,
    })
    common_checks(soak, end, checks, code)
    shutil.rmtree(parent, ignore_errors=True)
    return marks, checks, {"exit": code, "end": end, "inspect": found}


def children(pid):
    try:
        out = subprocess.run(["pgrep", "-P", str(pid)], capture_output=True, text=True).stdout
    except OSError:
        return []
    return [int(x) for x in out.split()]


def sync_completions(samples, merge_s=0.05):
    """Sync completion times in seconds on the soak's own clock: each
    sample's `t_ms - since_sync_ms`, so stdout delivery does not move them
    (RV-204). Samples seeing the same completion agree to a few ms."""
    times = sorted((v["t_ms"] - output(v)["since_sync_ms"]) / 1000 for v in samples
                   if "t_ms" in v and output(v).get("since_sync_ms") is not None)
    completions = []
    for t in times:
        if not completions or t - completions[-1] > merge_s:
            completions.append(round(t, 3))
    return completions


def f5(args, out):
    """SIGKILL without warning at several points, read back, restart.

    Scope (RV-204): SIGKILL keeps the page cache, so this shows what can be
    read after the process dies, not power-loss durability (that is the UPS
    power-cut test). The lower bound is the accepted count before the
    previous sync completed; accepted records may still have been queued,
    so it is a lenient estimate, not proof that the last sync was durable.
    Checks available before a kill: misses, worker state and heap samples
    from the last samples, and the exit by the intended SIGKILL."""
    parent = pathlib.Path(args.shm_dir) / "f5"
    parent.mkdir(parents=True)
    marks = []
    trials = []
    for i, delay in enumerate(args.f5_delays):
        trial_dir = parent / f"kill-{i}"
        trial_dir.mkdir()
        soak = Soak(args, out, f"soak-{i}", trial_dir, 3600, 1024)
        soak.start_event()
        phase(marks, f"trial {i}: kill after {delay}s")
        time.sleep(delay)
        killed_at = now()
        soak.process.send_signal(signal.SIGKILL)
        code = soak.wait(30)
        timed = [(at, v) for at, v in soak.lines.snapshot() if v.get("event") == "sample"]
        samples = [v for _, v in timed]
        last_at, last = timed[-1]
        completions = sync_completions(samples)
        lower = (0, 0)
        if len(completions) >= 2:
            before = [v for v in samples if v["t_ms"] / 1000 <= completions[-2]]
            if before:
                lower = counts(before[-1])
        since_last = killed_at - last_at + 0.5
        upper = (counts(last)[0] + int(CONTROL_HZ * since_last) + 100,
                 counts(last)[1] + int(FRAME_HZ * since_last) + 2)
        found = inspect(args, trial_dir, upper[0], upper[1], active=True)
        c, f = found["control"], found["frames"]
        trial = {
            "delay_s": delay, "syncs_seen": len(completions),
            "lower": lower, "upper": upper,
            "control_seen": c["seen"], "frames_seen": f["seen"],
            "starts_at_0": c["first"] in (0, None) and f["first"] in (0, None),
            "no_holes": c["holes"] == 0 and f["holes"] == 0,
            "at_least_before_previous_sync": c["seen"] >= lower[0] and f["seen"] >= lower[1],
            "closed_files_intact": found["found"]["torn_files"] == 0
            and found["found"]["unreadable_files"] == 0,
            "torn_active": found["torn_active"], "active_files": found["active_files"],
            "misses_0": last["counts"]["misses"] == 0,
            "killed_by_sigkill": code == -signal.SIGKILL,
            "worker_alive": not any(v.get("worker_failed") for v in samples),
            "heap_samples": any(v.get("heap") for v in samples),
        }
        trial["pass"] = all(trial[k] for k in (
            "starts_at_0", "no_holes", "at_least_before_previous_sync", "closed_files_intact",
            "misses_0", "killed_by_sigkill", "worker_alive", "heap_samples")) \
            and len(completions) >= 2
        trials.append(trial)
    # Restart in the same parent: a new session directory, a clean run.
    phase(marks, "restart")
    restart = Soak(args, out, "soak-restart", parent / "kill-0", 10, 1024)
    code = restart.wait(60)
    end = restart.lines.wait_for("end", 1)
    sessions = [p for p in (parent / "kill-0").iterdir() if p.is_dir()]
    checks = {
        "trials_passed": sum(t["pass"] for t in trials),
        "trials": len(trials),
        "all_trials_pass": all(t["pass"] for t in trials),
        "restart_new_session": len(sessions) == 2,
        "restart_success": code == 0 and end is not None and end["success"] is True,
    }
    shutil.rmtree(parent, ignore_errors=True)
    return marks, checks, {"trials": trials, "restart_end": end}


# --- f6: InfluxDB outage (TASK-047) -------------------------------------------

def f6_window(batches, fault_start_ms, fault_end_ms, recover_s, grace_ms=1000):
    """Judge the writer's batch log around one DB fault (unix ms). A batch
    that ends inside the fault (after `grace_ms`, the time Docker needs to
    apply it) must not be Acknowledged: no DB could answer then. A batch begun
    during the fault may end after it, for example a retry the resumed DB
    answers; those are reported apart (`straddling_outcomes`). The first
    Acknowledged batch begun after the fault must end within `recover_s`, and
    every later batch must be Acknowledged. `Busy` is the writer's bounded
    refusal and is allowed; other preflight errors are not."""
    attempts = [b for b in batches if "at_unix_ms" in b]

    def ends(b):
        return b["at_unix_ms"] + b.get("ms", 0)

    def tally(group):
        counted = {}
        for b in group:
            key = b.get("outcome") or b.get("error")
            counted[key] = counted.get(key, 0) + 1
        return counted

    fault = [b for b in attempts if fault_start_ms + grace_ms <= ends(b) < fault_end_ms]
    straddling = [b for b in attempts if b["at_unix_ms"] < fault_end_ms <= ends(b)
                  and b["at_unix_ms"] >= fault_start_ms]
    outcomes = tally(fault)
    after = [b for b in attempts if b["at_unix_ms"] >= fault_end_ms]
    first = next((b for b in after if b.get("outcome") == "Acknowledged"), None)
    recovery_s = None if first is None else (first["at_unix_ms"] + first.get("ms", 0) - fault_end_ms) / 1000
    later = [b for b in after if first is not None and b["at_unix_ms"] > first["at_unix_ms"]]
    return {
        "pre_acknowledged": any(b.get("outcome") == "Acknowledged" and b["at_unix_ms"] < fault_start_ms
                                for b in attempts),
        "fault_attempted": bool(fault),
        "fault_no_ack": not any(b.get("outcome") == "Acknowledged" for b in fault),
        "fault_outcomes": outcomes,
        "straddling_outcomes": tally(straddling),
        "recovered": recovery_s is not None and recovery_s <= recover_s,
        "recovery_s": recovery_s,
        "post_all_acknowledged": first is not None and all(b.get("outcome") == "Acknowledged" for b in later),
        "worker_errors_0": all(b.get("error") in (None, "Busy") for b in attempts),
    }


def f6_db_checks(batches, stored):
    """Channel 1 stores each point's index. Every Acknowledged point must be
    stored, and a stored point must come from an Acknowledged or Unknown
    batch (never from a skipped, NotSent, Rejected or refused one)."""
    acked, unknown = set(), set()
    for b in batches:
        if b.get("outcome") == "Acknowledged":
            acked.update(range(b["from"], b["to"]))
        elif b.get("outcome") == "Unknown":
            unknown.update(range(b["from"], b["to"]))
    return {
        "db_acked_present": acked <= stored,
        "db_acked_missing": len(acked - stored),
        "db_only_attempted": stored <= acked | unknown,
        "db_unexpected": len(stored - acked - unknown),
        "db_unknown_stored": len(stored & unknown),
        "db_points": len(stored),
    }


def flux_values(csv_text):
    """`_value` of every row of a Flux CSV answer (one or more tables)."""
    values, column = set(), None
    for line in csv_text.splitlines():
        cells = line.split(",")
        if not line.strip():
            column = None
        elif column is None:
            column = cells.index("_value")
        else:
            values.add(int(cells[column]))
    return values


def influx_pin():
    spec = importlib.util.spec_from_file_location("check_influxdb", ROOT / "scripts/check-influxdb.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.IMAGE, module.VERSION


class Influx:
    """One InfluxDB 2.9.1 container on a fixed loopback port, data in /dev/shm."""

    def __init__(self, args, name):
        import secrets
        image, self.version = influx_pin()
        self.port = args.port + 10
        self.container = f"dot-logger-s2-{name}-{os.getpid()}"
        self.data = pathlib.Path(args.shm_dir) / f"{name}-influx"
        out = subprocess.run(["stat", "-f", "-c", "%T", args.shm_dir], capture_output=True, text=True)
        if out.stdout.strip() != "tmpfs":
            raise RuntimeError(f"{args.shm_dir} is not tmpfs; the DB must not write to disk")
        shutil.rmtree(self.data, ignore_errors=True)
        (self.data / "data").mkdir(parents=True)
        (self.data / "etc").mkdir()
        subprocess.run(["docker", "run", "-d", "--name", self.container,
                        "--label", "dot-logger.test=tc008", "--cpus", "2", "--memory", "512m",
                        "--pids-limit", "128", "-p", f"127.0.0.1:{self.port}:8086",
                        "-v", f"{self.data / 'data'}:/var/lib/influxdb2",
                        "-v", f"{self.data / 'etc'}:/etc/influxdb2", image],
                       check=True, capture_output=True)
        self.endpoint = f"http://127.0.0.1:{self.port}"
        self.opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        self.wait_ready(45)
        self.token = secrets.token_urlsafe(48)
        self.org, self.bucket = "dot-logger-test", "soak"
        setup = {"username": "soak", "password": secrets.token_urlsafe(24), "org": self.org,
                 "bucket": self.bucket, "token": self.token, "retentionPeriodSeconds": 0}
        request = urllib.request.Request(self.endpoint + "/api/v2/setup", data=json.dumps(setup).encode(),
                                         headers={"Content-Type": "application/json"})
        with self.opener.open(request, timeout=15) as response:
            assert response.status == 201

    def env(self):
        return {"INFLUX_HOST": self.endpoint, "INFLUX_ORG": self.org,
                "INFLUX_BUCKET": self.bucket, "INFLUX_TOKEN": self.token}

    def wait_ready(self, seconds):
        end = now() + seconds
        while now() < end:
            try:
                with self.opener.open(self.endpoint + "/health", timeout=1) as response:
                    health = json.load(response)
                if health.get("status") == "pass" and health.get("version", "").lstrip("v") == self.version:
                    return True
            except (OSError, ValueError, urllib.error.URLError):
                pass
            time.sleep(0.2)
        return False

    def docker(self, *command):
        subprocess.run(["docker", *command], check=True, capture_output=True)

    def stored_indices(self):
        flux = ('from(bucket: "soak") |> range(start: 0) '
                '|> filter(fn: (r) => r._measurement == "soak_db" and r._field == "integer" and r.channel == "1") '
                '|> keep(columns: ["_value"])')
        request = urllib.request.Request(
            self.endpoint + "/api/v2/query?org=" + self.org, data=flux.encode(),
            headers={"Authorization": "Token " + self.token, "Content-Type": "application/vnd.flux",
                     "Accept": "text/csv"})
        with self.opener.open(request, timeout=120) as response:
            return flux_values(response.read().decode())

    def remove(self):
        subprocess.run(["docker", "rm", "-f", self.container], capture_output=True)
        shutil.rmtree(self.data, ignore_errors=True)


F6_FAULTS = {
    "f6a": (lambda db: db.docker("pause", db.container), lambda db: db.docker("unpause", db.container)),
    "f6b": (lambda db: db.docker("stop", db.container), lambda db: db.docker("start", db.container)),
    "f6c": (lambda db: db.docker("network", "disconnect", "bridge", db.container),
            lambda db: db.docker("network", "connect", "bridge", db.container)),
}


def f6(args, out, name):
    """InfluxDB outage beside the soak load; the logger itself has no DB output."""
    parent = pathlib.Path(args.shm_dir) / name
    parent.mkdir(parents=True)
    db = Influx(args, name)
    marks = []
    try:
        log = out / "batches.jsonl"
        total = args.pre + args.f6_fault + args.post + 20
        soak = Soak(args, out, "soak", parent, total, 256, ["--influx", str(log)], env_extra=db.env())
        soak.start_event()
        phase(marks, "pre")
        wait_samples(soak, args.pre)
        begin, end_fault = F6_FAULTS[name]
        phase(marks, "fault")
        fault_start = time.time()
        begin(db)
        wait_samples(soak, args.f6_fault)
        phase(marks, "recover")
        fault_end = time.time()
        end_fault(db)
        checks = {"db_ready_after_fault": db.wait_ready(60)}
        code = soak.wait(total + 120)
        end = soak.lines.wait_for("end", 1)
        control, frames = counts(end)
        found = inspect(args, parent, control, frames)
        batches = [json.loads(line) for line in log.read_text().splitlines()]
        checks.update(f6_window(batches, fault_start * 1000, fault_end * 1000, args.f6_recover))
        checks.update(f6_db_checks(batches, db.stored_indices()))
        influx = end.get("influx") or {}
        checks.update({
            "batch_log_complete": bool(batches) and batches[-1].get("end") is True,
            "file_failed_records_0": end["outputs"][0]["failed_records"] == 0,
            "file_rejected_0": end["counts"]["control_rejected"] + end["counts"]["frames_rejected"] == 0,
            "files_gapless": holes(found) == 0 and found["found"]["torn_files"] == 0,
            "shutdown_success": end["success"] is True,
            "influx_points_skipped": influx.get("points_skipped"),
            "influx_max_call_ms": influx.get("max_call_ms"),
            "influx_busy": influx.get("busy"),
        })
        common_checks(soak, end, checks, code)
        return marks, checks, {"exit": code, "end": end, "inspect": found,
                               "fault_unix": [fault_start, fault_end]}
    finally:
        db.remove()
        shutil.rmtree(parent, ignore_errors=True)


SCENARIOS = {"f1": f1, "f2": f2, "f3a": f3a, "f3b": f3b, "f4": f4, "f5": f5,
             "f6a": lambda a, o: f6(a, o, "f6a"), "f6b": lambda a, o: f6(a, o, "f6b"),
             "f6c": lambda a, o: f6(a, o, "f6c")}
# Values that are observations, not pass/fail conditions.
OBSERVED = {"rejected_during_fault", "failed_records", "heap_growth_bytes",
            "f2a_gap_events_after_stops", "f2a_disconnect_outcome", "f2b_gap_outcome",
            "f2b_disconnect_outcome", "silent_stop_outcome", "kill_outcome",
            "after_resume_ended", "after_resume_outcome",
            "report_failed_counts", "trials_passed", "trials",
            "bytes_free_at_fault", "fault_outcomes", "straddling_outcomes", "recovery_s", "db_acked_missing",
            "db_unexpected", "db_unknown_stored", "db_points", "influx_points_skipped",
            "influx_max_call_ms", "influx_busy"}

def judge(name, checks):
    """Pass only when every pass/fail check is True and, except for F5 (its
    own per-trial checks), every shared check is present."""
    if name != "f5" and any(k not in checks for k in REQUIRED):
        return False
    return all(v is True for k, v in checks.items()
               if k not in OBSERVED and isinstance(v, bool))


marks_start = [0.0]
args_global = None


def main():
    global args_global
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("scenarios", nargs="+", choices=sorted(SCENARIOS))
    parser.add_argument("--out", required=True, help="result directory (created)")
    parser.add_argument("--soak", default=TARGET / "release/examples/soak", type=pathlib.Path)
    parser.add_argument("--cli", default=TARGET / "release/dot-logger", type=pathlib.Path)
    parser.add_argument("--image", default="postgres:16",
                        help="any local image with chroot; the host binaries run")
    parser.add_argument("--disk-dir", default=str(pathlib.Path.home() / "s3-data/task024-s2"),
                        help="f1 writes here; f3a/f3b mount a tmpfs below it")
    parser.add_argument("--shm-dir", default="/dev/shm/task024-s2")
    parser.add_argument("--device", default="259:4", help="MAJ:MIN of the disk under --disk-dir")
    parser.add_argument("--no-rt", dest="rt", action="store_false")
    parser.add_argument("--sample-ms", type=int, default=200)
    parser.add_argument("--port", type=int, default=47301)
    parser.add_argument("--pre", type=int, default=60)
    parser.add_argument("--post", type=int, default=60)
    parser.add_argument("--f1-fault", type=int, default=180)
    parser.add_argument("--f1-bps", type=int, default=1 << 20)
    parser.add_argument("--f2-stops", type=int, default=3)
    parser.add_argument("--f2-stop", type=int, default=8)
    parser.add_argument("--f2-gap", type=int, default=20)
    parser.add_argument("--f2-long", type=int, default=120)
    parser.add_argument("--f3-fault", type=int, default=120)
    parser.add_argument("--tmpfs-mib", type=int, default=512)
    parser.add_argument("--f4-fault", type=int, default=15)
    parser.add_argument("--f5-delays", type=lambda s: [int(x) for x in s.split(",")],
                        default=[7, 13, 19, 23, 29, 37])
    parser.add_argument("--f6-fault", type=int, default=600, help="seconds of each DB outage")
    parser.add_argument("--f6-recover", type=int, default=30,
                        help="seconds after the outage within which writes must be acknowledged again")
    parser.add_argument("--heap-slack-mib", type=int, default=16)
    parser.add_argument("--keep", action="store_true", help="keep f1 files on disk")
    args = parser.parse_args()
    args_global = args
    base = pathlib.Path(args.out)
    base.mkdir(parents=True)
    pathlib.Path(args.shm_dir).mkdir(parents=True, exist_ok=True)
    pathlib.Path(args.disk_dir).mkdir(parents=True, exist_ok=True)
    failed = 0
    for name in args.scenarios:
        out = base / name
        out.mkdir()
        marks_start[0] = now()
        started = time.time()
        try:
            marks, checks, detail = SCENARIOS[name](args, out)
            verdict = judge(name, checks)
            error = None
        except Exception as e:  # report and go on with the next scenario
            marks, checks, detail, verdict, error = [], {}, {}, False, repr(e)
        finally:
            leftover = subprocess.run(
                ["docker", "ps", "-aq", "--filter", f"name=dot-logger-s2-.*-{os.getpid()}$"],
                capture_output=True, text=True).stdout.split()
            if leftover:
                subprocess.run(["docker", "rm", "-f", *leftover], capture_output=True)
        result = {"scenario": name, "pass": verdict, "error": error,
                  "started_unix": started, "seconds": round(time.time() - started, 1),
                  "phases": marks, "checks": checks, "detail": detail}
        (out / "result.json").write_text(json.dumps(result, indent=2, default=str))
        failed += not verdict
        print(json.dumps({"scenario": name, "pass": verdict, "error": error,
                          "seconds": result["seconds"], "checks": checks}, default=str),
              flush=True)
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
