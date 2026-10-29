/// One channel's complete-chunk range, independent of wall-clock ordering.
#[derive(Clone, Copy, Debug)]
pub struct ChunkIndexEntry {
    /// Original product channel ID.
    pub channel_id: u32,
    /// Minimum monotonic timestamp.
    pub mono_min: u64,
    /// Maximum monotonic timestamp.
    pub mono_max: u64,
    /// Minimum signed wall-clock timestamp.
    pub real_min: i64,
    /// Maximum signed wall-clock timestamp.
    pub real_max: i64,
    /// Start of a complete container chunk.
    pub chunk_offset: u64,
    /// Number of messages for this channel in the chunk.
    pub record_count: u64,
}
