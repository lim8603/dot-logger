//! Named presets and per-writer sizing (API §19). A [`Sizing`] fills only the
//! loss values a [`Budget`](super::Budget) leaves unset; user values win.

use std::time::Duration;

use super::budget::WriterQuota;

/// Slot margin over `rate × absorb` (API §19.1, Human 2026-11-07).
const SLOT_MARGIN_NUM: u128 = 5;
const SLOT_MARGIN_DEN: u128 = 4;
/// Payload size of the small records the presets assume (local-pc-v1 P1).
const SMALL_BYTES: usize = 256;
const DEFAULT_ABSORB: Duration = Duration::from_secs(1);

/// One record stream of a writer: records per second and largest payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Stream {
    hz: u32,
    bytes: usize,
}

/// Per-writer load and loss values from which `start()` computes a budget.
///
/// Slots per pool are `⌈rate × absorb × 1.25⌉`: the writer rides out a file IO
/// stall of `absorb` without RT rejection. Memory is the exact core
/// requirement plus 10 %. Every writer declared without its own quota gets
/// this load. Values set on the builder's `Budget` are kept.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sizing {
    small: Option<Stream>,
    raw: Option<Stream>,
    absorb: Duration,
    /// The user chose `absorb`; a profile never changes it.
    absorb_set: bool,
    machine: Option<Machine>,
    pub(crate) sync_every: Duration,
    pub(crate) drop_deadline: Duration,
}

/// What a calibration profile contributes (API §19.8).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Machine {
    /// Twice the longest observed IO stall.
    pub(crate) absorb: Duration,
    /// 0.6 × the sustained write estimate.
    pub(crate) load_limit: u64,
    /// Memory the logger can count on, when it was read.
    pub(crate) available_memory: Option<u64>,
}

impl Sizing {
    /// No streams yet; the two loss values are explicit. Absorb defaults to 1 s.
    pub fn new(sync_every: Duration, drop_deadline: Duration) -> Self {
        Self {
            small: None,
            raw: None,
            absorb: DEFAULT_ABSORB,
            absorb_set: false,
            machine: None,
            sync_every,
            drop_deadline,
        }
    }
    /// Small records copied by `try_log`: `hz` per second of at most `bytes`.
    pub fn small(mut self, hz: u32, bytes: usize) -> Self {
        self.small = Some(Stream { hz, bytes });
        self
    }
    /// Zero-copy originals: `hz` per second of at most `bytes`.
    pub fn raw(mut self, hz: u32, bytes: usize) -> Self {
        self.raw = Some(Stream { hz, bytes });
        self
    }
    /// File IO stall a writer absorbs before RT admission rejects. A value
    /// set here is kept even with a [`profile`](Self::profile).
    pub fn absorb(mut self, absorb: Duration) -> Self {
        self.absorb = absorb;
        self.absorb_set = true;
        self
    }
    /// Opt in to a machine calibration (API §19.8): unless the absorb was set
    /// with [`absorb`](Self::absorb), it becomes at least twice the longest IO
    /// stall the profile observed. [`plan`](super::LoggerBuilder::plan) then
    /// warns when the memory budget exceeds the memory the profile saw
    /// available, or the declared load exceeds 0.6 × its sustained estimate.
    #[cfg(feature = "calibrate")]
    pub fn profile(mut self, profile: &crate::calibrate::Profile) -> Self {
        self.machine = Some(Machine {
            absorb: profile.absorb(),
            load_limit: profile.load_limit_bytes_per_second(),
            available_memory: profile.available_memory_bytes(),
        });
        self
    }

    pub(crate) fn machine(&self) -> Option<Machine> {
        self.machine
    }

    /// Absorb in effect: the user's, else the larger of the preset's and the
    /// profile's.
    fn effective_absorb(&self) -> Duration {
        match self.machine {
            Some(m) if !self.absorb_set => self.absorb.max(m.absorb),
            _ => self.absorb,
        }
    }

    /// Bytes per second one writer with this load offers, metadata included.
    pub(crate) fn offered_bytes_per_second(&self) -> u64 {
        [self.small_or_metadata(), self.raw]
            .into_iter()
            .flatten()
            .map(|s| u64::from(s.hz).saturating_mul(s.bytes as u64))
            .fold(0, u64::saturating_add)
    }

    fn small_or_metadata(&self) -> Option<Stream> {
        self.small.or(self.raw.map(|raw| Stream {
            hz: raw.hz,
            bytes: SMALL_BYTES,
        }))
    }
    /// Replace the ordinary sync period (loss window on power failure).
    pub fn sync_every(mut self, period: Duration) -> Self {
        self.sync_every = period;
        self
    }
    /// Replace the bounded shutdown performed when the logger is dropped.
    pub fn drop_deadline(mut self, deadline: Duration) -> Self {
        self.drop_deadline = deadline;
        self
    }

    fn slots(&self, stream: Stream) -> usize {
        // Saturates; an unrepresentable count becomes `usize::MAX` and the
        // budget check rejects it (RV-123).
        let wanted = u128::from(stream.hz)
            .saturating_mul(self.effective_absorb().as_nanos())
            .saturating_mul(SLOT_MARGIN_NUM);
        let slots = wanted.div_ceil(1_000_000_000 * SLOT_MARGIN_DEN).max(1);
        usize::try_from(slots).unwrap_or(usize::MAX)
    }

    /// Writer quota for this load. A writer always needs small slots; with
    /// only raw streams the small pool mirrors the raw rate for metadata.
    pub(crate) fn quota(&self) -> WriterQuota {
        let small = self.small_or_metadata();
        let mut quota = small.map_or(WriterQuota::small(0, 0), |s| {
            WriterQuota::small(self.slots(s), s.bytes)
        });
        if let Some(raw) = self.raw {
            quota = quota.with_raw(self.slots(raw), raw.bytes);
        }
        quota
    }
}

/// Named sizing profiles (API §19.1). Each returns a [`Sizing`] that can be
/// adjusted further, e.g. `Preset::control(1000).sync_every(..)`. Numbers
/// follow the approved local-pc-v1 profile: 256-byte control records, 1 s
/// sync, 3 s drop deadline and a 1 s absorb.
pub enum Preset {}

impl Preset {
    /// Control loop: `hz` small 256-byte records per writer.
    pub fn control(hz: u32) -> Sizing {
        Sizing::new(Duration::from_secs(1), Duration::from_secs(3)).small(hz, SMALL_BYTES)
    }
    /// Sensor originals: `hz` zero-copy payloads of `bytes` plus small
    /// 256-byte metadata records at the same rate.
    pub fn sensor(hz: u32, bytes: usize) -> Sizing {
        Self::control(hz).raw(hz, bytes)
    }
    /// Black box: small 256-byte records with durability first — 100 ms sync
    /// and a 2 s absorb. Pair it with `Retention::ring`.
    pub fn black_box(hz: u32) -> Sizing {
        let mut sizing =
            Sizing::new(Duration::from_millis(100), Duration::from_secs(3)).small(hz, SMALL_BYTES);
        // The preset's own absorb, not the user's: a profile may lengthen it.
        sizing.absorb = Duration::from_secs(2);
        sizing
    }
}
