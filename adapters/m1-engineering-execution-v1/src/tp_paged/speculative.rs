//! Paired engineering KV settlement, not protected publication or GPU proof.
//!
//! Both pools are owned so ordinary full-batch commits cannot bypass settlement.
//! Candidate IDs are untrusted proposals. Only an opaque target-driver result
//! supplies verification choices. Draft consumed-input completion is sealed only
//! inside the separately admitted paged `Draft06B` driver, never by a caller claim.

use super::{
    EngineeringTpBatchCompletionV1, EngineeringTpPageRowV1, EngineeringTpPagedErrorV1,
    EngineeringTpPagedPoolV1, EngineeringTpPreparedBatchV1, EngineeringTpSequenceIdV1, State,
    count, validate_state,
};
use ferric_spec::completion::CompletionEpoch;
use ferric_spec::{
    GreedyCommit, Identity, QWEN3_VOCABULARY_SIZE, Qwen3ModelRole, Qwen3PlanSelection, RequestId,
    SpeculativeKvInputSource, SpeculativeKvRoundIndex, verify_greedy_round,
};

/// Fail-closed engineering settlement rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineeringTpSpeculativeErrorV1 {
    /// Underlying pool state or reservation rejected the operation.
    Pool(EngineeringTpPagedErrorV1),
    /// The exact finite round index or retained identity does not match.
    Index,
    /// Target/draft identities, committed inputs, or role bindings differ.
    Binding,
    /// A prior transition is incomplete, repeated, or terminal.
    Phase,
    /// An opaque completion does not bind this exact work ticket.
    Completion,
    /// Target choices are malformed or not bound to every verification row.
    Choices,
    /// The next epoch cannot be represented.
    Exhausted,
}

type Error = EngineeringTpSpeculativeErrorV1;
/// Result for this non-authoritative paired metadata API.
pub type EngineeringTpSpeculativeResultV1<T> = std::result::Result<T, Error>;
type Result<T> = EngineeringTpSpeculativeResultV1<T>;

impl From<EngineeringTpPagedErrorV1> for Error {
    fn from(error: EngineeringTpPagedErrorV1) -> Self {
        Self::Pool(error)
    }
}

/// Observable transition phase. No phase itself asserts device completion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineeringTpSpeculativePhaseV1 {
    Ready,
    Reserved,
    DraftCatchUpRequired,
    DraftCatchUpReserved,
    Terminal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WorkIdentity {
    request: RequestId,
    epoch: CompletionEpoch,
    plan: Identity,
    role: Qwen3ModelRole,
    pool: u64,
    batch: u64,
    catch_up: bool,
}

/// Non-clone target work, minted after marking the exact reservation submitted.
pub struct EngineeringTpSpeculativeTargetWorkV1<'a> {
    identity: WorkIdentity,
    batch: &'a EngineeringTpPreparedBatchV1,
    rows: usize,
}

impl EngineeringTpSpeculativeTargetWorkV1<'_> {
    pub(crate) fn batch(&self) -> &EngineeringTpPreparedBatchV1 {
        self.batch
    }

    pub(crate) fn validate(&self) -> bool {
        self.identity.role == Qwen3ModelRole::Target8B
            && !self.identity.catch_up
            && matches!(self.rows, 5 | 9 | 17)
            && self.rows == self.batch.rows.len()
            && self.identity.pool == self.batch.pool
            && self.identity.batch == self.batch.id
    }

    pub(crate) fn seal(
        self,
        completion: EngineeringTpBatchCompletionV1,
        choices: Vec<u32>,
        output_rows: Vec<usize>,
    ) -> Result<EngineeringTpSpeculativeTargetResultV1> {
        if !self.validate()
            || completion.pool != self.identity.pool
            || completion.batch != self.identity.batch
        {
            return Err(Error::Completion);
        }
        if output_rows != (0..self.rows).collect::<Vec<_>>()
            || choices.len() != self.rows
            || choices.iter().any(|token| *token >= QWEN3_VOCABULARY_SIZE)
        {
            return Err(Error::Choices);
        }
        Ok(EngineeringTpSpeculativeTargetResultV1 {
            identity: self.identity,
            rows: inputs(self.batch),
            output_rows,
            choices,
            completion,
        })
    }
}

/// Opaque target choices and exact row selection, sealed inside the driver.
/// Ordinary mutable `EngineeringTpBatchOutputV2` cannot be converted to this type.
///
/// ```compile_fail
/// use ferric_m1_engineering_execution_v1::tp_paged::speculative::EngineeringTpSpeculativeTargetResultV1;
/// fn replace(result: &mut EngineeringTpSpeculativeTargetResultV1) {
///     result.choices = vec![42];
/// }
/// ```
#[derive(Debug)]
pub struct EngineeringTpSpeculativeTargetResultV1 {
    identity: WorkIdentity,
    rows: Vec<EngineeringTpPageRowV1>,
    output_rows: Vec<usize>,
    choices: Vec<u32>,
    completion: EngineeringTpBatchCompletionV1,
}

/// Exact draft input work, consumed only by the role-checked paged draft driver.
pub struct EngineeringTpSpeculativeDraftWorkV1<'a> {
    identity: WorkIdentity,
    batch: &'a EngineeringTpPreparedBatchV1,
}

impl EngineeringTpSpeculativeDraftWorkV1<'_> {
    /// Read-only prepared inputs for the role-checked paged draft driver.
    #[must_use]
    pub const fn batch(&self) -> &EngineeringTpPreparedBatchV1 {
        self.batch
    }

    /// Distinguishes the one missing accepted input from a proposal round.
    #[must_use]
    pub const fn is_catch_up(&self) -> bool {
        self.identity.catch_up
    }

    pub(crate) fn validate(&self) -> bool {
        self.identity.role == Qwen3ModelRole::Draft06B
            && self.identity.pool == self.batch.pool
            && self.identity.batch == self.batch.id
            && if self.identity.catch_up {
                self.batch.rows.len() == 1
            } else {
                matches!(self.batch.rows.len(), 4 | 8 | 16)
            }
    }

    pub(crate) fn seal(
        self,
        completion: EngineeringTpBatchCompletionV1,
    ) -> Result<EngineeringTpSpeculativeDraftResultV1> {
        if !self.validate()
            || completion.pool != self.identity.pool
            || completion.batch != self.identity.batch
        {
            return Err(Error::Completion);
        }
        Ok(EngineeringTpSpeculativeDraftResultV1 {
            identity: self.identity,
            rows: inputs(self.batch),
            completion,
        })
    }

    #[cfg(test)]
    fn complete_for_test(self) -> EngineeringTpSpeculativeDraftResultV1 {
        EngineeringTpSpeculativeDraftResultV1 {
            identity: self.identity,
            rows: inputs(self.batch),
            completion: EngineeringTpBatchCompletionV1::after_all_ranks(self.batch),
        }
    }
}

/// Opaque consumed-input completion sealed only by the paged `Draft06B` driver.
/// No public constructor or ordinary batch-output conversion exists. Candidate
/// IDs remain untrusted proposals; this does not certify draft argmax choices.
///
/// ```compile_fail
/// use ferric_m1_engineering_execution_v1::tp_paged::speculative::EngineeringTpSpeculativeDraftWorkV1;
/// fn fabricate(work: EngineeringTpSpeculativeDraftWorkV1<'_>) {
///     let _result = work.complete_for_test();
/// }
/// ```
#[derive(Debug)]
pub struct EngineeringTpSpeculativeDraftResultV1 {
    identity: WorkIdentity,
    rows: Vec<EngineeringTpPageRowV1>,
    completion: EngineeringTpBatchCompletionV1,
}

#[derive(Debug)]
struct Round {
    index: SpeculativeKvRoundIndex,
    target: EngineeringTpPreparedBatchV1,
    draft: EngineeringTpPreparedBatchV1,
    target_result: Option<EngineeringTpSpeculativeTargetResultV1>,
    draft_result: Option<EngineeringTpSpeculativeDraftResultV1>,
}

#[derive(Debug)]
enum Phase {
    Ready,
    Round(Box<Round>),
    CatchUpRequired {
        token: u32,
    },
    CatchUp {
        token: u32,
        batch: EngineeringTpPreparedBatchV1,
    },
    Terminal,
}

/// Engineering receipt only; not a `StepPublication` or protected KV capability.
#[derive(Debug)]
pub struct EngineeringTpSpeculativeSettlementV1 {
    request: RequestId,
    epoch: CompletionEpoch,
    commit: GreedyCommit,
    target_cursor: u32,
    draft_cursor: u32,
}

impl EngineeringTpSpeculativeSettlementV1 {
    #[must_use]
    pub const fn request(&self) -> RequestId {
        self.request
    }
    #[must_use]
    pub const fn completion_epoch(&self) -> CompletionEpoch {
        self.epoch
    }
    #[must_use]
    pub fn greedy_commit(&self) -> &GreedyCommit {
        &self.commit
    }
    #[must_use]
    pub const fn target_cursor(&self) -> u32 {
        self.target_cursor
    }
    #[must_use]
    pub const fn draft_cursor(&self) -> u32 {
        self.draft_cursor
    }
    #[must_use]
    pub const fn draft_catch_up_required(&self) -> bool {
        self.target_cursor != self.draft_cursor
    }
}

/// Owns two independent role pools and one request's exact settlement state.
///
/// Existing target-only pool APIs are unchanged. No mutable pool access or
/// arbitrary rollback is exposed. Driver lifetimes/close remain the caller's
/// responsibility. This metadata owner does not launch or cancel GPU work.
#[derive(Debug)]
pub struct EngineeringTpSpeculativeKvV1 {
    target: EngineeringTpPagedPoolV1,
    draft: EngineeringTpPagedPoolV1,
    target_sequence: EngineeringTpSequenceIdV1,
    draft_sequence: EngineeringTpSequenceIdV1,
    request: RequestId,
    epoch: CompletionEpoch,
    plan: Identity,
    target_selection: Qwen3PlanSelection,
    draft_selection: Qwen3PlanSelection,
    anchor: u32,
    phase: Phase,
}

impl EngineeringTpSpeculativeKvV1 {
    /// Takes role pools after equal committed prefill; the anchor is not resident.
    /// Candidate IDs in `first` are untrusted proposals, not draft argmax evidence.
    /// # Errors
    /// Rejects malformed indexing, shared model identity, unequal committed
    /// tokens/cursors, stale sequences, or busy/poisoned pools. On error the
    /// caller must still close its resident drivers; no GPU resources are freed.
    pub fn new(
        target: EngineeringTpPagedPoolV1,
        target_sequence: EngineeringTpSequenceIdV1,
        draft: EngineeringTpPagedPoolV1,
        draft_sequence: EngineeringTpSequenceIdV1,
        first: &SpeculativeKvRoundIndex,
    ) -> Result<Self> {
        first.validate().map_err(|_| Error::Index)?;
        target.require_idle()?;
        draft.require_idle()?;
        target.check_invariants()?;
        draft.check_invariants()?;
        target.require_sequence(target_sequence)?;
        draft.require_sequence(draft_sequence)?;
        if target.identity == draft.identity
            || target.scope.model == draft.scope.model
            || target.state.sequences[&target_sequence.serial].tokens
                != draft.state.sequences[&draft_sequence.serial].tokens
            || target.committed_position(target_sequence)? != first.target_pre_committed
            || draft.committed_position(draft_sequence)? != first.draft_pre_committed
        {
            return Err(Error::Binding);
        }
        Ok(Self {
            target,
            draft,
            target_sequence,
            draft_sequence,
            request: first.request,
            epoch: first.completion_epoch,
            plan: first.plan_id,
            target_selection: first.target_selection,
            draft_selection: first.draft_selection,
            anchor: first.round_anchor,
            phase: Phase::Ready,
        })
    }

    /// Read-only pool state; mutable metadata never escapes this owner.
    #[must_use]
    pub const fn pool(&self, role: Qwen3ModelRole) -> &EngineeringTpPagedPoolV1 {
        match role {
            Qwen3ModelRole::Target8B => &self.target,
            Qwen3ModelRole::Draft06B => &self.draft,
        }
    }

    #[must_use]
    pub const fn phase(&self) -> EngineeringTpSpeculativePhaseV1 {
        match self.phase {
            Phase::Ready => EngineeringTpSpeculativePhaseV1::Ready,
            Phase::Round(_) => EngineeringTpSpeculativePhaseV1::Reserved,
            Phase::CatchUpRequired { .. } => EngineeringTpSpeculativePhaseV1::DraftCatchUpRequired,
            Phase::CatchUp { .. } => EngineeringTpSpeculativePhaseV1::DraftCatchUpReserved,
            Phase::Terminal => EngineeringTpSpeculativePhaseV1::Terminal,
        }
    }

    #[must_use]
    pub const fn next_epoch(&self) -> CompletionEpoch {
        self.epoch
    }
    /// Deferred correction/bonus, never an implicitly committed input.
    #[must_use]
    pub const fn next_anchor(&self) -> u32 {
        self.anchor
    }

    /// Reserves exactly K+1 target inputs and K draft consumed inputs atomically.
    /// # Errors
    /// Rejects a stale request/epoch/plan/role, noncanonical rows or insufficient
    /// pages. A failed second reservation aborts the first without committing it.
    pub fn reserve_round(&mut self, index: &SpeculativeKvRoundIndex) -> Result<()> {
        if !matches!(self.phase, Phase::Ready) {
            return Err(Error::Phase);
        }
        index
            .validate_for(
                self.request,
                self.epoch,
                &self.plan,
                self.target_selection,
                self.draft_selection,
            )
            .map_err(|_| Error::Index)?;
        if index.round_anchor != self.anchor
            || self.target.committed_position(self.target_sequence)? != index.target_pre_committed
            || self.draft.committed_position(self.draft_sequence)? != index.draft_pre_committed
        {
            return Err(Error::Binding);
        }
        self.epoch.value.checked_add(1).ok_or(Error::Exhausted)?;
        let target_rows = indexed_rows(index, Qwen3ModelRole::Target8B, self.target_sequence)?;
        let draft_rows = indexed_rows(index, Qwen3ModelRole::Draft06B, self.draft_sequence)?;
        let target = self.target.reserve_batch(&target_rows)?;
        let draft = match self.draft.reserve_batch(&draft_rows) {
            Ok(batch) => batch,
            Err(error) => {
                self.target.abort_batch(&target)?;
                return Err(error.into());
            }
        };
        self.phase = Phase::Round(Box::new(Round {
            index: *index,
            target,
            draft,
            target_result: None,
            draft_result: None,
        }));
        Ok(())
    }

    /// Marks target submission before handing work to the target driver.
    /// # Errors
    /// Rejects missing, repeated, catch-up or terminal work.
    pub fn target_work(&mut self) -> Result<EngineeringTpSpeculativeTargetWorkV1<'_>> {
        let Phase::Round(round) = &self.phase else {
            return Err(Error::Phase);
        };
        self.target.begin_submission(&round.target)?;
        Ok(EngineeringTpSpeculativeTargetWorkV1 {
            identity: self.identity(Qwen3ModelRole::Target8B, &round.target, false),
            batch: &round.target,
            rows: usize::from(round.index.draft_token_count) + 1,
        })
    }

    /// Marks draft submission. Production completion requires a future draft driver.
    /// # Errors
    /// Rejects missing, repeated or terminal work.
    pub fn draft_work(&mut self) -> Result<EngineeringTpSpeculativeDraftWorkV1<'_>> {
        let (batch, catch_up) = match &self.phase {
            Phase::Round(round) => (&round.draft, false),
            Phase::CatchUp { batch, .. } => (batch, true),
            _ => return Err(Error::Phase),
        };
        self.draft.begin_submission(batch)?;
        Ok(EngineeringTpSpeculativeDraftWorkV1 {
            identity: self.identity(Qwen3ModelRole::Draft06B, batch, catch_up),
            batch,
        })
    }

    /// Retains an exact opaque target result; arbitrary vectors are not accepted.
    /// # Errors
    /// A substituted/replayed result terminalizes any submitted paired work.
    pub fn record_target(&mut self, result: EngineeringTpSpeculativeTargetResultV1) -> Result<()> {
        let checked = (|| {
            let Phase::Round(round) = &self.phase else {
                return Err(Error::Phase);
            };
            self.validate_result(
                self.identity(Qwen3ModelRole::Target8B, &round.target, false),
                result.identity,
                &round.target,
                &result.rows,
                &result.completion,
            )?;
            if round.target_result.is_some() {
                return Err(Error::Completion);
            }
            if result.output_rows != (0..round.target.rows.len()).collect::<Vec<_>>()
                || result.choices.len() != round.target.rows.len()
                || result
                    .choices
                    .iter()
                    .any(|token| *token >= QWEN3_VOCABULARY_SIZE)
            {
                return Err(Error::Choices);
            }
            Ok(())
        })();
        if let Err(error) = checked {
            self.terminalize_if_submitted();
            return Err(error);
        }
        if let Phase::Round(round) = &mut self.phase {
            round.target_result = Some(result);
        }
        Ok(())
    }

    /// Records draft input completion, or commits the sealed one-row catch-up.
    /// # Errors
    /// Rejects stale role/epoch/batch/rows and terminalizes submitted uncertainty.
    pub fn record_draft(&mut self, result: EngineeringTpSpeculativeDraftResultV1) -> Result<()> {
        let checked = (|| {
            let (batch, catch_up) = match &self.phase {
                Phase::Round(round) if round.draft_result.is_none() => (&round.draft, false),
                Phase::CatchUp { batch, .. } => (batch, true),
                _ => return Err(Error::Phase),
            };
            self.validate_result(
                self.identity(Qwen3ModelRole::Draft06B, batch, catch_up),
                result.identity,
                batch,
                &result.rows,
                &result.completion,
            )?;
            if catch_up {
                let end = self.target.committed_position(self.target_sequence)?;
                let next = prefix_state(&self.draft, batch, self.draft_sequence, end)?;
                if next.sequences[&self.draft_sequence.serial].tokens
                    != self.target.state.sequences[&self.target_sequence.serial].tokens
                {
                    return Err(Error::Binding);
                }
                Ok(Some(next))
            } else {
                Ok(None)
            }
        })();
        match checked {
            Err(error) => {
                self.terminalize_if_submitted();
                Err(error)
            }
            Ok(Some(state)) => {
                self.draft.state = state;
                self.draft.pending = None;
                self.phase = Phase::Ready;
                Ok(())
            }
            Ok(None) => {
                if let Phase::Round(round) = &mut self.phase {
                    round.draft_result = Some(result);
                }
                Ok(())
            }
        }
    }

    /// Derives A from sealed target choices, then atomically retains accepted inputs.
    /// No fallible operation runs after both prefix-state preflights succeed.
    /// # Errors
    /// Rejects missing completion, invalid verification or prefix invariants.
    /// Submitted invariant/choice uncertainty terminalizes both pools.
    pub fn settle(&mut self) -> Result<EngineeringTpSpeculativeSettlementV1> {
        let Phase::Round(round) = &self.phase else {
            return Err(Error::Phase);
        };
        let (Some(target_result), Some(_)) = (&round.target_result, &round.draft_result) else {
            return Err(Error::Phase);
        };
        let checked = (|| {
            let k = usize::from(round.index.draft_token_count);
            let commit =
                verify_greedy_round(&round.index.draft_tokens[..k], &target_result.choices)
                    .map_err(|_| Error::Choices)?;
            let accepted =
                u8::try_from(commit.accepted_draft_tokens()).map_err(|_| Error::Choices)?;
            let target_end = round
                .index
                .target_commit_end(accepted)
                .ok_or(Error::Index)?;
            let draft_end = round.index.draft_commit_end(accepted).ok_or(Error::Index)?;
            let target = prefix_state(
                &self.target,
                &round.target,
                self.target_sequence,
                target_end,
            )?;
            let draft = prefix_state(&self.draft, &round.draft, self.draft_sequence, draft_end)?;
            let target_tokens = &target.sequences[&self.target_sequence.serial].tokens;
            let draft_tokens = &draft.sequences[&self.draft_sequence.serial].tokens;
            if target_tokens.get(..draft_tokens.len()) != Some(draft_tokens.as_slice()) {
                return Err(Error::Binding);
            }
            let phase = if usize::from(accepted) == k {
                Phase::CatchUpRequired {
                    token: round.index.draft_tokens[k - 1],
                }
            } else {
                Phase::Ready
            };
            let epoch =
                CompletionEpoch::new(self.epoch.value.checked_add(1).ok_or(Error::Exhausted)?);
            Ok((
                target,
                draft,
                phase,
                epoch,
                EngineeringTpSpeculativeSettlementV1 {
                    request: self.request,
                    epoch: self.epoch,
                    commit,
                    target_cursor: target_end,
                    draft_cursor: draft_end,
                },
            ))
        })();
        let (target, draft, phase, epoch, receipt) = match checked {
            Ok(value) => value,
            Err(error) => {
                self.terminalize_if_submitted();
                return Err(error);
            }
        };
        self.target.state = target;
        self.draft.state = draft;
        self.target.pending = None;
        self.draft.pending = None;
        self.anchor = receipt.commit.target_correction_or_bonus();
        self.epoch = epoch;
        self.phase = phase;
        Ok(receipt)
    }

    /// Reserves only the last accepted candidate missing from draft KV.
    /// # Errors
    /// Rejects other phases, stale cursor relation, OOM or poisoned state.
    pub fn reserve_draft_catch_up(&mut self) -> Result<()> {
        let Phase::CatchUpRequired { token } = self.phase else {
            return Err(Error::Phase);
        };
        let position = self.draft.committed_position(self.draft_sequence)?;
        if position.checked_add(1) != Some(self.target.committed_position(self.target_sequence)?) {
            return Err(Error::Binding);
        }
        let batch = self.draft.reserve_batch(&[EngineeringTpPageRowV1 {
            sequence: self.draft_sequence,
            token,
            position,
        }])?;
        self.phase = Phase::CatchUp { token, batch };
        Ok(())
    }

    /// Aborts only before any role submitted; committed state/epoch are unchanged.
    /// # Errors
    /// Submitted work cannot abort and must complete or terminalize.
    pub fn abort_unsubmitted(&mut self) -> Result<()> {
        if self.submitted() {
            return Err(Error::Phase);
        }
        match &self.phase {
            Phase::Round(round) => {
                self.target.require_batch(&round.target)?;
                self.draft.require_batch(&round.draft)?;
                self.target.pending = None;
                self.draft.pending = None;
                self.phase = Phase::Ready;
            }
            Phase::CatchUp { token, batch } => {
                self.draft.require_batch(batch)?;
                let token = *token;
                self.draft.pending = None;
                self.phase = Phase::CatchUpRequired { token };
            }
            _ => return Err(Error::Phase),
        }
        Ok(())
    }

    /// Quarantines both owned pools after any submitted GPU failure/uncertainty.
    /// Drivers must be closed separately; this never frees possibly-live storage.
    /// # Errors
    /// Rejects absence of submitted work rather than inventing a failed GPU step.
    pub fn fail_submitted(&mut self) -> Result<()> {
        if !self.submitted() {
            return Err(Error::Phase);
        }
        self.terminalize_if_submitted();
        Ok(())
    }

    fn identity(
        &self,
        role: Qwen3ModelRole,
        batch: &EngineeringTpPreparedBatchV1,
        catch_up: bool,
    ) -> WorkIdentity {
        WorkIdentity {
            request: self.request,
            epoch: self.epoch,
            plan: self.plan,
            role,
            pool: batch.pool,
            batch: batch.id,
            catch_up,
        }
    }

    fn validate_result(
        &self,
        expected: WorkIdentity,
        actual: WorkIdentity,
        batch: &EngineeringTpPreparedBatchV1,
        rows: &[EngineeringTpPageRowV1],
        completion: &EngineeringTpBatchCompletionV1,
    ) -> Result<()> {
        let pool = self.pool(expected.role);
        pool.require_batch(batch)?;
        if expected != actual
            || rows != inputs(batch)
            || completion.pool != expected.pool
            || completion.batch != expected.batch
            || !pool
                .pending
                .as_ref()
                .is_some_and(|pending| pending.submitted)
        {
            return Err(Error::Completion);
        }
        Ok(())
    }

    fn submitted(&self) -> bool {
        [&self.target, &self.draft].iter().any(|pool| {
            pool.pending
                .as_ref()
                .is_some_and(|pending| pending.submitted)
        })
    }

    fn terminalize_if_submitted(&mut self) {
        if self.submitted() {
            self.target.poisoned = true;
            self.draft.poisoned = true;
            self.phase = Phase::Terminal;
        }
    }
}

fn inputs(batch: &EngineeringTpPreparedBatchV1) -> Vec<EngineeringTpPageRowV1> {
    batch.rows.iter().map(|row| row.input).collect()
}

fn indexed_rows(
    index: &SpeculativeKvRoundIndex,
    role: Qwen3ModelRole,
    sequence: EngineeringTpSequenceIdV1,
) -> Result<Vec<EngineeringTpPageRowV1>> {
    let n = index.draft_token_count + u8::from(role == Qwen3ModelRole::Target8B);
    (0..n)
        .map(|ordinal| {
            let input = match role {
                Qwen3ModelRole::Target8B => index.target_input(ordinal),
                Qwen3ModelRole::Draft06B => index.draft_input(ordinal),
            }
            .ok_or(Error::Index)?;
            let token = match input.source {
                SpeculativeKvInputSource::RoundAnchor { token }
                | SpeculativeKvInputSource::DraftCandidate { token, .. } => token,
            };
            Ok(EngineeringTpPageRowV1 {
                sequence,
                token,
                position: input.logical_position,
            })
        })
        .collect()
}

fn prefix_state(
    pool: &EngineeringTpPagedPoolV1,
    batch: &EngineeringTpPreparedBatchV1,
    sequence_id: EngineeringTpSequenceIdV1,
    end: u32,
) -> Result<State> {
    pool.require_batch(batch)?;
    pool.check_invariants()?;
    let pending = pool.pending.as_ref().ok_or(Error::Phase)?;
    if !pending.submitted {
        return Err(Error::Phase);
    }
    let before = pool
        .state
        .sequences
        .get(&sequence_id.serial)
        .ok_or(Error::Binding)?;
    let mut next = pending.proposed.clone();
    let sequence = next
        .sequences
        .get_mut(&sequence_id.serial)
        .ok_or(Error::Binding)?;
    if end < count(before.tokens.len())
        || end > count(sequence.tokens.len())
        || sequence.tokens.get(..before.tokens.len()) != Some(before.tokens.as_slice())
        || sequence.tokens.len() - before.tokens.len() != batch.rows.len()
        || batch.rows.iter().enumerate().any(|(ordinal, row)| {
            row.input.sequence != sequence_id
                || row.input.position as usize != before.tokens.len() + ordinal
                || sequence.tokens.get(row.input.position as usize) != Some(&row.input.token)
        })
    {
        return Err(Error::Binding);
    }
    let retained = end.div_ceil(16) as usize;
    for page_id in &sequence.pages[retained..] {
        let page = &mut next.pages[*page_id as usize];
        if page.refs != 1 || page.cached || before.pages.contains(page_id) {
            return Err(Error::Binding);
        }
        page.refs = 0;
    }
    sequence.pages.truncate(retained);
    sequence.tokens.truncate(end as usize);
    validate_state(&next, pool.limits)?;
    Ok(next)
}

#[cfg(test)]
pub(crate) mod tests;
