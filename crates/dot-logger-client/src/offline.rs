use crate::Cancellation;
use dot_logger::{
    query::{QueryBudget, QueryConfig, QueryError, QuerySource, Snapshot},
    storage::StorageChannel,
};
use std::{fs::File, path::PathBuf};

/// Explicit original registration and finite offline query limits.
/// The caller keeps all source prefixes immutable while the snapshot exists.
pub struct OfflineConfig {
    /// Original capture session identity.
    pub session: u128,
    /// Original channels in their registration order, including schema metadata.
    pub channels: Vec<StorageChannel>,
    /// Source/index/page/blob/cumulative-read bounds.
    pub limits: QueryConfig,
    /// Deadline in the caller's monotonic nanosecond domain.
    pub deadline_ns: u64,
}
/// One immutable read-only file and its published complete prefix.
pub struct OfflineSource {
    /// Expected original user configuration revision of this file.
    pub revision: u64,
    /// Existing source file; opened read-only without modifying the original.
    pub path: PathBuf,
    /// Original distinct segment sequence.
    pub sequence: u64,
    /// Explicit complete prefix for an active file, or full frozen length for a
    /// closed immutable file when None. No complete prefix is guessed after damage.
    pub safe_offset: Option<u64>,
}
/// Open bounded independent read-only descriptors and construct the core snapshot.
/// On error all partially opened readers close before their reservation is freed.
/// Native Snapshot/Replay APIs retain their cursor, blob, expiry and cancellation
/// semantics; this function starts no worker/runtime and performs no clock conversion.
pub fn open_snapshot(
    budget: &QueryBudget,
    config: OfflineConfig,
    sources: Vec<OfflineSource>,
    now_ns: u64,
    cancel: &Cancellation,
) -> Result<Snapshot<File>, QueryError> {
    if sources.is_empty() || sources.len() > config.limits.max_segments {
        return Err(QueryError::Limit);
    }
    let mut readers = Vec::with_capacity(sources.len());
    let mut paths = Vec::with_capacity(sources.len());
    for source in sources {
        if cancel.is_cancelled() {
            return Err(QueryError::Cancelled);
        }
        // Reject special sources before opening: a FIFO open itself could block
        // indefinitely. The host still guarantees no concurrent path replacement.
        if !std::fs::metadata(&source.path)
            .map_err(QueryError::Io)?
            .is_file()
        {
            return Err(QueryError::InvalidInput);
        }
        let canonical = std::fs::canonicalize(&source.path).map_err(QueryError::Io)?;
        if paths.contains(&canonical) {
            return Err(QueryError::InvalidInput);
        }
        paths.push(canonical);
        let file = File::open(source.path).map_err(QueryError::Io)?;
        let metadata = file.metadata().map_err(QueryError::Io)?;
        let safe = source.safe_offset.unwrap_or(metadata.len());
        if !metadata.is_file() {
            return Err(QueryError::InvalidInput);
        }
        if safe > metadata.len() || safe > config.limits.max_source_bytes {
            return Err(QueryError::Limit);
        }
        readers.push(QuerySource::active(
            file,
            source.sequence,
            source.revision,
            safe,
        ));
    }
    Snapshot::new(
        budget,
        config.session,
        config.channels,
        readers,
        config.limits,
        now_ns,
        config.deadline_ns,
        cancel.as_atomic(),
    )
}
