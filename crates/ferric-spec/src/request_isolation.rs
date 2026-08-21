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
    append_physical_page, cancel_physical_kv, commit_physical_kv, map_initialized_token,
    release_retired_page, retire_cancelled_tail, rollback_physical_token, write_physical_token,
    KvQuiescenceAuthority, LogicalKvState, PhysicalKvError, PhysicalKvLifecycle,
    PhysicalKvLocation, PhysicalKvState, PhysicalPageId,
};
use crate::scheduling::{LifecyclePhase, RequestState};
use crate::{Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection, RequestId};
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
