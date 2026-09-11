//! Contracted engineering ownership for a persistent, rank-shared paged KV pool.
//!
//! This is not protected Engine page custody or a Verus refinement. The caller
//! must bind one pool to one resident GPU allocation set and report completion
//! only after every layer/rank has completed. Exact tokens and model/session
//! identity are necessary cache keys, not a proof of GPU numerical correctness.
//! Submitted uncertainty permanently quarantines the entire pool. Host storage
//! is bounded by the admitted limits; host allocation failure may abort.

use crate::tp_artifact::{EngineeringTpArtifactV1, LargeKvBindingV9};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Tokens per physical page, shared across every rank and layer.
pub const ENGINEERING_TP_PAGE_TOKENS_V1: u32 = 16;
/// Maximum token rows in one prepared GPU batch.
pub const ENGINEERING_TP_MAX_BATCH_ROWS_V1: usize = 16;

static NEXT_POOL_ID: AtomicU64 = AtomicU64::new(1);

/// Exact externally derived model and resident-device-session identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EngineeringTpPoolScopeV1 {
    /// Canonical model identity; must be nonzero.
    pub model: [u8; 32],
    /// Unique resident driver session identity; must be nonzero.
    pub session: [u8; 32],
}

/// Fail-closed host metadata errors, conferring no execution authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineeringTpPagedErrorV1 {
    /// Zero or unsupported resource limit or identity.
    InvalidLimits,
    /// The pool/model/session does not match the caller's retained identity.
    ScopeMismatch,
    /// Another batch retains all mutable pool metadata.
    Busy,
    /// A submitted failure permanently quarantined this pool.
    Poisoned,
    /// Sequence identity is stale, foreign, or absent.
    UnknownSequence,
    /// Batch identity is stale, foreign, or absent.
    UnknownBatch,
    /// The request table is full.
    SequenceCapacity,
    /// No complete atomic reservation fits in the physical pool.
    OutOfPages,
    /// Token rows are empty, oversized, out of bounds, or noncontiguous.
    InvalidRows,
    /// Operation is invalid for the current submission phase.
    SubmissionPhase,
    /// Deterministic cache clock moved backwards.
    ClockRegression,
    /// A bounded counter or monotonically unique identity is exhausted.
    Exhausted,
    /// Internal page/refcount/tree correspondence did not hold.
    Invariant,
}

/// Result for the non-authoritative paged metadata API.
pub type EngineeringTpPagedResultV1<T> = std::result::Result<T, EngineeringTpPagedErrorV1>;
type Result<T> = EngineeringTpPagedResultV1<T>;
use EngineeringTpPagedErrorV1 as Error;

/// Independently bounded host metadata and physical allocation geometry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineeringTpKvPoolProfileV1 {
    /// Frozen v2/v5 physical bound, at most 512 pages.
    Legacy,
    /// Explicit v9 TP1 physical bound, at most 16384 pages.
    LargeV9,
}

/// Independently bounded host metadata and physical allocation geometry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EngineeringTpPagedLimitsV1 {
    profile: EngineeringTpKvPoolProfileV1,
    context_tokens: u32,
    sequences: u32,
    physical_pages: u32,
    cache_ttl: u64,
}

impl EngineeringTpPagedLimitsV1 {
    /// Fixes the pool envelope before any device allocations are created.
    /// # Errors
    /// Rejects zero limits or context/sequences/pages above 8192/32/512.
    pub fn new(
        context_tokens: u32,
        sequences: u32,
        physical_pages: u32,
        cache_ttl: u64,
    ) -> Result<Self> {
        Self::new_bounded(
            context_tokens,
            sequences,
            physical_pages,
            cache_ttl,
            EngineeringTpKvPoolProfileV1::Legacy,
        )
    }

    /// Explicit larger physical pool; logical context and sequence bounds are unchanged.
    /// # Errors
    /// Rejects context/sequences/pages outside 1..8192/1..32/1..16384, or zero TTL.
    pub fn new_large_kv32(
        context_tokens: u32,
        sequences: u32,
        physical_pages: u32,
        cache_ttl: u64,
    ) -> Result<Self> {
        Self::new_bounded(
            context_tokens,
            sequences,
            physical_pages,
            cache_ttl,
            EngineeringTpKvPoolProfileV1::LargeV9,
        )
    }

    fn new_bounded(
        context_tokens: u32,
        sequences: u32,
        physical_pages: u32,
        cache_ttl: u64,
        profile: EngineeringTpKvPoolProfileV1,
    ) -> Result<Self> {
        let maximum = match profile {
            EngineeringTpKvPoolProfileV1::Legacy => 512,
            EngineeringTpKvPoolProfileV1::LargeV9 => 16384,
        };
        if !(1..=8192).contains(&context_tokens)
            || !(1..=32).contains(&sequences)
            || !(1..=maximum).contains(&physical_pages)
            || cache_ttl == 0
        {
            return Err(Error::InvalidLimits);
        }
        Ok(Self {
            profile,
            context_tokens,
            sequences,
            physical_pages,
            cache_ttl,
        })
    }

    /// Explicit physical kernel envelope, even when the actual page count is small.
    #[must_use]
    pub const fn profile(self) -> EngineeringTpKvPoolProfileV1 {
        self.profile
    }

    /// Exact physical token allocation extent; never a logical sequence capacity.
    /// # Errors
    /// Rejects arithmetic overflow rather than using a truncated physical extent.
    pub fn physical_token_capacity(self) -> Result<u32> {
        self.physical_pages
            .checked_mul(ENGINEERING_TP_PAGE_TOKENS_V1)
            .ok_or(Error::Exhausted)
    }

    /// Exact TP1 target-Qwen3-8B K/V array payload, excluding other allocations.
    /// # Errors
    /// Rejects an arithmetic overflow instead of truncating allocation geometry.
    pub fn target_kv_payload_bytes(self) -> Result<u64> {
        u64::from(self.physical_pages)
            .checked_mul(16)
            .and_then(|v| v.checked_mul(36))
            .and_then(|v| v.checked_mul(2))
            .and_then(|v| v.checked_mul(1024))
            .and_then(|v| v.checked_mul(2))
            .ok_or(Error::Exhausted)
    }

    /// Maximum initialized logical token prefix per request.
    #[must_use]
    pub const fn context_tokens(self) -> u32 {
        self.context_tokens
    }
    /// Maximum simultaneously retained requests.
    #[must_use]
    pub const fn max_sequences(self) -> u32 {
        self.sequences
    }
    /// Number of physical slots in each rank/layer allocation.
    #[must_use]
    pub const fn physical_page_count(self) -> u32 {
        self.physical_pages
    }
    /// Exact GPU page-table row stride, including unused sentinel entries.
    #[must_use]
    pub const fn page_table_stride(self) -> u32 {
        self.context_tokens.div_ceil(16)
    }
    /// Deterministic caller-clock lifetime of a retained prefix node.
    #[must_use]
    pub const fn cache_ttl(self) -> u64 {
        self.cache_ttl
    }
}

/// Pool-instance-bound monotone sequence identity; cannot be constructed externally.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct EngineeringTpSequenceIdV1 {
    pool: u64,
    serial: u64,
}

/// One scheduler-selected token write, before page reservation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EngineeringTpPageRowV1 {
    /// Opaque sequence returned by this pool's admission.
    pub sequence: EngineeringTpSequenceIdV1,
    /// Exact input token whose KV will be written.
    pub token: u32,
    /// Absolute logical position, contiguous from the committed prefix.
    pub position: u32,
}

/// Successful admission, retaining references to an exact immutable prefix.
#[derive(Debug)]
pub struct EngineeringTpPrefixHitV1 {
    sequence: EngineeringTpSequenceIdV1,
    hit_tokens: u32,
    pages: Vec<u32>,
}

impl EngineeringTpPrefixHitV1 {
    /// Newly admitted live sequence.
    #[must_use]
    pub const fn sequence(&self) -> EngineeringTpSequenceIdV1 {
        self.sequence
    }
    /// Complete-page cached tokens; always less than the full prompt length.
    #[must_use]
    pub const fn hit_tokens(&self) -> u32 {
        self.hit_tokens
    }
    /// Number of retained complete physical pages.
    #[must_use]
    pub const fn hit_pages(&self) -> u32 {
        self.hit_tokens / 16
    }
    /// Read-only logical-to-physical mapping for the admitted prefix.
    #[must_use]
    pub fn physical_pages(&self) -> &[u32] {
        &self.pages
    }
}

/// Sealed, read-only GPU row metadata minted by atomic reservation.
#[derive(Debug)]
pub struct EngineeringTpPreparedRowV1 {
    input: EngineeringTpPageRowV1,
    pages: Vec<u32>,
    write_page: u32,
}

impl EngineeringTpPreparedRowV1 {
    /// Request identity corresponding to this row.
    #[must_use]
    pub const fn sequence(&self) -> EngineeringTpSequenceIdV1 {
        self.input.sequence
    }
    /// Exact input token.
    #[must_use]
    pub const fn token(&self) -> u32 {
        self.input.token
    }
    /// Absolute position, including any retained prefix.
    #[must_use]
    pub const fn position(&self) -> u32 {
        self.input.position
    }
    /// Complete reserved logical-to-physical mapping; attention uses position for causality.
    #[must_use]
    pub fn physical_pages(&self) -> &[u32] {
        &self.pages
    }
    /// Exclusive, unpublished physical page containing this row's writable slot.
    #[must_use]
    pub const fn writable_physical_page(&self) -> u32 {
        self.write_page
    }
    /// Exact writable token offset within that exclusive page.
    #[must_use]
    pub const fn writable_token_offset(&self) -> u32 {
        self.input.position % 16
    }
}

/// Move-only batch metadata. Dropping it does not release an outstanding reservation.
#[derive(Debug)]
pub struct EngineeringTpPreparedBatchV1 {
    pool: u64,
    scope: EngineeringTpPoolScopeV1,
    id: u64,
    limits: EngineeringTpPagedLimitsV1,
    rows: Vec<EngineeringTpPreparedRowV1>,
}

impl EngineeringTpPreparedBatchV1 {
    /// Instance identity additionally prevents same-scope pool substitution.
    #[must_use]
    pub const fn pool_identity(&self) -> u64 {
        self.pool
    }
    /// Bound model and resident session.
    #[must_use]
    pub const fn scope(&self) -> EngineeringTpPoolScopeV1 {
        self.scope
    }
    /// Monotone batch ID; an aborted ID is never reused.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }
    /// Admitted allocation/context envelope.
    #[must_use]
    pub const fn limits(&self) -> EngineeringTpPagedLimitsV1 {
        self.limits
    }
    /// Exact GPU row stride.
    #[must_use]
    pub const fn page_table_stride(&self) -> u32 {
        self.limits.page_table_stride()
    }
    /// Exact per-rank/layer physical allocation page count.
    #[must_use]
    pub const fn physical_page_count(&self) -> u32 {
        self.limits.physical_page_count()
    }
    /// Maximum logical context.
    #[must_use]
    pub const fn context_tokens(&self) -> u32 {
        self.limits.context_tokens()
    }
    /// All selected rows, in scheduler order.
    #[must_use]
    pub fn rows(&self) -> &[EngineeringTpPreparedRowV1] {
        &self.rows
    }
}

/// Engineering driver completion, not protected or independently authenticated authority.
#[derive(Debug)]
pub struct EngineeringTpBatchCompletionV1 {
    pool: u64,
    batch: u64,
}

impl EngineeringTpBatchCompletionV1 {
    /// Only the crate's driver may mint this after every layer/rank has completed.
    pub(crate) const fn after_all_ranks(batch: &EngineeringTpPreparedBatchV1) -> Self {
        Self {
            pool: batch.pool,
            batch: batch.id,
        }
    }

    fn into_identity(self) -> (u64, u64) {
        (self.pool, self.batch)
    }
}

/// Bounded counters and allocation state, including any outstanding reservation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EngineeringTpPagedStatsV1 {
    /// Number of admitted requests (including requests awaiting retirement).
    pub sequences: u32,
    /// Immediately reusable physical pages; zero for a poisoned pool.
    pub free_pages: u32,
    /// Physical pages retained by requests and/or radix nodes.
    pub retained_pages: u32,
    /// Immutable complete-page radix nodes.
    pub cached_pages: u32,
    /// Pages unavailable forever after submitted uncertainty.
    pub quarantined_pages: u32,
    /// Admission calls that reused at least one full page.
    pub prefix_hits: u64,
    /// Cumulative exact reused token count, including repeated reuse.
    pub hit_tokens: u64,
    /// Cumulative reused page count, including repeated reuse.
    pub hit_pages: u64,
    /// Number of deliberately evicted or expired radix nodes.
    pub evicted_pages: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct PhysicalPage {
    refs: u32,
    cached: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct Sequence {
    tokens: Vec<u32>,
    pages: Vec<u32>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct RadixNode {
    parent: Option<usize>,
    edge: [u32; 16],
    page: u32,
    touched: u64,
    expires: u64,
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct State {
    pages: Vec<PhysicalPage>,
    sequences: BTreeMap<u64, Sequence>,
    nodes: Vec<Option<RadixNode>>,
    clock: u64,
    next_sequence: u64,
    prefix_hits: u64,
    hit_tokens: u64,
    hit_pages: u64,
    evicted_pages: u64,
}
#[derive(Debug)]
struct Pending {
    id: u64,
    submitted: bool,
    proposed: State,
}

/// One resident allocation set's non-authoritative logical ownership and radix index.
///
/// There is at most one pending batch. All other mutating operations return Busy;
/// cancellation must wait for completion, or poison the whole pool on uncertainty.
#[derive(Debug)]
pub struct EngineeringTpPagedPoolV1 {
    identity: u64,
    scope: EngineeringTpPoolScopeV1,
    limits: EngineeringTpPagedLimitsV1,
    row_capacity: usize,
    large_kv: Option<LargeKvBindingV9>,
    state: State,
    pending: Option<Pending>,
    next_batch: u64,
    poisoned: bool,
}

impl EngineeringTpPagedPoolV1 {
    /// Creates host metadata only. The driver must bind its fresh GPU allocations to this identity.
    /// # Errors
    /// Rejects empty scope identities or exhausted process-local pool IDs.
    pub fn new(
        scope: EngineeringTpPoolScopeV1,
        limits: EngineeringTpPagedLimitsV1,
    ) -> Result<Self> {
        Self::new_bounded(scope, limits, ENGINEERING_TP_MAX_BATCH_ROWS_V1, None)
    }

    /// Creates a separately selected 32-row metadata envelope for the v5 image.
    /// # Errors
    /// Rejects empty scope identities or exhausted process-local pool IDs.
    pub fn new_wide32(
        scope: EngineeringTpPoolScopeV1,
        limits: EngineeringTpPagedLimitsV1,
    ) -> Result<Self> {
        Self::new_bounded(scope, limits, 32, None)
    }

    /// Binds an explicitly larger metadata pool to an independently admitted v9 image.
    /// # Errors
    /// Rejects legacy limits/images, invalid scope, or exhausted pool identities.
    pub fn new_large_kv32(
        scope: EngineeringTpPoolScopeV1,
        limits: EngineeringTpPagedLimitsV1,
        artifact: &EngineeringTpArtifactV1,
    ) -> Result<Self> {
        let binding = artifact.large_kv_binding().ok_or(Error::InvalidLimits)?;
        Self::new_bounded(scope, limits, 32, Some(binding))
    }

    pub(crate) const fn large_kv_binding(&self) -> Option<LargeKvBindingV9> {
        self.large_kv
    }

    #[cfg(test)]
    pub(crate) fn new_large_kv32_recording(
        scope: EngineeringTpPoolScopeV1,
        limits: EngineeringTpPagedLimitsV1,
    ) -> Result<Self> {
        Self::new_bounded(scope, limits, 32, Some(LargeKvBindingV9::recording()))
    }

    fn new_bounded(
        scope: EngineeringTpPoolScopeV1,
        limits: EngineeringTpPagedLimitsV1,
        row_capacity: usize,
        large_kv: Option<LargeKvBindingV9>,
    ) -> Result<Self> {
        if scope.model == [0; 32]
            || scope.session == [0; 32]
            || (limits.profile == EngineeringTpKvPoolProfileV1::LargeV9) != large_kv.is_some()
            || (large_kv.is_some() && row_capacity != 32)
        {
            return Err(Error::InvalidLimits);
        }
        let identity = NEXT_POOL_ID
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map_err(|_| Error::Exhausted)?;
        Ok(Self {
            identity,
            scope,
            limits,
            row_capacity,
            large_kv,
            state: State {
                pages: vec![PhysicalPage::default(); limits.physical_pages as usize],
                nodes: vec![None; limits.physical_pages as usize],
                sequences: BTreeMap::new(),
                clock: 0,
                next_sequence: 1,
                prefix_hits: 0,
                hit_tokens: 0,
                hit_pages: 0,
                evicted_pages: 0,
            },
            pending: None,
            next_batch: 1,
            poisoned: false,
        })
    }
    /// Globally monotone process-local instance ID, independent of caller scope reuse.
    #[must_use]
    pub const fn identity(&self) -> u64 {
        self.identity
    }
    /// Exact model/session binding.
    #[must_use]
    pub const fn scope(&self) -> EngineeringTpPoolScopeV1 {
        self.scope
    }
    /// Fixed geometry for resident GPU allocation.
    #[must_use]
    pub const fn limits(&self) -> EngineeringTpPagedLimitsV1 {
        self.limits
    }
    /// Maximum rows in one physical transaction, independent of page size.
    #[must_use]
    pub const fn row_capacity(&self) -> usize {
        self.row_capacity
    }
    /// True only before any admission/reservation, suitable for fresh driver construction.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.poisoned
            && self.pending.is_none()
            && self.state.next_sequence == 1
            && self.next_batch == 1
    }

    /// Admits a prompt, retaining only exact complete-page cached prefixes.
    /// At least the final prompt token is replayed to obtain actual logits.
    /// # Errors
    /// Rejects foreign scope, prompt/sequence bounds, clock regression, busy or poisoned state.
    pub fn open_sequence(
        &mut self,
        scope: EngineeringTpPoolScopeV1,
        prompt: &[u32],
        now: u64,
    ) -> Result<EngineeringTpPrefixHitV1> {
        if scope != self.scope {
            return Err(Error::ScopeMismatch);
        }
        if prompt.is_empty() || prompt.len() > self.limits.context_tokens as usize {
            return Err(Error::InvalidRows);
        }
        let identity = self.identity;
        let limits = self.limits;
        self.transact(|state| {
            if state.sequences.len() >= limits.sequences as usize {
                return Err(Error::SequenceCapacity);
            }
            advance_clock(state, now)?;
            let expires = now.checked_add(limits.cache_ttl).ok_or(Error::Exhausted)?;
            expire_nodes(state, now)?;
            let serial = state.next_sequence;
            state.next_sequence = serial.checked_add(1).ok_or(Error::Exhausted)?;
            let mut parent = None;
            let mut pages = Vec::new();
            for chunk in prompt[..(prompt.len() - 1) / 16 * 16].chunks_exact(16) {
                let edge: [u32; 16] = chunk.try_into().map_err(|_| Error::Invariant)?;
                let Some(index) = find_child(state, parent, &edge) else {
                    break;
                };
                let node = state.nodes[index].as_mut().ok_or(Error::Invariant)?;
                if node.expires <= now {
                    break;
                }
                node.touched = now;
                node.expires = expires;
                let page = node.page;
                state.pages[page as usize].refs = state.pages[page as usize]
                    .refs
                    .checked_add(1)
                    .ok_or(Error::Exhausted)?;
                pages.push(page);
                parent = Some(index);
            }
            let hit_pages = count(pages.len());
            let hit_tokens = hit_pages * 16;
            if hit_pages != 0 {
                state.prefix_hits = state.prefix_hits.checked_add(1).ok_or(Error::Exhausted)?;
                state.hit_pages = state
                    .hit_pages
                    .checked_add(u64::from(hit_pages))
                    .ok_or(Error::Exhausted)?;
                state.hit_tokens = state
                    .hit_tokens
                    .checked_add(u64::from(hit_tokens))
                    .ok_or(Error::Exhausted)?;
            }
            state.sequences.insert(
                serial,
                Sequence {
                    tokens: prompt[..hit_tokens as usize].to_vec(),
                    pages: pages.clone(),
                },
            );
            Ok(EngineeringTpPrefixHitV1 {
                sequence: EngineeringTpSequenceIdV1 {
                    pool: identity,
                    serial,
                },
                hit_tokens,
                pages,
            })
        })
    }

    /// Atomically reserves every selected row, without publishing any token KV.
    /// Each sequence's rows must extend its committed prefix in input order.
    /// No pages, clock, refcounts or request progress change on failure.
    /// # Errors
    /// Rejects stale/foreign IDs, noncontiguous rows, context bounds, OOM, busy or poisoned state.
    pub fn reserve_batch(
        &mut self,
        rows: &[EngineeringTpPageRowV1],
    ) -> Result<EngineeringTpPreparedBatchV1> {
        self.require_idle()?;
        if rows.is_empty() || rows.len() > self.row_capacity {
            return Err(Error::InvalidRows);
        }
        let next_batch = self.next_batch.checked_add(1).ok_or(Error::Exhausted)?;
        let mut proposed = self.state.clone();
        for row in rows {
            self.require_sequence(row.sequence)?;
            let sequence = proposed
                .sequences
                .get_mut(&row.sequence.serial)
                .ok_or(Error::UnknownSequence)?;
            if row.position >= self.limits.context_tokens
                || row.position as usize != sequence.tokens.len()
            {
                return Err(Error::InvalidRows);
            }
            if row.position.is_multiple_of(16) {
                let free = proposed
                    .pages
                    .iter()
                    .position(|page| page.refs == 0 && !page.cached)
                    .ok_or(Error::OutOfPages)?;
                proposed.pages[free].refs = 1;
                sequence.pages.push(count(free));
            }
            let page = sequence.pages[row.position as usize / 16];
            let physical = &proposed.pages[page as usize];
            if physical.refs != 1 || physical.cached {
                return Err(Error::Invariant);
            }
            sequence.tokens.push(row.token);
        }
        validate_state(&proposed, self.limits)?;
        let prepared_rows = rows
            .iter()
            .map(|row| {
                let pages = proposed.sequences[&row.sequence.serial].pages.clone();
                EngineeringTpPreparedRowV1 {
                    input: *row,
                    write_page: pages[row.position as usize / 16],
                    pages,
                }
            })
            .collect();
        let id = self.next_batch;
        self.next_batch = next_batch;
        self.pending = Some(Pending {
            id,
            submitted: false,
            proposed,
        });
        Ok(EngineeringTpPreparedBatchV1 {
            pool: self.identity,
            scope: self.scope,
            id,
            limits: self.limits,
            rows: prepared_rows,
        })
    }

    /// Marks uncertainty BEFORE the first device-side operation for this batch.
    /// # Errors
    /// Rejects stale/foreign batches, duplicate submission, or a poisoned pool.
    pub fn begin_submission(&mut self, batch: &EngineeringTpPreparedBatchV1) -> Result<()> {
        self.require_batch(batch)?;
        let pending = self.pending.as_mut().ok_or(Error::UnknownBatch)?;
        if pending.submitted {
            return Err(Error::SubmissionPhase);
        }
        pending.submitted = true;
        Ok(())
    }

    /// Abandons an unsubmitted reservation, retaining committed state unchanged.
    /// # Errors
    /// Rejects submitted uncertainty, stale/foreign batches, or a poisoned pool.
    pub fn abort_batch(&mut self, batch: &EngineeringTpPreparedBatchV1) -> Result<()> {
        self.require_batch(batch)?;
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.submitted)
        {
            return Err(Error::SubmissionPhase);
        }
        self.pending = None;
        Ok(())
    }

    /// Publishes only after the driver reports every layer and rank complete.
    /// Completion is a crate-private engineering contract, not hardware proof.
    /// # Errors
    /// Rejects stale/foreign completion, missing submission, or poisoned state.
    pub fn commit_batch(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
        completion: EngineeringTpBatchCompletionV1,
    ) -> Result<()> {
        self.require_batch(batch)?;
        if completion.into_identity() != (self.identity, batch.id) {
            return Err(Error::UnknownBatch);
        }
        if !self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.submitted)
        {
            return Err(Error::SubmissionPhase);
        }
        let pending = self.pending.take().ok_or(Error::UnknownBatch)?;
        self.state = pending.proposed;
        Ok(())
    }

    /// Permanently quarantines ALL physical slots after a submitted uncertainty.
    /// Closing every resident worker is the caller's only recovery; no free list is exposed.
    /// # Errors
    /// Rejects stale/foreign batches or an unsubmitted batch (which may instead abort).
    pub fn quarantine_batch(&mut self, batch: &EngineeringTpPreparedBatchV1) -> Result<()> {
        self.require_batch(batch)?;
        if !self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.submitted)
        {
            return Err(Error::SubmissionPhase);
        }
        self.poisoned = true;
        Ok(())
    }

    /// Retires a completed request, optionally retaining complete immutable pages.
    /// The partial final page is never cached or shared. Existing matching radix
    /// edges retain their original physical page; duplicate computed pages are released.
    /// # Errors
    /// Rejects stale/foreign IDs, clock regression, busy or poisoned state.
    pub fn retire_sequence(
        &mut self,
        sequence: EngineeringTpSequenceIdV1,
        publish_prefix: bool,
        now: u64,
    ) -> Result<()> {
        self.require_sequence(sequence)?;
        let limits = self.limits;
        self.transact(|state| {
            advance_clock(state, now)?;
            let expires = now.checked_add(limits.cache_ttl).ok_or(Error::Exhausted)?;
            expire_nodes(state, now)?;
            let retired = state
                .sequences
                .remove(&sequence.serial)
                .ok_or(Error::UnknownSequence)?;
            if publish_prefix {
                let mut parent = None;
                for (logical, chunk) in retired.tokens.chunks_exact(16).enumerate() {
                    let edge: [u32; 16] = chunk.try_into().map_err(|_| Error::Invariant)?;
                    let index = if let Some(index) = find_child(state, parent, &edge) {
                        index
                    } else {
                        let page = retired.pages[logical];
                        if state.pages[page as usize].cached {
                            return Err(Error::Invariant);
                        }
                        let index = state
                            .nodes
                            .iter()
                            .position(Option::is_none)
                            .ok_or(Error::Invariant)?;
                        let physical = &mut state.pages[page as usize];
                        physical.refs = physical.refs.checked_add(1).ok_or(Error::Exhausted)?;
                        physical.cached = true;
                        state.nodes[index] = Some(RadixNode {
                            parent,
                            edge,
                            page,
                            touched: now,
                            expires,
                        });
                        index
                    };
                    let node = state.nodes[index].as_mut().ok_or(Error::Invariant)?;
                    node.touched = now;
                    node.expires = expires;
                    parent = Some(index);
                }
            }
            drop_sequence_refs(state, &retired)
        })
    }

    /// Cancels an idle request without cache publication. A pending batch blocks
    /// cancellation until completion, or requires quarantine after uncertainty.
    /// # Errors
    /// Rejects stale/foreign IDs, busy or poisoned state.
    pub fn cancel_sequence(&mut self, sequence: EngineeringTpSequenceIdV1) -> Result<()> {
        self.require_sequence(sequence)?;
        self.transact(|state| {
            let retired = state
                .sequences
                .remove(&sequence.serial)
                .ok_or(Error::UnknownSequence)?;
            drop_sequence_refs(state, &retired)
        })
    }

    /// Expires cache nodes at a deterministic monotone caller clock.
    /// Live request references remain valid even when their cache pin expires.
    /// # Errors
    /// Rejects clock regression, exhausted counters, busy or poisoned state.
    pub fn expire(&mut self, now: u64) -> Result<()> {
        self.transact(|state| {
            advance_clock(state, now)?;
            expire_nodes(state, now)
        })
    }

    /// Makes at least `pages_needed` physical pages free, atomically or unchanged.
    /// Eviction is oldest-leaf-first and removes only cache-only pages. A live
    /// request reference pins its page against eviction and physical reuse.
    /// # Errors
    /// Rejects an unattainable free-page target, busy or poisoned state.
    pub fn evict_unused(&mut self, pages_needed: u32) -> Result<()> {
        if pages_needed > self.limits.physical_pages {
            return Err(Error::OutOfPages);
        }
        self.transact(|state| {
            while free_pages(state) < pages_needed {
                let candidate = state
                    .nodes
                    .iter()
                    .enumerate()
                    .filter_map(|(index, node)| {
                        let node = node.as_ref()?;
                        (state.pages[node.page as usize].refs == 1 && is_leaf(state, index))
                            .then_some((node.touched, index))
                    })
                    .min()
                    .map(|(_, index)| index)
                    .ok_or(Error::OutOfPages)?;
                remove_node(state, candidate)?;
            }
            Ok(())
        })
    }

    /// Current committed token position, never the uncommitted reservation end.
    /// # Errors
    /// Rejects foreign/stale request identity or a poisoned pool.
    pub fn committed_position(&self, sequence: EngineeringTpSequenceIdV1) -> Result<u32> {
        self.require_sequence(sequence)?;
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        Ok(count(self.state.sequences[&sequence.serial].tokens.len()))
    }

    /// Reads bounded ownership and cumulative cache counters.
    #[must_use]
    pub fn stats(&self) -> EngineeringTpPagedStatsV1 {
        let state = self
            .pending
            .as_ref()
            .map_or(&self.state, |pending| &pending.proposed);
        let free = free_pages(state);
        EngineeringTpPagedStatsV1 {
            sequences: count(state.sequences.len()),
            free_pages: if self.poisoned { 0 } else { free },
            retained_pages: if self.poisoned {
                0
            } else {
                self.limits.physical_pages - free
            },
            cached_pages: if self.poisoned {
                0
            } else {
                count(state.nodes.iter().filter(|node| node.is_some()).count())
            },
            quarantined_pages: if self.poisoned {
                self.limits.physical_pages
            } else {
                0
            },
            prefix_hits: state.prefix_hits,
            hit_tokens: state.hit_tokens,
            hit_pages: state.hit_pages,
            evicted_pages: state.evicted_pages,
        }
    }

    /// Rechecks host page/refcount/tree invariants; grants no GPU authority.
    /// # Errors
    /// Rejects internal correspondence violations or a poisoned pool.
    pub fn check_invariants(&self) -> Result<()> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        validate_state(&self.state, self.limits)?;
        if let Some(pending) = &self.pending {
            validate_state(&pending.proposed, self.limits)?;
        }
        Ok(())
    }

    fn require_idle(&self) -> Result<()> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        if self.pending.is_some() {
            return Err(Error::Busy);
        }
        Ok(())
    }
    fn require_sequence(&self, sequence: EngineeringTpSequenceIdV1) -> Result<()> {
        if sequence.pool != self.identity || !self.state.sequences.contains_key(&sequence.serial) {
            return Err(Error::UnknownSequence);
        }
        Ok(())
    }
    fn require_batch(&self, batch: &EngineeringTpPreparedBatchV1) -> Result<()> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        if batch.pool != self.identity
            || batch.scope != self.scope
            || batch.limits != self.limits
            || self
                .pending
                .as_ref()
                .is_none_or(|pending| pending.id != batch.id)
        {
            return Err(Error::UnknownBatch);
        }
        Ok(())
    }
    fn transact<T>(&mut self, apply: impl FnOnce(&mut State) -> Result<T>) -> Result<T> {
        self.require_idle()?;
        let mut proposed = self.state.clone();
        let result = apply(&mut proposed)?;
        validate_state(&proposed, self.limits)?;
        self.state = proposed;
        Ok(result)
    }
}

fn count(value: usize) -> u32 {
    u32::try_from(value).expect("admitted host count fits u32")
}
fn free_pages(state: &State) -> u32 {
    count(state.pages.iter().filter(|page| page.refs == 0).count())
}
fn advance_clock(state: &mut State, now: u64) -> Result<()> {
    if now < state.clock {
        return Err(Error::ClockRegression);
    }
    state.clock = now;
    Ok(())
}
fn find_child(state: &State, parent: Option<usize>, edge: &[u32; 16]) -> Option<usize> {
    state.nodes.iter().position(|node| {
        node.as_ref()
            .is_some_and(|node| node.parent == parent && &node.edge == edge)
    })
}
fn is_leaf(state: &State, index: usize) -> bool {
    !state
        .nodes
        .iter()
        .flatten()
        .any(|node| node.parent == Some(index))
}
fn remove_node(state: &mut State, index: usize) -> Result<()> {
    if !is_leaf(state, index) {
        return Err(Error::Invariant);
    }
    let node = state.nodes[index].take().ok_or(Error::Invariant)?;
    let page = &mut state.pages[node.page as usize];
    if !page.cached {
        return Err(Error::Invariant);
    }
    page.cached = false;
    page.refs = page.refs.checked_sub(1).ok_or(Error::Invariant)?;
    state.evicted_pages = state.evicted_pages.checked_add(1).ok_or(Error::Exhausted)?;
    Ok(())
}
fn expire_nodes(state: &mut State, now: u64) -> Result<()> {
    loop {
        let candidate = state.nodes.iter().enumerate().find_map(|(index, node)| {
            node.as_ref()
                .and_then(|node| (node.expires <= now && is_leaf(state, index)).then_some(index))
        });
        let Some(index) = candidate else {
            return Ok(());
        };
        remove_node(state, index)?;
    }
}
fn drop_sequence_refs(state: &mut State, sequence: &Sequence) -> Result<()> {
    for page in &sequence.pages {
        state.pages[*page as usize].refs = state.pages[*page as usize]
            .refs
            .checked_sub(1)
            .ok_or(Error::Invariant)?;
    }
    Ok(())
}
fn validate_state(state: &State, limits: EngineeringTpPagedLimitsV1) -> Result<()> {
    if state.pages.len() != limits.physical_pages as usize
        || state.nodes.len() != state.pages.len()
        || state.sequences.len() > limits.sequences as usize
    {
        return Err(Error::Invariant);
    }
    let mut refs = vec![0_u32; state.pages.len()];
    let mut cached = vec![false; state.pages.len()];
    let mut edges = std::collections::BTreeSet::new();
    for (serial, sequence) in &state.sequences {
        if *serial == 0
            || *serial >= state.next_sequence
            || sequence.tokens.len() > limits.context_tokens as usize
            || sequence.pages.len() != sequence.tokens.len().div_ceil(16)
        {
            return Err(Error::Invariant);
        }
        let mut unique = std::collections::BTreeSet::new();
        for page in &sequence.pages {
            let index = *page as usize;
            if index >= refs.len() || !unique.insert(*page) {
                return Err(Error::Invariant);
            }
            refs[index] += 1;
        }
        if !sequence.tokens.len().is_multiple_of(16) {
            let page = *sequence.pages.last().ok_or(Error::Invariant)? as usize;
            if state.pages[page].cached || state.pages[page].refs != 1 {
                return Err(Error::Invariant);
            }
        }
    }
    for (index, node) in state
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(i, node)| node.as_ref().map(|node| (i, node)))
    {
        let page = node.page as usize;
        if page >= refs.len()
            || cached[page]
            || !edges.insert((node.parent, node.edge))
            || node.touched > state.clock
            || node.expires <= node.touched
        {
            return Err(Error::Invariant);
        }
        cached[page] = true;
        refs[page] += 1;
        let mut parent = node.parent;
        let mut depth = 1;
        while let Some(ancestor) = parent {
            if ancestor == index || depth >= state.nodes.len() {
                return Err(Error::Invariant);
            }
            parent = state
                .nodes
                .get(ancestor)
                .and_then(Option::as_ref)
                .ok_or(Error::Invariant)?
                .parent;
            depth += 1;
        }
        if depth > limits.page_table_stride() as usize {
            return Err(Error::Invariant);
        }
    }
    if state
        .pages
        .iter()
        .enumerate()
        .any(|(index, page)| page.refs != refs[index] || page.cached != cached[index])
    {
        return Err(Error::Invariant);
    }
    Ok(())
}

#[cfg(test)]
#[path = "tp_paged/tests.rs"]
mod tests;
