//! Exact logical validation for generated M1 runtime patch inputs.
//!
//! This module validates only the four logical U32 arrays declared by the
//! generated runner. It constructs no allocation, address, packet, kernel,
//! completion, inference-result, hardware, or performance authority.

use crate::{Qwen3ExecutionMode, Qwen3PlanDimensions, StepPlan, TokenId, QWEN3_VOCABULARY_SIZE};
use core::fmt;
use vstd::prelude::*;

verus! {

/// Untrusted owned values for the four generated M1 logical patch slots.
///
/// The flattened token and position arrays are sequence-major. The two length
/// arrays contain one scalar per selected sequence.
#[verifier::allow(autoderive_clone_without_spec)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct M1StepInputCandidate {
    plan: StepPlan,
    token_ids: Vec<TokenId>,
    position_ids: Vec<u32>,
    active_lengths: Vec<u32>,
    context_lengths: Vec<u32>,
}

impl M1StepInputCandidate {
    pub closed spec fn plan_spec(&self) -> StepPlan {
        self.plan
    }

    pub closed spec fn token_ids_spec(&self) -> Seq<TokenId> {
        self.token_ids@
    }

    pub closed spec fn position_ids_spec(&self) -> Seq<u32> {
        self.position_ids@
    }

    pub closed spec fn active_lengths_spec(&self) -> Seq<u32> {
        self.active_lengths@
    }

    pub closed spec fn context_lengths_spec(&self) -> Seq<u32> {
        self.context_lengths@
    }

    /// Constructs one owned candidate and consumes all four input vectors.
    #[must_use]
    pub fn new(
        plan: StepPlan,
        token_ids: Vec<TokenId>,
        position_ids: Vec<u32>,
        active_lengths: Vec<u32>,
        context_lengths: Vec<u32>,
    ) -> (candidate: Self)
        ensures
            candidate.plan_spec() == plan,
            candidate.token_ids_spec() == token_ids@,
            candidate.position_ids_spec() == position_ids@,
            candidate.active_lengths_spec() == active_lengths@,
            candidate.context_lengths_spec() == context_lengths@,
    {
        Self {
            plan,
            token_ids,
            position_ids,
            active_lengths,
            context_lengths,
        }
    }

    /// Returns the exact logical step plan bound to these values.
    #[must_use]
    pub const fn plan(&self) -> (plan: StepPlan)
        ensures plan == self.plan_spec(),
    {
        self.plan
    }

    /// Returns the candidate token-ID patch values.
    #[must_use]
    pub fn token_ids(&self) -> (tokens: &[TokenId])
        ensures tokens@ == self.token_ids_spec(),
    {
        &self.token_ids
    }

    /// Returns the candidate logical-position patch values.
    #[must_use]
    pub fn position_ids(&self) -> (positions: &[u32])
        ensures positions@ == self.position_ids_spec(),
    {
        &self.position_ids
    }

    /// Returns the candidate per-sequence active widths.
    #[must_use]
    pub fn active_lengths(&self) -> (lengths: &[u32])
        ensures lengths@ == self.active_lengths_spec(),
    {
        &self.active_lengths
    }

    /// Returns the candidate per-sequence committed-context lengths.
    #[must_use]
    pub fn context_lengths(&self) -> (lengths: &[u32])
        ensures lengths@ == self.context_lengths_spec(),
    {
        &self.context_lengths
    }

    /// Recovers the exact plan and four owned vectors for correction or retry.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (parts: (StepPlan, Vec<TokenId>, Vec<u32>, Vec<u32>, Vec<u32>))
        ensures
            parts.0 == self.plan_spec(),
            parts.1@ == self.token_ids_spec(),
            parts.2@ == self.position_ids_spec(),
            parts.3@ == self.active_lengths_spec(),
            parts.4@ == self.context_lengths_spec(),
    {
        (
            self.plan,
            self.token_ids,
            self.position_ids,
            self.active_lengths,
            self.context_lengths,
        )
    }
}

/// Stable fail-closed reason for rejecting one logical input candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum M1StepInputError {
    /// The step plan combines a mode with a bucket owned by another mode.
    InvalidSelection,
    /// Flattened token-ID count differs from `sequences * active_tokens`.
    TokenCount { expected: usize, actual: usize },
    /// Flattened position-ID count differs from `sequences * active_tokens`.
    PositionCount { expected: usize, actual: usize },
    /// Active-length count differs from the selected sequence count.
    ActiveLengthCount { expected: usize, actual: usize },
    /// Context-length count differs from the selected sequence count.
    ContextLengthCount { expected: usize, actual: usize },
    /// Checked dimension conversion or flattened-length multiplication failed.
    DimensionOverflow,
    /// A token is outside the canonical Qwen3 vocabulary.
    TokenOutOfRange { index: usize, token: TokenId },
    /// A per-sequence active length differs from the selected bucket width.
    ActiveLengthMismatch {
        sequence: usize,
        expected: u32,
        actual: u32,
    },
    /// A selected sequence has no committed logical context.
    ZeroContext { sequence: usize },
    /// A committed context is shorter than the selected active width.
    ContextBelowActive {
        sequence: usize,
        active: u32,
        actual: u32,
    },
    /// A committed context exceeds the selected bucket capacity.
    ContextExceedsCapacity {
        sequence: usize,
        capacity: u32,
        actual: u32,
    },
    /// Prefill context differs from the exact finite prefill bucket.
    PrefillContextMismatch {
        sequence: usize,
        expected: u32,
        actual: u32,
    },
    /// A logical position differs from its exact contiguous sequence position.
    PositionMismatch {
        sequence: usize,
        active_index: usize,
        expected: u32,
        actual: u32,
    },
}

/// Retry-safe rejection retaining the exact unchanged candidate.
#[derive(Debug, PartialEq, Eq)]
pub struct M1StepInputRejection {
    error: M1StepInputError,
    candidate: M1StepInputCandidate,
}

impl M1StepInputRejection {
    pub closed spec fn error_spec(&self) -> M1StepInputError {
        self.error
    }

    pub closed spec fn candidate_plan_spec(&self) -> StepPlan {
        self.candidate.plan_spec()
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

    /// Returns the stable diagnostic without consuming retained inputs.
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
            parts.1.plan_spec() == self.candidate_plan_spec(),
            parts.1.token_ids_spec() == self.candidate_token_ids_spec(),
            parts.1.position_ids_spec() == self.candidate_position_ids_spec(),
            parts.1.active_lengths_spec() == self.candidate_active_lengths_spec(),
            parts.1.context_lengths_spec() == self.candidate_context_lengths_spec(),
    {
        (self.error, self.candidate)
    }
}

/// Linear custody of one exact source-level validated input set.
///
/// This value is intentionally not `Clone`. It exposes only logical slices and
/// carries no allocation, address, packet, kernel, completion, hardware, or
/// performance authority.
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
}

impl ValidatedM1StepInputs {
    pub closed spec fn plan_spec(&self) -> StepPlan {
        self.candidate.plan_spec()
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

    pub closed spec fn dimensions_spec(&self) -> Qwen3PlanDimensions {
        self.dimensions
    }

    /// Exact logical relation carried by this private-constructor typestate.
    pub closed spec fn valid(&self) -> bool {
        let selection = self.plan_spec().selection_spec();
        &&& m1_step_input_candidate_valid(&self.candidate)
        &&& selection.bucket.dimensions_spec(selection.role, selection.mode)
            == Some(self.dimensions)
    }

    /// Returns the retained exact logical step plan.
    #[must_use]
    pub const fn plan(&self) -> (plan: StepPlan)
        ensures plan == self.plan_spec(),
    {
        self.candidate.plan()
    }

    /// Returns the canonical dimensions derived from the retained selection.
    #[must_use]
    pub const fn dimensions(&self) -> (dimensions: Qwen3PlanDimensions)
        ensures dimensions == self.dimensions_spec(),
    {
        self.dimensions
    }

    /// Returns the validated flattened token-ID patch values.
    #[must_use]
    pub fn token_ids(&self) -> (tokens: &[TokenId])
        ensures tokens@ == self.token_ids_spec(),
    {
        self.candidate.token_ids()
    }

    /// Returns the validated flattened logical-position patch values.
    #[must_use]
    pub fn position_ids(&self) -> (positions: &[u32])
        ensures positions@ == self.position_ids_spec(),
    {
        self.candidate.position_ids()
    }

    /// Returns the validated per-sequence active widths.
    #[must_use]
    pub fn active_lengths(&self) -> (lengths: &[u32])
        ensures lengths@ == self.active_lengths_spec(),
    {
        self.candidate.active_lengths()
    }

    /// Returns the validated per-sequence committed-context lengths.
    #[must_use]
    pub fn context_lengths(&self) -> (lengths: &[u32])
        ensures lengths@ == self.context_lengths_spec(),
    {
        self.candidate.context_lengths()
    }
}

/// Exact linear result of one logical patch-input validation attempt.
#[must_use]
#[derive(Debug, PartialEq, Eq)]
pub enum M1StepInputValidationOutcome {
    /// Every exact source-level patch-input obligation passed.
    Validated(ValidatedM1StepInputs),
    /// Validation failed and the exact unchanged candidate was retained.
    Rejected(M1StepInputRejection),
}

impl M1StepInputValidationOutcome {
    pub closed spec fn is_validated_spec(&self) -> bool {
        match self {
            Self::Validated(_) => true,
            Self::Rejected(_) => false,
        }
    }
}

/// Mathematical validity of all four logical generated patch inputs.
pub closed spec fn m1_step_input_candidate_valid(candidate: &M1StepInputCandidate) -> bool {
    let selection = candidate.plan_spec().selection_spec();
    match selection.bucket.dimensions_spec(selection.role, selection.mode) {
        None => false,
        Some(dimensions) => {
            let sequences = dimensions.sequences as int;
            let active = dimensions.active_tokens as int;
            &&& candidate.token_ids_spec().len() == sequences * active
            &&& candidate.position_ids_spec().len() == sequences * active
            &&& candidate.active_lengths_spec().len() == sequences
            &&& candidate.context_lengths_spec().len() == sequences
            &&& forall|index: int|
                0 <= index < candidate.token_ids_spec().len()
                    ==> candidate.token_ids_spec()[index] < QWEN3_VOCABULARY_SIZE
            &&& forall|sequence: int|
                0 <= sequence < sequences ==> {
                    let context = candidate.context_lengths_spec()[sequence];
                    &&& candidate.active_lengths_spec()[sequence] == dimensions.active_tokens
                    &&& context > 0
                    &&& context >= dimensions.active_tokens
                    &&& context <= dimensions.context_tokens
                    &&& (selection.mode == Qwen3ExecutionMode::Prefill
                        ==> context == dimensions.context_tokens)
                }
            &&& forall|sequence: int, active_index: int|
                0 <= sequence < sequences && 0 <= active_index < active ==> {
                    let context = candidate.context_lengths_spec()[sequence];
                    candidate.position_ids_spec()[sequence * active + active_index] as int
                        == context as int - active + active_index
                }
        },
    }
}

fn rejection(
    error: M1StepInputError,
    candidate: M1StepInputCandidate,
) -> (rejection: M1StepInputRejection)
    ensures
        rejection.error_spec() == error,
        rejection.candidate_plan_spec() == candidate.plan_spec(),
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
) -> (outcome: M1StepInputValidationOutcome)
    ensures
        !outcome.is_validated_spec(),
        match outcome {
            M1StepInputValidationOutcome::Rejected(failure) => {
                &&& failure.error_spec() == error
                &&& failure.candidate_plan_spec() == candidate.plan_spec()
                &&& failure.candidate_token_ids_spec() == candidate.token_ids_spec()
                &&& failure.candidate_position_ids_spec() == candidate.position_ids_spec()
                &&& failure.candidate_active_lengths_spec() == candidate.active_lengths_spec()
                &&& failure.candidate_context_lengths_spec() == candidate.context_lengths_spec()
            },
            M1StepInputValidationOutcome::Validated(_) => false,
        },
{
    M1StepInputValidationOutcome::Rejected(rejection(error, candidate))
}

/// Consumes and validates all four generated M1 logical patch arrays.
///
/// Selection validation runs before all dimension arithmetic. Flattened
/// lengths use checked scalar conversion and multiplication. For each sequence,
/// zero-based positions must be exactly `[context - active, context)`, so the
/// final active position is `context - 1`.
///
/// Every rejection retains the exact unchanged [`StepPlan`] and candidate
/// vectors for diagnosis or retry. Success grants only source-level logical
/// custody, not allocation, address, packet, kernel, completion, runtime,
/// hardware, performance, or qualification authority.
pub fn validate_m1_step_inputs(
    candidate: M1StepInputCandidate,
) -> M1StepInputValidationOutcome {
    let selection = candidate.plan().selection();
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

    let sequences = match usize::try_from(dimensions.sequences) {
        Ok(sequences) => sequences,
        Err(_) => return rejected(M1StepInputError::DimensionOverflow, candidate),
    };
    let active_tokens = match usize::try_from(dimensions.active_tokens) {
        Ok(active_tokens) => active_tokens,
        Err(_) => return rejected(M1StepInputError::DimensionOverflow, candidate),
    };
    let flattened_tokens = match sequences.checked_mul(active_tokens) {
        Some(flattened_tokens) => flattened_tokens,
        None => return rejected(M1StepInputError::DimensionOverflow, candidate),
    };

    if candidate.token_ids().len() != flattened_tokens {
        let actual = candidate.token_ids().len();
        return rejected(
            M1StepInputError::TokenCount {
                expected: flattened_tokens,
                actual,
            },
            candidate,
        );
    }
    if candidate.position_ids().len() != flattened_tokens {
        let actual = candidate.position_ids().len();
        return rejected(
            M1StepInputError::PositionCount {
                expected: flattened_tokens,
                actual,
            },
            candidate,
        );
    }
    if candidate.active_lengths().len() != sequences {
        let actual = candidate.active_lengths().len();
        return rejected(
            M1StepInputError::ActiveLengthCount {
                expected: sequences,
                actual,
            },
            candidate,
        );
    }
    if candidate.context_lengths().len() != sequences {
        let actual = candidate.context_lengths().len();
        return rejected(
            M1StepInputError::ContextLengthCount {
                expected: sequences,
                actual,
            },
            candidate,
        );
    }

    for (index, &token) in candidate.token_ids().iter().enumerate() {
        if token >= QWEN3_VOCABULARY_SIZE {
            return rejected(
                M1StepInputError::TokenOutOfRange { index, token },
                candidate,
            );
        }
    }

    for sequence in 0..sequences {
        let active_length = candidate.active_lengths()[sequence];
        if active_length != dimensions.active_tokens {
            return rejected(
                M1StepInputError::ActiveLengthMismatch {
                    sequence,
                    expected: dimensions.active_tokens,
                    actual: active_length,
                },
                candidate,
            );
        }
        let context = candidate.context_lengths()[sequence];
        if context == 0 {
            return rejected(M1StepInputError::ZeroContext { sequence }, candidate);
        }
        if matches!(selection.mode, Qwen3ExecutionMode::Prefill)
            && context != dimensions.context_tokens
        {
            return rejected(
                M1StepInputError::PrefillContextMismatch {
                    sequence,
                    expected: dimensions.context_tokens,
                    actual: context,
                },
                candidate,
            );
        }
        if context < dimensions.active_tokens {
            return rejected(
                M1StepInputError::ContextBelowActive {
                    sequence,
                    active: dimensions.active_tokens,
                    actual: context,
                },
                candidate,
            );
        }
        if context > dimensions.context_tokens {
            return rejected(
                M1StepInputError::ContextExceedsCapacity {
                    sequence,
                    capacity: dimensions.context_tokens,
                    actual: context,
                },
                candidate,
            );
        }
        let position_start = context - dimensions.active_tokens;
        let sequence_start = match sequence.checked_mul(active_tokens) {
            Some(sequence_start) => sequence_start,
            None => return rejected(M1StepInputError::DimensionOverflow, candidate),
        };
        for active_index in 0..active_tokens {
            let active_offset = match u32::try_from(active_index) {
                Ok(active_offset) => active_offset,
                Err(_) => return rejected(M1StepInputError::DimensionOverflow, candidate),
            };
            let expected = match position_start.checked_add(active_offset) {
                Some(expected) => expected,
                None => return rejected(M1StepInputError::DimensionOverflow, candidate),
            };
            let flat_index = match sequence_start.checked_add(active_index) {
                Some(flat_index) => flat_index,
                None => return rejected(M1StepInputError::DimensionOverflow, candidate),
            };
            let actual = candidate.position_ids()[flat_index];
            if actual != expected {
                return rejected(
                    M1StepInputError::PositionMismatch {
                        sequence,
                        active_index,
                        expected,
                        actual,
                    },
                    candidate,
                );
            }
        }
    }

    M1StepInputValidationOutcome::Validated(ValidatedM1StepInputs {
        candidate,
        dimensions,
    })
}

} // verus!

impl fmt::Display for M1StepInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 logical step input rejected: {self:?}")
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

    fn plan(selection: Qwen3PlanSelection) -> StepPlan {
        StepPlan::new(
            RequestId::new(7, 11),
            CompletionEpoch::new(13),
            Identity::new([0x51; 32]),
            selection,
        )
    }

    fn candidate(selection: Qwen3PlanSelection, contexts: Vec<u32>) -> M1StepInputCandidate {
        let dimensions = selection
            .bucket
            .dimensions(selection.role, selection.mode)
            .expect("test selection is canonical");
        let sequences = usize::try_from(dimensions.sequences).expect("bounded sequences");
        let active = usize::try_from(dimensions.active_tokens).expect("bounded active width");
        assert_eq!(contexts.len(), sequences);
        let total = sequences
            .checked_mul(active)
            .expect("bounded flattened input");
        let token_ids = (0..total)
            .map(|index| {
                u32::try_from(index).expect("bounded token fixture") % QWEN3_VOCABULARY_SIZE
            })
            .collect::<Vec<_>>();
        let mut position_ids = Vec::with_capacity(total);
        for context in &contexts {
            let start = *context - dimensions.active_tokens;
            for offset in 0..dimensions.active_tokens {
                position_ids.push(start + offset);
            }
        }
        M1StepInputCandidate::new(
            plan(selection),
            token_ids,
            position_ids,
            vec![dimensions.active_tokens; sequences],
            contexts,
        )
    }

    fn minimum_contexts(selection: Qwen3PlanSelection) -> Vec<u32> {
        let dimensions = selection
            .bucket
            .dimensions(selection.role, selection.mode)
            .expect("canonical selection");
        let context = match selection.mode {
            Qwen3ExecutionMode::Prefill => dimensions.context_tokens,
            Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative => {
                dimensions.active_tokens
            }
        };
        vec![context; usize::try_from(dimensions.sequences).expect("bounded sequences")]
    }

    fn maximum_contexts(selection: Qwen3PlanSelection) -> Vec<u32> {
        let dimensions = selection
            .bucket
            .dimensions(selection.role, selection.mode)
            .expect("canonical selection");
        vec![
            dimensions.context_tokens;
            usize::try_from(dimensions.sequences).expect("bounded sequences")
        ]
    }

    fn expect_validated(candidate: M1StepInputCandidate) {
        let expected = candidate.clone();
        let M1StepInputValidationOutcome::Validated(validated) = validate_m1_step_inputs(candidate)
        else {
            panic!("canonical candidate was rejected");
        };
        assert_eq!(validated.plan(), expected.plan());
        assert_eq!(validated.token_ids(), expected.token_ids());
        assert_eq!(validated.position_ids(), expected.position_ids());
        assert_eq!(validated.active_lengths(), expected.active_lengths());
        assert_eq!(validated.context_lengths(), expected.context_lengths());
    }

    fn expect_rejected(
        candidate: M1StepInputCandidate,
        expected_error: M1StepInputError,
    ) -> M1StepInputCandidate {
        let expected = candidate.clone();
        let M1StepInputValidationOutcome::Rejected(rejection) = validate_m1_step_inputs(candidate)
        else {
            panic!("hostile candidate was accepted");
        };
        assert_eq!(rejection.error(), expected_error);
        assert_eq!(rejection.candidate(), &expected);
        assert_eq!(rejection.candidate().plan(), expected.plan());
        let (actual_error, recovered) = rejection.into_parts();
        assert_eq!(actual_error, expected_error);
        assert_eq!(recovered, expected);
        recovered
    }

    #[test]
    fn all_twenty_two_selections_accept_exact_minimum_and_maximum_contexts() {
        for selection in VALID_SELECTIONS {
            expect_validated(candidate(selection, minimum_contexts(selection)));
            expect_validated(candidate(selection, maximum_contexts(selection)));
        }
    }

    #[test]
    fn multi_sequence_contexts_are_independent_and_positions_end_at_context_minus_one() {
        for selection in [
            VALID_SELECTIONS[5],
            VALID_SELECTIONS[8],
            VALID_SELECTIONS[16],
            VALID_SELECTIONS[19],
        ] {
            let dimensions = selection
                .bucket
                .dimensions(selection.role, selection.mode)
                .expect("canonical multi-sequence selection");
            let contexts = (0..dimensions.sequences)
                .map(|sequence| {
                    if sequence % 2 == 0 {
                        dimensions.active_tokens
                    } else {
                        dimensions.context_tokens
                    }
                })
                .collect::<Vec<_>>();
            let input = candidate(selection, contexts.clone());
            let active = usize::try_from(dimensions.active_tokens).expect("bounded active");
            for (sequence, context) in contexts.iter().enumerate() {
                let last = sequence * active + active - 1;
                assert_eq!(input.position_ids()[last], context - 1);
            }
            expect_validated(input);
        }
    }

    #[test]
    fn invalid_selection_is_rejected_before_candidate_lengths() {
        let invalid = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::PrefillS1T128,
        );
        let input = M1StepInputCandidate::new(plan(invalid), vec![], vec![], vec![], vec![]);
        expect_rejected(input, M1StepInputError::InvalidSelection);
    }

    #[test]
    fn every_patch_length_mutation_is_rejected_with_exact_recovery() {
        let selection = VALID_SELECTIONS[5];
        let dimensions = selection
            .bucket
            .dimensions(selection.role, selection.mode)
            .expect("canonical selection");
        let canonical = candidate(selection, minimum_contexts(selection));

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
        changed.context_lengths.push(dimensions.active_tokens);
        expect_rejected(
            changed,
            M1StepInputError::ContextLengthCount {
                expected: 8,
                actual: 9,
            },
        );
    }

    #[test]
    fn token_active_context_and_position_mutations_fail_closed() {
        let selection = VALID_SELECTIONS[10];
        let dimensions = selection
            .bucket
            .dimensions(selection.role, selection.mode)
            .expect("canonical selection");
        let canonical = candidate(selection, maximum_contexts(selection));

        let mut changed = canonical.clone();
        changed.token_ids[0] = QWEN3_VOCABULARY_SIZE;
        expect_rejected(
            changed,
            M1StepInputError::TokenOutOfRange {
                index: 0,
                token: QWEN3_VOCABULARY_SIZE,
            },
        );

        let mut changed = canonical.clone();
        changed.active_lengths[0] -= 1;
        expect_rejected(
            changed,
            M1StepInputError::ActiveLengthMismatch {
                sequence: 0,
                expected: dimensions.active_tokens,
                actual: dimensions.active_tokens - 1,
            },
        );

        let mut changed = canonical.clone();
        changed.context_lengths[0] = 0;
        expect_rejected(changed, M1StepInputError::ZeroContext { sequence: 0 });

        let mut changed = canonical.clone();
        changed.context_lengths[0] = dimensions.active_tokens - 1;
        expect_rejected(
            changed,
            M1StepInputError::ContextBelowActive {
                sequence: 0,
                active: dimensions.active_tokens,
                actual: dimensions.active_tokens - 1,
            },
        );

        let mut changed = canonical.clone();
        changed.context_lengths[0] = dimensions.context_tokens + 1;
        expect_rejected(
            changed,
            M1StepInputError::ContextExceedsCapacity {
                sequence: 0,
                capacity: dimensions.context_tokens,
                actual: dimensions.context_tokens + 1,
            },
        );

        for active_index in [
            0,
            usize::try_from(dimensions.active_tokens).unwrap() / 2,
            usize::try_from(dimensions.active_tokens).unwrap() - 1,
        ] {
            let mut changed = canonical.clone();
            let actual = changed.position_ids[active_index] + 1;
            changed.position_ids[active_index] = actual;
            expect_rejected(
                changed,
                M1StepInputError::PositionMismatch {
                    sequence: 0,
                    active_index,
                    expected: dimensions.context_tokens - dimensions.active_tokens
                        + u32::try_from(active_index).unwrap(),
                    actual,
                },
            );
        }
    }

    #[test]
    fn prefill_context_must_equal_the_exact_bucket() {
        let selection = VALID_SELECTIONS[2];
        let dimensions = selection
            .bucket
            .dimensions(selection.role, selection.mode)
            .expect("canonical prefill selection");
        let mut changed = candidate(selection, maximum_contexts(selection));
        changed.context_lengths[0] = dimensions.context_tokens - 1;
        expect_rejected(
            changed,
            M1StepInputError::PrefillContextMismatch {
                sequence: 0,
                expected: dimensions.context_tokens,
                actual: dimensions.context_tokens - 1,
            },
        );
    }

    #[test]
    fn speculative_role_swap_rederives_the_exact_active_width() {
        let target = VALID_SELECTIONS[7];
        let draft = VALID_SELECTIONS[18];
        let target_dimensions = target
            .bucket
            .dimensions(target.role, target.mode)
            .expect("canonical target selection");
        let mut changed = candidate(target, minimum_contexts(target));
        changed.plan = plan(draft);
        expect_rejected(
            changed,
            M1StepInputError::TokenCount {
                expected: usize::try_from(
                    draft
                        .bucket
                        .dimensions(draft.role, draft.mode)
                        .expect("canonical draft selection")
                        .active_tokens,
                )
                .expect("bounded draft active width"),
                actual: usize::try_from(target_dimensions.active_tokens)
                    .expect("bounded target active width"),
            },
        );
    }
}
