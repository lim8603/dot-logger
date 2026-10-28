//! Caller-owned bounded completion pages and general-path deadline waits.
use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;
/// Frozen producer-local ranges. Later acceptance is not included, and producer
/// bounds are sampled independently rather than at one global atomic instant.
#[derive(Clone, Copy)]
pub struct CompletionScope<'scope> {
    pub(crate) session_id: u128,
    pub(crate) ranges: &'scope [ProducerRange],
}
impl CompletionScope<'_> {
    /// Registered producer ranges, frozen by construction.
    pub fn ranges(&self) -> &[ProducerRange] {
        self.ranges
    }
}
struct QueryPermit<'a>(&'a AtomicUsize);
impl Drop for QueryPermit<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Release);
    }
}
/// One borrowed query with bounded page work and a finite concurrency permit.
/// Keep caller range/request buffers alive; no payload or history allocation is owned.
pub struct CompletionQuery<'owner, 'scope, 'requests> {
    completion: Completion<'owner>,
    scope: CompletionScope<'scope>,
    requests: &'requests [TargetRequest],
    range: usize,
    sequence: u64,
    target: usize,
    summary: QuerySummary,
    _permit: QueryPermit<'owner>,
}
impl<'owner> Completion<'owner> {
    /// Validate and freeze caller-owned producer ranges. Unknown/future/duplicate
    /// producers and invalid sequence ranges are rejected before waiting.
    pub fn scope(self, ranges: &[ProducerRange]) -> Result<CompletionScope<'_>, CompletionError> {
        for (i, range) in ranges.iter().enumerate() {
            let Some(p) = self.registry.producers.get(range.producer_id as usize) else {
                return Err(CompletionError::InvalidScope);
            };
            if range.first == 0
                || (range.last < range.first && !(range.first == 1 && range.last == 0))
                || range.last > p.accepted.load(Ordering::Acquire)
                || ranges[..i]
                    .iter()
                    .any(|r| r.producer_id == range.producer_id)
            {
                return Err(CompletionError::InvalidScope);
            }
        }
        Ok(CompletionScope {
            session_id: self.registry.session_id,
            ranges,
        })
    }
    /// Sample each producer acceptance bound into caller storage. Empty producers
    /// are represented explicitly. Scope creation does not perform IO or sync.
    pub fn capture_scope(
        self,
        buffer: &mut [ProducerRange],
    ) -> Result<CompletionScope<'_>, CompletionError> {
        if buffer.len() < self.registry.producers.len() {
            return Err(CompletionError::BufferTooSmall);
        }
        for (i, producer) in self.registry.producers.iter().enumerate() {
            buffer[i] = ProducerRange {
                producer_id: i as u32,
                first: 1,
                last: producer.accepted.load(Ordering::Acquire),
            };
        }
        self.scope(&buffer[..self.registry.producers.len()])
    }
    /// Acquire a bounded concurrent-query permit and validate every requested
    /// capability. Conflicting metadata activity returns Busy instead of spinning.
    pub fn query<'s, 'r>(
        self,
        scope: CompletionScope<'s>,
        requests: &'r [TargetRequest],
    ) -> Result<CompletionQuery<'owner, 's, 'r>, CompletionError> {
        if scope.session_id != self.registry.session_id || requests.is_empty() {
            return Err(CompletionError::InvalidScope);
        }
        self.scope(scope.ranges)?;
        for (i, request) in requests.iter().enumerate() {
            let index = self.registry.target(request.target_id)?;
            if !self.registry.targets[index]
                .supported
                .contains(request.boundary)
            {
                return Err(CompletionError::UnsupportedBoundary);
            }
            if requests[..i]
                .iter()
                .any(|r| r.target_id == request.target_id)
            {
                return Err(CompletionError::InvalidScope);
            }
        }
        let count: u128 = scope
            .ranges
            .iter()
            .map(|r| {
                if r.last >= r.first {
                    u128::from(r.last - r.first) + 1
                } else {
                    0
                }
            })
            .sum();
        if count
            .checked_mul(requests.len() as u128)
            .is_none_or(|count| count > u128::from(u64::MAX))
        {
            return Err(CompletionError::InvalidScope);
        }
        let readers = &self.registry.readers;
        let current = readers.load(Ordering::Acquire);
        if current >= self.registry.limits.max_queries {
            return Err(CompletionError::QueryCapacity);
        }
        readers
            .compare_exchange(current, current + 1, Ordering::AcqRel, Ordering::Relaxed)
            .map_err(|_| CompletionError::Busy)?;
        let mut query = CompletionQuery {
            completion: self,
            scope,
            requests,
            range: 0,
            sequence: 1,
            target: 0,
            summary: QuerySummary::default(),
            _permit: QueryPermit(readers),
        };
        query.restart();
        Ok(query)
    }
}
impl CompletionQuery<'_, '_, '_> {
    /// Restart observation of the same frozen selection, preserving its permit.
    pub fn restart(&mut self) {
        self.range = 0;
        self.target = 0;
        self.summary = QuerySummary::default();
        self.seek_range();
    }
    fn seek_range(&mut self) {
        while self.range < self.scope.ranges.len() {
            let range = self.scope.ranges[self.range];
            if range.first <= range.last {
                self.sequence = range.first;
                return;
            }
            self.range += 1;
        }
        self.summary.finished = true;
    }
    /// Initialize at most min(buffer length, configured page_items) rows. A partial
    /// scan cannot claim full success. Unused caller cells are left untouched.
    pub fn page(
        &mut self,
        buffer: &mut [Option<CompletionItem>],
    ) -> Result<PageReport, CompletionError> {
        if buffer.is_empty() {
            return Err(CompletionError::BufferTooSmall);
        }
        let mut items = 0;
        while items < buffer.len().min(self.completion.registry.limits.page_items)
            && !self.summary.finished
        {
            let range = self.scope.ranges[self.range];
            let request = self.requests[self.target];
            let receipt = CaptureReceipt {
                session_id: self.scope.session_id,
                producer_id: range.producer_id,
                accepted_seq: self.sequence,
            };
            let status = match self
                .completion
                .poll(receipt, request.target_id, request.boundary)
            {
                Ok(status) => status,
                Err(CompletionError::Busy) => RecordStatus {
                    confirmed: Boundaries::default(),
                    outcome: QueryOutcome::Busy,
                },
                Err(error) => return Err(error),
            };
            let field = match status.outcome {
                QueryOutcome::Complete => &mut self.summary.complete,
                QueryOutcome::Pending => &mut self.summary.pending,
                QueryOutcome::Failed(_) => &mut self.summary.failed,
                QueryOutcome::Dropped(_) => &mut self.summary.dropped,
                QueryOutcome::Unknown => &mut self.summary.unknown,
                QueryOutcome::Expired => &mut self.summary.expired,
                QueryOutcome::NotRouted => &mut self.summary.not_routed,
                QueryOutcome::Busy => &mut self.summary.busy,
            };
            accumulate(field, 1, &mut self.summary.overflowed);
            buffer[items] = Some(CompletionItem {
                receipt,
                request,
                status,
            });
            items += 1;
            self.target += 1;
            if self.target == self.requests.len() {
                self.target = 0;
                if self.sequence == range.last {
                    self.range += 1;
                    self.seek_range();
                } else {
                    self.sequence += 1;
                }
            }
        }
        Ok(PageReport {
            items,
            summary: self.summary,
        })
    }
    /// Wait on the general path, checking deadline/cancellation between bounded
    /// pages. The last page and cumulative scan summary are returned. A timeout or
    /// cancellation never starts sync, cancels outputs, or rewrites their outcomes.
    pub fn wait(
        &mut self,
        deadline: Instant,
        cancel: &AtomicBool,
        buffer: &mut [Option<CompletionItem>],
    ) -> Result<WaitReport, CompletionError> {
        self.restart();
        let mut last = None;
        loop {
            let cancelled = cancel.load(Ordering::Acquire);
            let timed_out = Instant::now() >= deadline;
            if cancelled || timed_out {
                let page = match last {
                    Some(page) => page,
                    None => self.page(buffer)?,
                };
                return Ok(WaitReport {
                    outcome: if cancelled {
                        WaitOutcome::Cancelled
                    } else {
                        WaitOutcome::TimedOut
                    },
                    page,
                });
            }
            let page = self.page(buffer)?;
            last = Some(page);
            if page.summary.is_complete() {
                return Ok(WaitReport {
                    outcome: WaitOutcome::Complete,
                    page,
                });
            }
            if page.summary.is_settled() {
                return Ok(WaitReport {
                    outcome: WaitOutcome::Settled,
                    page,
                });
            }
            if page.summary.finished {
                std::thread::yield_now();
                self.restart();
            }
        }
    }
}
