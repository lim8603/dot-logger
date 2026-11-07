//! Worker CPU placement (API §18.4). Linux uses `sched_setaffinity`; other
//! platforms report the placement as not applied.

#[cfg(target_os = "linux")]
use crate::platform::{CapabilityStatus, PlatformCapabilities};

/// CPUs this process may use and whether placement is supported.
#[cfg(target_os = "linux")]
pub(crate) fn observe() -> PlatformCapabilities {
    match rustix::thread::sched_getaffinity(None) {
        Ok(set) => PlatformCapabilities {
            available_cpu_ids: (0..rustix::thread::CpuSet::MAX_CPU)
                .filter(|&cpu| set.is_set(cpu))
                .collect(),
            worker_affinity: CapabilityStatus::Supported,
            ..PlatformCapabilities::default()
        },
        Err(_) => PlatformCapabilities {
            worker_affinity: CapabilityStatus::Failed,
            ..PlatformCapabilities::default()
        },
    }
}

/// Restrict the calling thread to `cpus`.
#[cfg(target_os = "linux")]
pub(crate) fn pin_current(cpus: &[usize]) -> std::io::Result<()> {
    let mut set = rustix::thread::CpuSet::new();
    for &cpu in cpus {
        set.set(cpu);
    }
    rustix::thread::sched_setaffinity(None, &set).map_err(std::io::Error::from)
}

/// Restrict the calling thread to `cpus`.
#[cfg(not(target_os = "linux"))]
pub(crate) fn pin_current(_cpus: &[usize]) -> std::io::Result<()> {
    Err(std::io::ErrorKind::Unsupported.into())
}
