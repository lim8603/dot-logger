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
    pub(crate) sync_every: Duration,
    pub(crate) drop_deadline: Duration,
}

impl Sizing {
    /// No streams yet; the two loss values are explicit. Absorb defaults to 1 s.
    pub fn new(sync_every: Duration, drop_deadline: Duration) -> Self {
        Self {
            small: None,
            raw: None,
            absorb: DEFAULT_ABSORB,
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
    /// File IO stall a writer absorbs before RT admission rejects.
    pub fn absorb(mut self, absorb: Duration) -> Self {
        self.absorb = absorb;
        self
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
            .saturating_mul(self.absorb.as_nanos())
            .saturating_mul(SLOT_MARGIN_NUM);
        let slots = wanted.div_ceil(1_000_000_000 * SLOT_MARGIN_DEN).max(1);
        usize::try_from(slots).unwrap_or(usize::MAX)
    }

    /// Writer quota for this load. A writer always needs small slots; with
    /// only raw streams the small pool mirrors the raw rate for metadata.
    pub(crate) fn quota(&self) -> WriterQuota {
        let small = self.small.or(self.raw.map(|raw| Stream {
            hz: raw.hz,
            bytes: SMALL_BYTES,
        }));
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
        Sizing::new(Duration::from_millis(100), Duration::from_secs(3))
            .small(hz, SMALL_BYTES)
            .absorb(Duration::from_secs(2))
    }
}
