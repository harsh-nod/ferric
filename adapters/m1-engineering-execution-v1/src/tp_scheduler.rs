//! Bounded continuous scheduling for the isolated engineering TP path.
//!
//! This host scheduler is Contracted, not a Verus proof of scheduling or GPU
//! completion. IDs are local to one controller. A caller must commit the paged
//! KV transaction only after every GPU rank completes, then call `complete`.
//! Pre-submission backpressure may call `abort`; submitted failure must call
//! `fail`. No operation here grants device, cache, or protected M1 authority.

/// Maximum simultaneously retained requests, including unretired terminals.
pub const TP_MAX_REQUESTS_V1: usize = 32;
/// Maximum token rows in one physical GPU batch.
pub const TP_MAX_BATCH_ROWS_V1: usize = 16;
/// Maximum committed KV positions for one request.
pub const TP_MAX_CONTEXT_V1: u32 = 8192;

/// Instance-local slot identity, invalidated by retirement followed by reuse.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TpRequestIdV1 {
    /// Slot in this scheduler, never a device pointer or cache capability.
    pub slot: u8,
    /// Nonzero monotonically increasing generation of that slot.
    pub generation: u64,
}

/// Whether this row produces a publishable greedy choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TpBatchRowKindV1 {
    /// A known prompt token before its final position; discard logits.
    PrefillIntermediate,
    /// The final prompt token produces the first output token.
    PrefillFinal,
    /// The prior output token is consumed to produce the next output token.
    Decode,
}

/// One causal input row; same-request rows are contiguous and position ordered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TpBatchRowV1 {
    /// Current request generation.
    pub request: TpRequestIdV1,
    /// Input token, not the token that this row will generate.
    pub token_id: u32,
    /// Absolute zero-based KV position to append.
    pub absolute_position: u32,
    /// Whether a returned choice is required or prohibited.
    pub kind: TpBatchRowKindV1,
}

/// Bounded, immutable description of the scheduler's outstanding batch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpScheduledBatchV1 {
    id: u64,
    tick: u64,
    prepared_ns: u64,
    rows: Vec<TpBatchRowV1>,
}

impl TpScheduledBatchV1 {
    /// Instance-local nonzero batch generation.
    #[must_use]
    pub fn id(&self) -> u64 {
        self.id
    }
    /// Caller-provided monotonic admission tick.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.tick
    }
    /// Monotonic controller time immediately before pool reservation.
    #[must_use]
    pub fn prepared_ns(&self) -> u64 {
        self.prepared_ns
    }
    /// Exact row order to pass unchanged to the page pool and driver.
    #[must_use]
    pub fn rows(&self) -> &[TpBatchRowV1] {
        &self.rows
    }
}

/// Exactly one choice for each final-prompt or decode row, in any order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TpRowChoiceV1 {
    /// Index into `TpScheduledBatchV1::rows`.
    pub row_index: usize,
    /// GPU-produced greedy token, checked against the configured vocabulary.
    pub token_id: u32,
}

/// A successfully committed, publishable output and its measurement event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TpOutputEventV1 {
    /// Producing request generation.
    pub request: TpRequestIdV1,
    /// Zero-based ordinal in this request's generated stream.
    pub output_index: u32,
    /// Newly generated token.
    pub token_id: u32,
    /// Controller completion time; subtract arrival for TTFT or prior output for TPOT.
    pub completed_ns: u64,
    /// This output reached the admitted fixed-length output limit.
    pub finished: bool,
}

/// Current logical state, independent of page-pool ownership or GPU authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TpRequestStateV1 {
    /// Some prompt tokens still require execution.
    Prefill,
    /// The last generated choice is ready to become the next input token.
    Decode,
    /// The fixed output count was reached; explicit retirement is still required.
    Completed,
    /// Cancellation was recorded; explicit retirement is still required.
    Cancelled,
}

/// Validated request admission; weight storage is deliberately absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpRequestAdmissionV1 {
    /// Nonempty input token sequence from the admitted tokenizer/model.
    pub prompt_tokens: Vec<u32>,
    /// Positive fixed output count, including the first prompt-final choice.
    pub max_new_tokens: u32,
    /// Page-pool-confirmed initialized prefix, strictly shorter than the prompt.
    pub cached_prefix_tokens: u32,
    /// First scheduler tick at which this request is eligible.
    pub arrival_tick: u64,
    /// Monotonic arrival time, which may precede its admission during prior GPU work.
    pub arrival_ns: u64,
}

/// Retained per-request progress and raw timing events, including after completion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpRequestRecordV1 {
    /// Slot and generation for joining controller/page-pool records.
    pub id: TpRequestIdV1,
    /// Full original prompt, including any cached prefix.
    pub prompt_tokens: Vec<u32>,
    /// Admitted output limit.
    pub max_new_tokens: u32,
    /// Initial prefix supplied by the page pool, not recomputed from token equality.
    pub cached_prefix_tokens: u32,
    /// Successful GPU positions plus the externally confirmed cached prefix.
    pub committed_position: u32,
    /// Every successfully published choice in output order.
    pub generated_tokens: Vec<u32>,
    /// One controller timestamp per generated token, preserving TTFT/TPOT inputs.
    pub output_timestamps_ns: Vec<u64>,
    /// Requested first admission tick.
    pub arrival_tick: u64,
    /// Original arrival time; never replaced by the first scheduled tick.
    pub arrival_ns: u64,
    /// Terminal cancellation time, if explicitly cancelled.
    pub cancelled_ns: Option<u64>,
}

impl TpRequestRecordV1 {
    /// Derives logical phase without conferring page-release authority.
    #[must_use]
    pub fn state(&self) -> TpRequestStateV1 {
        if self.cancelled_ns.is_some() {
            TpRequestStateV1::Cancelled
        } else if self.generated_tokens.len() == self.max_new_tokens as usize {
            TpRequestStateV1::Completed
        } else if usize::try_from(self.committed_position)
            .is_ok_and(|position| position < self.prompt_tokens.len())
        {
            TpRequestStateV1::Prefill
        } else {
            TpRequestStateV1::Decode
        }
    }
}

/// Fail-closed errors; invalid inputs do not partially advance requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TpSchedulerErrorV1 {
    /// Constructor limits are unsupported.
    InvalidConfiguration,
    /// Prompt, vocabulary, prefix, or output/context bound is invalid.
    InvalidAdmission,
    /// All slots remain occupied, including terminal records not yet retired.
    AtCapacity,
    /// Every vacant slot has exhausted its generation counter.
    GenerationOverflow,
    /// A slot/generation does not name a current request.
    StaleRequest,
    /// A batch is outstanding; request membership and cancellation are frozen.
    Busy,
    /// The supplied batch generation is absent or different.
    StaleBatch,
    /// A submitted batch failed; the entire scheduler is permanently unusable.
    Poisoned,
    /// The token budget is zero or exceeds the configured physical row limit.
    InvalidBudget,
    /// Caller tick/time regressed, or completion predates preparation/arrival.
    InvalidClock,
    /// Batch generation cannot increase without overflow.
    BatchOverflow,
    /// A required output is missing/duplicated/invalid, or an intermediate row has one.
    InvalidChoices,
    /// Completion or cancellation has not occurred, or terminal state was already reached.
    InvalidState,
}

#[derive(Debug)]
struct Slot {
    generation: u64,
    request: Option<TpRequestRecordV1>,
}

#[derive(Debug)]
struct Pending {
    batch: TpScheduledBatchV1,
    decode_cursor: usize,
    prefill_cursor: usize,
    single_row_prefill: bool,
}

/// Continuous, decode-priority scheduler with bounded prefill starvation.
///
/// Decode and prefill have independent rotating cursors. When both are ready,
/// budgets above one reserve at least one prefill row; a one-row budget
/// alternates phases after successful commits. Aborted batches do not consume
/// fairness turns. A prefill chunk never crosses into decode because that
/// input depends on the not-yet-produced first choice.
#[derive(Debug)]
pub struct EngineeringTpSchedulerV1 {
    vocabulary: u32,
    context_limit: u32,
    max_rows: usize,
    prefill_chunk: usize,
    slots: [Slot; TP_MAX_REQUESTS_V1],
    pending: Option<Pending>,
    last_batch_id: u64,
    last_tick: u64,
    last_time_ns: u64,
    decode_cursor: usize,
    prefill_cursor: usize,
    single_row_prefill: bool,
    poisoned: bool,
}

impl EngineeringTpSchedulerV1 {
    /// Creates an empty scheduler, independent of resident GPU weights.
    ///
    /// # Errors
    /// Requires vocabulary>0, context1..=8192, and chunk/row limits1..=16.
    pub fn new(
        vocabulary: u32,
        context_limit: u32,
        max_rows: usize,
        prefill_chunk: usize,
    ) -> Result<Self, TpSchedulerErrorV1> {
        if vocabulary == 0
            || !(1..=TP_MAX_CONTEXT_V1).contains(&context_limit)
            || !(1..=TP_MAX_BATCH_ROWS_V1).contains(&max_rows)
            || !(1..=max_rows).contains(&prefill_chunk)
        {
            return Err(TpSchedulerErrorV1::InvalidConfiguration);
        }
        Ok(Self {
            vocabulary,
            context_limit,
            max_rows,
            prefill_chunk,
            slots: std::array::from_fn(|_| Slot {
                generation: 0,
                request: None,
            }),
            pending: None,
            last_batch_id: 0,
            last_tick: 0,
            last_time_ns: 0,
            decode_cursor: 0,
            prefill_cursor: 0,
            single_row_prefill: false,
            poisoned: false,
        })
    }

    /// Admits another request between batches; completed peers need not retire first.
    ///
    /// # Errors
    /// Rejects pending/poisoned state, invalid geometry/tokens/time, or exhausted slots.
    pub fn admit(
        &mut self,
        admission: TpRequestAdmissionV1,
        now_ns: u64,
    ) -> Result<TpRequestIdV1, TpSchedulerErrorV1> {
        self.require_idle()?;
        if admission.arrival_ns > now_ns || now_ns < self.last_time_ns {
            return Err(TpSchedulerErrorV1::InvalidClock);
        }
        let prompt_len = u32::try_from(admission.prompt_tokens.len())
            .map_err(|_| TpSchedulerErrorV1::InvalidAdmission)?;
        let total_positions = prompt_len
            .checked_add(admission.max_new_tokens)
            .and_then(|sum| sum.checked_sub(1));
        if prompt_len == 0
            || admission.max_new_tokens == 0
            || admission.cached_prefix_tokens >= prompt_len
            || total_positions.is_none_or(|count| count > self.context_limit)
            || admission
                .prompt_tokens
                .iter()
                .any(|&token| token >= self.vocabulary)
        {
            return Err(TpSchedulerErrorV1::InvalidAdmission);
        }
        let slot_index = self
            .slots
            .iter()
            .position(|slot| slot.request.is_none() && slot.generation < u64::MAX)
            .ok_or_else(|| {
                if self.slots.iter().any(|slot| slot.request.is_none()) {
                    TpSchedulerErrorV1::GenerationOverflow
                } else {
                    TpSchedulerErrorV1::AtCapacity
                }
            })?;
        let id = TpRequestIdV1 {
            slot: u8::try_from(slot_index).map_err(|_| TpSchedulerErrorV1::InvalidState)?,
            generation: self.slots[slot_index].generation + 1,
        };
        let output_capacity = usize::try_from(admission.max_new_tokens)
            .map_err(|_| TpSchedulerErrorV1::InvalidAdmission)?;
        let request = TpRequestRecordV1 {
            id,
            prompt_tokens: admission.prompt_tokens,
            max_new_tokens: admission.max_new_tokens,
            cached_prefix_tokens: admission.cached_prefix_tokens,
            committed_position: admission.cached_prefix_tokens,
            generated_tokens: Vec::with_capacity(output_capacity),
            output_timestamps_ns: Vec::with_capacity(output_capacity),
            arrival_tick: admission.arrival_tick,
            arrival_ns: admission.arrival_ns,
            cancelled_ns: None,
        };
        self.slots[slot_index].generation = id.generation;
        self.slots[slot_index].request = Some(request);
        self.last_time_ns = now_ns;
        Ok(id)
    }

    /// Borrows retained progress for controller accounting and pool consistency checks.
    ///
    /// # Errors
    /// Rejects an absent slot or stale generation, even when that slot is reused.
    pub fn request(&self, id: TpRequestIdV1) -> Result<&TpRequestRecordV1, TpSchedulerErrorV1> {
        self.slots
            .get(usize::from(id.slot))
            .and_then(|slot| slot.request.as_ref())
            .filter(|request| request.id == id)
            .ok_or(TpSchedulerErrorV1::StaleRequest)
    }

    /// Number of occupied slots, including terminal records awaiting retirement.
    #[must_use]
    pub fn retained_requests(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| slot.request.is_some())
            .count()
    }

    /// Whether any submitted failure permanently prohibited reuse.
    #[must_use]
    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    /// Prepares exact causal rows without advancing any committed request progress.
    ///
    /// # Errors
    /// Rejects invalid budget/clock, pending or poisoned state, or batch-ID overflow.
    pub fn prepare(
        &mut self,
        tick: u64,
        now_ns: u64,
        token_budget: usize,
    ) -> Result<Option<TpScheduledBatchV1>, TpSchedulerErrorV1> {
        self.require_idle()?;
        if !(1..=self.max_rows).contains(&token_budget) {
            return Err(TpSchedulerErrorV1::InvalidBudget);
        }
        if tick < self.last_tick || now_ns < self.last_time_ns {
            return Err(TpSchedulerErrorV1::InvalidClock);
        }
        let decode = self.ready(TpRequestStateV1::Decode, self.decode_cursor, tick, now_ns);
        let prefill = self.ready(TpRequestStateV1::Prefill, self.prefill_cursor, tick, now_ns);
        if decode.is_empty() && prefill.is_empty() {
            self.last_tick = tick;
            self.last_time_ns = now_ns;
            return Ok(None);
        }
        let id = self
            .last_batch_id
            .checked_add(1)
            .ok_or(TpSchedulerErrorV1::BatchOverflow)?;
        let both_ready = !decode.is_empty() && !prefill.is_empty();
        let decode_limit = if both_ready {
            if token_budget == 1 {
                usize::from(!self.single_row_prefill)
            } else {
                token_budget - 1
            }
        } else {
            token_budget
        };
        let mut rows = Vec::with_capacity(token_budget);
        let mut next_decode_cursor = self.decode_cursor;
        let mut next_prefill_cursor = self.prefill_cursor;
        for index in decode.into_iter().take(decode_limit) {
            let request = self.slots[index]
                .request
                .as_ref()
                .ok_or(TpSchedulerErrorV1::InvalidState)?;
            rows.push(TpBatchRowV1 {
                request: request.id,
                token_id: *request
                    .generated_tokens
                    .last()
                    .ok_or(TpSchedulerErrorV1::InvalidState)?,
                absolute_position: request.committed_position,
                kind: TpBatchRowKindV1::Decode,
            });
            next_decode_cursor = (index + 1) % TP_MAX_REQUESTS_V1;
        }
        for index in prefill {
            let remaining = token_budget - rows.len();
            if remaining == 0 {
                break;
            }
            let request = self.slots[index]
                .request
                .as_ref()
                .ok_or(TpSchedulerErrorV1::InvalidState)?;
            let start = request.committed_position as usize;
            let count = (request.prompt_tokens.len() - start)
                .min(remaining)
                .min(self.prefill_chunk);
            for position in start..start + count {
                rows.push(TpBatchRowV1 {
                    request: request.id,
                    token_id: request.prompt_tokens[position],
                    absolute_position: u32::try_from(position)
                        .map_err(|_| TpSchedulerErrorV1::InvalidState)?,
                    kind: if position + 1 == request.prompt_tokens.len() {
                        TpBatchRowKindV1::PrefillFinal
                    } else {
                        TpBatchRowKindV1::PrefillIntermediate
                    },
                });
            }
            next_prefill_cursor = (index + 1) % TP_MAX_REQUESTS_V1;
        }
        let batch = TpScheduledBatchV1 {
            id,
            tick,
            prepared_ns: now_ns,
            rows,
        };
        self.pending = Some(Pending {
            batch: batch.clone(),
            decode_cursor: next_decode_cursor,
            prefill_cursor: next_prefill_cursor,
            single_row_prefill: if both_ready && token_budget == 1 {
                !self.single_row_prefill
            } else {
                self.single_row_prefill
            },
        });
        self.last_batch_id = id;
        self.last_tick = tick;
        self.last_time_ns = now_ns;
        Ok(Some(batch))
    }

    /// Commits all rows atomically after the caller's successful GPU and pool transaction.
    ///
    /// # Errors
    /// Invalid generation, time, or output roster leaves the entire pending batch unchanged.
    /// GPU/pool failure must use `fail`, not a fabricated successful completion.
    pub fn complete(
        &mut self,
        batch_id: u64,
        choices: &[TpRowChoiceV1],
        completed_ns: u64,
    ) -> Result<Vec<TpOutputEventV1>, TpSchedulerErrorV1> {
        let pending = self.pending(batch_id)?;
        if completed_ns < pending.batch.prepared_ns {
            return Err(TpSchedulerErrorV1::InvalidClock);
        }
        let mut by_row = vec![None; pending.batch.rows.len()];
        for choice in choices {
            let Some(row) = pending.batch.rows.get(choice.row_index) else {
                return Err(TpSchedulerErrorV1::InvalidChoices);
            };
            if row.kind == TpBatchRowKindV1::PrefillIntermediate
                || choice.token_id >= self.vocabulary
                || by_row[choice.row_index].replace(choice.token_id).is_some()
            {
                return Err(TpSchedulerErrorV1::InvalidChoices);
            }
        }
        let mut advances = [0_u32; TP_MAX_REQUESTS_V1];
        let mut events = Vec::with_capacity(choices.len());
        for (index, row) in pending.batch.rows.iter().enumerate() {
            let request = self.request(row.request)?;
            let slot = usize::from(row.request.slot);
            let expected_position = request
                .committed_position
                .checked_add(advances[slot])
                .ok_or(TpSchedulerErrorV1::InvalidState)?;
            if row.absolute_position != expected_position || expected_position >= self.context_limit
            {
                return Err(TpSchedulerErrorV1::InvalidState);
            }
            advances[slot] += 1;
            if row.kind != TpBatchRowKindV1::PrefillIntermediate {
                let token_id = by_row[index].ok_or(TpSchedulerErrorV1::InvalidChoices)?;
                let output_index = u32::try_from(request.generated_tokens.len())
                    .map_err(|_| TpSchedulerErrorV1::InvalidState)?;
                if output_index >= request.max_new_tokens {
                    return Err(TpSchedulerErrorV1::InvalidState);
                }
                events.push(TpOutputEventV1 {
                    request: row.request,
                    output_index,
                    token_id,
                    completed_ns,
                    finished: output_index + 1 == request.max_new_tokens,
                });
            }
        }
        // Every fallible check precedes publication; membership is frozen while pending.
        let pending = self.pending.take().ok_or(TpSchedulerErrorV1::StaleBatch)?;
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if let Some(request) = slot.request.as_mut() {
                request.committed_position += advances[index];
                for event in events.iter().filter(|event| event.request == request.id) {
                    request.generated_tokens.push(event.token_id);
                    request.output_timestamps_ns.push(event.completed_ns);
                }
            }
        }
        self.decode_cursor = pending.decode_cursor;
        self.prefill_cursor = pending.prefill_cursor;
        self.single_row_prefill = pending.single_row_prefill;
        self.last_tick = pending.batch.tick;
        self.last_time_ns = completed_ns;
        Ok(events)
    }

    /// Aborts before any GPU submission, preserving request progress and fairness turns.
    ///
    /// # Errors
    /// Rejects an absent/mismatched batch or a poisoned scheduler. Never use after submission.
    pub fn abort(&mut self, batch_id: u64) -> Result<(), TpSchedulerErrorV1> {
        self.pending(batch_id)?;
        self.pending = None;
        Ok(())
    }

    /// Permanently quarantines scheduler reuse after a submitted batch fails.
    ///
    /// # Errors
    /// Rejects an absent/mismatched batch or an already poisoned scheduler.
    pub fn fail(&mut self, batch_id: u64) -> Result<(), TpSchedulerErrorV1> {
        self.pending(batch_id)?;
        self.poisoned = true;
        Ok(())
    }

    /// Records cancellation between batches; retirement remains explicit.
    ///
    /// # Errors
    /// Rejects pending/poisoned state, stale ID, terminal state, or regressing time.
    pub fn cancel(&mut self, id: TpRequestIdV1, now_ns: u64) -> Result<(), TpSchedulerErrorV1> {
        self.require_idle()?;
        let request = self.request(id)?;
        if matches!(
            request.state(),
            TpRequestStateV1::Completed | TpRequestStateV1::Cancelled
        ) {
            return Err(TpSchedulerErrorV1::InvalidState);
        }
        if now_ns < request.arrival_ns || now_ns < self.last_time_ns {
            return Err(TpSchedulerErrorV1::InvalidClock);
        }
        self.slots[usize::from(id.slot)]
            .request
            .as_mut()
            .ok_or(TpSchedulerErrorV1::StaleRequest)?
            .cancelled_ns = Some(now_ns);
        self.last_time_ns = now_ns;
        Ok(())
    }

    /// Removes only a completed/cancelled record after the caller retires its pool sequence.
    ///
    /// # Errors
    /// Rejects pending/poisoned state, stale ID, or nonterminal progress.
    pub fn retire(&mut self, id: TpRequestIdV1) -> Result<TpRequestRecordV1, TpSchedulerErrorV1> {
        self.require_idle()?;
        if !matches!(
            self.request(id)?.state(),
            TpRequestStateV1::Completed | TpRequestStateV1::Cancelled
        ) {
            return Err(TpSchedulerErrorV1::InvalidState);
        }
        self.slots[usize::from(id.slot)]
            .request
            .take()
            .ok_or(TpSchedulerErrorV1::StaleRequest)
    }

    fn require_idle(&self) -> Result<(), TpSchedulerErrorV1> {
        if self.poisoned {
            Err(TpSchedulerErrorV1::Poisoned)
        } else if self.pending.is_some() {
            Err(TpSchedulerErrorV1::Busy)
        } else {
            Ok(())
        }
    }

    fn pending(&self, batch_id: u64) -> Result<&Pending, TpSchedulerErrorV1> {
        if self.poisoned {
            return Err(TpSchedulerErrorV1::Poisoned);
        }
        self.pending
            .as_ref()
            .filter(|pending| pending.batch.id == batch_id)
            .ok_or(TpSchedulerErrorV1::StaleBatch)
    }

    fn ready(&self, state: TpRequestStateV1, cursor: usize, tick: u64, now_ns: u64) -> Vec<usize> {
        (0..TP_MAX_REQUESTS_V1)
            .map(|offset| (cursor + offset) % TP_MAX_REQUESTS_V1)
            .filter(|&index| {
                self.slots[index].request.as_ref().is_some_and(|request| {
                    request.state() == state
                        && request.arrival_tick <= tick
                        && request.arrival_ns <= now_ns
                })
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "tp_scheduler/tests.rs"]
mod tests;
