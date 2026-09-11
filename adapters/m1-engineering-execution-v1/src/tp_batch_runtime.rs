//! Continuous engineering request coordination over one resident paged TP group.
//!
//! Scheduling and page metadata commit only after the entire physical batch.
//! Any submitted failure stops reuse permanently. This does not implement HTTP,
//! protected execution, or an unlimited device ring; the driver retains its
//! explicit packet bound. No model weights are cloned per request.

use crate::tp_execution::batched::{EngineeringTpBatchExecutionV2, EngineeringTpBatchOutputV2};
use crate::tp_execution::{EngineeringTpRankTransportV1, TpResult};
use crate::tp_paged::{
    EngineeringTpPageRowV1, EngineeringTpPagedErrorV1, EngineeringTpPagedPoolV1,
    EngineeringTpPagedStatsV1, EngineeringTpPreparedBatchV1, EngineeringTpSequenceIdV1,
};
use crate::tp_scheduler::{
    EngineeringTpSchedulerV1, TpBatchRowKindV1, TpBatchRowV1, TpOutputEventV1,
    TpRequestAdmissionV1, TpRequestIdV1, TpRequestRecordV1, TpRequestStateV1, TpRowChoiceV1,
};

/// Actual GPU work boundary; external code cannot fabricate the pool completion token.
pub trait EngineeringTpBatchRunnerV2 {
    /// Optional diagnostic binding within the same submission transaction.
    /// # Errors
    /// Rejects stale selected diagnostic row identities.
    fn bind_numerical_rows(&mut self, _batch: u64, _rows: &[TpBatchRowV1]) -> TpResult<()> {
        Ok(())
    }
    /// Optional diagnostic manifest, never a performance qualification.
    /// # Errors
    /// Rejects incomplete selected captures or failed teardown.
    fn finish_numerical_capture(&mut self) -> TpResult<Option<serde_json::Value>> {
        Ok(None)
    }
    /// Physical workspace and admitted kernel row envelope, never a logical hint.
    fn row_capacity(&self) -> usize {
        16
    }
    /// Executes every layer and rank for the supplied immutable prepared batch.
    /// # Errors
    /// Rejects invalid metadata, exhausted budgets, or incomplete device work.
    fn execute_batch(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
        output_rows: &[usize],
    ) -> TpResult<EngineeringTpBatchOutputV2>;
    /// Independently expected kernel increments, before accepting completion.
    fn expected_dispatch_counts(&self, _published: usize) -> Vec<u64> {
        self.dispatch_counts()
            .iter()
            .enumerate()
            .map(|(rank, _)| if rank == 0 { 544 } else { 540 })
            .collect()
    }
    /// Number of physical rows sent through the vocabulary projection.
    fn output_head_rows(&self, physical: usize, _published: usize) -> usize {
        physical
    }
    /// Successful dispatch counts, in rank order.
    fn dispatch_counts(&self) -> Vec<u64>;
    /// Confirms all child processes were closed and reaped.
    /// # Errors
    /// Reports incomplete teardown after attempting all ranks.
    fn close(&mut self) -> TpResult<()>;
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpBatchRunnerV2
    for EngineeringTpBatchExecutionV2<R>
{
    fn bind_numerical_rows(&mut self, batch: u64, rows: &[TpBatchRowV1]) -> TpResult<()> {
        self.bind_numerical_rows(batch, rows)
    }
    fn finish_numerical_capture(&mut self) -> TpResult<Option<serde_json::Value>> {
        self.finish_numerical_capture()
    }
    fn row_capacity(&self) -> usize {
        self.row_capacity()
    }
    fn execute_batch(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
        output_rows: &[usize],
    ) -> TpResult<EngineeringTpBatchOutputV2> {
        self.execute_selected(batch, output_rows)
    }
    fn expected_dispatch_counts(&self, published: usize) -> Vec<u64> {
        self.expected_dispatch_counts(published)
    }
    fn output_head_rows(&self, physical: usize, published: usize) -> usize {
        self.output_head_rows(physical, published)
    }
    fn dispatch_counts(&self) -> Vec<u64> {
        self.dispatch_counts()
    }
    fn close(&mut self) -> TpResult<()> {
        self.close()
    }
}

/// A request admitted against initialized cached pages, not merely equal text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EngineeringTpAdmissionV2 {
    /// Generational scheduler ID used for subsequent operations.
    pub request: TpRequestIdV1,
    /// Number of initialized prompt tokens actually reused.
    pub cached_tokens: u32,
    /// Number of complete immutable physical pages reused.
    pub cached_pages: u32,
}

/// Raw row and output events for one committed GPU batch.
#[derive(Debug)]
pub struct EngineeringTpBatchReportV2 {
    /// Scheduler batch generation, distinct from the pool's transaction ID.
    pub batch_id: u64,
    /// Pool transaction whose physical work was committed.
    pub pool_batch_id: u64,
    /// Exact ordered token rows, including intermediate prefill rows.
    pub rows: Vec<TpBatchRowV1>,
    /// Publishable choices only, after page and request-state commit.
    pub outputs: Vec<TpOutputEventV1>,
    /// Controller time before scheduling/reservation.
    pub started_ns: u64,
    /// Controller time after physical completion.
    pub completed_ns: u64,
    /// Per-rank dispatch increments for this batch.
    pub rank_dispatch_counts: Vec<u64>,
    /// Physical rows whose logits were computed, including any baseline discarded rows.
    pub output_head_rows: usize,
}

/// Resident scheduler, shared physical KV ownership, and one GPU group.
pub struct EngineeringTpBatchRuntimeV2<G: EngineeringTpBatchRunnerV2> {
    gpu: G,
    pool: EngineeringTpPagedPoolV1,
    scheduler: EngineeringTpSchedulerV1,
    bindings: Vec<(TpRequestIdV1, EngineeringTpSequenceIdV1)>,
    row_budget: usize,
    retain_prefixes: bool,
    world_size: usize,
    poisoned: bool,
    closed: bool,
}

impl<G: EngineeringTpBatchRunnerV2> EngineeringTpBatchRuntimeV2<G> {
    /// Joins a fresh pool and scheduler with the driver already bound to that pool.
    /// # Errors
    /// Rejects preexisting request/cache metadata or an unsupported row budget.
    pub fn new(
        gpu: G,
        pool: EngineeringTpPagedPoolV1,
        scheduler: EngineeringTpSchedulerV1,
        row_budget: usize,
        retain_prefixes: bool,
    ) -> TpResult<Self> {
        Self::new_bounded(gpu, pool, scheduler, row_budget, retain_prefixes, 16)
    }

    /// Joins an explicit 32-row pool, driver and scheduler without microbatch splitting.
    /// # Errors
    /// Rejects mismatched physical capacities or preexisting request/cache metadata.
    pub fn new_wide32(
        gpu: G,
        pool: EngineeringTpPagedPoolV1,
        scheduler: EngineeringTpSchedulerV1,
        row_budget: usize,
        retain_prefixes: bool,
    ) -> TpResult<Self> {
        Self::new_bounded(gpu, pool, scheduler, row_budget, retain_prefixes, 32)
    }

    fn new_bounded(
        mut gpu: G,
        pool: EngineeringTpPagedPoolV1,
        scheduler: EngineeringTpSchedulerV1,
        row_budget: usize,
        retain_prefixes: bool,
        row_capacity: usize,
    ) -> TpResult<Self> {
        let counts = gpu.dispatch_counts();
        if !pool.is_empty()
            || scheduler.retained_requests() != 0
            || scheduler.is_poisoned()
            || !(1..=row_capacity).contains(&row_budget)
            || pool.row_capacity() != row_capacity
            || gpu.row_capacity() != row_capacity
            || scheduler.context_limit() != pool.limits().context_tokens()
            || row_budget > scheduler.max_batch_rows()
            || !matches!(counts.len(), 1 | 2 | 8)
            || counts.iter().any(|&count| count != 0)
        {
            let _ = gpu.close();
            return Err(
                "batch runtime requires fresh state and matching physical row envelopes".into(),
            );
        }
        Ok(Self {
            gpu,
            pool,
            scheduler,
            bindings: Vec::new(),
            row_budget,
            retain_prefixes,
            world_size: counts.len(),
            poisoned: false,
            closed: false,
        })
    }

    /// Admits another request between physical batches without uploading weights.
    /// # Errors
    /// Rejects invalid input/time, full metadata capacity, closed or poisoned state.
    pub fn admit(
        &mut self,
        mut admission: TpRequestAdmissionV1,
        tick: u64,
        now_ns: u64,
    ) -> TpResult<EngineeringTpAdmissionV2> {
        self.require_ready()?;
        if admission.cached_prefix_tokens != 0 {
            return Err("callers cannot assert initialized cached prefix tokens".into());
        }
        self.pool
            .expire(tick)
            .map_err(|e| format!("page expiration: {e:?}"))?;
        let hit = self
            .pool
            .open_sequence(self.pool.scope(), &admission.prompt_tokens, tick)
            .map_err(|e| format!("page admission: {e:?}"))?;
        admission.cached_prefix_tokens = hit.hit_tokens();
        let request = match self.scheduler.admit(admission, now_ns) {
            Ok(request) => request,
            Err(error) => {
                if let Err(cancel) = self.pool.cancel_sequence(hit.sequence()) {
                    self.poisoned = true;
                    return Err(format!(
                        "scheduler admission: {error:?}; page cancellation: {cancel:?}"
                    ));
                }
                return Err(format!("scheduler admission: {error:?}"));
            }
        };
        self.bindings.push((request, hit.sequence()));
        Ok(EngineeringTpAdmissionV2 {
            request,
            cached_tokens: hit.hit_tokens(),
            cached_pages: hit.hit_pages(),
        })
    }

    /// Schedules one mixed prefill/decode batch and commits only after every rank.
    ///
    /// A pre-submission page shortage retries smaller token budgets down to one.
    /// If none fit, the runtime remains usable for explicit request cancellation.
    /// `completion_clock` must return a monotonic controller timestamp.
    ///
    /// # Errors
    /// Reports backpressure, invalid time/state, or a permanently poisoned submitted batch.
    pub fn step(
        &mut self,
        tick: u64,
        now_ns: u64,
        completion_clock: impl FnOnce() -> u64,
    ) -> TpResult<Option<EngineeringTpBatchReportV2>> {
        self.require_ready()?;
        let mut budget = self.row_budget;
        let (scheduled, prepared) = loop {
            let Some(scheduled) = self
                .scheduler
                .prepare(tick, now_ns, budget)
                .map_err(|e| format!("schedule: {e:?}"))?
            else {
                return Ok(None);
            };
            let rows = scheduled
                .rows()
                .iter()
                .map(|row| {
                    Ok(EngineeringTpPageRowV1 {
                        sequence: self.sequence(row.request)?,
                        token: row.token_id,
                        position: row.absolute_position,
                    })
                })
                .collect::<TpResult<Vec<_>>>();
            let rows = match rows {
                Ok(rows) => rows,
                Err(error) => {
                    self.poisoned = true;
                    let _ = self.scheduler.abort(scheduled.id());
                    return Err(error);
                }
            };
            let reservation = match self.pool.reserve_batch(&rows) {
                Err(EngineeringTpPagedErrorV1::OutOfPages) => {
                    // A new logical page starts only at a page boundary. Eviction
                    // targets the free count for this complete proposed batch.
                    let needed = rows.iter().filter(|row| row.position % 16 == 0).count();
                    let needed = u32::try_from(needed).map_err(|_| "page count overflow")?;
                    match self.pool.evict_unused(needed) {
                        Ok(()) => self.pool.reserve_batch(&rows),
                        Err(error) => Err(error),
                    }
                }
                result => result,
            };
            match reservation {
                Ok(prepared) => break (scheduled, prepared),
                Err(error) => {
                    self.scheduler
                        .abort(scheduled.id())
                        .map_err(|e| format!("schedule abort: {e:?}"))?;
                    if error == EngineeringTpPagedErrorV1::OutOfPages && budget > 1 {
                        budget = (budget / 2).max(1);
                        continue;
                    }
                    return Err(format!("page reservation: {error:?}"));
                }
            }
        };
        let before = self.gpu.dispatch_counts();
        let output_rows = scheduled
            .rows()
            .iter()
            .enumerate()
            .filter(|(_, row)| row.kind != TpBatchRowKindV1::PrefillIntermediate)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let expected_counts = self.gpu.expected_dispatch_counts(output_rows.len());
        let output_head_rows = self
            .gpu
            .output_head_rows(scheduled.rows().len(), output_rows.len());
        if let Err(error) = self.pool.begin_submission(&prepared) {
            let aborted = self.pool.abort_batch(&prepared);
            let scheduled_abort = self.scheduler.abort(scheduled.id());
            if aborted.is_err() || scheduled_abort.is_err() {
                self.poisoned = true;
            }
            return Err(format!(
                "begin page submission: {error:?}; abort={aborted:?}/{scheduled_abort:?}"
            ));
        }
        let committed = (|| {
            self.gpu
                .bind_numerical_rows(scheduled.id(), scheduled.rows())?;
            let output = self.gpu.execute_batch(&prepared, &output_rows)?;
            let completed_ns = completion_clock();
            if output.choices.len() != output_rows.len() || completed_ns < now_ns {
                return Err("GPU row count or completion clock drifted".into());
            }
            let choices = output_rows
                .iter()
                .zip(output.choices)
                .map(|(&row_index, token_id)| TpRowChoiceV1 {
                    row_index,
                    token_id,
                })
                .collect::<Vec<_>>();
            let after = self.gpu.dispatch_counts();
            if before.len() != self.world_size || after.len() != self.world_size {
                return Err("rank dispatch roster drifted".into());
            }
            let rank_dispatch_counts = after
                .iter()
                .zip(before)
                .map(|(after, before)| {
                    after
                        .checked_sub(before)
                        .ok_or_else(|| "rank dispatch count regressed".to_owned())
                })
                .collect::<TpResult<Vec<_>>>()?;
            if expected_counts.len() != self.world_size || rank_dispatch_counts != expected_counts {
                return Err("rank did not complete the exact Qwen3-8B batch schedule".into());
            }
            self.scheduler
                .validate_completion(scheduled.id(), &choices, completed_ns)
                .map_err(|e| format!("scheduler completion preflight: {e:?}"))?;
            self.pool
                .commit_batch(&prepared, output.completion)
                .map_err(|e| format!("page commit: {e:?}"))?;
            let outputs = self
                .scheduler
                .complete(scheduled.id(), &choices, completed_ns)
                .map_err(|e| format!("scheduler completion: {e:?}"))?;
            Ok(EngineeringTpBatchReportV2 {
                batch_id: scheduled.id(),
                pool_batch_id: prepared.id(),
                rows: scheduled.rows().to_vec(),
                outputs,
                started_ns: now_ns,
                completed_ns,
                rank_dispatch_counts,
                output_head_rows,
            })
        })();
        match committed {
            Ok(report) => Ok(Some(report)),
            Err(error) => {
                self.poisoned = true;
                let _ = self.pool.quarantine_batch(&prepared);
                let _ = self.scheduler.fail(scheduled.id());
                Err(error)
            }
        }
    }

    /// Cancels an idle request without publishing its prefix into the cache.
    /// # Errors
    /// Rejects stale/terminal requests, pending work, bad time, or unusable state.
    pub fn cancel(&mut self, request: TpRequestIdV1, now_ns: u64) -> TpResult<()> {
        self.require_ready()?;
        self.scheduler
            .cancel(request, now_ns)
            .map_err(|e| format!("cancel: {e:?}"))
    }

    /// Releases a terminal request; successful complete pages may remain in radix cache.
    /// # Errors
    /// Rejects nonterminal/stale requests, invalid cache time, or unusable state.
    pub fn retire(&mut self, request: TpRequestIdV1, tick: u64) -> TpResult<TpRequestRecordV1> {
        self.require_ready()?;
        let state = self
            .scheduler
            .request(request)
            .map_err(|e| format!("request: {e:?}"))?
            .state();
        if !matches!(
            state,
            TpRequestStateV1::Completed | TpRequestStateV1::Cancelled
        ) {
            return Err("only terminal requests can retire".into());
        }
        let sequence = self.sequence(request)?;
        if state == TpRequestStateV1::Cancelled {
            self.pool
                .cancel_sequence(sequence)
                .map_err(|e| format!("cancel pages: {e:?}"))?;
        } else {
            self.pool
                .retire_sequence(sequence, self.retain_prefixes, tick)
                .map_err(|e| format!("retire pages: {e:?}"))?;
        }
        match self.scheduler.retire(request) {
            Ok(record) => {
                self.bindings.retain(|(id, _)| *id != request);
                Ok(record)
            }
            Err(error) => {
                self.poisoned = true;
                Err(format!("retire request: {error:?}"))
            }
        }
    }

    /// Borrows committed request progress without granting page-release authority.
    /// # Errors
    /// Rejects stale request IDs.
    pub fn request(&self, id: TpRequestIdV1) -> TpResult<&TpRequestRecordV1> {
        self.scheduler
            .request(id)
            .map_err(|e| format!("request: {e:?}"))
    }

    /// Bounded page/cache statistics, not measured numerical correctness.
    #[must_use]
    pub fn page_stats(&self) -> EngineeringTpPagedStatsV1 {
        self.pool.stats()
    }

    /// Number of active or unretired terminal requests.
    #[must_use]
    pub fn retained_requests(&self) -> usize {
        self.scheduler.retained_requests()
    }

    /// Cumulative successful kernel dispatches in rank order.
    #[must_use]
    pub fn dispatch_counts(&self) -> Vec<u64> {
        self.gpu.dispatch_counts()
    }

    /// Closes the resident group; no subsequent admission or reuse is permitted.
    /// # Errors
    /// Reports incomplete worker teardown.
    pub fn close(&mut self) -> TpResult<()> {
        self.closed = true;
        self.gpu.close()
    }

    /// Publishes optional complete diagnostic artifacts after clean model teardown.
    /// # Errors
    /// Rejects incomplete or failed captures.
    pub fn finish_numerical_capture(&mut self) -> TpResult<Option<serde_json::Value>> {
        self.gpu.finish_numerical_capture()
    }

    fn sequence(&self, id: TpRequestIdV1) -> TpResult<EngineeringTpSequenceIdV1> {
        self.bindings
            .iter()
            .find(|(request, _)| *request == id)
            .map(|(_, sequence)| *sequence)
            .ok_or_else(|| "scheduler/page request binding is absent".into())
    }

    fn require_ready(&self) -> TpResult<()> {
        if self.closed || self.poisoned {
            Err("resident TP runtime is closed or poisoned".into())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
