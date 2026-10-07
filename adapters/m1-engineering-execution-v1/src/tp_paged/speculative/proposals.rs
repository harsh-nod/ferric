//! Private K-row draft transaction; tentative inputs never use ordinary commits.

use super::{
    EngineeringTpSpeculativeKvV1, Error, Phase, Result, Round, WorkIdentity, indexed_rows, inputs,
};
use crate::tp_paged::{
    EngineeringTpBatchCompletionV1, EngineeringTpPageRowV1, EngineeringTpPreparedBatchV1,
    EngineeringTpPreparedRowV1, State, validate_state,
};
use ferric_spec::{
    CorrectionBonusKvDisposition, QWEN3_VOCABULARY_SIZE, Qwen3ModelRole, Qwen3PlanBucket,
    SpeculativeKvInterval, SpeculativeKvRoundIndex,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ProposalIdentity {
    round: WorkIdentity,
    ordinal: u8,
    batch: u64,
}

/// Move-only private one-row work, issued only after both role reservations exist.
/// Dropping a ticket leaves the pair blocked and submitted; it does not abort KV.
pub struct EngineeringTpDraftProposalWorkV1<'a> {
    identity: ProposalIdentity,
    batch: &'a EngineeringTpPreparedBatchV1,
}

impl EngineeringTpDraftProposalWorkV1<'_> {
    /// Exact current input; future unknown inputs and target reservation stay private.
    #[must_use]
    pub const fn batch(&self) -> &EngineeringTpPreparedBatchV1 {
        self.batch
    }

    #[must_use]
    pub const fn ordinal(&self) -> u8 {
        self.identity.ordinal
    }

    pub(crate) fn validate(&self) -> bool {
        self.identity.round.role == Qwen3ModelRole::Draft06B
            && !self.identity.round.catch_up
            && self.identity.ordinal < 16
            && self.batch.rows.len() == 1
            && self.batch.pool == self.identity.round.pool
            && self.batch.id == self.identity.batch
            && self.batch.id > self.identity.round.batch
    }

    pub(crate) fn seal(
        self,
        completion: EngineeringTpBatchCompletionV1,
        choices: Vec<u32>,
        output_rows: &[usize],
    ) -> Result<EngineeringTpDraftProposalResultV1> {
        if !self.validate()
            || completion.pool != self.batch.pool
            || completion.batch != self.batch.id
        {
            return Err(Error::Completion);
        }
        if output_rows != [0] || choices.len() != 1 || choices[0] >= QWEN3_VOCABULARY_SIZE {
            return Err(Error::Choices);
        }
        let choice = choices.into_iter().next().ok_or(Error::Choices)?;
        Ok(EngineeringTpDraftProposalResultV1 {
            identity: self.identity,
            input: self.batch.rows[0].input,
            choice,
            completion,
        })
    }
}

/// One driver-sealed choice, not token publication or a target acceptance claim.
/// Ordinary mutable outputs have no conversion to this result.
///
/// ```compile_fail
/// use ferric_m1_engineering_execution_v1::tp_paged::speculative::EngineeringTpDraftProposalResultV1;
/// fn replace(result: &mut EngineeringTpDraftProposalResultV1) {
///     result.choice = 42;
/// }
/// ```
#[derive(Debug)]
pub struct EngineeringTpDraftProposalResultV1 {
    identity: ProposalIdentity,
    input: EngineeringTpPageRowV1,
    choice: u32,
    completion: EngineeringTpBatchCompletionV1,
}

impl EngineeringTpDraftProposalResultV1 {
    /// Read-only diagnostic identity; no completion or publication authority escapes.
    #[must_use]
    pub const fn request(&self) -> ferric_spec::RequestId {
        self.identity.round.request
    }

    #[must_use]
    pub const fn completion_epoch(&self) -> ferric_spec::completion::CompletionEpoch {
        self.identity.round.epoch
    }

    #[must_use]
    pub const fn pool_identity(&self) -> u64 {
        self.identity.round.pool
    }

    #[must_use]
    pub const fn batch_id(&self) -> u64 {
        self.identity.batch
    }

    #[must_use]
    pub const fn ordinal(&self) -> u8 {
        self.identity.ordinal
    }

    /// Input actually consumed by this completed one-row forward.
    #[must_use]
    pub const fn input(&self) -> EngineeringTpPageRowV1 {
        self.input
    }

    /// Observed choice only; callers cannot replace the retained sealed value.
    #[must_use]
    pub const fn choice(&self) -> u32 {
        self.choice
    }
}

#[derive(Debug)]
pub(super) struct ProposalRound {
    pub(super) target: EngineeringTpPreparedBatchV1,
    pub(super) draft: EngineeringTpPreparedBatchV1,
    index: SpeculativeKvRoundIndex,
    first_batch: u64,
    results: Vec<EngineeringTpDraftProposalResultV1>,
    active: Option<EngineeringTpPreparedBatchV1>,
}

/// Retains each real row completion; no synthetic whole-batch completion is minted.
#[derive(Debug)]
pub(super) struct CompletedDraftProposals {
    first_batch: u64,
    results: Vec<EngineeringTpDraftProposalResultV1>,
}

impl CompletedDraftProposals {
    pub(super) fn validate(
        &self,
        owner: &EngineeringTpSpeculativeKvV1,
        round: &Round,
    ) -> Result<()> {
        round
            .index
            .validate_for(
                owner.request,
                owner.epoch,
                &owner.plan,
                owner.target_selection,
                owner.draft_selection,
            )
            .map_err(|_| Error::Index)?;
        if self.results.len() != usize::from(round.index.draft_token_count)
            || inputs(&round.target)
                != indexed_rows(
                    &round.index,
                    Qwen3ModelRole::Target8B,
                    owner.target_sequence,
                )?
            || inputs(&round.draft)
                != indexed_rows(&round.index, Qwen3ModelRole::Draft06B, owner.draft_sequence)?
        {
            return Err(Error::Binding);
        }
        let identity = owner.identity(Qwen3ModelRole::Draft06B, &round.draft, false);
        for (ordinal, result) in self.results.iter().enumerate() {
            validate_result(
                result,
                identity,
                u8::try_from(ordinal).map_err(|_| Error::Index)?,
                self.first_batch
                    .checked_add(ordinal as u64)
                    .ok_or(Error::Exhausted)?,
                round.draft.rows[ordinal].input,
            )?;
            if result.choice != round.index.draft_tokens[ordinal] {
                return Err(Error::Choices);
            }
        }
        Ok(())
    }
}

impl EngineeringTpSpeculativeKvV1 {
    /// Reserves target K+1 and draft K capacity before exposing the first draft row.
    /// K and all identities come from the attached plan; no candidate IDs are supplied.
    /// Unresolved tokens are private placeholders, never returned to either driver.
    /// # Errors
    /// Rejects non-ready state, unequal cursors, bounds, exhausted IDs or either OOM.
    /// A failed draft reservation aborts the unsubmitted target reservation.
    pub fn reserve_proposals(&mut self) -> Result<()> {
        if !matches!(self.phase, Phase::Ready) {
            return Err(Error::Phase);
        }
        let index = proposal_index(self)?;
        self.epoch.value.checked_add(1).ok_or(Error::Exhausted)?;
        let next_draft_batch = self
            .draft
            .next_batch
            .checked_add(u64::from(index.draft_token_count) + 1)
            .ok_or(Error::Exhausted)?;
        let target_rows = indexed_rows(&index, Qwen3ModelRole::Target8B, self.target_sequence)?;
        let draft_rows = indexed_rows(&index, Qwen3ModelRole::Draft06B, self.draft_sequence)?;
        let target = self.target.reserve_batch(&target_rows)?;
        let draft = match self.draft.reserve_batch(&draft_rows) {
            Ok(batch) => batch,
            Err(error) => {
                self.target.abort_batch(&target)?;
                return Err(error.into());
            }
        };
        let first_batch = self.draft.next_batch;
        self.draft.next_batch = next_draft_batch;
        self.phase = Phase::Proposing(Box::new(ProposalRound {
            target,
            draft,
            index,
            first_batch,
            results: Vec::with_capacity(usize::from(index.draft_token_count)),
            active: None,
        }));
        Ok(())
    }

    /// Issues only the anchor or the previous sealed draft choice at the next cursor.
    /// The aggregate draft reservation stays submitted between all one-row forwards.
    /// # Errors
    /// Rejects an outstanding/dropped ticket, completed proposal phase or stale metadata.
    pub fn proposal_work(&mut self) -> Result<EngineeringTpDraftProposalWorkV1<'_>> {
        let Phase::Proposing(round) = &self.phase else {
            return Err(Error::Phase);
        };
        if round.active.is_some()
            || round.results.len() >= usize::from(round.index.draft_token_count)
        {
            return Err(Error::Phase);
        }
        self.target.require_batch(&round.target)?;
        self.draft.require_batch(&round.draft)?;
        let ordinal = u8::try_from(round.results.len()).map_err(|_| Error::Index)?;
        let identity = ProposalIdentity {
            round: self.identity(Qwen3ModelRole::Draft06B, &round.draft, false),
            ordinal,
            batch: round
                .first_batch
                .checked_add(u64::from(ordinal))
                .ok_or(Error::Exhausted)?,
        };
        if ordinal == 0 {
            self.draft.begin_submission(&round.draft)?;
        } else if !self
            .draft
            .pending
            .as_ref()
            .is_some_and(|pending| pending.submitted)
        {
            self.terminalize_if_submitted();
            return Err(Error::Completion);
        }
        let Phase::Proposing(round) = &mut self.phase else {
            unreachable!("proposal phase was checked without a phase mutation");
        };
        let row = &round.draft.rows[usize::from(ordinal)];
        let batch = EngineeringTpPreparedBatchV1 {
            pool: round.draft.pool,
            scope: round.draft.scope,
            id: identity.batch,
            limits: round.draft.limits,
            rows: vec![EngineeringTpPreparedRowV1 {
                input: row.input,
                pages: row.pages.clone(),
                write_page: row.write_page,
            }],
        };
        Ok(EngineeringTpDraftProposalWorkV1 {
            identity,
            batch: round.active.insert(batch),
        })
    }

    /// Feeds a driver-sealed choice into the next private row, never a committed prefix.
    /// After exactly K results, exposes the resolved K+1 target verification ticket.
    /// # Errors
    /// A stale, foreign, duplicate or malformed submitted result terminalizes both pools.
    pub fn record_proposal(&mut self, result: EngineeringTpDraftProposalResultV1) -> Result<()> {
        let checked = self.preflight_proposal(&result);
        let (target_state, draft_state, index) = match checked {
            Ok(states) => states,
            Err(error) => {
                self.terminalize_if_submitted();
                return Err(error);
            }
        };
        let (Some(target), Some(draft)) = (&mut self.target.pending, &mut self.draft.pending)
        else {
            self.terminalize_if_submitted();
            return Err(Error::Completion);
        };
        let Phase::Proposing(mut round) = std::mem::replace(&mut self.phase, Phase::Terminal)
        else {
            unreachable!("successful proposal preflight retains its phase");
        };
        let ordinal = round.results.len();
        round.target.rows[ordinal + 1].input.token = result.choice;
        if ordinal + 1 < round.draft.rows.len() {
            round.draft.rows[ordinal + 1].input.token = result.choice;
        }
        target.proposed = target_state;
        draft.proposed = draft_state;
        round.index = index;
        round.active = None;
        round.results.push(result);
        self.phase = if round.results.len() == usize::from(index.draft_token_count) {
            Phase::Round(Box::new(Round {
                index,
                target: round.target,
                draft: round.draft,
                target_result: None,
                draft_result: None,
                proposal_results: Some(CompletedDraftProposals {
                    first_batch: round.first_batch,
                    results: round.results,
                }),
            }))
        } else {
            Phase::Proposing(round)
        };
        Ok(())
    }

    fn preflight_proposal(
        &self,
        result: &EngineeringTpDraftProposalResultV1,
    ) -> Result<(State, State, SpeculativeKvRoundIndex)> {
        let Phase::Proposing(round) = &self.phase else {
            return Err(Error::Phase);
        };
        self.target.require_batch(&round.target)?;
        self.draft.require_batch(&round.draft)?;
        self.target.check_invariants()?;
        self.draft.check_invariants()?;
        let active = round.active.as_ref().ok_or(Error::Phase)?;
        let ordinal = round.results.len();
        if ordinal >= usize::from(round.index.draft_token_count)
            || self
                .target
                .pending
                .as_ref()
                .is_none_or(|pending| pending.submitted)
            || self
                .draft
                .pending
                .as_ref()
                .is_none_or(|pending| !pending.submitted)
        {
            return Err(Error::Completion);
        }
        validate_result(
            result,
            self.identity(Qwen3ModelRole::Draft06B, &round.draft, false),
            u8::try_from(ordinal).map_err(|_| Error::Index)?,
            active.id,
            round.draft.rows[ordinal].input,
        )?;
        if inputs(active) != [result.input]
            || active.id
                != round
                    .first_batch
                    .checked_add(ordinal as u64)
                    .ok_or(Error::Exhausted)?
        {
            return Err(Error::Binding);
        }
        let mut target = self
            .target
            .pending
            .as_ref()
            .ok_or(Error::Phase)?
            .proposed
            .clone();
        let mut draft = self
            .draft
            .pending
            .as_ref()
            .ok_or(Error::Phase)?
            .proposed
            .clone();
        let position = round.index.target_pre_committed as usize + ordinal + 1;
        *target
            .sequences
            .get_mut(&self.target_sequence.serial)
            .and_then(|sequence| sequence.tokens.get_mut(position))
            .ok_or(Error::Binding)? = result.choice;
        if ordinal + 1 < usize::from(round.index.draft_token_count) {
            *draft
                .sequences
                .get_mut(&self.draft_sequence.serial)
                .and_then(|sequence| sequence.tokens.get_mut(position))
                .ok_or(Error::Binding)? = result.choice;
        }
        let mut index = round.index;
        index.draft_tokens[ordinal] = result.choice;
        index
            .validate_for(
                self.request,
                self.epoch,
                &self.plan,
                self.target_selection,
                self.draft_selection,
            )
            .map_err(|_| Error::Index)?;
        validate_state(&target, self.target.limits)?;
        validate_state(&draft, self.draft.limits)?;
        Ok((target, draft, index))
    }
}

fn validate_result(
    result: &EngineeringTpDraftProposalResultV1,
    round: WorkIdentity,
    ordinal: u8,
    batch: u64,
    input: EngineeringTpPageRowV1,
) -> Result<()> {
    if result.identity
        != (ProposalIdentity {
            round,
            ordinal,
            batch,
        })
        || batch <= round.batch
        || result.input != input
        || result.completion.pool != round.pool
        || result.completion.batch != batch
    {
        return Err(Error::Completion);
    }
    if result.choice >= QWEN3_VOCABULARY_SIZE {
        return Err(Error::Choices);
    }
    Ok(())
}

fn proposal_index(owner: &EngineeringTpSpeculativeKvV1) -> Result<SpeculativeKvRoundIndex> {
    let k: u8 = match owner.target_selection.bucket {
        Qwen3PlanBucket::SpeculativeS1K4C8192 | Qwen3PlanBucket::SpeculativeS8K4C8192 => 4,
        Qwen3PlanBucket::SpeculativeS1K8C8192 => 8,
        Qwen3PlanBucket::SpeculativeS1K16C8192 => 16,
        _ => return Err(Error::Index),
    };
    let cursor = owner.target.committed_position(owner.target_sequence)?;
    if owner.draft.committed_position(owner.draft_sequence)? != cursor {
        return Err(Error::Binding);
    }
    let mut target_commit_ends = [0; 17];
    let mut draft_commit_ends = [0; 17];
    for accepted in 0..=k {
        target_commit_ends[usize::from(accepted)] = cursor
            .checked_add(u32::from(accepted) + 1)
            .ok_or(Error::Exhausted)?;
        draft_commit_ends[usize::from(accepted)] = cursor
            .checked_add(u32::from((accepted + 1).min(k)))
            .ok_or(Error::Exhausted)?;
    }
    let index = SpeculativeKvRoundIndex {
        request: owner.request,
        completion_epoch: owner.epoch,
        plan_id: owner.plan,
        target_selection: owner.target_selection,
        draft_selection: owner.draft_selection,
        draft_token_count: k,
        round_anchor: owner.anchor,
        draft_tokens: [0; 16],
        target_pre_committed: cursor,
        draft_pre_committed: cursor,
        target_tentative: SpeculativeKvInterval {
            start: cursor,
            end: target_commit_ends[usize::from(k)],
        },
        draft_tentative: SpeculativeKvInterval {
            start: cursor,
            end: draft_commit_ends[usize::from(k)],
        },
        target_commit_ends,
        draft_commit_ends,
        correction_bonus: CorrectionBonusKvDisposition::DeferredUntilNextStep,
    };
    index
        .validate_for(
            owner.request,
            owner.epoch,
            &owner.plan,
            owner.target_selection,
            owner.draft_selection,
        )
        .map_err(|_| Error::Index)?;
    Ok(index)
}

#[cfg(test)]
mod tests;
