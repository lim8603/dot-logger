//! Explicit machine calibration for presets.
//!
//! [`Calibration::run`] is meant for installation or commissioning: it writes
//! a bounded amount of data to a dedicated directory on the filesystem the
//! logger will use, syncs it, removes it, and returns a [`Profile`]. Nothing
//! runs at logger start and nothing is applied without
//! [`Sizing::profile`](crate::logger::Sizing::profile).
//!
//! A profile is a snapshot, not a guarantee: page cache, temperature, CPU
//! frequency and the device's own state change its numbers. A short run
//! overstates the sustained rate of SSDs with a write cache, so the sustained
//! rate is only ever an [`Estimate`] with its assumption spelled out. Values
//! that could not be read are [`Fact::Unavailable`], never zero.
//!
//! [`Calibration::run`]: crate::calibrate::Calibration::run
//! [`Profile`]: crate::calibrate::Profile
//! [`Estimate`]: crate::calibrate::Estimate
//! [`Fact::Unavailable`]: crate::calibrate::Fact::Unavailable

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

const FORMAT: &str = "dot-logger-calibration";
const VERSION: u64 = 1;
const GIB: u64 = 1024 * 1024 * 1024;
/// Write unit; large enough to stream, small enough to stop promptly.
const BLOCK: u64 = 8 * 1024 * 1024;
/// Data sync period during the run, the presets' ordinary sync period.
const SYNC_EVERY: Duration = Duration::from_secs(1);
/// Absorb = longest observed IO stall × this margin.
const STALL_MARGIN: u32 = 2;
/// Declared load above this fraction of the sustained estimate is warned
/// about (TASK-023's normal load is 0.6 × S).
const LOAD_NUM: u64 = 3;
const LOAD_DEN: u64 = 5;
const ASSUMPTION: &str = "half the burst rate of a short run; an SSD write cache \
     and the page cache make a short run overstate the sustained rate";

/// A bounded, explicitly requested write-and-sync measurement.
#[derive(Clone, Debug)]
pub struct Calibration {
    directory: PathBuf,
    write_limit: u64,
    time_limit: Duration,
    disk_reserve: u64,
}

impl Calibration {
    /// Measure the filesystem holding `directory`, an existing absolute
    /// directory. Defaults: write at most 4 GiB or 15 s, keep 1 GiB free.
    pub fn new(directory: impl AsRef<Path>) -> Self {
        Self {
            directory: directory.as_ref().to_path_buf(),
            write_limit: 4 * GIB,
            time_limit: Duration::from_secs(15),
            disk_reserve: GIB,
        }
    }
    /// Most bytes to write (non-zero).
    pub fn write_limit(mut self, bytes: u64) -> Self {
        self.write_limit = bytes;
        self
    }
    /// Longest time to keep writing (non-zero); the run stops after the
    /// first block that reaches it, then syncs.
    pub fn time_limit(mut self, limit: Duration) -> Self {
        self.time_limit = limit;
        self
    }
    /// Free space the run must leave; the write limit shrinks to fit.
    pub fn disk_reserve(mut self, bytes: u64) -> Self {
        self.disk_reserve = bytes;
        self
    }

    /// Write, sync and remove a probe file below a new directory inside
    /// `directory`. `cancel` is checked before every block. The probe is
    /// removed on success, error and cancellation alike.
    pub fn run(&self, cancel: &AtomicBool) -> Result<Profile, CalibrateError> {
        let valid_dir = self.directory.is_absolute()
            && self.directory.file_name().is_some()
            && !self
                .directory
                .components()
                .any(|c| c == Component::ParentDir);
        if self.write_limit == 0 || self.time_limit.is_zero() || !valid_dir {
            return Err(CalibrateError::InvalidInput);
        }
        if cancel.load(Ordering::Acquire) {
            return Err(CalibrateError::Cancelled);
        }
        let free = crate::diagnostics::linux_available_bytes(&self.directory)
            .map_err(CalibrateError::Io)?;
        let room = free.saturating_sub(self.disk_reserve);
        if room == 0 {
            return Err(CalibrateError::NoSpace {
                available: free,
                reserve: self.disk_reserve,
            });
        }
        let (limit, limit_stop) = if room < self.write_limit {
            (room, Stop::DiskReserve)
        } else {
            (self.write_limit, Stop::WriteLimit)
        };
        let host = host::read(Path::new("/"), &self.directory, free);
        let probe = Probe::create(&self.directory).map_err(CalibrateError::Io)?;
        let write = self.measure(&probe.file()?, limit, limit_stop, cancel);
        let cleaned = probe.remove();
        let write = write?;
        cleaned.map_err(CalibrateError::Io)?;
        Ok(Profile {
            directory: self.directory.clone(),
            measured_unix_s: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
            host,
            write,
        })
    }

    fn measure(
        &self,
        file: &File,
        limit: u64,
        limit_stop: Stop,
        cancel: &AtomicBool,
    ) -> Result<WriteMeasurement, CalibrateError> {
        let block = pattern(limit.min(BLOCK));
        let mut file: &File = file;
        let mut m = WriteMeasurement {
            bytes: 0,
            elapsed: Duration::ZERO,
            syncs: 0,
            sync_max: Duration::ZERO,
            write_call_max: Duration::ZERO,
            stopped_by: limit_stop,
        };
        let sync = |file: &File, m: &mut WriteMeasurement| -> io::Result<()> {
            let t = Instant::now();
            file.sync_data()?;
            m.sync_max = m.sync_max.max(t.elapsed());
            m.syncs += 1;
            Ok(())
        };
        let start = Instant::now();
        let mut last_sync = start;
        loop {
            if cancel.load(Ordering::Acquire) {
                return Err(CalibrateError::Cancelled);
            }
            let n = usize::try_from((limit - m.bytes).min(BLOCK))
                .map_err(|_| CalibrateError::InvalidInput)?;
            let t = Instant::now();
            file.write_all(&block[..n]).map_err(CalibrateError::Io)?;
            m.write_call_max = m.write_call_max.max(t.elapsed());
            m.bytes += n as u64;
            if last_sync.elapsed() >= SYNC_EVERY {
                sync(file, &mut m).map_err(CalibrateError::Io)?;
                last_sync = Instant::now();
            }
            if m.bytes >= limit {
                break;
            }
            if start.elapsed() >= self.time_limit {
                m.stopped_by = Stop::TimeLimit;
                break;
            }
        }
        sync(file, &mut m).map_err(CalibrateError::Io)?;
        m.elapsed = start.elapsed();
        Ok(m)
    }
}

/// Incompressible-looking bytes, so a compressing device is not flattered.
fn pattern(len: u64) -> Vec<u8> {
    let mut x: u64 = 0x9e37_79b9_7f4a_7c15;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x.to_le_bytes()[0]
        })
        .collect()
}

/// Exclusive measurement directory and its probe file; dropping removes both.
struct Probe {
    dir: PathBuf,
    removed: bool,
}
impl Probe {
    fn create(parent: &Path) -> io::Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = parent.join(format!(
            "dot-logger-calibrate-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir)?;
        Ok(Self {
            dir,
            removed: false,
        })
    }
    fn path(&self) -> PathBuf {
        self.dir.join("probe.bin")
    }
    fn file(&self) -> Result<File, CalibrateError> {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.path())
            .map_err(CalibrateError::Io)
    }
    fn remove(mut self) -> io::Result<()> {
        match fs::remove_file(self.path()) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
        fs::remove_dir(&self.dir)?;
        self.removed = true;
        Ok(())
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        if !self.removed {
            let _ = fs::remove_file(self.path());
            let _ = fs::remove_dir(&self.dir);
        }
    }
}

/// Why a calibration produced no profile. The probe file is gone in every case.
#[derive(Debug)]
pub enum CalibrateError {
    /// Zero limit, or a relative, root or `..` directory.
    InvalidInput,
    /// The reserve leaves no room for a single byte.
    NoSpace {
        /// Free bytes observed before writing.
        available: u64,
        /// Requested reserve.
        reserve: u64,
    },
    /// The cancel flag was set.
    Cancelled,
    /// Filesystem error, including an unsupported free-space probe.
    Io(io::Error),
}
impl std::fmt::Display for CalibrateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("calibration: invalid limit or directory"),
            Self::NoSpace { available, reserve } => write!(
                f,
                "calibration: {available} bytes free leave nothing above the {reserve}-byte reserve"
            ),
            Self::Cancelled => f.write_str("calibration: cancelled"),
            Self::Io(e) => write!(f, "calibration: {e}"),
        }
    }
}
impl std::error::Error for CalibrateError {}

/// Which limit ended the write phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stop {
    /// The requested byte limit.
    WriteLimit,
    /// The requested time limit.
    TimeLimit,
    /// The byte limit shrunk to stay above the disk reserve.
    DiskReserve,
}
impl Stop {
    fn name(self) -> &'static str {
        match self {
            Self::WriteLimit => "WriteLimit",
            Self::TimeLimit => "TimeLimit",
            Self::DiskReserve => "DiskReserve",
        }
    }
}

/// Observed results of the write phase.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteMeasurement {
    /// Bytes written.
    pub bytes: u64,
    /// From the first write to the end of the final data sync.
    pub elapsed: Duration,
    /// Data syncs, including the final one.
    pub syncs: u32,
    /// Longest single data sync.
    pub sync_max: Duration,
    /// Longest single block write call (dirty-page throttling shows here).
    pub write_call_max: Duration,
    /// Limit that ended the run.
    pub stopped_by: Stop,
}

/// A host value as read, or why it could not be read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Fact<T> {
    /// Read from the operating system at measurement time.
    Observed(T),
    /// Not readable; never treated as zero or as success.
    Unavailable(String),
}

/// Host facts recorded with a profile. Only memory feeds a preset warning;
/// the rest is recorded for the deployment record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Host {
    /// Kernel release.
    pub kernel: Fact<String>,
    /// Whether the running kernel is PREEMPT_RT.
    pub preempt_rt: Fact<bool>,
    /// Online CPU identifiers.
    pub cpus_online: Fact<Vec<usize>>,
    /// CPUs isolated from the scheduler.
    pub cpus_isolated: Fact<Vec<usize>>,
    /// CPU frequency governor of CPU 0.
    pub governor: Fact<String>,
    /// Physical memory.
    pub memory_total_bytes: Fact<u64>,
    /// Memory available without swapping, at measurement time.
    pub memory_available_bytes: Fact<u64>,
    /// This process's cgroup memory limit; `None` when unlimited.
    pub cgroup_memory_max_bytes: Fact<Option<u64>>,
    /// This process's cgroup memory use.
    pub cgroup_memory_current_bytes: Fact<u64>,
    /// Filesystem type of the measured directory.
    pub filesystem: Fact<String>,
    /// Free bytes of that filesystem before the run.
    pub free_bytes: Fact<u64>,
}

/// A model value with its assumption.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Estimate {
    /// Estimated sustained write rate.
    pub bytes_per_second: u64,
    /// What the estimate assumes.
    pub assumption: &'static str,
}

/// Result of one calibration: what was measured, where, and the host facts.
/// Derived values are recomputed from these on load.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Profile {
    directory: PathBuf,
    measured_unix_s: u64,
    host: Host,
    write: WriteMeasurement,
}

impl Profile {
    /// Measured directory; the profile fits outputs on the same device.
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    /// Wall-clock time of the measurement, seconds since the Unix epoch.
    pub fn measured_unix_s(&self) -> u64 {
        self.measured_unix_s
    }
    /// Host facts.
    pub fn host(&self) -> &Host {
        &self.host
    }
    /// Write phase results.
    pub fn write(&self) -> &WriteMeasurement {
        &self.write
    }
    /// Observed rate of the run, data syncs included.
    pub fn burst_bytes_per_second(&self) -> u64 {
        let ns = self.write.elapsed.as_nanos().max(1);
        u64::try_from(u128::from(self.write.bytes) * 1_000_000_000 / ns).unwrap_or(u64::MAX)
    }
    /// Sustained write rate: half the burst rate (see the assumption).
    pub fn sustained_estimate(&self) -> Estimate {
        Estimate {
            bytes_per_second: self.burst_bytes_per_second() / 2,
            assumption: ASSUMPTION,
        }
    }
    /// Longest single IO stall observed (data sync or block write).
    pub fn stall(&self) -> Duration {
        self.write.sync_max.max(self.write.write_call_max)
    }
    /// Absorb time this profile asks of a preset: twice the longest stall.
    pub fn absorb(&self) -> Duration {
        self.stall().saturating_mul(STALL_MARGIN)
    }
    /// Declared load above this rate is warned about: 0.6 × sustained.
    pub fn load_limit_bytes_per_second(&self) -> u64 {
        let sustained = u128::from(self.sustained_estimate().bytes_per_second);
        u64::try_from(sustained * u128::from(LOAD_NUM) / u128::from(LOAD_DEN)).unwrap_or(u64::MAX)
    }
    /// Memory the logger can count on: `MemAvailable`, lowered to the cgroup
    /// headroom when both cgroup values were read. `None` when unread.
    pub fn available_memory_bytes(&self) -> Option<u64> {
        let Fact::Observed(available) = self.host.memory_available_bytes else {
            return None;
        };
        let cgroup = match (
            &self.host.cgroup_memory_max_bytes,
            &self.host.cgroup_memory_current_bytes,
        ) {
            (Fact::Observed(Some(max)), Fact::Observed(current)) => {
                Some(max.saturating_sub(*current))
            }
            _ => None,
        };
        Some(cgroup.map_or(available, |c| c.min(available)))
    }

    /// Write the profile as JSON to a new file; an existing file is kept and
    /// reported as `AlreadyExists`.
    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(self.to_json().as_bytes())?;
        file.write_all(b"\n")?;
        file.sync_all()
    }
    /// Read a profile written by [`save`](Self::save).
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ProfileError> {
        let text = fs::read_to_string(path).map_err(|e| ProfileError::Io(e.kind()))?;
        Self::from_json(&text)
    }
    /// JSON text of format version 1, with derived values for readers.
    pub fn to_json(&self) -> String {
        let w = &self.write;
        let h = &self.host;
        let sustained = self.sustained_estimate();
        let value = json!({
            "format": FORMAT,
            "version": VERSION,
            "measured_unix_s": self.measured_unix_s,
            "directory": self.directory.to_string_lossy(),
            "host": {
                "kernel": fact(&h.kernel, |v| json!(v)),
                "preempt_rt": fact(&h.preempt_rt, |v| json!(v)),
                "cpus_online": fact(&h.cpus_online, |v| json!(v)),
                "cpus_isolated": fact(&h.cpus_isolated, |v| json!(v)),
                "governor": fact(&h.governor, |v| json!(v)),
                "memory_total_bytes": fact(&h.memory_total_bytes, |v| json!(v)),
                "memory_available_bytes": fact(&h.memory_available_bytes, |v| json!(v)),
                "cgroup_memory_max_bytes": fact(&h.cgroup_memory_max_bytes, |v| json!(v)),
                "cgroup_memory_current_bytes": fact(&h.cgroup_memory_current_bytes, |v| json!(v)),
                "filesystem": fact(&h.filesystem, |v| json!(v)),
                "free_bytes": fact(&h.free_bytes, |v| json!(v)),
            },
            "write": {
                "bytes": w.bytes,
                "elapsed_ns": nanos(w.elapsed),
                "syncs": w.syncs,
                "sync_max_ns": nanos(w.sync_max),
                "write_call_max_ns": nanos(w.write_call_max),
                "stopped_by": w.stopped_by.name(),
            },
            "derived": {
                "note": "recomputed on load; a snapshot, not a guarantee",
                "burst_bytes_per_second": self.burst_bytes_per_second(),
                "sustained_bytes_per_second": {
                    "estimated": sustained.bytes_per_second,
                    "assumption": sustained.assumption,
                },
                "stall_ns": nanos(self.stall()),
                "absorb_ns": nanos(self.absorb()),
                "load_limit_bytes_per_second": self.load_limit_bytes_per_second(),
            },
        });
        serde_json::to_string_pretty(&value).unwrap_or_default()
    }
    /// Parse JSON text of format version 1. Derived values are ignored.
    pub fn from_json(text: &str) -> Result<Self, ProfileError> {
        let root: Value = serde_json::from_str(text).map_err(|_| ProfileError::Syntax)?;
        if root.get("format").and_then(Value::as_str) != Some(FORMAT) {
            return Err(ProfileError::Format);
        }
        let version = u64_at(&root, "version", "version")?;
        if version != VERSION {
            return Err(ProfileError::Version(version));
        }
        let host = obj(&root, "host", "host")?;
        let write = obj(&root, "write", "write")?;
        let stopped_by = match write.get("stopped_by").and_then(Value::as_str) {
            Some("WriteLimit") => Stop::WriteLimit,
            Some("TimeLimit") => Stop::TimeLimit,
            Some("DiskReserve") => Stop::DiskReserve,
            _ => return Err(ProfileError::Field("write.stopped_by")),
        };
        Ok(Self {
            directory: PathBuf::from(
                root.get("directory")
                    .and_then(Value::as_str)
                    .ok_or(ProfileError::Field("directory"))?,
            ),
            measured_unix_s: u64_at(&root, "measured_unix_s", "measured_unix_s")?,
            host: Host {
                kernel: read_fact(host, "kernel", "host.kernel", |v| {
                    v.as_str().map(str::to_owned)
                })?,
                preempt_rt: read_fact(host, "preempt_rt", "host.preempt_rt", Value::as_bool)?,
                cpus_online: read_fact(host, "cpus_online", "host.cpus_online", cpu_list)?,
                cpus_isolated: read_fact(host, "cpus_isolated", "host.cpus_isolated", cpu_list)?,
                governor: read_fact(host, "governor", "host.governor", |v| {
                    v.as_str().map(str::to_owned)
                })?,
                memory_total_bytes: read_fact(
                    host,
                    "memory_total_bytes",
                    "host.memory_total_bytes",
                    Value::as_u64,
                )?,
                memory_available_bytes: read_fact(
                    host,
                    "memory_available_bytes",
                    "host.memory_available_bytes",
                    Value::as_u64,
                )?,
                cgroup_memory_max_bytes: read_fact(
                    host,
                    "cgroup_memory_max_bytes",
                    "host.cgroup_memory_max_bytes",
                    |v| {
                        if v.is_null() {
                            Some(None)
                        } else {
                            v.as_u64().map(Some)
                        }
                    },
                )?,
                cgroup_memory_current_bytes: read_fact(
                    host,
                    "cgroup_memory_current_bytes",
                    "host.cgroup_memory_current_bytes",
                    Value::as_u64,
                )?,
                filesystem: read_fact(host, "filesystem", "host.filesystem", |v| {
                    v.as_str().map(str::to_owned)
                })?,
                free_bytes: read_fact(host, "free_bytes", "host.free_bytes", Value::as_u64)?,
            },
            write: WriteMeasurement {
                bytes: u64_in(write, "bytes", "write.bytes")?,
                elapsed: Duration::from_nanos(u64_in(write, "elapsed_ns", "write.elapsed_ns")?),
                syncs: u32::try_from(u64_in(write, "syncs", "write.syncs")?)
                    .map_err(|_| ProfileError::Field("write.syncs"))?,
                sync_max: Duration::from_nanos(u64_in(write, "sync_max_ns", "write.sync_max_ns")?),
                write_call_max: Duration::from_nanos(u64_in(
                    write,
                    "write_call_max_ns",
                    "write.write_call_max_ns",
                )?),
                stopped_by,
            },
        })
    }
}

fn nanos(d: Duration) -> u64 {
    u64::try_from(d.as_nanos()).unwrap_or(u64::MAX)
}
fn fact<T>(f: &Fact<T>, to: impl Fn(&T) -> Value) -> Value {
    match f {
        Fact::Observed(v) => json!({ "observed": to(v) }),
        Fact::Unavailable(reason) => json!({ "unavailable": reason }),
    }
}
fn obj<'a>(
    v: &'a Value,
    key: &str,
    field: &'static str,
) -> Result<&'a Map<String, Value>, ProfileError> {
    v.get(key)
        .and_then(Value::as_object)
        .ok_or(ProfileError::Field(field))
}
fn u64_at(v: &Value, key: &str, field: &'static str) -> Result<u64, ProfileError> {
    v.get(key)
        .and_then(Value::as_u64)
        .ok_or(ProfileError::Field(field))
}
fn u64_in(m: &Map<String, Value>, key: &str, field: &'static str) -> Result<u64, ProfileError> {
    m.get(key)
        .and_then(Value::as_u64)
        .ok_or(ProfileError::Field(field))
}
fn cpu_list(v: &Value) -> Option<Vec<usize>> {
    v.as_array()?
        .iter()
        .map(|c| c.as_u64().and_then(|c| usize::try_from(c).ok()))
        .collect()
}
fn read_fact<T>(
    m: &Map<String, Value>,
    key: &str,
    field: &'static str,
    parse: impl Fn(&Value) -> Option<T>,
) -> Result<Fact<T>, ProfileError> {
    let entry = m
        .get(key)
        .and_then(Value::as_object)
        .ok_or(ProfileError::Field(field))?;
    if let Some(v) = entry.get("observed") {
        return parse(v)
            .map(Fact::Observed)
            .ok_or(ProfileError::Field(field));
    }
    entry
        .get("unavailable")
        .and_then(Value::as_str)
        .map(|r| Fact::Unavailable(r.to_owned()))
        .ok_or(ProfileError::Field(field))
}

/// Why a profile could not be read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileError {
    /// The file could not be read.
    Io(io::ErrorKind),
    /// Not JSON.
    Syntax,
    /// JSON, but not a dot-logger calibration profile.
    Format,
    /// A profile format version this build does not read.
    Version(u64),
    /// A required field is missing or has the wrong type.
    Field(&'static str),
}
impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "calibration profile: {self:?}")
    }
}
impl std::error::Error for ProfileError {}

mod host {
    //! Host facts read from procfs/sysfs below `root` ("/" in production).
    use super::{Fact, Host};
    use std::path::Path;

    pub(super) fn read(root: &Path, directory: &Path, free: u64) -> Host {
        #[cfg(target_os = "linux")]
        {
            linux(root, directory, free)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (root, directory, free);
            fn none<T>() -> Fact<T> {
                Fact::Unavailable("unsupported platform".into())
            }
            Host {
                kernel: none(),
                preempt_rt: none(),
                cpus_online: none(),
                cpus_isolated: none(),
                governor: none(),
                memory_total_bytes: none(),
                memory_available_bytes: none(),
                cgroup_memory_max_bytes: none(),
                cgroup_memory_current_bytes: none(),
                filesystem: none(),
                free_bytes: none(),
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn linux(root: &Path, directory: &Path, free: u64) -> Host {
        use std::io::ErrorKind;
        let text = |rel: &str| -> Fact<String> {
            match std::fs::read_to_string(root.join(rel)) {
                Ok(s) => Fact::Observed(s.trim().to_owned()),
                Err(e) => Fact::Unavailable(format!("{rel}: {e}")),
            }
        };
        let then = |f: Fact<String>, parse: &dyn Fn(&str) -> Option<u64>| -> Fact<u64> {
            match f {
                Fact::Observed(s) => parse(&s)
                    .map(Fact::Observed)
                    .unwrap_or_else(|| Fact::Unavailable(format!("unparsable: {s}"))),
                Fact::Unavailable(r) => Fact::Unavailable(r),
            }
        };
        let cpus = |rel: &str| match text(rel) {
            Fact::Observed(s) => parse_cpus(&s)
                .map(Fact::Observed)
                .unwrap_or_else(|| Fact::Unavailable(format!("unparsable: {s}"))),
            Fact::Unavailable(r) => Fact::Unavailable(r),
        };
        // The file exists only on PREEMPT_RT kernels.
        let preempt_rt = match std::fs::read_to_string(root.join("sys/kernel/realtime")) {
            Ok(s) => Fact::Observed(s.trim() == "1"),
            Err(e) if e.kind() == ErrorKind::NotFound => Fact::Observed(false),
            Err(e) => Fact::Unavailable(format!("sys/kernel/realtime: {e}")),
        };
        let meminfo = text("proc/meminfo");
        let mem = |key: &str| {
            then(meminfo.clone(), &|s: &str| {
                s.lines()
                    .find_map(|l| l.strip_prefix(key))
                    .and_then(|rest| rest.trim().strip_suffix("kB"))
                    .and_then(|kb| kb.trim().parse::<u64>().ok())
                    .and_then(|kb| kb.checked_mul(1024))
            })
        };
        // cgroup v2: the "0::<path>" line of /proc/self/cgroup.
        let cgroup = match text("proc/self/cgroup") {
            Fact::Observed(s) => s
                .lines()
                .find_map(|l| l.strip_prefix("0::"))
                .map(|p| Fact::Observed(format!("sys/fs/cgroup{p}")))
                .unwrap_or_else(|| Fact::Unavailable("no cgroup v2 entry".into())),
            Fact::Unavailable(r) => Fact::Unavailable(r),
        };
        let in_cgroup = |file: &str| match &cgroup {
            Fact::Observed(dir) => text(&format!("{dir}/{file}")),
            Fact::Unavailable(r) => Fact::Unavailable(r.clone()),
        };
        let cgroup_max = match in_cgroup("memory.max") {
            Fact::Observed(s) if s == "max" => Fact::Observed(None),
            Fact::Observed(s) => s
                .parse()
                .map(|n| Fact::Observed(Some(n)))
                .unwrap_or_else(|_| Fact::Unavailable(format!("unparsable: {s}"))),
            Fact::Unavailable(r) => Fact::Unavailable(r),
        };
        Host {
            kernel: text("proc/sys/kernel/osrelease"),
            preempt_rt,
            cpus_online: cpus("sys/devices/system/cpu/online"),
            cpus_isolated: cpus("sys/devices/system/cpu/isolated"),
            governor: text("sys/devices/system/cpu/cpu0/cpufreq/scaling_governor"),
            memory_total_bytes: mem("MemTotal:"),
            memory_available_bytes: mem("MemAvailable:"),
            cgroup_memory_max_bytes: cgroup_max,
            cgroup_memory_current_bytes: then(in_cgroup("memory.current"), &|s: &str| {
                s.parse().ok()
            }),
            filesystem: match rustix::fs::statfs(directory) {
                Ok(s) => Fact::Observed(fs_name(i128::from(s.f_type))),
                Err(e) => Fact::Unavailable(format!("statfs: {e}")),
            },
            free_bytes: Fact::Observed(free),
        }
    }

    /// "0-3,8,10-11" → [0, 1, 2, 3, 8, 10, 11]; empty → [].
    #[cfg(target_os = "linux")]
    pub(super) fn parse_cpus(s: &str) -> Option<Vec<usize>> {
        let mut cpus = Vec::new();
        for part in s.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            match part.split_once('-') {
                Some((a, b)) => {
                    let (a, b) = (a.parse::<usize>().ok()?, b.parse::<usize>().ok()?);
                    if a > b {
                        return None;
                    }
                    cpus.extend(a..=b);
                }
                None => cpus.push(part.parse().ok()?),
            }
        }
        Some(cpus)
    }

    #[cfg(target_os = "linux")]
    fn fs_name(magic: i128) -> String {
        match magic {
            0xEF53 => "ext4".into(),
            0x5846_5342 => "xfs".into(),
            0x9123_683E => "btrfs".into(),
            0x0102_1994 => "tmpfs".into(),
            0xF2F5_2010 => "f2fs".into(),
            0x2FC1_2FC1 => "zfs".into(),
            0x6969 => "nfs".into(),
            0x794C_7630 => "overlayfs".into(),
            other => format!("magic 0x{other:x}"),
        }
    }

    #[cfg(all(test, target_os = "linux"))]
    #[allow(clippy::unwrap_used)]
    mod tests {
        use super::*;

        #[test]
        fn cpu_lists_parse_ranges_and_reject_garbage() {
            assert_eq!(parse_cpus("0-3,8,10-11"), Some(vec![0, 1, 2, 3, 8, 10, 11]));
            assert_eq!(parse_cpus(""), Some(vec![]));
            assert_eq!(parse_cpus("3-1"), None);
            assert_eq!(parse_cpus("x"), None);
        }

        #[test]
        fn unreadable_values_are_unavailable_not_zero() {
            // An empty root: nothing under proc/ or sys/ exists.
            let root = std::env::temp_dir()
                .join(format!("dot-logger-calibrate-host-{}", std::process::id()));
            std::fs::create_dir_all(&root).unwrap();
            let host = read(&root, &root, 7);
            std::fs::remove_dir_all(&root).unwrap();
            assert!(matches!(host.kernel, Fact::Unavailable(_)));
            assert!(matches!(host.cpus_online, Fact::Unavailable(_)));
            assert!(matches!(host.governor, Fact::Unavailable(_)));
            assert!(matches!(host.memory_total_bytes, Fact::Unavailable(_)));
            assert!(matches!(host.memory_available_bytes, Fact::Unavailable(_)));
            assert!(matches!(host.cgroup_memory_max_bytes, Fact::Unavailable(_)));
            // No realtime file means a non-RT kernel.
            assert_eq!(host.preempt_rt, Fact::Observed(false));
            assert_eq!(host.free_bytes, Fact::Observed(7));
        }
    }
}
