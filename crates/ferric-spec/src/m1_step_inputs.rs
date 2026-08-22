//! Structural validation for generated M1 runtime patch inputs.
//!
//! This module validates only a logical lane roster and the four fixed-shape
//! U32 arrays declared by the generated runner. Public [`StepPlan`] values do
//! not authenticate scheduler dispatch or KV authority. This typestate alone
//! does not establish an end-to-end kernel binding, masking, launch,
//! allocation, address, packet, completion, runtime, hardware, or performance
//! claim.

use crate::{
    Qwen3ExecutionMode, Qwen3PlanDimensions, Qwen3PlanSelection, StepPlan, TokenId,
    M1_MAX_ACTIVE_SEQUENCES, QWEN3_VOCABULARY_SIZE,
};
use core::fmt;
use vstd::prelude::*;

verus! {

/// Untrusted structural lane roster and generated patch-slot values.
///
/// Token and position rows are sequence-major and always use the selected
/// bucket's full active-width capacity. `ContextLengths` are pre-step committed
/// lengths. `None` lanes and row elements beyond each live active width are
/// canonical zero padding.
#[verifier::allow(autoderive_clone_without_spec)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct M1StepInputCandidate {
    selection: Qwen3PlanSelection,
    lanes: Vec<Option<StepPlan>>,
    token_ids: Vec<TokenId>,
    position_ids: Vec<u32>,
    active_lengths: Vec<u32>,
    context_lengths: Vec<u32>,
}

/// Exact owned parts of one untrusted structural candidate.
pub type M1StepInputParts = (
    Qwen3PlanSelection,
    Vec<Option<StepPlan>>,
    Vec<TokenId>,
    Vec<u32>,
    Vec<u32>,
    Vec<u32>,
);

impl M1StepInputCandidate {
    pub closed spec fn selection_spec(&self) -> Qwen3PlanSelection { self.selection }
    pub closed spec fn lanes_spec(&self) -> Seq<Option<StepPlan>> { self.lanes@ }
    pub closed spec fn token_ids_spec(&self) -> Seq<TokenId> { self.token_ids@ }
    pub closed spec fn position_ids_spec(&self) -> Seq<u32> { self.position_ids@ }
    pub closed spec fn active_lengths_spec(&self) -> Seq<u32> { self.active_lengths@ }
    pub closed spec fn context_lengths_spec(&self) -> Seq<u32> { self.context_lengths@ }

    /// Constructs one owned untrusted candidate and consumes every vector.
    #[must_use]
    pub fn new(
        selection: Qwen3PlanSelection,
        lanes: Vec<Option<StepPlan>>,
        token_ids: Vec<TokenId>,
        position_ids: Vec<u32>,
        active_lengths: Vec<u32>,
        context_lengths: Vec<u32>,
    ) -> (candidate: Self)
        ensures
            candidate.selection_spec() == selection,
            candidate.lanes_spec() == lanes@,
            candidate.token_ids_spec() == token_ids@,
            candidate.position_ids_spec() == position_ids@,
            candidate.active_lengths_spec() == active_lengths@,
            candidate.context_lengths_spec() == context_lengths@,
    {
        Self {
            selection,
            lanes,
            token_ids,
            position_ids,
            active_lengths,
            context_lengths,
        }
    }

    /// Returns the explicit untrusted role, mode, and bucket selection.
    #[must_use]
    pub const fn selection(&self) -> (selection: Qwen3PlanSelection)
        ensures selection == self.selection_spec(),
    {
        self.selection
    }

    /// Returns the bucket-capacity structural lane roster.
    #[must_use]
    pub fn lanes(&self) -> (lanes: &[Option<StepPlan>])
        ensures lanes@ == self.lanes_spec(),
    {
        &self.lanes
    }

    /// Returns the fixed-shape sequence-major token rows.
    #[must_use]
    pub fn token_ids(&self) -> (tokens: &[TokenId])
        ensures tokens@ == self.token_ids_spec(),
    {
        &self.token_ids
    }

    /// Returns the fixed-shape sequence-major logical-position rows.
    #[must_use]
    pub fn position_ids(&self) -> (positions: &[u32])
        ensures positions@ == self.position_ids_spec(),
    {
        &self.position_ids
    }

    /// Returns one active width per bucket lane.
    #[must_use]
    pub fn active_lengths(&self) -> (lengths: &[u32])
        ensures lengths@ == self.active_lengths_spec(),
    {
        &self.active_lengths
    }

    /// Returns one pre-step committed length per bucket lane.
    #[must_use]
    pub fn context_lengths(&self) -> (lengths: &[u32])
        ensures lengths@ == self.context_lengths_spec(),
    {
        &self.context_lengths
    }

    /// Recovers the exact selection, lane roster, and four owned arrays.
    #[must_use]
    pub fn into_parts(self) -> (parts: M1StepInputParts)
        ensures
            parts.0 == self.selection_spec(),
            parts.1@ == self.lanes_spec(),
            parts.2@ == self.token_ids_spec(),
            parts.3@ == self.position_ids_spec(),
            parts.4@ == self.active_lengths_spec(),
            parts.5@ == self.context_lengths_spec(),
    {
        (
            self.selection,
            self.lanes,
            self.token_ids,
            self.position_ids,
            self.active_lengths,
            self.context_lengths,
        )
    }
}

/// Stable fail-closed reason for rejecting one structural candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum M1StepInputError {
    InvalidSelection,
    LaneRosterCount { expected: usize, actual: usize },
    TokenCount { expected: usize, actual: usize },
    PositionCount { expected: usize, actual: usize },
    ActiveLengthCount { expected: usize, actual: usize },
    ContextLengthCount { expected: usize, actual: usize },
    DimensionOverflow,
    EmptyLivePrefix,
    LiveLaneAfterInactive { lane: usize },
    PlanSelectionMismatch { lane: usize },
    AbsentPlanIdentity { lane: usize },
    PlanIdentityMismatch { lane: usize },
    ZeroCompletionEpoch { lane: usize },
    CompletionEpochMismatch { lane: usize },
    RequestSlotOutOfRange { lane: usize, slot: u32 },
    ZeroRequestGeneration { lane: usize },
    DuplicateRequestSlot { first_lane: usize, lane: usize, slot: u32 },
    PrefillActiveOutOfRange { lane: usize, capacity: u32, actual: u32 },
    ActiveLengthMismatch { lane: usize, expected: u32, actual: u32 },
    PrefillCommittedContextNonZero { lane: usize, actual: u32 },
    ContextLengthOverflow { lane: usize },
    ContextExceedsCapacity {
        lane: usize,
        committed: u32,
        active: u32,
        capacity: u32,
    },
    InactiveLengthPadding { lane: usize, active: u32, context: u32 },
    TokenOutOfRange { lane: usize, active_index: usize, token: TokenId },
    PositionMismatch {
        lane: usize,
        active_index: usize,
        expected: u32,
        actual: u32,
    },
    TokenPaddingNonZero { lane: usize, active_index: usize, actual: TokenId },
    PositionPaddingNonZero { lane: usize, active_index: usize, actual: u32 },
}

/// Retry-safe rejection retaining the exact unchanged structural candidate.
#[derive(Debug, PartialEq, Eq)]
pub struct M1StepInputRejection {
    error: M1StepInputError,
    candidate: M1StepInputCandidate,
}

impl M1StepInputRejection {
    pub closed spec fn error_spec(&self) -> M1StepInputError { self.error }
    pub closed spec fn candidate_selection_spec(&self) -> Qwen3PlanSelection {
        self.candidate.selection_spec()
    }
    pub closed spec fn candidate_lanes_spec(&self) -> Seq<Option<StepPlan>> {
        self.candidate.lanes_spec()
    }
    pub closed spec fn candidate_token_ids_spec(&self) -> Seq<TokenId> {
        self.candidate.token_ids_spec()
    }
    pub closed spec fn candidate_position_ids_spec(&self) -> Seq<u32> {
        self.candidate.position_ids_spec()
    }
    pub closed spec fn candidate_active_lengths_spec(&self) -> Seq<u32> {
        self.candidate.active_lengths_spec()
    }
    pub closed spec fn candidate_context_lengths_spec(&self) -> Seq<u32> {
        self.candidate.context_lengths_spec()
    }

    /// Returns the stable diagnostic without consuming retained values.
    #[must_use]
    pub const fn error(&self) -> (error: M1StepInputError)
        ensures error == self.error_spec(),
    {
        self.error
    }

    /// Returns the exact unchanged candidate for diagnosis.
    #[must_use]
    pub const fn candidate(&self) -> &M1StepInputCandidate {
        &self.candidate
    }

    /// Recovers the diagnostic and exact unchanged candidate for retry.
    #[must_use]
    pub fn into_parts(self) -> (parts: (M1StepInputError, M1StepInputCandidate))
        ensures
            parts.0 == self.error_spec(),
            parts.1.selection_spec() == self.candidate_selection_spec(),
            parts.1.lanes_spec() == self.candidate_lanes_spec(),
            parts.1.token_ids_spec() == self.candidate_token_ids_spec(),
            parts.1.position_ids_spec() == self.candidate_position_ids_spec(),
            parts.1.active_lengths_spec() == self.candidate_active_lengths_spec(),
            parts.1.context_lengths_spec() == self.candidate_context_lengths_spec(),
    {
        (self.error, self.candidate)
    }
}

/// Non-clone custody of one structurally valid lane roster and input arrays.
///
/// This is structural only. It is not scheduler dispatch, KV, masking, launch,
/// allocation, address, packet, completion, runtime, machine, or performance
/// authority.
///
/// ```compile_fail
/// use ferric_spec::ValidatedM1StepInputs;
///
/// fn duplicate(validated: ValidatedM1StepInputs) {
///     let _second = validated.clone();
/// }
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct ValidatedM1StepInputs {
    candidate: M1StepInputCandidate,
    dimensions: Qwen3PlanDimensions,
    live_lanes: u32,
}

impl ValidatedM1StepInputs {
    pub closed spec fn selection_spec(&self) -> Qwen3PlanSelection {
        self.candidate.selection_spec()
    }
    pub closed spec fn lanes_spec(&self) -> Seq<Option<StepPlan>> {
        self.candidate.lanes_spec()
    }
    pub closed spec fn token_ids_spec(&self) -> Seq<TokenId> {
        self.candidate.token_ids_spec()
    }
    pub closed spec fn position_ids_spec(&self) -> Seq<u32> {
        self.candidate.position_ids_spec()
    }
    pub closed spec fn active_lengths_spec(&self) -> Seq<u32> {
        self.candidate.active_lengths_spec()
    }
    pub closed spec fn context_lengths_spec(&self) -> Seq<u32> {
        self.candidate.context_lengths_spec()
    }
    pub closed spec fn dimensions_spec(&self) -> Qwen3PlanDimensions { self.dimensions }
    pub closed spec fn live_lanes_spec(&self) -> u32 { self.live_lanes }

    /// Exact verifier relation carried by this private-constructor typestate.
    pub closed spec fn valid(&self) -> bool {
        let selection = self.selection_spec();
        &&& m1_step_input_candidate_valid(&self.candidate)
        &&& selection.bucket.dimensions_spec(selection.role, selection.mode)
            == Some(self.dimensions)
        &&& self.live_lanes > 0
        &&& self.live_lanes <= self.dimensions.sequences
        &&& forall|lane: int|
            0 <= lane < self.live_lanes as int ==> self.lanes_spec()[lane].is_some()
        &&& forall|lane: int|
            self.live_lanes as int <= lane < self.lanes_spec().len()
                ==> self.lanes_spec()[lane].is_none()
    }

    /// Returns the retained explicit selection.
    #[must_use]
    pub const fn selection(&self) -> (selection: Qwen3PlanSelection)
        ensures selection == self.selection_spec(),
    {
        self.candidate.selection()
    }

    /// Returns the retained structural lane roster.
    #[must_use]
    pub fn lanes(&self) -> (lanes: &[Option<StepPlan>])
        ensures lanes@ == self.lanes_spec(),
    {
        self.candidate.lanes()
    }

    /// Returns canonical selected bucket dimensions.
    #[must_use]
    pub const fn dimensions(&self) -> (dimensions: Qwen3PlanDimensions)
        ensures dimensions == self.dimensions_spec(),
    {
        self.dimensions
    }

    /// Returns the nonempty live-prefix length.
    #[must_use]
    pub const fn live_lane_count(&self) -> (lanes: u32)
        ensures lanes == self.live_lanes_spec(),
    {
        self.live_lanes
    }

    /// Returns validated fixed-shape token rows.
    #[must_use]
    pub fn token_ids(&self) -> (tokens: &[TokenId])
        ensures tokens@ == self.token_ids_spec(),
    {
        self.candidate.token_ids()
    }

    /// Returns validated fixed-shape position rows.
    #[must_use]
    pub fn position_ids(&self) -> (positions: &[u32])
        ensures positions@ == self.position_ids_spec(),
    {
        self.candidate.position_ids()
    }

    /// Returns validated per-lane active widths.
    #[must_use]
    pub fn active_lengths(&self) -> (lengths: &[u32])
        ensures lengths@ == self.active_lengths_spec(),
    {
        self.candidate.active_lengths()
    }

    /// Returns validated pre-step committed lengths.
    #[must_use]
    pub fn context_lengths(&self) -> (lengths: &[u32])
        ensures lengths@ == self.context_lengths_spec(),
    {
        self.candidate.context_lengths()
    }
}

/// Exact linear result of structural validation.
#[must_use]
#[derive(Debug, PartialEq, Eq)]
pub enum M1StepInputValidationOutcome {
    Validated(ValidatedM1StepInputs),
    Rejected(M1StepInputRejection),
}

impl M1StepInputValidationOutcome {
    pub closed spec fn is_validated_spec(&self) -> bool {
        matches!(self, Self::Validated(_))
    }
}

closed spec fn identity_present(identity: crate::Identity) -> bool {
    exists|index: int|
        0 <= index < identity.bytes_spec().len() && identity.bytes_spec()[index] != 0
}

closed spec fn live_plan_valid(
    candidate: &M1StepInputCandidate,
    dimensions: Qwen3PlanDimensions,
    live_lanes: int,
    lane: int,
) -> bool
    recommends
        0 < live_lanes <= candidate.lanes_spec().len(),
        0 <= lane < live_lanes,
        candidate.lanes_spec()[0].is_some(),
        candidate.lanes_spec()[lane].is_some(),
{
    let selection = candidate.selection_spec();
    let plan = candidate.lanes_spec()[lane].unwrap();
    let first = candidate.lanes_spec()[0].unwrap();
    let active = candidate.active_lengths_spec()[lane];
    let committed = candidate.context_lengths_spec()[lane];
    let width = dimensions.active_tokens as int;
    let row = lane * width;
    &&& plan.selection_spec() == selection
    &&& identity_present(plan.plan_id_spec())
    &&& plan.plan_id_spec() == first.plan_id_spec()
    &&& plan.completion_epoch_spec().value > 0
    &&& plan.completion_epoch_spec() == first.completion_epoch_spec()
    &&& plan.request_spec().slot_spec() < M1_MAX_ACTIVE_SEQUENCES
    &&& plan.request_spec().generation_spec() > 0
    &&& forall|prior: int|
        0 <= prior < lane ==> {
            let prior_plan = candidate.lanes_spec()[prior].unwrap();
            prior_plan.request_spec().slot_spec() != plan.request_spec().slot_spec()
        }
    &&& match selection.mode {
        Qwen3ExecutionMode::Prefill => 0 < active <= dimensions.active_tokens && committed == 0,
        Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative => {
            active == dimensions.active_tokens
        },
    }
    &&& committed as int + active as int <= dimensions.context_tokens as int
    &&& forall|active_index: int|
        0 <= active_index < active as int ==> {
            &&& candidate.token_ids_spec()[row + active_index] < QWEN3_VOCABULARY_SIZE
            &&& candidate.position_ids_spec()[row + active_index] as int
                == committed as int + active_index
        }
    &&& forall|active_index: int|
        active as int <= active_index < width ==> {
            &&& candidate.token_ids_spec()[row + active_index] == 0
            &&& candidate.position_ids_spec()[row + active_index] == 0
        }
}

closed spec fn inactive_lane_valid(
    candidate: &M1StepInputCandidate,
    dimensions: Qwen3PlanDimensions,
    lane: int,
) -> bool {
    let width = dimensions.active_tokens as int;
    let row = lane * width;
    &&& candidate.active_lengths_spec()[lane] == 0
    &&& candidate.context_lengths_spec()[lane] == 0
    &&& forall|active_index: int|
        0 <= active_index < width ==> {
            &&& candidate.token_ids_spec()[row + active_index] == 0
            &&& candidate.position_ids_spec()[row + active_index] == 0
        }
}

/// Exact mathematical validity of a structural multi-lane candidate.
pub closed spec fn m1_step_input_candidate_valid(candidate: &M1StepInputCandidate) -> bool {
    let selection = candidate.selection_spec();
    match selection.bucket.dimensions_spec(selection.role, selection.mode) {
        None => false,
        Some(dimensions) => {
            let sequences = dimensions.sequences as int;
            let width = dimensions.active_tokens as int;
            &&& candidate.lanes_spec().len() == sequences
            &&& candidate.token_ids_spec().len() == sequences * width
            &&& candidate.position_ids_spec().len() == sequences * width
            &&& candidate.active_lengths_spec().len() == sequences
            &&& candidate.context_lengths_spec().len() == sequences
            &&& exists|live_lanes: int|
                0 < live_lanes <= sequences
                && candidate.lanes_spec()[0].is_some()
                && forall|lane: int|
                    0 <= lane < live_lanes ==> {
                        &&& candidate.lanes_spec()[lane].is_some()
                        &&& live_plan_valid(candidate, dimensions, live_lanes, lane)
                    }
                && forall|lane: int|
                    live_lanes <= lane < sequences ==> {
                        &&& candidate.lanes_spec()[lane].is_none()
                        &&& inactive_lane_valid(candidate, dimensions, lane)
                    }
        },
    }
}

closed spec fn selected_dimensions(
    candidate: &M1StepInputCandidate,
    dimensions: Qwen3PlanDimensions,
) -> bool {
    let selection = candidate.selection_spec();
    selection.bucket.dimensions_spec(selection.role, selection.mode) == Some(dimensions)
}

closed spec fn exact_flat_location(
    candidate: &M1StepInputCandidate,
    dimensions: Qwen3PlanDimensions,
    lane: usize,
    active_index: usize,
) -> bool {
    &&& lane < dimensions.sequences
    &&& active_index < dimensions.active_tokens
    &&& (lane as int * dimensions.active_tokens as int + active_index as int)
        < candidate.token_ids_spec().len()
    &&& (lane as int * dimensions.active_tokens as int + active_index as int)
        < candidate.position_ids_spec().len()
}

/// Verifier-facing relation between a diagnostic and an invalid candidate.
///
/// Exact scalar values remain part of the diagnostic. The relation is
/// intentionally structural and does not authenticate any public plan value.
pub closed spec fn m1_step_input_error_matches(
    error: M1StepInputError,
    candidate: &M1StepInputCandidate,
) -> bool {
    !m1_step_input_candidate_valid(candidate)
        && match error {
            M1StepInputError::InvalidSelection => {
                let selection = candidate.selection_spec();
                selection.bucket.dimensions_spec(selection.role, selection.mode).is_none()
            },
            M1StepInputError::LaneRosterCount { expected, actual } => {
                candidate.lanes_spec().len() == actual
                    && exists|dimensions: Qwen3PlanDimensions|
                        candidate.selection_spec().bucket.dimensions_spec(
                            candidate.selection_spec().role,
                            candidate.selection_spec().mode,
                        ) == Some(dimensions)
                            && dimensions.sequences as int == expected as int
            },
            M1StepInputError::TokenCount { expected, actual } => {
                &&& candidate.token_ids_spec().len() == actual
                &&& exists|dimensions: Qwen3PlanDimensions|
                    selected_dimensions(candidate, dimensions)
                        && expected as int
                            == dimensions.sequences as int * dimensions.active_tokens as int
                        && expected != actual
            },
            M1StepInputError::PositionCount { expected, actual } => {
                &&& candidate.position_ids_spec().len() == actual
                &&& exists|dimensions: Qwen3PlanDimensions|
                    selected_dimensions(candidate, dimensions)
                        && expected as int
                            == dimensions.sequences as int * dimensions.active_tokens as int
                        && expected != actual
            },
            M1StepInputError::ActiveLengthCount { expected, actual } => {
                &&& candidate.active_lengths_spec().len() == actual
                &&& exists|dimensions: Qwen3PlanDimensions|
                    selected_dimensions(candidate, dimensions)
                        && expected == dimensions.sequences
                        && expected != actual
            },
            M1StepInputError::ContextLengthCount { expected, actual } => {
                &&& candidate.context_lengths_spec().len() == actual
                &&& exists|dimensions: Qwen3PlanDimensions|
                    selected_dimensions(candidate, dimensions)
                        && expected == dimensions.sequences
                        && expected != actual
            },
            M1StepInputError::DimensionOverflow => false,
            M1StepInputError::EmptyLivePrefix => {
                forall|lane: int|
                    0 <= lane < candidate.lanes_spec().len()
                        ==> candidate.lanes_spec()[lane].is_none()
            },
            M1StepInputError::LiveLaneAfterInactive { lane } => {
                lane < candidate.lanes_spec().len()
                    && candidate.lanes_spec()[lane as int].is_some()
                    && exists|prior: int|
                        0 <= prior < lane && candidate.lanes_spec()[prior].is_none()
            },
            M1StepInputError::PlanSelectionMismatch { lane } => {
                lane < candidate.lanes_spec().len()
                    && candidate.lanes_spec()[lane as int].is_some()
                    && candidate.lanes_spec()[lane as int].unwrap().selection_spec()
                        != candidate.selection_spec()
            },
            M1StepInputError::AbsentPlanIdentity { lane } => {
                lane < candidate.lanes_spec().len()
                    && candidate.lanes_spec()[lane as int].is_some()
                    && !identity_present(candidate.lanes_spec()[lane as int].unwrap().plan_id_spec())
            },
            M1StepInputError::PlanIdentityMismatch { lane } => {
                lane < candidate.lanes_spec().len()
                    && candidate.lanes_spec()[0].is_some()
                    && candidate.lanes_spec()[lane as int].is_some()
                    && candidate.lanes_spec()[lane as int].unwrap().plan_id_spec()
                        != candidate.lanes_spec()[0].unwrap().plan_id_spec()
            },
            M1StepInputError::ZeroCompletionEpoch { lane } => {
                lane < candidate.lanes_spec().len()
                    && candidate.lanes_spec()[lane as int].is_some()
                    && candidate.lanes_spec()[lane as int].unwrap().completion_epoch_spec().value == 0
            },
            M1StepInputError::CompletionEpochMismatch { lane } => {
                lane < candidate.lanes_spec().len()
                    && candidate.lanes_spec()[0].is_some()
                    && candidate.lanes_spec()[lane as int].is_some()
                    && candidate.lanes_spec()[lane as int].unwrap().completion_epoch_spec()
                        != candidate.lanes_spec()[0].unwrap().completion_epoch_spec()
            },
            M1StepInputError::RequestSlotOutOfRange { lane, slot } => {
                lane < candidate.lanes_spec().len()
                    && candidate.lanes_spec()[lane as int].is_some()
                    && candidate.lanes_spec()[lane as int].unwrap().request_spec().slot_spec() == slot
                    && slot >= M1_MAX_ACTIVE_SEQUENCES
            },
            M1StepInputError::ZeroRequestGeneration { lane } => {
                lane < candidate.lanes_spec().len()
                    && candidate.lanes_spec()[lane as int].is_some()
                    && candidate.lanes_spec()[lane as int].unwrap().request_spec().generation_spec() == 0
            },
            M1StepInputError::DuplicateRequestSlot { first_lane, lane, slot } => {
                first_lane < lane
                    && lane < candidate.lanes_spec().len()
                    && candidate.lanes_spec()[first_lane as int].is_some()
                    && candidate.lanes_spec()[lane as int].is_some()
                    && candidate.lanes_spec()[first_lane as int].unwrap().request_spec().slot_spec() == slot
                    && candidate.lanes_spec()[lane as int].unwrap().request_spec().slot_spec() == slot
            },
            M1StepInputError::PrefillActiveOutOfRange { lane, capacity, actual } => {
                &&& candidate.selection_spec().mode == Qwen3ExecutionMode::Prefill
                &&& lane < candidate.active_lengths_spec().len()
                &&& candidate.active_lengths_spec()[lane as int] == actual
                &&& exists|dimensions: Qwen3PlanDimensions|
                    selected_dimensions(candidate, dimensions)
                        && capacity == dimensions.active_tokens
                        && (actual == 0 || actual > capacity)
            },
            M1StepInputError::ActiveLengthMismatch { lane, expected, actual } => {
                &&& candidate.selection_spec().mode != Qwen3ExecutionMode::Prefill
                &&& lane < candidate.active_lengths_spec().len()
                &&& candidate.active_lengths_spec()[lane as int] == actual
                &&& exists|dimensions: Qwen3PlanDimensions|
                    selected_dimensions(candidate, dimensions)
                        && expected == dimensions.active_tokens
                        && actual != expected
            },
            M1StepInputError::PrefillCommittedContextNonZero { lane, actual } => {
                lane < candidate.context_lengths_spec().len()
                    && candidate.context_lengths_spec()[lane as int] == actual
                    && actual != 0
            },
            M1StepInputError::ContextLengthOverflow { lane } => {
                &&& lane < candidate.active_lengths_spec().len()
                &&& lane < candidate.context_lengths_spec().len()
                &&& candidate.context_lengths_spec()[lane as int] as int
                    + candidate.active_lengths_spec()[lane as int] as int > u32::MAX as int
            },
            M1StepInputError::ContextExceedsCapacity {
                lane, committed, active, capacity,
            } => {
                &&& lane < candidate.context_lengths_spec().len()
                &&& lane < candidate.active_lengths_spec().len()
                &&& candidate.context_lengths_spec()[lane as int] == committed
                &&& candidate.active_lengths_spec()[lane as int] == active
                &&& exists|dimensions: Qwen3PlanDimensions|
                    selected_dimensions(candidate, dimensions)
                        && capacity == dimensions.context_tokens
                        && committed as int + active as int > capacity as int
            },
            M1StepInputError::InactiveLengthPadding { lane, active, context } => {
                &&& lane < candidate.lanes_spec().len()
                &&& lane < candidate.active_lengths_spec().len()
                &&& lane < candidate.context_lengths_spec().len()
                &&& candidate.lanes_spec()[lane as int].is_none()
                &&& candidate.active_lengths_spec()[lane as int] == active
                &&& candidate.context_lengths_spec()[lane as int] == context
                &&& (active != 0 || context != 0)
            },
            M1StepInputError::TokenOutOfRange { lane, active_index, token } => {
                &&& lane < candidate.lanes_spec().len()
                &&& lane < candidate.active_lengths_spec().len()
                &&& candidate.lanes_spec()[lane as int].is_some()
                &&& active_index < candidate.active_lengths_spec()[lane as int]
                &&& token >= QWEN3_VOCABULARY_SIZE
                &&& exists|dimensions: Qwen3PlanDimensions|
                    selected_dimensions(candidate, dimensions)
                        && exact_flat_location(candidate, dimensions, lane, active_index)
                        && candidate.token_ids_spec()[
                            lane as int * dimensions.active_tokens as int + active_index as int
                        ] == token
            },
            M1StepInputError::PositionMismatch {
                lane, active_index, expected, actual,
            } => {
                &&& lane < candidate.lanes_spec().len()
                &&& lane < candidate.active_lengths_spec().len()
                &&& lane < candidate.context_lengths_spec().len()
                &&& candidate.lanes_spec()[lane as int].is_some()
                &&& active_index < candidate.active_lengths_spec()[lane as int]
                &&& expected as int
                    == candidate.context_lengths_spec()[lane as int] as int + active_index as int
                &&& actual != expected
                &&& exists|dimensions: Qwen3PlanDimensions|
                    selected_dimensions(candidate, dimensions)
                        && exact_flat_location(candidate, dimensions, lane, active_index)
                        && candidate.position_ids_spec()[
                            lane as int * dimensions.active_tokens as int + active_index as int
                        ] == actual
            },
            M1StepInputError::TokenPaddingNonZero { lane, active_index, actual } => {
                &&& lane < candidate.lanes_spec().len()
                &&& lane < candidate.active_lengths_spec().len()
                &&& actual != 0
                &&& (candidate.lanes_spec()[lane as int].is_none()
                    || active_index >= candidate.active_lengths_spec()[lane as int])
                &&& exists|dimensions: Qwen3PlanDimensions|
                    selected_dimensions(candidate, dimensions)
                        && exact_flat_location(candidate, dimensions, lane, active_index)
                        && candidate.token_ids_spec()[
                            lane as int * dimensions.active_tokens as int + active_index as int
                        ] == actual
            },
            M1StepInputError::PositionPaddingNonZero { lane, active_index, actual } => {
                &&& lane < candidate.lanes_spec().len()
                &&& lane < candidate.active_lengths_spec().len()
                &&& actual != 0
                &&& (candidate.lanes_spec()[lane as int].is_none()
                    || active_index >= candidate.active_lengths_spec()[lane as int])
                &&& exists|dimensions: Qwen3PlanDimensions|
                    selected_dimensions(candidate, dimensions)
                        && exact_flat_location(candidate, dimensions, lane, active_index)
                        && candidate.position_ids_spec()[
                            lane as int * dimensions.active_tokens as int + active_index as int
                        ] == actual
            },
        }
}

fn rejection(
    error: M1StepInputError,
    candidate: M1StepInputCandidate,
) -> (rejection: M1StepInputRejection)
    ensures
        rejection.error_spec() == error,
        rejection.candidate_selection_spec() == candidate.selection_spec(),
        rejection.candidate_lanes_spec() == candidate.lanes_spec(),
        rejection.candidate_token_ids_spec() == candidate.token_ids_spec(),
        rejection.candidate_position_ids_spec() == candidate.position_ids_spec(),
        rejection.candidate_active_lengths_spec() == candidate.active_lengths_spec(),
        rejection.candidate_context_lengths_spec() == candidate.context_lengths_spec(),
{
    M1StepInputRejection { error, candidate }
}

fn rejected(
    error: M1StepInputError,
    candidate: M1StepInputCandidate,
) -> M1StepInputValidationOutcome {
    M1StepInputValidationOutcome::Rejected(rejection(error, candidate))
}

fn validate_shape(
    candidate: &M1StepInputCandidate,
    dimensions: Qwen3PlanDimensions,
) -> Result<(usize, usize, usize), M1StepInputError> {
    let sequences = usize::try_from(dimensions.sequences)
        .map_err(|_error| M1StepInputError::DimensionOverflow)?;
    let width = usize::try_from(dimensions.active_tokens)
        .map_err(|_error| M1StepInputError::DimensionOverflow)?;
    let flattened = sequences
        .checked_mul(width)
        .ok_or(M1StepInputError::DimensionOverflow)?;
    if candidate.lanes().len() != sequences {
        return Err(M1StepInputError::LaneRosterCount {
            expected: sequences,
            actual: candidate.lanes().len(),
        });
    }
    if candidate.token_ids().len() != flattened {
        return Err(M1StepInputError::TokenCount {
            expected: flattened,
            actual: candidate.token_ids().len(),
        });
    }
    if candidate.position_ids().len() != flattened {
        return Err(M1StepInputError::PositionCount {
            expected: flattened,
            actual: candidate.position_ids().len(),
        });
    }
    if candidate.active_lengths().len() != sequences {
        return Err(M1StepInputError::ActiveLengthCount {
            expected: sequences,
            actual: candidate.active_lengths().len(),
        });
    }
    if candidate.context_lengths().len() != sequences {
        return Err(M1StepInputError::ContextLengthCount {
            expected: sequences,
            actual: candidate.context_lengths().len(),
        });
    }
    Ok((sequences, width, flattened))
}

fn live_prefix(candidate: &M1StepInputCandidate) -> Result<usize, M1StepInputError> {
    let mut live_lanes = 0usize;
    let mut inactive_seen = false;
    for (lane, plan) in candidate.lanes().iter().enumerate() {
        match plan {
            Some(_) if inactive_seen => {
                return Err(M1StepInputError::LiveLaneAfterInactive { lane });
            }
            Some(_) => live_lanes += 1,
            None => inactive_seen = true,
        }
    }
    if live_lanes == 0 {
        return Err(M1StepInputError::EmptyLivePrefix);
    }
    Ok(live_lanes)
}

fn validate_live_plans(
    candidate: &M1StepInputCandidate,
    live_lanes: usize,
) -> Result<(), M1StepInputError> {
    let selection = candidate.selection();
    let first = candidate.lanes()[0].expect("nonempty live prefix has lane zero");
    let first_identity = *first.plan_id();
    let first_epoch = first.completion_epoch();
    for lane in 0..live_lanes {
        let plan = candidate.lanes()[lane].expect("live prefix contains plans");
        if !plan.selection().matches(selection) {
            return Err(M1StepInputError::PlanSelectionMismatch { lane });
        }
        if !plan.plan_id().is_present() {
            return Err(M1StepInputError::AbsentPlanIdentity { lane });
        }
        if !plan.plan_id().equals(&first_identity) {
            return Err(M1StepInputError::PlanIdentityMismatch { lane });
        }
        if plan.completion_epoch().value() == 0 {
            return Err(M1StepInputError::ZeroCompletionEpoch { lane });
        }
        if plan.completion_epoch() != first_epoch {
            return Err(M1StepInputError::CompletionEpochMismatch { lane });
        }
        let request = plan.request();
        if request.slot() >= M1_MAX_ACTIVE_SEQUENCES {
            return Err(M1StepInputError::RequestSlotOutOfRange {
                lane,
                slot: request.slot(),
            });
        }
        if request.generation() == 0 {
            return Err(M1StepInputError::ZeroRequestGeneration { lane });
        }
        for first_lane in 0..lane {
            let prior = candidate.lanes()[first_lane].expect("prior live lane");
            if prior.request().slot() == request.slot() {
                return Err(M1StepInputError::DuplicateRequestSlot {
                    first_lane,
                    lane,
                    slot: request.slot(),
                });
            }
        }
    }
    Ok(())
}

fn validate_rows(
    candidate: &M1StepInputCandidate,
    dimensions: Qwen3PlanDimensions,
    live_lanes: usize,
    sequences: usize,
    width: usize,
) -> Result<(), M1StepInputError> {
    let selection = candidate.selection();
    for lane in 0..sequences {
        let active = candidate.active_lengths()[lane];
        let committed = candidate.context_lengths()[lane];
        let live = lane < live_lanes;
        if live {
            match selection.mode {
                Qwen3ExecutionMode::Prefill => {
                    if active == 0 || active > dimensions.active_tokens {
                        return Err(M1StepInputError::PrefillActiveOutOfRange {
                            lane,
                            capacity: dimensions.active_tokens,
                            actual: active,
                        });
                    }
                    if committed != 0 {
                        return Err(M1StepInputError::PrefillCommittedContextNonZero {
                            lane,
                            actual: committed,
                        });
                    }
                }
                Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative => {
                    if active != dimensions.active_tokens {
                        return Err(M1StepInputError::ActiveLengthMismatch {
                            lane,
                            expected: dimensions.active_tokens,
                            actual: active,
                        });
                    }
                }
            }
            let end = committed
                .checked_add(active)
                .ok_or(M1StepInputError::ContextLengthOverflow { lane })?;
            if end > dimensions.context_tokens {
                return Err(M1StepInputError::ContextExceedsCapacity {
                    lane,
                    committed,
                    active,
                    capacity: dimensions.context_tokens,
                });
            }
        } else if active != 0 || committed != 0 {
            return Err(M1StepInputError::InactiveLengthPadding {
                lane,
                active,
                context: committed,
            });
        }

        let row = lane
            .checked_mul(width)
            .ok_or(M1StepInputError::DimensionOverflow)?;
        let active_usize = usize::try_from(active)
            .map_err(|_error| M1StepInputError::DimensionOverflow)?;
        for active_index in 0..width {
            let flat = row
                .checked_add(active_index)
                .ok_or(M1StepInputError::DimensionOverflow)?;
            if live && active_index < active_usize {
                let token = candidate.token_ids()[flat];
                if token >= QWEN3_VOCABULARY_SIZE {
                    return Err(M1StepInputError::TokenOutOfRange {
                        lane,
                        active_index,
                        token,
                    });
                }
                let offset = u32::try_from(active_index)
                    .map_err(|_error| M1StepInputError::DimensionOverflow)?;
                let expected = committed
                    .checked_add(offset)
                    .ok_or(M1StepInputError::ContextLengthOverflow { lane })?;
                let actual = candidate.position_ids()[flat];
                if actual != expected {
                    return Err(M1StepInputError::PositionMismatch {
                        lane,
                        active_index,
                        expected,
                        actual,
                    });
                }
            } else {
                let token = candidate.token_ids()[flat];
                if token != 0 {
                    return Err(M1StepInputError::TokenPaddingNonZero {
                        lane,
                        active_index,
                        actual: token,
                    });
                }
                let position = candidate.position_ids()[flat];
                if position != 0 {
                    return Err(M1StepInputError::PositionPaddingNonZero {
                        lane,
                        active_index,
                        actual: position,
                    });
                }
            }
        }
    }
    Ok(())
}

/// Consumes and structurally validates the lane roster and four patch arrays.
///
/// Mode/bucket validation precedes checked dimension arithmetic. Every
/// rejection retains the exact explicit selection, lane roster, plans, and
/// arrays for diagnosis or retry.
pub fn validate_m1_step_inputs(
    candidate: M1StepInputCandidate,
) -> (result: M1StepInputValidationOutcome)
    ensures
        result.is_validated_spec() == crate::m1_step_input_candidate_valid_spec(&candidate),
        match result {
            M1StepInputValidationOutcome::Validated(validated) => {
                &&& validated.valid()
                &&& validated.selection_spec() == candidate.selection_spec()
                &&& validated.lanes_spec() == candidate.lanes_spec()
                &&& validated.token_ids_spec() == candidate.token_ids_spec()
                &&& validated.position_ids_spec() == candidate.position_ids_spec()
                &&& validated.active_lengths_spec() == candidate.active_lengths_spec()
                &&& validated.context_lengths_spec() == candidate.context_lengths_spec()
            },
            M1StepInputValidationOutcome::Rejected(failure) => {
                &&& crate::m1_step_input_error_matches_spec(failure.error_spec(), &candidate)
                &&& failure.candidate_selection_spec() == candidate.selection_spec()
                &&& failure.candidate_lanes_spec() == candidate.lanes_spec()
                &&& failure.candidate_token_ids_spec() == candidate.token_ids_spec()
                &&& failure.candidate_position_ids_spec() == candidate.position_ids_spec()
                &&& failure.candidate_active_lengths_spec() == candidate.active_lengths_spec()
                &&& failure.candidate_context_lengths_spec() == candidate.context_lengths_spec()
            },
        },
{
    let selection = candidate.selection();
    if selection.validate().is_err() {
        return rejected(M1StepInputError::InvalidSelection, candidate);
    }
    let dimensions = match selection
        .bucket
        .dimensions(selection.role, selection.mode)
    {
        Some(dimensions) => dimensions,
        None => return rejected(M1StepInputError::InvalidSelection, candidate),
    };
    let (sequences, width, _) = match validate_shape(&candidate, dimensions) {
        Ok(shape) => shape,
        Err(error) => return rejected(error, candidate),
    };
    let live_lanes = match live_prefix(&candidate) {
        Ok(live_lanes) => live_lanes,
        Err(error) => return rejected(error, candidate),
    };
    if let Err(error) = validate_live_plans(&candidate, live_lanes) {
        return rejected(error, candidate);
    }
    if let Err(error) = validate_rows(&candidate, dimensions, live_lanes, sequences, width) {
        return rejected(error, candidate);
    }
    let live_lanes = match u32::try_from(live_lanes) {
        Ok(live_lanes) => live_lanes,
        Err(_) => return rejected(M1StepInputError::DimensionOverflow, candidate),
    };
    M1StepInputValidationOutcome::Validated(ValidatedM1StepInputs {
        candidate,
        dimensions,
        live_lanes,
    })
}

} // verus!

impl fmt::Display for M1StepInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 structural step input rejected: {self:?}")
    }
}

impl std::error::Error for M1StepInputError {}

#[cfg(test)]
mod tests {
    use super::{
        validate_m1_step_inputs, M1StepInputCandidate, M1StepInputError,
        M1StepInputValidationOutcome,
    };
    use crate::completion::CompletionEpoch;
    use crate::{
        Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection,
        RequestId, StepPlan, QWEN3_VOCABULARY_SIZE,
    };

    const VALID_SELECTIONS: [Qwen3PlanSelection; 22] = [
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        ),
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS8T128,
        ),
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T512,
        ),
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T2048,
        ),
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        ),
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        ),
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS32C8192,
        ),
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        ),
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS8K4C8192,
        ),
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K8C8192,
        ),
        selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K16C8192,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS8T128,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T512,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T2048,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS32C8192,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS8K4C8192,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K8C8192,
        ),
        selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K16C8192,
        ),
    ];

    const fn selection(
        role: Qwen3ModelRole,
        mode: Qwen3ExecutionMode,
        bucket: Qwen3PlanBucket,
    ) -> Qwen3PlanSelection {
        Qwen3PlanSelection { role, mode, bucket }
    }

    fn plan(
        selection: Qwen3PlanSelection,
        slot: u32,
        generation: u32,
        epoch: u64,
        identity_byte: u8,
    ) -> StepPlan {
        StepPlan::new(
            RequestId::new(slot, generation),
            CompletionEpoch::new(epoch),
            Identity::new([identity_byte; 32]),
            selection,
        )
    }

    fn candidate(
        selection: Qwen3PlanSelection,
        live_lanes: usize,
        active_lengths: Vec<u32>,
        committed_lengths: Vec<u32>,
    ) -> M1StepInputCandidate {
        let dimensions = selection
            .bucket
            .dimensions(selection.role, selection.mode)
            .expect("canonical selection");
        let sequences = usize::try_from(dimensions.sequences).expect("bounded sequences");
        let width = usize::try_from(dimensions.active_tokens).expect("bounded width");
        assert_eq!(active_lengths.len(), live_lanes);
        assert_eq!(committed_lengths.len(), live_lanes);
        assert!(live_lanes <= sequences);
        let mut lanes = Vec::with_capacity(sequences);
        let mut active = vec![0; sequences];
        let mut context = vec![0; sequences];
        let mut tokens = vec![0; sequences * width];
        let mut positions = vec![0; sequences * width];
        for lane in 0..live_lanes {
            lanes.push(Some(plan(
                selection,
                u32::try_from(lane).unwrap(),
                3,
                9,
                0x41,
            )));
            active[lane] = active_lengths[lane];
            context[lane] = committed_lengths[lane];
            for active_index in 0..usize::try_from(active[lane]).unwrap() {
                let flat = lane * width + active_index;
                tokens[flat] = u32::try_from(flat + 1).unwrap() % QWEN3_VOCABULARY_SIZE;
                positions[flat] = context[lane] + u32::try_from(active_index).unwrap();
            }
        }
        lanes.resize(sequences, None);
        M1StepInputCandidate::new(selection, lanes, tokens, positions, active, context)
    }

    fn canonical_candidate(
        selection: Qwen3PlanSelection,
        live_lanes: usize,
        maximum_committed: bool,
    ) -> M1StepInputCandidate {
        let dimensions = selection
            .bucket
            .dimensions(selection.role, selection.mode)
            .expect("canonical selection");
        let active = vec![dimensions.active_tokens; live_lanes];
        let committed = match selection.mode {
            Qwen3ExecutionMode::Prefill => vec![0; live_lanes],
            Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative => {
                let value = if maximum_committed {
                    dimensions.context_tokens - dimensions.active_tokens
                } else {
                    0
                };
                vec![value; live_lanes]
            }
        };
        candidate(selection, live_lanes, active, committed)
    }

    fn expect_validated(input: M1StepInputCandidate, expected_live: u32) {
        let expected = input.clone();
        let M1StepInputValidationOutcome::Validated(validated) = validate_m1_step_inputs(input)
        else {
            panic!("canonical structural candidate rejected");
        };
        assert_eq!(validated.selection(), expected.selection());
        assert_eq!(validated.lanes(), expected.lanes());
        assert_eq!(validated.token_ids(), expected.token_ids());
        assert_eq!(validated.position_ids(), expected.position_ids());
        assert_eq!(validated.active_lengths(), expected.active_lengths());
        assert_eq!(validated.context_lengths(), expected.context_lengths());
        assert_eq!(validated.live_lane_count(), expected_live);
    }

    fn expect_rejected(
        input: M1StepInputCandidate,
        expected_error: M1StepInputError,
    ) -> M1StepInputCandidate {
        let expected = input.clone();
        let M1StepInputValidationOutcome::Rejected(rejection) = validate_m1_step_inputs(input)
        else {
            panic!("hostile structural candidate accepted");
        };
        assert_eq!(rejection.error(), expected_error);
        assert_eq!(rejection.candidate(), &expected);
        let (error, recovered) = rejection.into_parts();
        assert_eq!(error, expected_error);
        assert_eq!(recovered, expected);
        recovered
    }

    #[test]
    fn all_twenty_two_selections_accept_full_capacity_and_context_boundaries() {
        for selection in VALID_SELECTIONS {
            let dimensions = selection
                .bucket
                .dimensions(selection.role, selection.mode)
                .expect("canonical selection");
            let lanes = usize::try_from(dimensions.sequences).expect("bounded sequences");
            expect_validated(
                canonical_candidate(selection, lanes, false),
                dimensions.sequences,
            );
            expect_validated(
                canonical_candidate(selection, lanes, true),
                dimensions.sequences,
            );
        }
    }

    #[test]
    fn partial_s8_s32_and_variable_prefill_widths_are_structurally_valid() {
        let prefill = VALID_SELECTIONS[1];
        expect_validated(candidate(prefill, 3, vec![1, 64, 128], vec![0, 0, 0]), 3);
        let decode = VALID_SELECTIONS[5];
        let mut mixed_generations = canonical_candidate(decode, 3, true);
        mixed_generations.lanes[1] = Some(plan(decode, 1, 7, 9, 0x41));
        expect_validated(mixed_generations, 3);
        expect_validated(canonical_candidate(VALID_SELECTIONS[6], 7, false), 7);
        expect_validated(canonical_candidate(VALID_SELECTIONS[8], 2, true), 2);
    }

    #[test]
    fn empty_live_roster_gap_and_roster_count_fail_with_exact_recovery() {
        let selection = VALID_SELECTIONS[5];
        let mut empty = canonical_candidate(selection, 1, false);
        empty.lanes.fill(None);
        empty.active_lengths.fill(0);
        expect_rejected(empty, M1StepInputError::EmptyLivePrefix);

        let mut gap = canonical_candidate(selection, 3, false);
        gap.lanes[1] = None;
        expect_rejected(gap, M1StepInputError::LiveLaneAfterInactive { lane: 2 });

        let mut short = canonical_candidate(selection, 1, false);
        short.lanes.pop();
        expect_rejected(
            short,
            M1StepInputError::LaneRosterCount {
                expected: 8,
                actual: 7,
            },
        );
    }

    #[test]
    fn every_fixed_shape_array_count_is_exact() {
        let canonical = canonical_candidate(VALID_SELECTIONS[5], 1, false);
        let mut changed = canonical.clone();
        changed.token_ids.pop();
        expect_rejected(
            changed,
            M1StepInputError::TokenCount {
                expected: 8,
                actual: 7,
            },
        );
        let mut changed = canonical.clone();
        changed.position_ids.push(0);
        expect_rejected(
            changed,
            M1StepInputError::PositionCount {
                expected: 8,
                actual: 9,
            },
        );
        let mut changed = canonical.clone();
        changed.active_lengths.pop();
        expect_rejected(
            changed,
            M1StepInputError::ActiveLengthCount {
                expected: 8,
                actual: 7,
            },
        );
        let mut changed = canonical;
        changed.context_lengths.push(0);
        expect_rejected(
            changed,
            M1StepInputError::ContextLengthCount {
                expected: 8,
                actual: 9,
            },
        );
    }

    #[test]
    fn per_lane_plan_selection_identity_epoch_and_request_drift_fail_closed() {
        let selection = VALID_SELECTIONS[5];
        let canonical = canonical_candidate(selection, 3, false);
        let mut changed = canonical.clone();
        changed.selection = VALID_SELECTIONS[16];
        expect_rejected(changed, M1StepInputError::PlanSelectionMismatch { lane: 0 });

        let mut changed = canonical.clone();
        changed.lanes[1] = Some(plan(VALID_SELECTIONS[16], 1, 3, 9, 0x41));
        expect_rejected(changed, M1StepInputError::PlanSelectionMismatch { lane: 1 });

        let mut changed = canonical.clone();
        changed.lanes[0] = Some(plan(selection, 0, 3, 9, 0));
        expect_rejected(changed, M1StepInputError::AbsentPlanIdentity { lane: 0 });

        let mut changed = canonical.clone();
        changed.lanes[1] = Some(plan(selection, 1, 3, 9, 0));
        expect_rejected(changed, M1StepInputError::AbsentPlanIdentity { lane: 1 });

        let mut changed = canonical.clone();
        changed.lanes[1] = Some(plan(selection, 1, 3, 9, 0x42));
        expect_rejected(changed, M1StepInputError::PlanIdentityMismatch { lane: 1 });

        let mut changed = canonical.clone();
        changed.lanes[1] = Some(plan(selection, 1, 3, 0, 0x41));
        expect_rejected(changed, M1StepInputError::ZeroCompletionEpoch { lane: 1 });

        let mut changed = canonical.clone();
        changed.lanes[0] = Some(plan(selection, 0, 3, 0, 0x41));
        expect_rejected(changed, M1StepInputError::ZeroCompletionEpoch { lane: 0 });

        let mut changed = canonical.clone();
        changed.lanes[1] = Some(plan(selection, 1, 3, 10, 0x41));
        expect_rejected(
            changed,
            M1StepInputError::CompletionEpochMismatch { lane: 1 },
        );

        let mut changed = canonical.clone();
        changed.lanes[1] = Some(plan(selection, 32, 3, 9, 0x41));
        expect_rejected(
            changed,
            M1StepInputError::RequestSlotOutOfRange { lane: 1, slot: 32 },
        );

        let mut changed = canonical.clone();
        changed.lanes[1] = Some(plan(selection, 1, 0, 9, 0x41));
        expect_rejected(changed, M1StepInputError::ZeroRequestGeneration { lane: 1 });

        let mut changed = canonical;
        changed.lanes[2] = Some(plan(selection, 1, 4, 9, 0x41));
        expect_rejected(
            changed,
            M1StepInputError::DuplicateRequestSlot {
                first_lane: 1,
                lane: 2,
                slot: 1,
            },
        );
    }

    #[test]
    fn live_context_active_token_position_and_padding_drift_fail_closed() {
        let prefill = VALID_SELECTIONS[0];
        let changed = candidate(prefill, 1, vec![0], vec![0]);
        expect_rejected(
            changed,
            M1StepInputError::PrefillActiveOutOfRange {
                lane: 0,
                capacity: 128,
                actual: 0,
            },
        );
        let mut changed = canonical_candidate(prefill, 1, false);
        changed.active_lengths[0] = 129;
        expect_rejected(
            changed,
            M1StepInputError::PrefillActiveOutOfRange {
                lane: 0,
                capacity: 128,
                actual: 129,
            },
        );
        let changed = candidate(prefill, 1, vec![1], vec![1]);
        expect_rejected(
            changed,
            M1StepInputError::PrefillCommittedContextNonZero { lane: 0, actual: 1 },
        );

        let decode = VALID_SELECTIONS[4];
        let mut changed = canonical_candidate(decode, 1, false);
        changed.active_lengths[0] = 0;
        expect_rejected(
            changed,
            M1StepInputError::ActiveLengthMismatch {
                lane: 0,
                expected: 1,
                actual: 0,
            },
        );

        let speculative = VALID_SELECTIONS[10];
        let dimensions = speculative
            .bucket
            .dimensions(speculative.role, speculative.mode)
            .expect("canonical speculative selection");
        let mut changed = canonical_candidate(speculative, 1, false);
        changed.active_lengths[0] -= 1;
        expect_rejected(
            changed,
            M1StepInputError::ActiveLengthMismatch {
                lane: 0,
                expected: dimensions.active_tokens,
                actual: dimensions.active_tokens - 1,
            },
        );

        let mut changed = canonical_candidate(speculative, 1, true);
        changed.context_lengths[0] += 1;
        expect_rejected(
            changed,
            M1StepInputError::ContextExceedsCapacity {
                lane: 0,
                committed: dimensions.context_tokens - dimensions.active_tokens + 1,
                active: dimensions.active_tokens,
                capacity: dimensions.context_tokens,
            },
        );

        let mut changed = canonical_candidate(speculative, 1, false);
        changed.token_ids[0] = QWEN3_VOCABULARY_SIZE;
        expect_rejected(
            changed,
            M1StepInputError::TokenOutOfRange {
                lane: 0,
                active_index: 0,
                token: QWEN3_VOCABULARY_SIZE,
            },
        );

        let mut changed = canonical_candidate(speculative, 1, false);
        changed.position_ids[1] += 1;
        expect_rejected(
            changed,
            M1StepInputError::PositionMismatch {
                lane: 0,
                active_index: 1,
                expected: 1,
                actual: 2,
            },
        );

        let mut changed = candidate(prefill, 1, vec![1], vec![0]);
        changed.token_ids[1] = 7;
        expect_rejected(
            changed,
            M1StepInputError::TokenPaddingNonZero {
                lane: 0,
                active_index: 1,
                actual: 7,
            },
        );

        let mut changed = candidate(prefill, 1, vec![1], vec![0]);
        changed.position_ids[1] = 9;
        expect_rejected(
            changed,
            M1StepInputError::PositionPaddingNonZero {
                lane: 0,
                active_index: 1,
                actual: 9,
            },
        );
    }

    #[test]
    fn inactive_lane_lengths_tokens_and_positions_are_zero_padded() {
        let selection = VALID_SELECTIONS[5];
        let mut changed = canonical_candidate(selection, 1, false);
        changed.active_lengths[1] = 1;
        expect_rejected(
            changed,
            M1StepInputError::InactiveLengthPadding {
                lane: 1,
                active: 1,
                context: 0,
            },
        );

        let mut changed = canonical_candidate(selection, 1, false);
        changed.context_lengths[1] = 1;
        expect_rejected(
            changed,
            M1StepInputError::InactiveLengthPadding {
                lane: 1,
                active: 0,
                context: 1,
            },
        );

        let mut changed = canonical_candidate(selection, 1, false);
        changed.token_ids[1] = 3;
        expect_rejected(
            changed,
            M1StepInputError::TokenPaddingNonZero {
                lane: 1,
                active_index: 0,
                actual: 3,
            },
        );

        let mut changed = canonical_candidate(selection, 1, false);
        changed.position_ids[1] = 4;
        expect_rejected(
            changed,
            M1StepInputError::PositionPaddingNonZero {
                lane: 1,
                active_index: 0,
                actual: 4,
            },
        );
    }

    #[test]
    fn invalid_mode_bucket_precedes_all_shape_arithmetic() {
        let invalid = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::PrefillS1T128,
        );
        let input = M1StepInputCandidate::new(invalid, vec![], vec![], vec![], vec![], vec![]);
        expect_rejected(input, M1StepInputError::InvalidSelection);
    }
}
