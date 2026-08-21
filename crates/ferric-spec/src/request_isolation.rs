//! Logical composition of continuous batching with target/draft paged KV.
//!
//! This two-request relational witness ranges over any two distinct slots in
//! the full 32-slot continuous batch. It connects exact scheduler generations
//! and lifecycle phases to request-owned target/draft KV projections, while
//! framing the other request's complete scheduler and physical state.
//!
//! This is source-level sequential semantics only. It provides no queue,
//! device, address, runtime, HSA, machine, timing, or performance refinement.

use crate::completion::CompletionEpoch;
use crate::continuous_batching::{
    apply_continuous_batch_step, ContinuousBatch, ContinuousBatchAction, ContinuousBatchError,
    ContinuousRequest, M1_CONTINUOUS_BATCH_CAPACITY,
};
use crate::paged_kv_refinement::{
    append_physical_page, apply_preflighted_physical_speculative_settlement, cancel_physical_kv,
    commit_physical_kv, map_initialized_token, preflight_physical_speculative_settlement,
    release_retired_page, retire_cancelled_tail, rollback_physical_token, write_physical_token,
    KvQuiescenceAuthority, LogicalKvState, PhysicalKvError, PhysicalKvLifecycle,
    PhysicalKvLocation, PhysicalKvState, PhysicalPageId,
};
use crate::scheduling::{LifecyclePhase, RequestState};
use crate::{
    Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection, RequestId,
    SpeculativeKvRoundIndex,
};
use vstd::prelude::*;

verus! {

/// Scheduler actions that do not bypass coordinated cancellation or detachment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IsolatedSchedulerAction {
    Admit,
    Dispatch { epoch: CompletionEpoch },
    CompleteExact { epoch: CompletionEpoch },
    Publish { epoch: CompletionEpoch, emitted_tokens: u8 },
    FinalizeKv,
}

/// Request-owned physical KV actions admitted by the scheduler lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IsolatedKvAction {
    AppendPage { page: PhysicalPageId },
    WriteToken { logical_position: u32 },
    Commit { accepted_tokens: u32 },
    RollbackToken { after_epoch: CompletionEpoch },
    RetireTail { after_epoch: CompletionEpoch },
}

/// Fail-closed rejection from the logical composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestIsolationError {
    SlotOutOfRange,
    SameRequestSlot,
    StaleRequest,
    TargetRoleRequired,
    DraftRoleRequired,
    PlanPairMismatch,
    WrongLifecycle,
    MissingEpoch,
    EpochMismatch,
    TentativeTokensRemain,
    NoExactQuiescence,
    RetiredPageCountExhausted,
    RetiredPageCountUnderflow,
    GenerationExhausted,
    AcceptedCountOutOfRange,
    InvalidSpeculativeIndex,
    Scheduler(ContinuousBatchError),
    Physical(PhysicalKvError),
}

/// Copyable observation of one request; the owned physical authority remains
/// private and non-clone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IsolatedRequestProjection {
    pub request: RequestId,
    pub target: LogicalKvState,
    pub draft: LogicalKvState,
    pub target_retired_pages: u32,
    pub draft_retired_pages: u32,
    pub quiescent_epoch: Option<CompletionEpoch>,
}

/// Target and draft physical KV owned by one exact scheduler generation.
#[derive(Debug, PartialEq, Eq)]
pub struct IsolatedRequestKv {
    request: RequestId,
    target: PhysicalKvState,
    draft: PhysicalKvState,
    target_retired_pages: u32,
    draft_retired_pages: u32,
    quiescent_epoch: CompletionEpoch,
    has_quiescent_epoch: bool,
}

/// Caller-supplied expectations that the round index must match exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IsolatedSpeculativeKvExpectation {
    request: RequestId,
    completion_epoch: CompletionEpoch,
    plan_id: Identity,
    target_selection: Qwen3PlanSelection,
    draft_selection: Qwen3PlanSelection,
}

impl IsolatedSpeculativeKvExpectation {
    pub closed spec fn request_spec(&self) -> RequestId { self.request }

    pub closed spec fn completion_epoch_spec(&self) -> CompletionEpoch { self.completion_epoch }

    pub closed spec fn plan_id_spec(&self) -> Identity { self.plan_id }

    pub closed spec fn target_selection_spec(&self) -> Qwen3PlanSelection {
        self.target_selection
    }

    pub closed spec fn draft_selection_spec(&self) -> Qwen3PlanSelection {
        self.draft_selection
    }

    #[must_use]
    pub const fn new(
        request: RequestId,
        completion_epoch: CompletionEpoch,
        plan_id: Identity,
        target_selection: Qwen3PlanSelection,
        draft_selection: Qwen3PlanSelection,
    ) -> Self {
        Self {
            request,
            completion_epoch,
            plan_id,
            target_selection,
            draft_selection,
        }
    }
}

/// Exact logical effects of one atomic two-role speculative KV settlement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IsolatedSpeculativeKvSettlement {
    pub accepted_draft_tokens: u8,
    pub target_commit_end: u32,
    pub draft_commit_end: u32,
    pub target_retired_pages: u32,
    pub draft_retired_pages: u32,
}

/// Private, non-clone permit for an infallible two-role settlement apply.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct IsolatedSpeculativeKvSettlementPermit {
    target: crate::paged_kv_refinement::PhysicalKvSettlementPermit,
    draft: crate::paged_kv_refinement::PhysicalKvSettlementPermit,
    outcome: IsolatedSpeculativeKvSettlement,
    next_target_retired_pages: u32,
    next_draft_retired_pages: u32,
}

pub closed spec fn target_role(role: Qwen3ModelRole) -> bool {
    match role {
        Qwen3ModelRole::Target8B => true,
        Qwen3ModelRole::Draft06B => false,
    }
}

pub closed spec fn draft_role(role: Qwen3ModelRole) -> bool {
    match role {
        Qwen3ModelRole::Draft06B => true,
        Qwen3ModelRole::Target8B => false,
    }
}

pub closed spec fn same_execution_mode(
    left: Qwen3ExecutionMode,
    right: Qwen3ExecutionMode,
) -> bool {
    match (left, right) {
        (Qwen3ExecutionMode::Prefill, Qwen3ExecutionMode::Prefill)
        | (Qwen3ExecutionMode::Decode, Qwen3ExecutionMode::Decode)
        | (Qwen3ExecutionMode::Speculative, Qwen3ExecutionMode::Speculative) => true,
        _ => false,
    }
}

pub closed spec fn same_plan_bucket(left: Qwen3PlanBucket, right: Qwen3PlanBucket) -> bool {
    match (left, right) {
        (Qwen3PlanBucket::PrefillS1T128, Qwen3PlanBucket::PrefillS1T128)
        | (Qwen3PlanBucket::PrefillS8T128, Qwen3PlanBucket::PrefillS8T128)
        | (Qwen3PlanBucket::PrefillS1T512, Qwen3PlanBucket::PrefillS1T512)
        | (Qwen3PlanBucket::PrefillS1T2048, Qwen3PlanBucket::PrefillS1T2048)
        | (Qwen3PlanBucket::DecodeS1C8192, Qwen3PlanBucket::DecodeS1C8192)
        | (Qwen3PlanBucket::DecodeS8C8192, Qwen3PlanBucket::DecodeS8C8192)
        | (Qwen3PlanBucket::DecodeS32C8192, Qwen3PlanBucket::DecodeS32C8192)
        | (
            Qwen3PlanBucket::SpeculativeS1K4C8192,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        )
        | (
            Qwen3PlanBucket::SpeculativeS8K4C8192,
            Qwen3PlanBucket::SpeculativeS8K4C8192,
        )
        | (
            Qwen3PlanBucket::SpeculativeS1K8C8192,
            Qwen3PlanBucket::SpeculativeS1K8C8192,
        )
        | (
            Qwen3PlanBucket::SpeculativeS1K16C8192,
            Qwen3PlanBucket::SpeculativeS1K16C8192,
        ) => true,
        _ => false,
    }
}

pub closed spec fn plan_pair_matches(
    target: Qwen3PlanSelection,
    draft: Qwen3PlanSelection,
) -> bool {
    target_role(target.role)
        && draft_role(draft.role)
        && same_execution_mode(target.mode, draft.mode)
        && same_plan_bucket(target.bucket, draft.bucket)
}

pub closed spec fn request_identity_matches(left: RequestId, right: RequestId) -> bool {
    left.slot_spec() == right.slot_spec()
        && left.generation_spec() == right.generation_spec()
}

pub closed spec fn active_physical_lifecycle(lifecycle: PhysicalKvLifecycle) -> bool {
    match lifecycle {
        PhysicalKvLifecycle::Active => true,
        PhysicalKvLifecycle::Cancelled { .. }
        | PhysicalKvLifecycle::RetiredAwaitingQuiescence { .. } => false,
    }
}

fn is_target_role(role: Qwen3ModelRole) -> (is_target: bool)
    ensures is_target == target_role(role),
{
    matches!(role, Qwen3ModelRole::Target8B)
}

fn is_draft_role(role: Qwen3ModelRole) -> (is_draft: bool)
    ensures is_draft == draft_role(role),
{
    matches!(role, Qwen3ModelRole::Draft06B)
}

fn modes_match(left: Qwen3ExecutionMode, right: Qwen3ExecutionMode) -> (same: bool)
    ensures same == same_execution_mode(left, right),
{
    matches!(
        (left, right),
        (Qwen3ExecutionMode::Prefill, Qwen3ExecutionMode::Prefill)
            | (Qwen3ExecutionMode::Decode, Qwen3ExecutionMode::Decode)
            | (
                Qwen3ExecutionMode::Speculative,
                Qwen3ExecutionMode::Speculative,
            )
    )
}

fn buckets_match(left: Qwen3PlanBucket, right: Qwen3PlanBucket) -> (same: bool)
    ensures same == same_plan_bucket(left, right),
{
    matches!(
        (left, right),
        (Qwen3PlanBucket::PrefillS1T128, Qwen3PlanBucket::PrefillS1T128)
            | (Qwen3PlanBucket::PrefillS8T128, Qwen3PlanBucket::PrefillS8T128)
            | (Qwen3PlanBucket::PrefillS1T512, Qwen3PlanBucket::PrefillS1T512)
            | (
                Qwen3PlanBucket::PrefillS1T2048,
                Qwen3PlanBucket::PrefillS1T2048,
            )
            | (Qwen3PlanBucket::DecodeS1C8192, Qwen3PlanBucket::DecodeS1C8192)
            | (Qwen3PlanBucket::DecodeS8C8192, Qwen3PlanBucket::DecodeS8C8192)
            | (
                Qwen3PlanBucket::DecodeS32C8192,
                Qwen3PlanBucket::DecodeS32C8192,
            )
            | (
                Qwen3PlanBucket::SpeculativeS1K4C8192,
                Qwen3PlanBucket::SpeculativeS1K4C8192,
            )
            | (
                Qwen3PlanBucket::SpeculativeS8K4C8192,
                Qwen3PlanBucket::SpeculativeS8K4C8192,
            )
            | (
                Qwen3PlanBucket::SpeculativeS1K8C8192,
                Qwen3PlanBucket::SpeculativeS1K8C8192,
            )
            | (
                Qwen3PlanBucket::SpeculativeS1K16C8192,
                Qwen3PlanBucket::SpeculativeS1K16C8192,
            )
    )
}

fn pair_matches(
    target: Qwen3PlanSelection,
    draft: Qwen3PlanSelection,
) -> (matches: bool)
    ensures matches == plan_pair_matches(target, draft),
{
    is_target_role(target.role)
        && is_draft_role(draft.role)
        && modes_match(target.mode, draft.mode)
        && buckets_match(target.bucket, draft.bucket)
}

impl IsolatedRequestKv {
    pub closed spec fn request_spec(&self) -> RequestId { self.request }

    pub closed spec fn target_selection_spec(&self) -> Qwen3PlanSelection {
        self.target.selection_spec()
    }

    pub closed spec fn draft_selection_spec(&self) -> Qwen3PlanSelection {
        self.draft.selection_spec()
    }

    pub closed spec fn projection_spec(&self) -> IsolatedRequestProjection {
        IsolatedRequestProjection {
            request: self.request,
            target: self.target.abstraction_spec(),
            draft: self.draft.abstraction_spec(),
            target_retired_pages: self.target_retired_pages,
            draft_retired_pages: self.draft_retired_pages,
            quiescent_epoch: if self.has_quiescent_epoch {
                Some(self.quiescent_epoch)
            } else {
                None
            },
        }
    }

    pub closed spec fn exact_physical_frame(&self, before: &Self) -> bool {
        self.request == before.request
            && self.target == before.target
            && self.draft == before.draft
            && self.target_retired_pages == before.target_retired_pages
            && self.draft_retired_pages == before.draft_retired_pages
            && self.quiescent_epoch == before.quiescent_epoch
            && self.has_quiescent_epoch == before.has_quiescent_epoch
    }

    /// Creates target and draft KV for one exact scheduler generation.
    ///
    /// # Errors
    ///
    /// Rejects out-of-range or zero-generation requests, swapped roles,
    /// mismatched mode/bucket pairs, and invalid physical selections.
    pub fn new(
        request: RequestId,
        target_selection: Qwen3PlanSelection,
        draft_selection: Qwen3PlanSelection,
    ) -> (result: Result<Self, RequestIsolationError>)
        ensures result.is_ok() ==> {
            let pair = result.unwrap();
            &&& request_identity_matches(pair.request_spec(), request)
            &&& pair.target_selection_spec() == target_selection
            &&& pair.draft_selection_spec() == draft_selection
            &&& plan_pair_matches(
                pair.target_selection_spec(),
                pair.draft_selection_spec(),
            )
            &&& pair.projection_spec().target.role == Qwen3ModelRole::Target8B
            &&& pair.projection_spec().draft.role == Qwen3ModelRole::Draft06B
            &&& pair.projection_spec().target_retired_pages == 0
            &&& pair.projection_spec().draft_retired_pages == 0
            &&& pair.projection_spec().quiescent_epoch.is_none()
        },
    {
        if request.slot() as usize >= M1_CONTINUOUS_BATCH_CAPACITY {
            return Err(RequestIsolationError::SlotOutOfRange);
        }
        if request.generation() == 0 {
            return Err(RequestIsolationError::StaleRequest);
        }
        if !is_target_role(target_selection.role) {
            return Err(RequestIsolationError::TargetRoleRequired);
        }
        if !is_draft_role(draft_selection.role) {
            return Err(RequestIsolationError::DraftRoleRequired);
        }
        if !pair_matches(target_selection, draft_selection) {
            return Err(RequestIsolationError::PlanPairMismatch);
        }
        let target = match PhysicalKvState::new(request, target_selection) {
            Ok(state) => state,
            Err(error) => return Err(RequestIsolationError::Physical(error)),
        };
        let draft = match PhysicalKvState::new(request, draft_selection) {
            Ok(state) => state,
            Err(error) => return Err(RequestIsolationError::Physical(error)),
        };
        Ok(Self {
            request,
            target,
            draft,
            target_retired_pages: 0,
            draft_retired_pages: 0,
            quiescent_epoch: CompletionEpoch::new(0),
            has_quiescent_epoch: false,
        })
    }

    #[must_use]
    pub const fn request(&self) -> (request: RequestId)
        ensures request == self.request_spec(),
    { self.request }

    #[must_use]
    pub const fn target_selection(&self) -> (selection: Qwen3PlanSelection)
        ensures selection == self.target_selection_spec(),
    { self.target.selection() }

    #[must_use]
    pub const fn draft_selection(&self) -> (selection: Qwen3PlanSelection)
        ensures selection == self.draft_selection_spec(),
    { self.draft.selection() }

    #[must_use]
    pub const fn projection(&self) -> (projection: IsolatedRequestProjection)
        ensures projection == self.projection_spec(),
    {
        IsolatedRequestProjection {
            request: self.request,
            target: self.target.logical_state(),
            draft: self.draft.logical_state(),
            target_retired_pages: self.target_retired_pages,
            draft_retired_pages: self.draft_retired_pages,
            quiescent_epoch: if self.has_quiescent_epoch {
                Some(self.quiescent_epoch)
            } else {
                None
            },
        }
    }

    fn set_quiescent_epoch(&mut self, epoch: CompletionEpoch)
        ensures
            final(self).request == old(self).request,
            final(self).target == old(self).target,
            final(self).draft == old(self).draft,
            final(self).target_retired_pages == old(self).target_retired_pages,
            final(self).draft_retired_pages == old(self).draft_retired_pages,
            final(self).quiescent_epoch == epoch,
            final(self).has_quiescent_epoch,
    {
        self.quiescent_epoch = epoch;
        self.has_quiescent_epoch = true;
    }
}

pub closed spec fn isolated_other_frame(
    before_batch: &ContinuousBatch,
    after_batch: &ContinuousBatch,
    before_other: &IsolatedRequestKv,
    after_other: &IsolatedRequestKv,
    selected_request: RequestId,
) -> bool {
    &&& after_other.exact_physical_frame(before_other)
    &&& if before_other.request_spec().slot_spec() < M1_CONTINUOUS_BATCH_CAPACITY
        && before_other.request_spec().slot_spec() != selected_request.slot_spec()
    {
        after_batch.slots_spec()[before_other.request_spec().slot_spec() as int]
            == before_batch.slots_spec()[before_other.request_spec().slot_spec() as int]
    } else {
        true
    }
}

fn requests_match(left: RequestId, right: RequestId) -> (matches: bool)
    ensures matches == request_identity_matches(left, right),
{
    proof { reveal(request_identity_matches); }
    left.slot() == right.slot() && left.generation() == right.generation()
}

fn validate_routing(
    batch: &ContinuousBatch,
    selected: &IsolatedRequestKv,
    other: &IsolatedRequestKv,
    request: RequestId,
) -> (result: Result<ContinuousRequest, RequestIsolationError>)
    requires batch.valid(),
    ensures match result {
        Ok(current) => {
            &&& current.valid()
            &&& request_identity_matches(selected.request_spec(), request)
            &&& selected.request_spec().slot_spec() < M1_CONTINUOUS_BATCH_CAPACITY
            &&& other.request_spec().slot_spec() < M1_CONTINUOUS_BATCH_CAPACITY
            &&& selected.request_spec().slot_spec() != other.request_spec().slot_spec()
        },
        Err(_) => true,
    },
{
    if selected.request.slot() as usize >= M1_CONTINUOUS_BATCH_CAPACITY
        || other.request.slot() as usize >= M1_CONTINUOUS_BATCH_CAPACITY
    {
        return Err(RequestIsolationError::SlotOutOfRange);
    }
    if selected.request.slot() == other.request.slot() {
        return Err(RequestIsolationError::SameRequestSlot);
    }
    if !requests_match(selected.request, request) {
        return Err(RequestIsolationError::StaleRequest);
    }
    match batch.request(request) {
        Some(current) => {
            proof {
                reveal(request_identity_matches);
                assert(request.slot_spec() < M1_CONTINUOUS_BATCH_CAPACITY);
                crate::continuous_batching::valid_continuous_batch_slot(
                    batch,
                    request.slot_spec() as int,
                );
                assert(current.valid());
            }
            Ok(current)
        },
        None => Err(RequestIsolationError::StaleRequest),
    }
}

fn scheduler_action(action: IsolatedSchedulerAction) -> ContinuousBatchAction {
    match action {
        IsolatedSchedulerAction::Admit => ContinuousBatchAction::Admit,
        IsolatedSchedulerAction::Dispatch { epoch } => ContinuousBatchAction::Dispatch { epoch },
        IsolatedSchedulerAction::CompleteExact { epoch } => {
            ContinuousBatchAction::CompleteExact { epoch }
        }
        IsolatedSchedulerAction::Publish { epoch, emitted_tokens } => {
            ContinuousBatchAction::Publish { epoch, emitted_tokens }
        }
        IsolatedSchedulerAction::FinalizeKv => ContinuousBatchAction::FinalizeKv,
    }
}

fn physical_active(state: LogicalKvState) -> (active: bool)
    ensures active == active_physical_lifecycle(state.lifecycle),
{
    matches!(state.lifecycle, PhysicalKvLifecycle::Active)
}

fn lifecycle_allows_kv(
    lifecycle: crate::scheduling::SequentialRequest,
    action: IsolatedKvAction,
) -> bool {
    match action {
        IsolatedKvAction::AppendPage { .. } | IsolatedKvAction::WriteToken { .. } => {
            matches!(
                (lifecycle.state, lifecycle.phase),
                (RequestState::InFlight, LifecyclePhase::Executing)
            )
        }
        IsolatedKvAction::Commit { .. } | IsolatedKvAction::RollbackToken { .. } => {
            matches!(
                (lifecycle.state, lifecycle.phase),
                (RequestState::InFlight, LifecyclePhase::AwaitingKv)
            )
        }
        IsolatedKvAction::RetireTail { .. } => {
            matches!(
                (lifecycle.state, lifecycle.phase),
                (RequestState::Retiring, LifecyclePhase::RetiringQuiescent)
            )
        }
    }
}

fn unit_page_result(
    result: Result<(), PhysicalKvError>,
) -> (converted: Result<Option<PhysicalPageId>, PhysicalKvError>)
    ensures
        converted.is_ok() == result.is_ok(),
        converted.is_err() == result.is_err(),
{
    match result {
        Ok(()) => Ok(None),
        Err(error) => Err(error),
    }
}

fn retired_page_result(
    result: Result<PhysicalPageId, PhysicalKvError>,
) -> (converted: Result<Option<PhysicalPageId>, PhysicalKvError>)
    ensures
        converted.is_ok() == result.is_ok(),
        converted.is_err() == result.is_err(),
{
    match result {
        Ok(page) => Ok(Some(page)),
        Err(error) => Err(error),
    }
}

/// Applies a scheduler-only action while preserving the other request's exact
/// scheduler slot and complete target/draft physical authority.
///
/// # Errors
///
/// Rejects stale routing, aliasing the selected and other slot, disabled
/// scheduler transitions, and finalization with tentative physical tokens.
pub fn apply_isolated_scheduler_step(
    batch: &mut ContinuousBatch,
    selected: &mut IsolatedRequestKv,
    other: &mut IsolatedRequestKv,
    request: RequestId,
    action: IsolatedSchedulerAction,
) -> (result: Result<(), RequestIsolationError>)
    requires old(batch).valid(),
    ensures
        final(batch).valid(),
        isolated_other_frame(
            old(batch),
            final(batch),
            old(other),
            final(other),
            request,
        ),
        result.is_err() ==> *final(selected) == *old(selected),
{
    let ghost entry_batch = *batch;
    let ghost entry_other = *other;
    proof {
        reveal(isolated_other_frame);
        reveal(IsolatedRequestKv::exact_physical_frame);
    }
    let current = validate_routing(batch, selected, other, request)?;
    if matches!(action, IsolatedSchedulerAction::FinalizeKv) {
        let projection = selected.projection();
        if !physical_active(projection.target)
            || !physical_active(projection.draft)
            || projection.target.committed_tokens != projection.target.resident_tokens
            || projection.draft.committed_tokens != projection.draft.resident_tokens
        {
            return Err(RequestIsolationError::TentativeTokensRemain);
        }
    }
    let routed = scheduler_action(action);
    match apply_continuous_batch_step(batch, request, routed) {
        Ok(()) => {}
        Err(error) => return Err(RequestIsolationError::Scheduler(error)),
    }
    if let IsolatedSchedulerAction::CompleteExact { epoch } = action {
        if matches!(
            (current.lifecycle().state, current.lifecycle().phase),
            (RequestState::Retiring, LifecyclePhase::RetiringExecuting)
        ) {
            selected.set_quiescent_epoch(epoch);
        }
    }
    proof {
        crate::continuous_batching::successful_batch_step_preserves_other_request(
            &entry_batch,
            batch,
            request,
            routed,
            entry_other.request_spec().slot_spec() as int,
        );
    }
    Ok(())
}

/// Cancels both target and draft ownership at the scheduler's exact active
/// epoch. Ready requests with no retained exact epoch fail closed in this
/// logical slice.
///
/// # Errors
///
/// Rejects stale routing, missing epochs, non-executing/non-awaiting lifecycle,
/// or any target/draft ownership mismatch before cancellation.
pub fn cancel_isolated_request(
    batch: &mut ContinuousBatch,
    selected: &mut IsolatedRequestKv,
    other: &mut IsolatedRequestKv,
    request: RequestId,
) -> (result: Result<(), RequestIsolationError>)
    requires old(batch).valid(),
    ensures
        final(batch).valid(),
        isolated_other_frame(
            old(batch),
            final(batch),
            old(other),
            final(other),
            request,
        ),
{
    let ghost entry_batch = *batch;
    let ghost entry_other = *other;
    proof {
        reveal(isolated_other_frame);
        reveal(IsolatedRequestKv::exact_physical_frame);
        reveal(active_physical_lifecycle);
        reveal(request_identity_matches);
    }
    let current = validate_routing(batch, selected, other, request)?;
    let Some(epoch) = current.active_epoch() else {
        return Err(RequestIsolationError::MissingEpoch);
    };
    if !matches!(
        (current.lifecycle().state, current.lifecycle().phase),
        (
            RequestState::InFlight,
            LifecyclePhase::Executing | LifecyclePhase::AwaitingKv,
        )
    ) {
        return Err(RequestIsolationError::WrongLifecycle);
    }
    let projection = selected.projection();
    if !physical_active(projection.target) || !physical_active(projection.draft) {
        return Err(RequestIsolationError::WrongLifecycle);
    }
    if !requests_match(projection.target.request, request)
        || !requests_match(projection.draft.request, request)
    {
        return Err(RequestIsolationError::StaleRequest);
    }
    proof {
        assert(request.slot_spec() < M1_CONTINUOUS_BATCH_CAPACITY);
        crate::continuous_batching::valid_continuous_batch_slot(
            batch,
            request.slot_spec() as int,
        );
        assert(current.valid());
        crate::continuous_batching::valid_continuous_active_epoch(current, epoch);
        crate::paged_kv_refinement::active_projection_enables_cancel(
            &selected.target,
            request,
            selected.target.selection_spec(),
            epoch,
        );
        crate::paged_kv_refinement::active_projection_enables_cancel(
            &selected.draft,
            request,
            selected.draft.selection_spec(),
            epoch,
        );
    }
    match apply_continuous_batch_step(batch, request, ContinuousBatchAction::Retire) {
        Ok(()) => {}
        Err(error) => return Err(RequestIsolationError::Scheduler(error)),
    }
    let target_selection = selected.target.selection();
    match cancel_physical_kv(&mut selected.target, request, target_selection, epoch) {
        Ok(()) => {}
        Err(error) => return Err(RequestIsolationError::Physical(error)),
    }
    let draft_selection = selected.draft.selection();
    match cancel_physical_kv(&mut selected.draft, request, draft_selection, epoch) {
        Ok(()) => {}
        Err(error) => return Err(RequestIsolationError::Physical(error)),
    }
    if matches!(
        (current.lifecycle().state, current.lifecycle().phase),
        (RequestState::InFlight, LifecyclePhase::AwaitingKv)
    ) {
        selected.set_quiescent_epoch(epoch);
    }
    proof {
        crate::continuous_batching::successful_batch_step_preserves_other_request(
            &entry_batch,
            batch,
            request,
            ContinuousBatchAction::Retire,
            entry_other.request_spec().slot_spec() as int,
        );
    }
    Ok(())
}

/// Applies one scheduler-admitted target or draft KV transition.
///
/// # Errors
///
/// Rejects stale request generations, wrong lifecycle/role/epoch, and the exact
/// fail-closed physical error. The batch and other request are immutable.
pub fn apply_isolated_kv_action(
    batch: &ContinuousBatch,
    selected: &mut IsolatedRequestKv,
    other: &mut IsolatedRequestKv,
    request: RequestId,
    role: Qwen3ModelRole,
    action: IsolatedKvAction,
) -> (result: Result<Option<PhysicalPageId>, RequestIsolationError>)
    requires batch.valid(),
    ensures
        *final(other) == *old(other),
        result.is_err() ==> *final(selected) == *old(selected),
{
    let current = validate_routing(batch, selected, other, request)?;
    if !lifecycle_allows_kv(current.lifecycle(), action) {
        return Err(RequestIsolationError::WrongLifecycle);
    }
    if let IsolatedKvAction::RollbackToken { after_epoch } = action {
        if current.active_epoch() != Some(after_epoch) {
            return Err(RequestIsolationError::EpochMismatch);
        }
    }
    if let IsolatedKvAction::RetireTail { after_epoch } = action {
        if selected.projection().quiescent_epoch != Some(after_epoch) {
            return Err(RequestIsolationError::NoExactQuiescence);
        }
    }
    let target = is_target_role(role);
    let before_page_count = if target {
        selected.target.page_count()
    } else {
        selected.draft.page_count()
    };
    let retired_count = if target {
        selected.target_retired_pages
    } else {
        selected.draft_retired_pages
    };
    let may_retire_page = matches!(
        action,
        IsolatedKvAction::RollbackToken { .. } | IsolatedKvAction::RetireTail { .. }
    );
    let next_retired_count = if may_retire_page {
        let Some(next) = retired_count.checked_add(1) else {
            return Err(RequestIsolationError::RetiredPageCountExhausted);
        };
        next
    } else {
        retired_count
    };
    let result = if target {
        let selection = selected.target.selection();
        match action {
            IsolatedKvAction::AppendPage { page } => {
                unit_page_result(append_physical_page(
                    &mut selected.target,
                    request,
                    selection,
                    page,
                ))
            }
            IsolatedKvAction::WriteToken { logical_position } => {
                unit_page_result(write_physical_token(
                    &mut selected.target,
                    request,
                    selection,
                    logical_position,
                ))
            }
            IsolatedKvAction::Commit { accepted_tokens } => {
                unit_page_result(commit_physical_kv(
                    &mut selected.target,
                    request,
                    selection,
                    accepted_tokens,
                ))
            }
            IsolatedKvAction::RollbackToken { after_epoch } => {
                unit_page_result(rollback_physical_token(
                    &mut selected.target,
                    request,
                    selection,
                    after_epoch,
                ))
            }
            IsolatedKvAction::RetireTail { after_epoch } => {
                retired_page_result(retire_cancelled_tail(
                    &mut selected.target,
                    request,
                    selection,
                    after_epoch,
                ))
            }
        }
    } else {
        let selection = selected.draft.selection();
        match action {
            IsolatedKvAction::AppendPage { page } => {
                unit_page_result(append_physical_page(
                    &mut selected.draft,
                    request,
                    selection,
                    page,
                ))
            }
            IsolatedKvAction::WriteToken { logical_position } => {
                unit_page_result(write_physical_token(
                    &mut selected.draft,
                    request,
                    selection,
                    logical_position,
                ))
            }
            IsolatedKvAction::Commit { accepted_tokens } => {
                unit_page_result(commit_physical_kv(
                    &mut selected.draft,
                    request,
                    selection,
                    accepted_tokens,
                ))
            }
            IsolatedKvAction::RollbackToken { after_epoch } => {
                unit_page_result(rollback_physical_token(
                    &mut selected.draft,
                    request,
                    selection,
                    after_epoch,
                ))
            }
            IsolatedKvAction::RetireTail { after_epoch } => {
                retired_page_result(retire_cancelled_tail(
                    &mut selected.draft,
                    request,
                    selection,
                    after_epoch,
                ))
            }
        }
    };
    let page = match result {
        Ok(page) => page,
        Err(error) => return Err(RequestIsolationError::Physical(error)),
    };
    let after_page_count = if target {
        selected.target.page_count()
    } else {
        selected.draft.page_count()
    };
    if before_page_count > after_page_count {
        if target {
            selected.target_retired_pages = next_retired_count;
        } else {
            selected.draft_retired_pages = next_retired_count;
        }
    }
    Ok(page)
}

pub closed spec fn isolated_speculative_settlement_transition(
    before: &IsolatedRequestKv,
    after: &IsolatedRequestKv,
    index: &SpeculativeKvRoundIndex,
    outcome: IsolatedSpeculativeKvSettlement,
) -> bool {
    &&& outcome.accepted_draft_tokens <= index.draft_token_count
    &&& index.target_commit_end_spec(outcome.accepted_draft_tokens)
        == Some(outcome.target_commit_end)
    &&& index.draft_commit_end_spec(outcome.accepted_draft_tokens)
        == Some(outcome.draft_commit_end)
    &&& after.request == before.request
    &&& after.quiescent_epoch == before.quiescent_epoch
    &&& after.has_quiescent_epoch == before.has_quiescent_epoch
    &&& after.target_retired_pages as int
        == before.target_retired_pages as int + outcome.target_retired_pages as int
    &&& after.draft_retired_pages as int
        == before.draft_retired_pages as int + outcome.draft_retired_pages as int
    &&& crate::paged_kv_refinement::physical_speculative_settlement_matches(
        &before.target,
        &after.target,
        index.completion_epoch,
        index.target_tentative.end,
        outcome.target_commit_end,
        outcome.target_retired_pages,
    )
    &&& crate::paged_kv_refinement::physical_speculative_settlement_matches(
        &before.draft,
        &after.draft,
        index.completion_epoch,
        index.draft_tentative.end,
        outcome.draft_commit_end,
        outcome.draft_retired_pages,
    )
}

impl IsolatedSpeculativeKvSettlementPermit {
    pub(crate) closed spec fn accepted_draft_tokens_spec(&self) -> u8 {
        self.outcome.accepted_draft_tokens
    }

    pub(crate) closed spec fn valid_for(
        &self,
        selected: &IsolatedRequestKv,
        index: &SpeculativeKvRoundIndex,
    ) -> bool {
        &&& self.outcome.accepted_draft_tokens <= index.draft_token_count
        &&& index.target_commit_end_spec(self.outcome.accepted_draft_tokens)
            == Some(self.outcome.target_commit_end)
        &&& index.draft_commit_end_spec(self.outcome.accepted_draft_tokens)
            == Some(self.outcome.draft_commit_end)
        &&& self.target.valid_for(&selected.target)
        &&& self.target.after_epoch_spec() == index.completion_epoch
        &&& self.target.pre_committed_spec() == index.target_pre_committed
        &&& self.target.tentative_end_spec() == index.target_tentative.end
        &&& self.target.commit_end_spec() == self.outcome.target_commit_end
        &&& self.target.retired_pages_spec() == self.outcome.target_retired_pages
        &&& self.draft.valid_for(&selected.draft)
        &&& self.draft.after_epoch_spec() == index.completion_epoch
        &&& self.draft.pre_committed_spec() == index.draft_pre_committed
        &&& self.draft.tentative_end_spec() == index.draft_tentative.end
        &&& self.draft.commit_end_spec() == self.outcome.draft_commit_end
        &&& self.draft.retired_pages_spec() == self.outcome.draft_retired_pages
        &&& self.next_target_retired_pages as int
            == selected.target_retired_pages as int
                + self.outcome.target_retired_pages as int
        &&& self.next_draft_retired_pages as int
            == selected.draft_retired_pages as int
                + self.outcome.draft_retired_pages as int
    }

}

/// Preflights routing, indexing, both physical roles, and retired accounting.
pub(crate) fn preflight_isolated_speculative_kv(
    batch: &ContinuousBatch,
    selected: &IsolatedRequestKv,
    other: &IsolatedRequestKv,
    index: &SpeculativeKvRoundIndex,
    accepted_draft_tokens: u8,
    expected: &IsolatedSpeculativeKvExpectation,
) -> (result: Result<IsolatedSpeculativeKvSettlementPermit, RequestIsolationError>)
    requires batch.valid(),
    ensures match result {
        Ok(permit) => {
            &&& index.valid_for(
                expected.request_spec(),
                expected.completion_epoch_spec(),
                expected.plan_id_spec(),
                expected.target_selection_spec(),
                expected.draft_selection_spec(),
            )
            &&& permit.valid_for(selected, index)
            &&& permit.accepted_draft_tokens_spec() == accepted_draft_tokens
        },
        Err(_) => true,
    },
{
    proof {
        reveal(IsolatedSpeculativeKvSettlementPermit::valid_for);
    }
    let current = validate_routing(batch, selected, other, expected.request)?;
    if !matches!(
        (current.lifecycle().state, current.lifecycle().phase),
        (RequestState::InFlight, LifecyclePhase::AwaitingKv)
    ) {
        return Err(RequestIsolationError::WrongLifecycle);
    }
    if current.active_epoch() != Some(expected.completion_epoch)
        || index.completion_epoch.value != expected.completion_epoch.value
    {
        return Err(RequestIsolationError::EpochMismatch);
    }
    if !selected.target.selection().matches(expected.target_selection)
        || !selected.draft.selection().matches(expected.draft_selection)
    {
        return Err(RequestIsolationError::PlanPairMismatch);
    }
    match index.validate_for(
        expected.request,
        expected.completion_epoch,
        &expected.plan_id,
        expected.target_selection,
        expected.draft_selection,
    ) {
        Ok(()) => {},
        Err(_) => return Err(RequestIsolationError::InvalidSpeculativeIndex),
    }
    assert(index.valid_for(
        expected.request,
        expected.completion_epoch,
        expected.plan_id,
        expected.target_selection,
        expected.draft_selection,
    ));
    if accepted_draft_tokens > index.draft_token_count {
        return Err(RequestIsolationError::AcceptedCountOutOfRange);
    }
    let Some(target_commit_end) = index.target_commit_end(accepted_draft_tokens) else {
        return Err(RequestIsolationError::AcceptedCountOutOfRange);
    };
    let Some(draft_commit_end) = index.draft_commit_end(accepted_draft_tokens) else {
        return Err(RequestIsolationError::AcceptedCountOutOfRange);
    };
    proof {
        index.valid_for_implies_valid(
            expected.request,
            expected.completion_epoch,
            expected.plan_id,
            expected.target_selection,
            expected.draft_selection,
        );
        index.rejected_tail_bounds(accepted_draft_tokens);
        assert(index.target_tentative.end as int - target_commit_end as int
            <= crate::M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS);
        assert(index.draft_tentative.end as int - draft_commit_end as int
            <= crate::M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS);
    }
    let target = match preflight_physical_speculative_settlement(
        &selected.target,
        expected.request,
        expected.target_selection,
        expected.completion_epoch,
        index.target_pre_committed,
        index.target_tentative.end,
        target_commit_end,
    ) {
        Ok(permit) => permit,
        Err(error) => return Err(RequestIsolationError::Physical(error)),
    };
    let draft = match preflight_physical_speculative_settlement(
        &selected.draft,
        expected.request,
        expected.draft_selection,
        expected.completion_epoch,
        index.draft_pre_committed,
        index.draft_tentative.end,
        draft_commit_end,
    ) {
        Ok(permit) => permit,
        Err(error) => return Err(RequestIsolationError::Physical(error)),
    };
    let target_retired_pages = target.retired_pages();
    let draft_retired_pages = draft.retired_pages();
    let Some(next_target_retired_pages) = selected
        .target_retired_pages
        .checked_add(target_retired_pages)
    else {
        return Err(RequestIsolationError::RetiredPageCountExhausted);
    };
    let Some(next_draft_retired_pages) = selected
        .draft_retired_pages
        .checked_add(draft_retired_pages)
    else {
        return Err(RequestIsolationError::RetiredPageCountExhausted);
    };
    let outcome = IsolatedSpeculativeKvSettlement {
        accepted_draft_tokens,
        target_commit_end,
        draft_commit_end,
        target_retired_pages,
        draft_retired_pages,
    };
    let permit = IsolatedSpeculativeKvSettlementPermit {
        target,
        draft,
        outcome,
        next_target_retired_pages,
        next_draft_retired_pages,
    };
    assert(permit.valid_for(selected, index));
    Ok(permit)
}

/// Applies a fully checked two-role settlement with no fallible second stage.
pub(crate) fn apply_preflighted_isolated_speculative_kv(
    selected: &mut IsolatedRequestKv,
    _index: &SpeculativeKvRoundIndex,
    permit: IsolatedSpeculativeKvSettlementPermit,
) -> (outcome: IsolatedSpeculativeKvSettlement)
    requires permit.valid_for(old(selected), _index),
    ensures isolated_speculative_settlement_transition(
        old(selected),
        final(selected),
        _index,
        outcome,
    ),
    outcome.accepted_draft_tokens == permit.accepted_draft_tokens_spec(),
{
    let ghost entry = *selected;
    proof {
        reveal(IsolatedSpeculativeKvSettlementPermit::valid_for);
        reveal(isolated_speculative_settlement_transition);
    }
    let outcome = permit.outcome;
    apply_preflighted_physical_speculative_settlement(&mut selected.target, permit.target);
    apply_preflighted_physical_speculative_settlement(&mut selected.draft, permit.draft);
    selected.target_retired_pages = permit.next_target_retired_pages;
    selected.draft_retired_pages = permit.next_draft_retired_pages;
    assert(isolated_speculative_settlement_transition(
        &entry,
        selected,
        _index,
        outcome,
    ));
    outcome
}

/// Atomically settles both speculative KV roles after complete immutable preflight.
///
/// This narrow primitive performs no publication. The caller must bind a
/// separately verified compact completion before using the returned endpoints.
///
/// # Errors
///
/// Rejects any stale expectation, scheduler/index mismatch, malformed physical
/// tail, or retired-page counter overflow before mutating either KV role.
pub fn settle_isolated_speculative_kv(
    batch: &ContinuousBatch,
    selected: &mut IsolatedRequestKv,
    other: &mut IsolatedRequestKv,
    index: &SpeculativeKvRoundIndex,
    accepted_draft_tokens: u8,
    expected: &IsolatedSpeculativeKvExpectation,
) -> (result: Result<IsolatedSpeculativeKvSettlement, RequestIsolationError>)
    requires batch.valid(),
    ensures
        *final(other) == *old(other),
        result.is_err() ==> *final(selected) == *old(selected),
        match result {
            Ok(outcome) => {
                &&& index.valid_for(
                    expected.request_spec(),
                    expected.completion_epoch_spec(),
                    expected.plan_id_spec(),
                    expected.target_selection_spec(),
                    expected.draft_selection_spec(),
                )
                &&& isolated_speculative_settlement_transition(
                    old(selected),
                    final(selected),
                    index,
                    outcome,
                )
            },
            Err(_) => true,
        },
{
    proof {
        reveal(isolated_speculative_settlement_transition);
    }
    let permit = preflight_isolated_speculative_kv(
        batch,
        selected,
        other,
        index,
        accepted_draft_tokens,
        expected,
    )?;
    let outcome = apply_preflighted_isolated_speculative_kv(selected, index, permit);
    Ok(outcome)
}

/// Resolves one request-owned initialized token only while its scheduler
/// generation and lifecycle make the physical projection reachable.
///
/// # Errors
///
/// Rejects stale routing, retiring/vacant state, wrong role, or the exact
/// physical mapping error.
pub fn map_isolated_token(
    batch: &ContinuousBatch,
    selected: &IsolatedRequestKv,
    other: &IsolatedRequestKv,
    request: RequestId,
    role: Qwen3ModelRole,
    logical_position: u32,
) -> Result<PhysicalKvLocation, RequestIsolationError>
    requires batch.valid(),
{
    let current = validate_routing(batch, selected, other, request)?;
    if !matches!(
        (current.lifecycle().state, current.lifecycle().phase),
        (RequestState::Ready, LifecyclePhase::Idle)
            | (
                RequestState::InFlight,
                LifecyclePhase::Executing | LifecyclePhase::AwaitingKv,
            )
    ) {
        return Err(RequestIsolationError::WrongLifecycle);
    }
    if is_target_role(role) {
        match map_initialized_token(
            &selected.target,
            request,
            selected.target.selection(),
            logical_position,
        ) {
            Ok(location) => Ok(location),
            Err(error) => Err(RequestIsolationError::Physical(error)),
        }
    } else {
        match map_initialized_token(
            &selected.draft,
            request,
            selected.draft.selection(),
            logical_position,
        ) {
            Ok(location) => Ok(location),
            Err(error) => Err(RequestIsolationError::Physical(error)),
        }
    }
}

/// Releases one retired page only from a scheduler-quiescent request and an
/// epoch recorded by its exact completion transition.
///
/// # Errors
///
/// Rejects stale request/role/epoch, early release, counter underflow, and the
/// exact physical release error. The other request remains exactly framed.
pub fn release_isolated_page(
    batch: &ContinuousBatch,
    selected: &mut IsolatedRequestKv,
    other: &mut IsolatedRequestKv,
    request: RequestId,
    role: Qwen3ModelRole,
    page: PhysicalPageId,
    exact_epoch: CompletionEpoch,
) -> (result: Result<PhysicalPageId, RequestIsolationError>)
    requires batch.valid(),
    ensures
        *final(other) == *old(other),
        result.is_err() ==> *final(selected) == *old(selected),
{
    let current = validate_routing(batch, selected, other, request)?;
    if !matches!(
        (current.lifecycle().state, current.lifecycle().phase),
        (RequestState::Retiring, LifecyclePhase::RetiringQuiescent)
    ) {
        return Err(RequestIsolationError::WrongLifecycle);
    }
    if !selected.has_quiescent_epoch {
        return Err(RequestIsolationError::NoExactQuiescence);
    }
    if selected.quiescent_epoch.value != exact_epoch.value {
        return Err(RequestIsolationError::EpochMismatch);
    }
    if exact_epoch.value == 0 {
        return Err(RequestIsolationError::NoExactQuiescence);
    }
    let target = is_target_role(role);
    let retired_count = if target {
        selected.target_retired_pages
    } else {
        selected.draft_retired_pages
    };
    if retired_count == 0 {
        return Err(RequestIsolationError::RetiredPageCountUnderflow);
    }
    let authority = KvQuiescenceAuthority::from_exact_completion(request, role, exact_epoch);
    let released = if target {
        match release_retired_page(&mut selected.target, page, &authority) {
            Ok(next) => next,
            Err(error) => return Err(RequestIsolationError::Physical(error)),
        }
    } else {
        match release_retired_page(&mut selected.draft, page, &authority) {
            Ok(next) => next,
            Err(error) => return Err(RequestIsolationError::Physical(error)),
        }
    };
    if target {
        selected.target_retired_pages -= 1;
    } else {
        selected.draft_retired_pages -= 1;
    }
    Ok(released)
}

/// Detaches a quiescent request only after both page tables are unreachable and
/// every composition-tracked retired physical generation has been released.
/// The selected slot advances generation and receives fresh empty target/draft
/// physical authority.
///
/// # Errors
///
/// Rejects stale routing, early detachment, remaining pages, missing exact
/// quiescence, or generation exhaustion. The other request remains framed.
pub fn detach_isolated_request(
    batch: &mut ContinuousBatch,
    selected: &mut IsolatedRequestKv,
    other: &mut IsolatedRequestKv,
    request: RequestId,
) -> (result: Result<RequestId, RequestIsolationError>)
    requires old(batch).valid(),
    ensures
        final(batch).valid(),
        isolated_other_frame(
            old(batch),
            final(batch),
            old(other),
            final(other),
            request,
        ),
{
    let ghost entry_batch = *batch;
    let ghost entry_other = *other;
    proof {
        reveal(isolated_other_frame);
        reveal(IsolatedRequestKv::exact_physical_frame);
    }
    let current = validate_routing(batch, selected, other, request)?;
    if !matches!(
        (current.lifecycle().state, current.lifecycle().phase),
        (RequestState::Retiring, LifecyclePhase::RetiringQuiescent)
    ) {
        return Err(RequestIsolationError::WrongLifecycle);
    }
    let projection = selected.projection();
    if projection.quiescent_epoch.is_none() {
        return Err(RequestIsolationError::NoExactQuiescence);
    }
    if projection.target.resident_tokens != 0
        || projection.draft.resident_tokens != 0
        || selected.target.page_count() != 0
        || selected.draft.page_count() != 0
        || projection.target_retired_pages != 0
        || projection.draft_retired_pages != 0
        || !matches!(
            projection.target.lifecycle,
            PhysicalKvLifecycle::RetiredAwaitingQuiescence { .. }
        )
        || !matches!(
            projection.draft.lifecycle,
            PhysicalKvLifecycle::RetiredAwaitingQuiescence { .. }
        )
    {
        return Err(RequestIsolationError::WrongLifecycle);
    }
    let Some(next_generation) = request.generation().checked_add(1) else {
        return Err(RequestIsolationError::GenerationExhausted);
    };
    let next_request = RequestId::new(request.slot(), next_generation);
    let replacement = IsolatedRequestKv::new(
        next_request,
        selected.target.selection(),
        selected.draft.selection(),
    )?;
    match apply_continuous_batch_step(batch, request, ContinuousBatchAction::DetachKv) {
        Ok(()) => {}
        Err(error) => return Err(RequestIsolationError::Scheduler(error)),
    }
    *selected = replacement;
    proof {
        crate::continuous_batching::successful_batch_step_preserves_other_request(
            &entry_batch,
            batch,
            request,
            ContinuousBatchAction::DetachKv,
            entry_other.request_spec().slot_spec() as int,
        );
    }
    Ok(next_request)
}

/// A composed frame entails exact scheduler and physical preservation for the
/// other request.
pub proof fn isolated_action_preserves_other_request(
    before_batch: &ContinuousBatch,
    after_batch: &ContinuousBatch,
    before_other: &IsolatedRequestKv,
    after_other: &IsolatedRequestKv,
    selected_request: RequestId,
)
    requires isolated_other_frame(
        before_batch,
        after_batch,
        before_other,
        after_other,
        selected_request,
    ),
    ensures
        after_other.exact_physical_frame(before_other),
        before_other.request_spec().slot_spec() < M1_CONTINUOUS_BATCH_CAPACITY
            && before_other.request_spec().slot_spec() != selected_request.slot_spec()
            ==> after_batch.slots_spec()[before_other.request_spec().slot_spec() as int]
                == before_batch.slots_spec()[before_other.request_spec().slot_spec() as int],
{
    reveal(isolated_other_frame);
}

} // verus!

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        settle_and_publish_speculative_step, AtomicSpeculativeStepError, CompactCompletionRecord,
        CorrectionBonusKvDisposition, PublicationPhase, ReservedStateDelta,
        SpeculativeCompletionError, SpeculativeKvInterval, SpeculativeTokenInputs, StepPlan,
        StepPublication, StepPublicationError, TokenId, M1_MAX_COMPLETION_TOKENS,
        M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS, M1_MAX_SPECULATIVE_KV_TARGET_INPUTS,
    };

    fn target_decode() -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        }
    }

    fn draft_decode() -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        }
    }

    fn pair(slot: u32) -> IsolatedRequestKv {
        IsolatedRequestKv::new(RequestId::new(slot, 1), target_decode(), draft_decode()).unwrap()
    }

    fn speculative_selection(role: Qwen3ModelRole, bucket: Qwen3PlanBucket) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role,
            mode: Qwen3ExecutionMode::Speculative,
            bucket,
        }
    }

    fn speculative_pair(slot: u32, bucket: Qwen3PlanBucket) -> IsolatedRequestKv {
        IsolatedRequestKv::new(
            RequestId::new(slot, 1),
            speculative_selection(Qwen3ModelRole::Target8B, bucket),
            speculative_selection(Qwen3ModelRole::Draft06B, bucket),
        )
        .unwrap()
    }

    fn exact_round_index(
        request: RequestId,
        epoch: CompletionEpoch,
        bucket: Qwen3PlanBucket,
        k: u8,
        base: u32,
    ) -> SpeculativeKvRoundIndex {
        let mut draft_tokens = [0; M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS];
        for (position, token) in draft_tokens.iter_mut().enumerate().take(usize::from(k)) {
            *token = 100 + u32::try_from(position).unwrap();
        }
        let mut target_commit_ends = [0; M1_MAX_SPECULATIVE_KV_TARGET_INPUTS];
        let mut draft_commit_ends = [0; M1_MAX_SPECULATIVE_KV_TARGET_INPUTS];
        for accepted in 0..=usize::from(k) {
            let accepted = u32::try_from(accepted).unwrap();
            target_commit_ends[accepted as usize] = base + accepted + 1;
            draft_commit_ends[accepted as usize] = base
                + if accepted < u32::from(k) {
                    accepted + 1
                } else {
                    u32::from(k)
                };
        }
        SpeculativeKvRoundIndex {
            request,
            completion_epoch: epoch,
            plan_id: Identity::new([23; 32]),
            target_selection: speculative_selection(Qwen3ModelRole::Target8B, bucket),
            draft_selection: speculative_selection(Qwen3ModelRole::Draft06B, bucket),
            draft_token_count: k,
            round_anchor: 77,
            draft_tokens,
            target_pre_committed: base,
            draft_pre_committed: base,
            target_tentative: SpeculativeKvInterval {
                start: base,
                end: base + u32::from(k) + 1,
            },
            draft_tentative: SpeculativeKvInterval {
                start: base,
                end: base + u32::from(k),
            },
            target_commit_ends,
            draft_commit_ends,
            correction_bonus: CorrectionBonusKvDisposition::DeferredUntilNextStep,
        }
    }

    fn write_role_interval(
        batch: &ContinuousBatch,
        selected: &mut IsolatedRequestKv,
        other: &mut IsolatedRequestKv,
        request: RequestId,
        role: Qwen3ModelRole,
        start: u32,
        end: u32,
    ) {
        for logical_position in start..end {
            if logical_position.is_multiple_of(crate::M1_KV_PAGE_TOKENS) {
                let page =
                    PhysicalPageId::new(role, logical_position / crate::M1_KV_PAGE_TOKENS, 1);
                apply_isolated_kv_action(
                    batch,
                    selected,
                    other,
                    request,
                    role,
                    IsolatedKvAction::AppendPage { page },
                )
                .unwrap();
            }
            apply_isolated_kv_action(
                batch,
                selected,
                other,
                request,
                role,
                IsolatedKvAction::WriteToken { logical_position },
            )
            .unwrap();
        }
    }

    fn prepared_speculative_round(
        bucket: Qwen3PlanBucket,
        k: u8,
        base: u32,
    ) -> (
        ContinuousBatch,
        IsolatedRequestKv,
        IsolatedRequestKv,
        SpeculativeKvRoundIndex,
    ) {
        let mut batch = ContinuousBatch::initial();
        let mut selected = speculative_pair(1, bucket);
        let mut other = speculative_pair(2, bucket);
        let request = selected.request();
        let first_epoch = CompletionEpoch::new(31);
        admit_and_dispatch(&mut batch, &mut selected, &mut other, first_epoch);
        if base > 0 {
            write_role_interval(
                &batch,
                &mut selected,
                &mut other,
                request,
                Qwen3ModelRole::Target8B,
                0,
                base,
            );
            write_role_interval(
                &batch,
                &mut selected,
                &mut other,
                request,
                Qwen3ModelRole::Draft06B,
                0,
                base,
            );
            apply_isolated_scheduler_step(
                &mut batch,
                &mut selected,
                &mut other,
                request,
                IsolatedSchedulerAction::CompleteExact { epoch: first_epoch },
            )
            .unwrap();
            for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
                apply_isolated_kv_action(
                    &batch,
                    &mut selected,
                    &mut other,
                    request,
                    role,
                    IsolatedKvAction::Commit {
                        accepted_tokens: base,
                    },
                )
                .unwrap();
            }
            apply_isolated_scheduler_step(
                &mut batch,
                &mut selected,
                &mut other,
                request,
                IsolatedSchedulerAction::Publish {
                    epoch: first_epoch,
                    emitted_tokens: 1,
                },
            )
            .unwrap();
            apply_isolated_scheduler_step(
                &mut batch,
                &mut selected,
                &mut other,
                request,
                IsolatedSchedulerAction::FinalizeKv,
            )
            .unwrap();
        }
        let round_epoch = if base == 0 {
            first_epoch
        } else {
            let epoch = CompletionEpoch::new(32);
            apply_isolated_scheduler_step(
                &mut batch,
                &mut selected,
                &mut other,
                request,
                IsolatedSchedulerAction::Dispatch { epoch },
            )
            .unwrap();
            epoch
        };
        write_role_interval(
            &batch,
            &mut selected,
            &mut other,
            request,
            Qwen3ModelRole::Target8B,
            base,
            base + u32::from(k) + 1,
        );
        write_role_interval(
            &batch,
            &mut selected,
            &mut other,
            request,
            Qwen3ModelRole::Draft06B,
            base,
            base + u32::from(k),
        );
        apply_isolated_scheduler_step(
            &mut batch,
            &mut selected,
            &mut other,
            request,
            IsolatedSchedulerAction::CompleteExact { epoch: round_epoch },
        )
        .unwrap();
        let index = exact_round_index(request, round_epoch, bucket, k, base);
        (batch, selected, other, index)
    }

    fn admit_and_dispatch(
        batch: &mut ContinuousBatch,
        selected: &mut IsolatedRequestKv,
        other: &mut IsolatedRequestKv,
        epoch: CompletionEpoch,
    ) {
        let request = selected.request();
        apply_isolated_scheduler_step(
            batch,
            selected,
            other,
            request,
            IsolatedSchedulerAction::Admit,
        )
        .unwrap();
        apply_isolated_scheduler_step(
            batch,
            selected,
            other,
            request,
            IsolatedSchedulerAction::Dispatch { epoch },
        )
        .unwrap();
    }

    fn settle_prepared(
        batch: &ContinuousBatch,
        selected: &mut IsolatedRequestKv,
        other: &mut IsolatedRequestKv,
        index: &SpeculativeKvRoundIndex,
        accepted: u8,
    ) -> Result<IsolatedSpeculativeKvSettlement, RequestIsolationError> {
        let expectation = IsolatedSpeculativeKvExpectation::new(
            index.request,
            index.completion_epoch,
            index.plan_id,
            index.target_selection,
            index.draft_selection,
        );
        settle_isolated_speculative_kv(batch, selected, other, index, accepted, &expectation)
    }

    fn live_draft_tokens(index: &SpeculativeKvRoundIndex) -> Vec<TokenId> {
        index.draft_tokens[..usize::from(index.draft_token_count)].to_vec()
    }

    fn completion_record(
        index: &SpeculativeKvRoundIndex,
        accepted: u8,
        emitted: &[TokenId],
    ) -> CompactCompletionRecord {
        let mut emitted_tokens = [0; M1_MAX_COMPLETION_TOKENS];
        emitted_tokens[..emitted.len()].copy_from_slice(emitted);
        CompactCompletionRecord {
            request: index.request,
            epoch: index.completion_epoch,
            plan_id: index.plan_id,
            accepted_draft_tokens: accepted,
            emitted_token_count: u8::try_from(emitted.len()).unwrap(),
            emitted_tokens,
        }
    }

    fn reserved_speculative_publication(
        index: &SpeculativeKvRoundIndex,
        accepted: u8,
        emitted: &[TokenId],
    ) -> StepPublication {
        let completion = completion_record(index, accepted, emitted);
        StepPublication::reserve(
            StepPlan::new(
                index.request,
                index.completion_epoch,
                index.plan_id,
                index.target_selection,
            ),
            ReservedStateDelta::from_compact_completion(completion, index.target_selection),
        )
    }

    fn exact_settlement_expectation(
        index: &SpeculativeKvRoundIndex,
    ) -> IsolatedSpeculativeKvExpectation {
        IsolatedSpeculativeKvExpectation::new(
            index.request,
            index.completion_epoch,
            index.plan_id,
            index.target_selection,
            index.draft_selection,
        )
    }

    fn token_inputs<'a>(
        draft_tokens: &'a [TokenId],
        target_choices: &'a [TokenId],
    ) -> SpeculativeTokenInputs<'a> {
        SpeculativeTokenInputs {
            draft_tokens,
            target_choices,
        }
    }

    #[test]
    fn zero_acceptance_commits_anchor_and_rolls_back_both_suffixes() {
        let (batch, mut selected, mut other, index) =
            prepared_speculative_round(Qwen3PlanBucket::SpeculativeS1K4C8192, 4, 0);
        let batch_before = batch;
        let other_before = other.projection();
        let outcome = settle_prepared(&batch, &mut selected, &mut other, &index, 0).unwrap();
        assert_eq!(outcome.target_commit_end, 1);
        assert_eq!(outcome.draft_commit_end, 1);
        assert_eq!(outcome.target_retired_pages, 0);
        assert_eq!(outcome.draft_retired_pages, 0);
        let projection = selected.projection();
        assert_eq!(projection.target.resident_tokens, 1);
        assert_eq!(projection.target.committed_tokens, 1);
        assert_eq!(projection.draft.resident_tokens, 1);
        assert_eq!(projection.draft.committed_tokens, 1);
        assert_eq!(batch, batch_before);
        assert_eq!(other.projection(), other_before);

        let before_replay = selected.projection();
        assert_eq!(
            settle_prepared(&batch, &mut selected, &mut other, &index, 0),
            Err(RequestIsolationError::Physical(
                PhysicalKvError::SettlementCursorMismatch
            ))
        );
        assert_eq!(selected.projection(), before_replay);
    }

    #[test]
    fn full_acceptance_keeps_every_resident_role_input() {
        let (batch, mut selected, mut other, index) =
            prepared_speculative_round(Qwen3PlanBucket::SpeculativeS1K4C8192, 4, 0);
        let outcome = settle_prepared(&batch, &mut selected, &mut other, &index, 4).unwrap();
        assert_eq!(outcome.target_commit_end, 5);
        assert_eq!(outcome.draft_commit_end, 4);
        assert_eq!(outcome.target_retired_pages, 0);
        assert_eq!(outcome.draft_retired_pages, 0);
        let projection = selected.projection();
        assert_eq!(projection.target.resident_tokens, 5);
        assert_eq!(projection.target.committed_tokens, 5);
        assert_eq!(projection.draft.resident_tokens, 4);
        assert_eq!(projection.draft.committed_tokens, 4);
    }

    #[test]
    fn nonaligned_commit_shrinks_retained_prefix_after_retiring_last_page() {
        let (batch, mut selected, mut other, index) =
            prepared_speculative_round(Qwen3PlanBucket::SpeculativeS1K16C8192, 16, 0);
        assert_eq!(selected.target.page_count(), 2);
        assert_eq!(selected.draft.page_count(), 1);
        let outcome = settle_prepared(&batch, &mut selected, &mut other, &index, 0).unwrap();
        assert_eq!(outcome.target_commit_end, 1);
        assert_eq!(outcome.target_retired_pages, 1);
        assert_eq!(outcome.draft_retired_pages, 0);
        assert_eq!(selected.target.page_count(), 1);
        assert_eq!(selected.draft.page_count(), 1);
        assert!(map_isolated_token(
            &batch,
            &selected,
            &other,
            index.request,
            Qwen3ModelRole::Target8B,
            0,
        )
        .is_ok());
        assert_eq!(
            map_isolated_token(
                &batch,
                &selected,
                &other,
                index.request,
                Qwen3ModelRole::Target8B,
                1,
            ),
            Err(RequestIsolationError::Physical(
                PhysicalKvError::LogicalPositionOutOfRange
            ))
        );
    }

    #[test]
    fn retirement_capacity_is_checked_before_either_role_mutates() {
        let (batch, mut selected, mut other, index) =
            prepared_speculative_round(Qwen3PlanBucket::SpeculativeS1K16C8192, 16, 0);
        selected.target_retired_pages = u32::MAX;
        let selected_before = selected.projection();
        let other_before = other.projection();
        assert_eq!(
            settle_prepared(&batch, &mut selected, &mut other, &index, 0),
            Err(RequestIsolationError::RetiredPageCountExhausted)
        );
        assert_eq!(selected.projection(), selected_before);
        assert_eq!(other.projection(), other_before);
        assert_eq!(selected.target.page_count(), 2);
        assert_eq!(selected.draft.page_count(), 1);
    }

    #[test]
    fn invalid_second_role_leaves_preflighted_target_unchanged() {
        let (batch, mut selected, mut other, index) =
            prepared_speculative_round(Qwen3PlanBucket::SpeculativeS1K4C8192, 4, 0);
        apply_isolated_kv_action(
            &batch,
            &mut selected,
            &mut other,
            index.request,
            Qwen3ModelRole::Draft06B,
            IsolatedKvAction::RollbackToken {
                after_epoch: index.completion_epoch,
            },
        )
        .unwrap();
        let selected_before = selected.projection();
        let target_pages_before = selected.target.page_count();
        assert_eq!(
            settle_prepared(&batch, &mut selected, &mut other, &index, 0),
            Err(RequestIsolationError::Physical(
                PhysicalKvError::SettlementIntervalMismatch
            ))
        );
        assert_eq!(selected.projection(), selected_before);
        assert_eq!(selected.target.page_count(), target_pages_before);
    }

    #[test]
    fn settlement_expectation_and_accepted_count_drift_fail_before_mutation() {
        let (batch, mut selected, mut other, index) =
            prepared_speculative_round(Qwen3PlanBucket::SpeculativeS1K4C8192, 4, 0);
        let exact = IsolatedSpeculativeKvExpectation::new(
            index.request,
            index.completion_epoch,
            index.plan_id,
            index.target_selection,
            index.draft_selection,
        );
        let before = selected.projection();

        let mut changed = exact;
        changed.request = RequestId::new(index.request.slot(), index.request.generation() + 1);
        assert_eq!(
            settle_isolated_speculative_kv(&batch, &mut selected, &mut other, &index, 0, &changed,),
            Err(RequestIsolationError::StaleRequest)
        );
        changed = exact;
        changed.completion_epoch = CompletionEpoch::new(index.completion_epoch.value + 1);
        assert_eq!(
            settle_isolated_speculative_kv(&batch, &mut selected, &mut other, &index, 0, &changed,),
            Err(RequestIsolationError::EpochMismatch)
        );
        changed = exact;
        changed.plan_id = Identity::new([24; 32]);
        assert_eq!(
            settle_isolated_speculative_kv(&batch, &mut selected, &mut other, &index, 0, &changed,),
            Err(RequestIsolationError::InvalidSpeculativeIndex)
        );
        changed = exact;
        changed.target_selection.bucket = Qwen3PlanBucket::SpeculativeS1K8C8192;
        assert_eq!(
            settle_isolated_speculative_kv(&batch, &mut selected, &mut other, &index, 0, &changed,),
            Err(RequestIsolationError::PlanPairMismatch)
        );
        assert_eq!(
            settle_isolated_speculative_kv(&batch, &mut selected, &mut other, &index, 5, &exact,),
            Err(RequestIsolationError::AcceptedCountOutOfRange)
        );
        assert_eq!(selected.projection(), before);
    }

    #[test]
    fn atomic_speculative_step_zero_partial_and_full_acceptance() {
        for (accepted, target_choices, emitted, target_end, draft_end) in [
            (0, vec![900, 901, 902, 903, 904], vec![900], 1, 1),
            (2, vec![100, 101, 900, 903, 904], vec![100, 101, 900], 3, 3),
            (
                4,
                vec![100, 101, 102, 103, 900],
                vec![100, 101, 102, 103, 900],
                5,
                4,
            ),
        ] {
            let (mut batch, mut selected, mut other, index) =
                prepared_speculative_round(Qwen3PlanBucket::SpeculativeS1K4C8192, 4, 0);
            let draft_tokens = live_draft_tokens(&index);
            let mut publication = reserved_speculative_publication(&index, accepted, &emitted);
            let expected = exact_settlement_expectation(&index);
            let batch_before = batch;
            let other_before = other.projection();

            let outcome = settle_and_publish_speculative_step(
                &mut batch,
                &mut publication,
                &mut selected,
                &mut other,
                &index,
                &expected,
                token_inputs(&draft_tokens, &target_choices),
            )
            .unwrap();

            assert_eq!(publication.phase(), PublicationPhase::Published);
            assert_eq!(outcome.settlement.accepted_draft_tokens, accepted);
            assert_eq!(outcome.settlement.target_commit_end, target_end);
            assert_eq!(outcome.settlement.draft_commit_end, draft_end);
            assert_eq!(
                &outcome.published_delta.emitted_tokens()[..emitted.len()],
                emitted.as_slice()
            );
            assert_eq!(
                outcome.published_delta.emitted_token_count() as usize,
                emitted.len()
            );
            assert_eq!(batch, batch_before);
            assert_eq!(other.projection(), other_before);
        }
    }

    #[test]
    fn atomic_speculative_step_rejects_stale_bindings_without_mutation() {
        let (mut batch, mut selected, mut other, mut index) =
            prepared_speculative_round(Qwen3PlanBucket::SpeculativeS1K4C8192, 4, 0);
        let draft_tokens = live_draft_tokens(&index);
        let target_choices = [100, 101, 900, 903, 904];
        let emitted = [100, 101, 900];
        let mut publication = reserved_speculative_publication(&index, 2, &emitted);
        let publication_before = reserved_speculative_publication(&index, 2, &emitted);
        let batch_before = batch;
        let selected_before = selected.projection();
        let other_before = other.projection();
        let exact = exact_settlement_expectation(&index);

        for (changed, error) in [
            (
                IsolatedSpeculativeKvExpectation {
                    request: RequestId::new(index.request.slot(), index.request.generation() + 1),
                    ..exact
                },
                RequestIsolationError::StaleRequest,
            ),
            (
                IsolatedSpeculativeKvExpectation {
                    completion_epoch: CompletionEpoch::new(index.completion_epoch.value + 1),
                    ..exact
                },
                RequestIsolationError::EpochMismatch,
            ),
            (
                IsolatedSpeculativeKvExpectation {
                    plan_id: Identity::new([24; 32]),
                    ..exact
                },
                RequestIsolationError::InvalidSpeculativeIndex,
            ),
            (
                IsolatedSpeculativeKvExpectation {
                    target_selection: Qwen3PlanSelection {
                        bucket: Qwen3PlanBucket::SpeculativeS1K8C8192,
                        ..index.target_selection
                    },
                    ..exact
                },
                RequestIsolationError::PlanPairMismatch,
            ),
        ] {
            assert_eq!(
                settle_and_publish_speculative_step(
                    &mut batch,
                    &mut publication,
                    &mut selected,
                    &mut other,
                    &index,
                    &changed,
                    token_inputs(&draft_tokens, &target_choices),
                ),
                Err(AtomicSpeculativeStepError::Kv(error))
            );
            assert_eq!(publication, publication_before);
            assert_eq!(batch, batch_before);
            assert_eq!(selected.projection(), selected_before);
            assert_eq!(other.projection(), other_before);
        }

        index.correction_bonus = CorrectionBonusKvDisposition::TargetResident;
        assert_eq!(
            settle_and_publish_speculative_step(
                &mut batch,
                &mut publication,
                &mut selected,
                &mut other,
                &index,
                &exact,
                token_inputs(&draft_tokens, &target_choices),
            ),
            Err(AtomicSpeculativeStepError::Kv(
                RequestIsolationError::InvalidSpeculativeIndex
            ))
        );
        assert_eq!(publication, publication_before);
        assert_eq!(batch, batch_before);
        assert_eq!(selected.projection(), selected_before);
        assert_eq!(other.projection(), other_before);
    }

    #[test]
    fn atomic_speculative_step_rejects_invalid_completion_and_draft_drift() {
        let (mut batch, mut selected, mut other, index) =
            prepared_speculative_round(Qwen3PlanBucket::SpeculativeS1K4C8192, 4, 0);
        let draft_tokens = live_draft_tokens(&index);
        let target_choices = [100, 101, 900, 903, 904];
        let expected = exact_settlement_expectation(&index);
        let batch_before = batch;
        let selected_before = selected.projection();
        let other_before = other.projection();

        let mut wrong_draft = draft_tokens.clone();
        wrong_draft[1] += 1;
        let mut publication = reserved_speculative_publication(&index, 2, &[100, 101, 900]);
        let publication_before = reserved_speculative_publication(&index, 2, &[100, 101, 900]);
        assert_eq!(
            settle_and_publish_speculative_step(
                &mut batch,
                &mut publication,
                &mut selected,
                &mut other,
                &index,
                &expected,
                token_inputs(&wrong_draft, &target_choices),
            ),
            Err(AtomicSpeculativeStepError::DraftTokensMismatch)
        );
        assert_eq!(publication, publication_before);

        let mut invalid_publication = reserved_speculative_publication(&index, 2, &[100, 101, 999]);
        let invalid_before = reserved_speculative_publication(&index, 2, &[100, 101, 999]);
        assert_eq!(
            settle_and_publish_speculative_step(
                &mut batch,
                &mut invalid_publication,
                &mut selected,
                &mut other,
                &index,
                &expected,
                token_inputs(&draft_tokens, &target_choices),
            ),
            Err(AtomicSpeculativeStepError::Publication(
                StepPublicationError::Speculative(SpeculativeCompletionError::EmittedTokenMismatch)
            ))
        );
        assert_eq!(invalid_publication, invalid_before);
        assert_eq!(batch, batch_before);
        assert_eq!(selected.projection(), selected_before);
        assert_eq!(other.projection(), other_before);
    }

    #[test]
    fn atomic_speculative_step_preflights_second_role_and_retired_capacity() {
        let (mut batch, mut selected, mut other, index) =
            prepared_speculative_round(Qwen3PlanBucket::SpeculativeS1K4C8192, 4, 0);
        apply_isolated_kv_action(
            &batch,
            &mut selected,
            &mut other,
            index.request,
            Qwen3ModelRole::Draft06B,
            IsolatedKvAction::RollbackToken {
                after_epoch: index.completion_epoch,
            },
        )
        .unwrap();
        let draft_tokens = live_draft_tokens(&index);
        let mut publication = reserved_speculative_publication(&index, 0, &[900]);
        let publication_before = reserved_speculative_publication(&index, 0, &[900]);
        let expected = exact_settlement_expectation(&index);
        let batch_before = batch;
        let selected_before = selected.projection();
        let other_before = other.projection();
        assert_eq!(
            settle_and_publish_speculative_step(
                &mut batch,
                &mut publication,
                &mut selected,
                &mut other,
                &index,
                &expected,
                token_inputs(&draft_tokens, &[900, 901, 902, 903, 904]),
            ),
            Err(AtomicSpeculativeStepError::Kv(
                RequestIsolationError::Physical(PhysicalKvError::SettlementIntervalMismatch)
            ))
        );
        assert_eq!(publication, publication_before);
        assert_eq!(batch, batch_before);
        assert_eq!(selected.projection(), selected_before);
        assert_eq!(other.projection(), other_before);

        let (mut batch, mut selected, mut other, index) =
            prepared_speculative_round(Qwen3PlanBucket::SpeculativeS1K16C8192, 16, 0);
        selected.target_retired_pages = u32::MAX;
        let draft_tokens = live_draft_tokens(&index);
        let mut target_choices = vec![900; 17];
        target_choices[0] = 999;
        let mut publication = reserved_speculative_publication(&index, 0, &[999]);
        let publication_before = reserved_speculative_publication(&index, 0, &[999]);
        let expected = exact_settlement_expectation(&index);
        let batch_before = batch;
        let selected_before = selected.projection();
        let other_before = other.projection();
        assert_eq!(
            settle_and_publish_speculative_step(
                &mut batch,
                &mut publication,
                &mut selected,
                &mut other,
                &index,
                &expected,
                token_inputs(&draft_tokens, &target_choices),
            ),
            Err(AtomicSpeculativeStepError::Kv(
                RequestIsolationError::RetiredPageCountExhausted
            ))
        );
        assert_eq!(publication, publication_before);
        assert_eq!(batch, batch_before);
        assert_eq!(selected.projection(), selected_before);
        assert_eq!(other.projection(), other_before);
    }

    #[test]
    fn atomic_speculative_step_publication_is_exactly_once() {
        let (mut batch, mut selected, mut other, index) =
            prepared_speculative_round(Qwen3PlanBucket::SpeculativeS1K4C8192, 4, 0);
        let draft_tokens = live_draft_tokens(&index);
        let target_choices = [100, 101, 900, 903, 904];
        let mut publication = reserved_speculative_publication(&index, 2, &[100, 101, 900]);
        let expected = exact_settlement_expectation(&index);
        settle_and_publish_speculative_step(
            &mut batch,
            &mut publication,
            &mut selected,
            &mut other,
            &index,
            &expected,
            token_inputs(&draft_tokens, &target_choices),
        )
        .unwrap();
        let batch_before = batch;
        let selected_before = selected.projection();
        let other_before = other.projection();
        let delta_before = publication.delta();

        assert_eq!(
            settle_and_publish_speculative_step(
                &mut batch,
                &mut publication,
                &mut selected,
                &mut other,
                &index,
                &expected,
                token_inputs(&draft_tokens, &target_choices),
            ),
            Err(AtomicSpeculativeStepError::Publication(
                StepPublicationError::WrongPhase
            ))
        );
        assert_eq!(publication.phase(), PublicationPhase::Published);
        assert_eq!(publication.delta(), delta_before);
        assert_eq!(batch, batch_before);
        assert_eq!(selected.projection(), selected_before);
        assert_eq!(other.projection(), other_before);
    }

    #[test]
    fn selected_target_write_frames_other_scheduler_and_both_kv_roles() {
        let mut batch = ContinuousBatch::initial();
        let mut first = pair(0);
        let mut second = pair(31);
        admit_and_dispatch(&mut batch, &mut first, &mut second, CompletionEpoch::new(7));
        apply_isolated_scheduler_step(
            &mut batch,
            &mut second,
            &mut first,
            RequestId::new(31, 1),
            IsolatedSchedulerAction::Admit,
        )
        .unwrap();
        let request = first.request();
        let other_before = second.projection();
        let other_scheduler_before = batch.request(second.request()).unwrap();
        let page = PhysicalPageId::new(Qwen3ModelRole::Target8B, 3, 1);
        apply_isolated_kv_action(
            &batch,
            &mut first,
            &mut second,
            request,
            Qwen3ModelRole::Target8B,
            IsolatedKvAction::AppendPage { page },
        )
        .unwrap();
        apply_isolated_kv_action(
            &batch,
            &mut first,
            &mut second,
            request,
            Qwen3ModelRole::Target8B,
            IsolatedKvAction::WriteToken {
                logical_position: 0,
            },
        )
        .unwrap();
        assert_eq!(second.projection(), other_before);
        assert_eq!(
            batch.request(second.request()).unwrap(),
            other_scheduler_before
        );
        assert_eq!(
            map_isolated_token(
                &batch,
                &first,
                &second,
                request,
                Qwen3ModelRole::Target8B,
                0,
            ),
            Ok(PhysicalKvLocation { page, offset: 0 })
        );
    }

    #[test]
    fn stale_generation_role_and_epoch_fail_closed() {
        let mut batch = ContinuousBatch::initial();
        let mut first = pair(1);
        let mut second = pair(2);
        admit_and_dispatch(&mut batch, &mut first, &mut second, CompletionEpoch::new(9));
        let first_before = first.projection();
        let second_before = second.projection();
        let request = first.request();
        assert_eq!(
            apply_isolated_kv_action(
                &batch,
                &mut first,
                &mut second,
                RequestId::new(1, 2),
                Qwen3ModelRole::Target8B,
                IsolatedKvAction::WriteToken {
                    logical_position: 0
                },
            ),
            Err(RequestIsolationError::StaleRequest)
        );
        assert_eq!(
            apply_isolated_kv_action(
                &batch,
                &mut first,
                &mut second,
                request,
                Qwen3ModelRole::Draft06B,
                IsolatedKvAction::AppendPage {
                    page: PhysicalPageId::new(Qwen3ModelRole::Target8B, 0, 1),
                },
            ),
            Err(RequestIsolationError::Physical(
                PhysicalKvError::RoleMismatch
            ))
        );
        assert_eq!(
            apply_isolated_scheduler_step(
                &mut batch,
                &mut first,
                &mut second,
                request,
                IsolatedSchedulerAction::CompleteExact {
                    epoch: CompletionEpoch::new(10),
                },
            ),
            Err(RequestIsolationError::Scheduler(
                ContinuousBatchError::EpochMismatch
            ))
        );
        assert_eq!(first.projection(), first_before);
        assert_eq!(second.projection(), second_before);
    }

    #[test]
    fn cancellation_retirement_and_exact_quiescence_are_composed() {
        let mut batch = ContinuousBatch::initial();
        let mut first = pair(4);
        let mut second = pair(5);
        let epoch = CompletionEpoch::new(17);
        admit_and_dispatch(&mut batch, &mut first, &mut second, epoch);
        let request = first.request();
        let page = PhysicalPageId::new(Qwen3ModelRole::Target8B, 0, 1);
        apply_isolated_kv_action(
            &batch,
            &mut first,
            &mut second,
            request,
            Qwen3ModelRole::Target8B,
            IsolatedKvAction::AppendPage { page },
        )
        .unwrap();
        apply_isolated_kv_action(
            &batch,
            &mut first,
            &mut second,
            request,
            Qwen3ModelRole::Target8B,
            IsolatedKvAction::WriteToken {
                logical_position: 0,
            },
        )
        .unwrap();
        cancel_isolated_request(&mut batch, &mut first, &mut second, request).unwrap();
        assert_eq!(
            release_isolated_page(
                &batch,
                &mut first,
                &mut second,
                request,
                Qwen3ModelRole::Target8B,
                page,
                epoch,
            ),
            Err(RequestIsolationError::WrongLifecycle)
        );
        apply_isolated_scheduler_step(
            &mut batch,
            &mut first,
            &mut second,
            request,
            IsolatedSchedulerAction::CompleteExact { epoch },
        )
        .unwrap();
        assert_eq!(
            release_isolated_page(
                &batch,
                &mut first,
                &mut second,
                request,
                Qwen3ModelRole::Target8B,
                page,
                CompletionEpoch::new(18),
            ),
            Err(RequestIsolationError::EpochMismatch)
        );
        assert_eq!(
            apply_isolated_kv_action(
                &batch,
                &mut first,
                &mut second,
                request,
                Qwen3ModelRole::Target8B,
                IsolatedKvAction::RetireTail { after_epoch: epoch },
            ),
            Ok(Some(page))
        );
        let next = release_isolated_page(
            &batch,
            &mut first,
            &mut second,
            request,
            Qwen3ModelRole::Target8B,
            page,
            epoch,
        )
        .unwrap();
        assert_eq!(next.generation(), 2);
    }

    #[test]
    fn detach_requires_both_roles_drained_and_advances_generation() {
        let mut batch = ContinuousBatch::initial();
        let mut first = pair(6);
        let mut second = pair(7);
        let epoch = CompletionEpoch::new(20);
        admit_and_dispatch(&mut batch, &mut first, &mut second, epoch);
        let request = first.request();
        cancel_isolated_request(&mut batch, &mut first, &mut second, request).unwrap();
        apply_isolated_scheduler_step(
            &mut batch,
            &mut first,
            &mut second,
            request,
            IsolatedSchedulerAction::CompleteExact { epoch },
        )
        .unwrap();
        let next = detach_isolated_request(&mut batch, &mut first, &mut second, request).unwrap();
        assert_eq!(next, RequestId::new(6, 2));
        assert_eq!(batch.request(request), None);
        assert_eq!(first.request(), next);
        assert_eq!(
            cancel_isolated_request(&mut batch, &mut first, &mut second, request),
            Err(RequestIsolationError::StaleRequest)
        );
    }

    #[test]
    fn ready_cancellation_without_exact_epoch_fails_closed() {
        let mut batch = ContinuousBatch::initial();
        let mut first = pair(8);
        let mut second = pair(9);
        let request = first.request();
        apply_isolated_scheduler_step(
            &mut batch,
            &mut first,
            &mut second,
            request,
            IsolatedSchedulerAction::Admit,
        )
        .unwrap();
        let before = first.projection();
        assert_eq!(
            cancel_isolated_request(&mut batch, &mut first, &mut second, request),
            Err(RequestIsolationError::MissingEpoch)
        );
        assert_eq!(first.projection(), before);
    }

    #[test]
    fn swapped_roles_mismatched_buckets_and_same_slot_are_rejected() {
        assert_eq!(
            IsolatedRequestKv::new(RequestId::new(0, 1), draft_decode(), target_decode(),),
            Err(RequestIsolationError::TargetRoleRequired)
        );
        let draft_prefill = Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Prefill,
            bucket: Qwen3PlanBucket::PrefillS1T128,
        };
        assert_eq!(
            IsolatedRequestKv::new(RequestId::new(0, 1), target_decode(), draft_prefill,),
            Err(RequestIsolationError::PlanPairMismatch)
        );
        let mut batch = ContinuousBatch::initial();
        let mut first = pair(3);
        let mut same = pair(3);
        let request = first.request();
        assert_eq!(
            apply_isolated_scheduler_step(
                &mut batch,
                &mut first,
                &mut same,
                request,
                IsolatedSchedulerAction::Admit,
            ),
            Err(RequestIsolationError::SameRequestSlot)
        );
    }
}
