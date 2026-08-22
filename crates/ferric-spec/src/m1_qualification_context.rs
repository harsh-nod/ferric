//! Exact M1 qualification-only construction of an 8,192-token context.
//!
//! This module specifies input-token coverage, prompt-context commits, compact
//! choice disposition, and qualification observation policy for the closed
//! S1, S8, and S32 shapes. It does not authorize execution, weaken the current
//! decode guard, or refine M2 chunked prefill.

use crate::Identity;
use vstd::prelude::*;

verus! {

/// Version of the closed qualification context-plan contract.
pub const M1_QUALIFICATION_CONTEXT_PLAN_VERSION: u32 = 1;
/// Exact number of supplied input tokens in every live qualification lane.
pub const M1_QUALIFICATION_TOKENS_PER_LANE: u32 = 8_192;
/// Teacher-forced priming steps before the final observed step.
pub const M1_QUALIFICATION_PROMPT_CONTEXT_TOKENS: u32 = 8_191;
/// Supplied input-token index used by the sole published qualification step.
pub const M1_QUALIFICATION_FINAL_INPUT_TOKEN: u32 = 8_191;
/// Exact number of unit-token steps in every lane's context plan.
pub const M1_QUALIFICATION_CONTEXT_PLAN_STEPS: usize = 8_192;

/// The only admitted lane groupings for M1 context-length qualification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum M1QualificationLaneGrouping {
    S1,
    S8,
    S32,
}

impl M1QualificationLaneGrouping {
    pub closed spec fn sequences_spec(self) -> u32 {
        match self {
            Self::S1 => 1,
            Self::S8 => 8,
            Self::S32 => 32,
        }
    }

    pub closed spec fn plan_identity_bytes_spec(self) -> Seq<u8> {
        match self {
            Self::S1 => seq![
                0xda, 0x4c, 0xe6, 0x39, 0xcc, 0xf6, 0xcb, 0x56,
                0x9b, 0xdc, 0x77, 0x67, 0xd1, 0xfc, 0x10, 0x1d,
                0x6a, 0x03, 0xe3, 0x21, 0xcd, 0xf5, 0x89, 0x87,
                0xef, 0x35, 0x61, 0xc1, 0x93, 0xbf, 0x68, 0x9b,
            ],
            Self::S8 => seq![
                0xa5, 0xa1, 0xe3, 0xd6, 0xfc, 0xd7, 0xce, 0xea,
                0x02, 0x61, 0x15, 0xa9, 0x9d, 0x7f, 0xf5, 0x4a,
                0xf5, 0xe4, 0x90, 0x26, 0x3e, 0x59, 0x6e, 0x83,
                0x03, 0x52, 0xe3, 0x8e, 0x29, 0xd9, 0xe3, 0xe0,
            ],
            Self::S32 => seq![
                0x7b, 0x43, 0x08, 0xbe, 0xb3, 0x88, 0x77, 0x43,
                0x9b, 0xc6, 0xa1, 0xdc, 0x43, 0xf7, 0x38, 0x53,
                0x24, 0x52, 0xf6, 0x81, 0x76, 0xe6, 0xe1, 0xde,
                0x91, 0x22, 0x00, 0x71, 0x01, 0xc1, 0xd8, 0x91,
            ],
        }
    }

    #[must_use]
    pub const fn sequences(self) -> (sequences: u32)
        ensures sequences == self.sequences_spec(),
    {
        match self {
            Self::S1 => 1,
            Self::S8 => 8,
            Self::S32 => 32,
        }
    }

    fn matches(self, other: Self) -> (matches: bool)
        ensures matches == (self == other),
    {
        matches!(
            (self, other),
            (Self::S1, Self::S1) | (Self::S8, Self::S8) | (Self::S32, Self::S32)
        )
    }
}

/// Returns the fixed reviewed identity for one grouping's exact v1 plan.
///
/// Each identity is a SHA-256 digest of the versioned domain, grouping, and
/// canonical unit-step policy. Constants make repeated construction stable.
#[must_use]
pub const fn m1_qualification_context_plan_identity(
    grouping: M1QualificationLaneGrouping,
) -> (identity: Identity)
    ensures identity.bytes_spec() == grouping.plan_identity_bytes_spec(),
{
    match grouping {
        M1QualificationLaneGrouping::S1 => Identity::new([
            0xda, 0x4c, 0xe6, 0x39, 0xcc, 0xf6, 0xcb, 0x56,
            0x9b, 0xdc, 0x77, 0x67, 0xd1, 0xfc, 0x10, 0x1d,
            0x6a, 0x03, 0xe3, 0x21, 0xcd, 0xf5, 0x89, 0x87,
            0xef, 0x35, 0x61, 0xc1, 0x93, 0xbf, 0x68, 0x9b,
        ]),
        M1QualificationLaneGrouping::S8 => Identity::new([
            0xa5, 0xa1, 0xe3, 0xd6, 0xfc, 0xd7, 0xce, 0xea,
            0x02, 0x61, 0x15, 0xa9, 0x9d, 0x7f, 0xf5, 0x4a,
            0xf5, 0xe4, 0x90, 0x26, 0x3e, 0x59, 0x6e, 0x83,
            0x03, 0x52, 0xe3, 0x8e, 0x29, 0xd9, 0xe3, 0xe0,
        ]),
        M1QualificationLaneGrouping::S32 => Identity::new([
            0x7b, 0x43, 0x08, 0xbe, 0xb3, 0x88, 0x77, 0x43,
            0x9b, 0xc6, 0xa1, 0xdc, 0x43, 0xf7, 0x38, 0x53,
            0x24, 0x52, 0xf6, 0x81, 0x76, 0xe6, 0xe1, 0xde,
            0x91, 0x22, 0x00, 0x71, 0x01, 0xc1, 0xd8, 0x91,
        ]),
    }
}

/// Half-open supplied-input or committed-prompt token interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct M1QualificationTokenRange {
    pub start: u32,
    pub end: u32,
}

impl M1QualificationTokenRange {
    fn matches(self, other: Self) -> (matches: bool)
        ensures matches == (self == other),
    {
        self.start == other.start && self.end == other.end
    }
}

/// Semantic role of one unit-token qualification context step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum M1QualificationContextStepKind {
    /// Commit one supplied prompt token without publishing a model output.
    TeacherForcedPromptContext,
    /// Commit the final supplied prompt token and publish its model output.
    FinalObserved,
}

impl M1QualificationContextStepKind {
    fn matches(self, other: Self) -> (matches: bool)
        ensures matches == (self == other),
    {
        matches!(
            (self, other),
            (Self::TeacherForcedPromptContext, Self::TeacherForcedPromptContext)
                | (Self::FinalObserved, Self::FinalObserved)
        )
    }
}

/// Disposition of the model compact choice computed by one step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum M1QualificationCompactChoiceDisposition {
    /// The choice is observed by the step protocol but not externally emitted.
    ObservedButSuppressed,
    /// The choice is observed and published as the qualification output.
    ObservedAndPublished,
}

impl M1QualificationCompactChoiceDisposition {
    fn matches(self, other: Self) -> (matches: bool)
        ensures matches == (self == other),
    {
        matches!(
            (self, other),
            (Self::ObservedButSuppressed, Self::ObservedButSuppressed)
                | (Self::ObservedAndPublished, Self::ObservedAndPublished)
        )
    }
}

/// Source policy for the step following a compact choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum M1QualificationNextInputPolicy {
    /// The next input is independently teacher-forced; it need not equal the
    /// compact choice produced by this step.
    IndependentTeacherForcedPrompt,
    /// Hostile substitution that feeds the compact choice back as the next
    /// prompt. This policy is never admitted by an M1 qualification plan.
    CompactChoiceFeedback,
    /// The terminal step has no subsequent supplied prompt token.
    Terminal,
}

impl M1QualificationNextInputPolicy {
    fn matches(self, other: Self) -> (matches: bool)
        ensures matches == (self == other),
    {
        matches!(
            (self, other),
            (Self::IndependentTeacherForcedPrompt, Self::IndependentTeacherForcedPrompt)
                | (Self::CompactChoiceFeedback, Self::CompactChoiceFeedback)
                | (Self::Terminal, Self::Terminal)
        )
    }
}

/// One exact unit-token step in qualification-only context construction.
///
/// `prompt_context_commits` and `externally_emitted_output_count` are distinct
/// fields. Every step commits one supplied prompt token. Priming steps also
/// observe a model compact choice, but suppress it and independently select the
/// next teacher-forced prompt token; no choice-to-prompt equality is required.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct M1QualificationContextStep {
    pub kind: M1QualificationContextStepKind,
    pub input_tokens: M1QualificationTokenRange,
    pub prompt_context_commits: M1QualificationTokenRange,
    pub compact_choice: M1QualificationCompactChoiceDisposition,
    pub next_input: M1QualificationNextInputPolicy,
    pub externally_emitted_output_count: u32,
    pub capture_qualification_output: bool,
}

impl M1QualificationContextStep {
    pub open spec fn valid_at(self, ordinal: u32) -> bool {
        if ordinal < M1_QUALIFICATION_PROMPT_CONTEXT_TOKENS {
            self.kind == M1QualificationContextStepKind::TeacherForcedPromptContext
                && self.input_tokens.start == ordinal
                && self.input_tokens.end == ordinal + 1
                && self.prompt_context_commits == self.input_tokens
                && self.compact_choice
                    == M1QualificationCompactChoiceDisposition::ObservedButSuppressed
                && self.next_input
                    == M1QualificationNextInputPolicy::IndependentTeacherForcedPrompt
                && self.externally_emitted_output_count == 0
                && !self.capture_qualification_output
        } else if ordinal == M1_QUALIFICATION_FINAL_INPUT_TOKEN {
            self.kind == M1QualificationContextStepKind::FinalObserved
                && self.input_tokens.start == M1_QUALIFICATION_FINAL_INPUT_TOKEN
                && self.input_tokens.end == M1_QUALIFICATION_TOKENS_PER_LANE
                && self.prompt_context_commits == self.input_tokens
                && self.compact_choice
                    == M1QualificationCompactChoiceDisposition::ObservedAndPublished
                && self.next_input == M1QualificationNextInputPolicy::Terminal
                && self.externally_emitted_output_count == 1
                && self.capture_qualification_output
        } else {
            false
        }
    }

    #[allow(clippy::bool_to_int_with_if)]
    fn validate_at(self, ordinal: u32) -> (result: Result<(), M1QualificationContextPlanError>)
        ensures result.is_ok() == self.valid_at(ordinal),
    {
        if ordinal >= M1_QUALIFICATION_TOKENS_PER_LANE {
            return Err(M1QualificationContextPlanError::StepCount {
                expected_steps: M1_QUALIFICATION_CONTEXT_PLAN_STEPS,
                actual_steps: M1_QUALIFICATION_CONTEXT_PLAN_STEPS + 1,
            });
        }
        let priming = ordinal < M1_QUALIFICATION_PROMPT_CONTEXT_TOKENS;
        let expected_kind = if priming {
            M1QualificationContextStepKind::TeacherForcedPromptContext
        } else {
            M1QualificationContextStepKind::FinalObserved
        };
        let expected_compact_choice = if priming {
            M1QualificationCompactChoiceDisposition::ObservedButSuppressed
        } else {
            M1QualificationCompactChoiceDisposition::ObservedAndPublished
        };
        let expected_next_input = if priming {
            M1QualificationNextInputPolicy::IndependentTeacherForcedPrompt
        } else {
            M1QualificationNextInputPolicy::Terminal
        };
        let expected_outputs = if priming { 0 } else { 1 };
        let expected_capture = !priming;
        let expected_end = ordinal + 1;

        if !self.kind.matches(expected_kind) {
            return Err(M1QualificationContextPlanError::StepKind { ordinal });
        }
        if self.input_tokens.start != ordinal {
            return Err(M1QualificationContextPlanError::TokenCoverageStart {
                ordinal,
                expected: ordinal,
                actual: self.input_tokens.start,
            });
        }
        if self.input_tokens.end != expected_end {
            return Err(M1QualificationContextPlanError::TokenCoverageEnd {
                ordinal,
                expected: expected_end,
                actual: self.input_tokens.end,
            });
        }
        if !self.prompt_context_commits.matches(self.input_tokens) {
            return Err(M1QualificationContextPlanError::PromptCommitMismatch { ordinal });
        }
        if !self.compact_choice.matches(expected_compact_choice) {
            return Err(M1QualificationContextPlanError::CompactChoiceDisposition { ordinal });
        }
        if !self.next_input.matches(expected_next_input) {
            return Err(M1QualificationContextPlanError::NextInputPolicy { ordinal });
        }
        if self.externally_emitted_output_count != expected_outputs {
            return Err(M1QualificationContextPlanError::ExternallyEmittedOutputCount { ordinal });
        }
        if self.capture_qualification_output != expected_capture {
            return Err(M1QualificationContextPlanError::CapturePolicy { ordinal });
        }
        Ok(())
    }
}

/// Closed M1 context-construction plan for one exact lane grouping.
#[verifier::allow(autoderive_clone_without_spec)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct M1QualificationContextPlan {
    pub version: u32,
    pub plan_id: Identity,
    pub grouping: M1QualificationLaneGrouping,
    pub tokens_per_lane: u32,
    pub steps: Vec<M1QualificationContextStep>,
}

impl M1QualificationContextPlan {
    /// Every supplied input and prompt commit is the exact unit interval at its
    /// ordinal. This excludes gaps, overlap, leading input, and trailing input.
    pub open spec fn has_exact_token_coverage(&self) -> bool {
        self.steps@.len() == M1_QUALIFICATION_CONTEXT_PLAN_STEPS as nat
            && forall|ordinal: int| 0 <= ordinal < self.steps@.len() ==> {
                &&& self.steps@[ordinal].input_tokens.start as int == ordinal
                &&& self.steps@[ordinal].input_tokens.end as int == ordinal + 1
                &&& self.steps@[ordinal].prompt_context_commits
                    == self.steps@[ordinal].input_tokens
            }
    }

    pub open spec fn every_step_is_valid(&self) -> bool {
        forall|ordinal: int| 0 <= ordinal < self.steps@.len()
            ==> self.steps@[ordinal].valid_at(ordinal as u32)
    }

    /// Mathematical acceptance relation for the exact expected grouping.
    pub open spec fn valid_for(&self, expected_grouping: M1QualificationLaneGrouping) -> bool {
        self.version == M1_QUALIFICATION_CONTEXT_PLAN_VERSION
            && self.grouping == expected_grouping
            && self.plan_id.bytes_spec() == expected_grouping.plan_identity_bytes_spec()
            && self.tokens_per_lane == M1_QUALIFICATION_TOKENS_PER_LANE
            && self.steps@.len() == M1_QUALIFICATION_CONTEXT_PLAN_STEPS as nat
            && self.every_step_is_valid()
            && self.has_exact_token_coverage()
    }

    proof fn exact_coverage_from_valid_steps(&self)
        requires
            self.steps@.len() == M1_QUALIFICATION_CONTEXT_PLAN_STEPS as nat,
            self.every_step_is_valid(),
        ensures self.has_exact_token_coverage(),
    {
        reveal(M1QualificationContextPlan::every_step_is_valid);
        reveal(M1QualificationContextPlan::has_exact_token_coverage);
        assert forall|ordinal: int| 0 <= ordinal < self.steps@.len() implies {
            &&& self.steps@[ordinal].input_tokens.start as int == ordinal
            &&& self.steps@[ordinal].input_tokens.end as int == ordinal + 1
            &&& self.steps@[ordinal].prompt_context_commits
                == self.steps@[ordinal].input_tokens
        } by {
            assert(self.steps@[ordinal].valid_at(ordinal as u32));
            reveal(M1QualificationContextStep::valid_at);
        }
    }

    /// Validates exact unit-token coverage, prompt commits, compact-choice
    /// disposition, capture policy, stable identity, and lane grouping.
    ///
    /// # Errors
    ///
    /// Fails closed on any header, grouping, identity, coverage, commit,
    /// choice, next-input, external-emission, or capture-policy substitution.
    pub fn validate(
        &self,
        expected_grouping: M1QualificationLaneGrouping,
    ) -> (result: Result<(), M1QualificationContextPlanError>)
        ensures result.is_ok() == self.valid_for(expected_grouping),
    {
        if self.version != M1_QUALIFICATION_CONTEXT_PLAN_VERSION {
            return Err(M1QualificationContextPlanError::UnsupportedVersion);
        }
        if !self.grouping.matches(expected_grouping) {
            return Err(M1QualificationContextPlanError::GroupingMismatch);
        }
        let expected_identity = m1_qualification_context_plan_identity(expected_grouping);
        if !self.plan_id.equals(&expected_identity) {
            return Err(M1QualificationContextPlanError::PlanIdentityMismatch);
        }
        if self.tokens_per_lane != M1_QUALIFICATION_TOKENS_PER_LANE {
            return Err(M1QualificationContextPlanError::TokensPerLane {
                expected_tokens: M1_QUALIFICATION_TOKENS_PER_LANE,
                actual_tokens: self.tokens_per_lane,
            });
        }
        if self.steps.len() != M1_QUALIFICATION_CONTEXT_PLAN_STEPS {
            return Err(M1QualificationContextPlanError::StepCount {
                expected_steps: M1_QUALIFICATION_CONTEXT_PLAN_STEPS,
                actual_steps: self.steps.len(),
            });
        }

        let mut ordinal = 0u32;
        while ordinal < M1_QUALIFICATION_TOKENS_PER_LANE
            invariant
                self.version == M1_QUALIFICATION_CONTEXT_PLAN_VERSION,
                self.grouping == expected_grouping,
                self.plan_id.bytes_spec() == expected_grouping.plan_identity_bytes_spec(),
                self.tokens_per_lane == M1_QUALIFICATION_TOKENS_PER_LANE,
                self.steps@.len() == M1_QUALIFICATION_CONTEXT_PLAN_STEPS as nat,
                0 <= ordinal <= M1_QUALIFICATION_TOKENS_PER_LANE,
                forall|prior: int| 0 <= prior < ordinal
                    ==> self.steps@[prior].valid_at(prior as u32),
            decreases M1_QUALIFICATION_TOKENS_PER_LANE - ordinal,
        {
            self.steps[ordinal as usize].validate_at(ordinal)?;
            ordinal += 1;
        }
        assert(self.every_step_is_valid()) by {
            reveal(M1QualificationContextPlan::every_step_is_valid);
        }
        proof {
            self.exact_coverage_from_valid_steps();
        }
        Ok(())
    }

    /// Exposes the complete qualification context contract after validation.
    pub proof fn expose_exact_context(
        &self,
        expected_grouping: M1QualificationLaneGrouping,
    )
        requires self.valid_for(expected_grouping),
        ensures
            expected_grouping.sequences_spec() == 1
                || expected_grouping.sequences_spec() == 8
                || expected_grouping.sequences_spec() == 32,
            self.tokens_per_lane == 8_192,
            self.has_exact_token_coverage(),
            forall|ordinal: int| 0 <= ordinal < 8_191 ==> {
                &&& self.steps@[ordinal].kind
                    == M1QualificationContextStepKind::TeacherForcedPromptContext
                &&& self.steps@[ordinal].prompt_context_commits
                    == self.steps@[ordinal].input_tokens
                &&& self.steps@[ordinal].input_tokens.end
                    == self.steps@[ordinal].input_tokens.start + 1
                &&& self.steps@[ordinal].compact_choice
                    == M1QualificationCompactChoiceDisposition::ObservedButSuppressed
                &&& self.steps@[ordinal].next_input
                    == M1QualificationNextInputPolicy::IndependentTeacherForcedPrompt
                &&& self.steps@[ordinal].externally_emitted_output_count == 0
                &&& !self.steps@[ordinal].capture_qualification_output
            },
            self.steps@[8_191].kind == M1QualificationContextStepKind::FinalObserved,
            self.steps@[8_191].input_tokens
                == (M1QualificationTokenRange { start: 8_191, end: 8_192 }),
            self.steps@[8_191].prompt_context_commits == self.steps@[8_191].input_tokens,
            self.steps@[8_191].compact_choice
                == M1QualificationCompactChoiceDisposition::ObservedAndPublished,
            self.steps@[8_191].next_input == M1QualificationNextInputPolicy::Terminal,
            self.steps@[8_191].externally_emitted_output_count == 1,
            self.steps@[8_191].capture_qualification_output,
    {
        reveal(M1QualificationContextPlan::valid_for);
        reveal(M1QualificationContextPlan::every_step_is_valid);
        reveal(M1QualificationContextStep::valid_at);
        reveal(M1QualificationLaneGrouping::sequences_spec);
    }
}

/// Constructs the unique v1 qualification context plan for one grouping.
#[must_use]
#[allow(clippy::bool_to_int_with_if)]
pub fn m1_qualification_context_plan(
    grouping: M1QualificationLaneGrouping,
) -> (plan: M1QualificationContextPlan)
    ensures plan.valid_for(grouping),
{
    let mut steps: Vec<M1QualificationContextStep> = Vec::new();
    let mut ordinal = 0u32;
    while ordinal < M1_QUALIFICATION_TOKENS_PER_LANE
        invariant
            0 <= ordinal <= M1_QUALIFICATION_TOKENS_PER_LANE,
            steps@.len() == ordinal as nat,
            forall|prior: int| 0 <= prior < ordinal
                ==> steps@[prior].valid_at(prior as u32),
        decreases M1_QUALIFICATION_TOKENS_PER_LANE - ordinal,
    {
        let priming = ordinal < M1_QUALIFICATION_PROMPT_CONTEXT_TOKENS;
        let step = M1QualificationContextStep {
            kind: if priming {
                M1QualificationContextStepKind::TeacherForcedPromptContext
            } else {
                M1QualificationContextStepKind::FinalObserved
            },
            input_tokens: M1QualificationTokenRange {
                start: ordinal,
                end: ordinal + 1,
            },
            prompt_context_commits: M1QualificationTokenRange {
                start: ordinal,
                end: ordinal + 1,
            },
            compact_choice: if priming {
                M1QualificationCompactChoiceDisposition::ObservedButSuppressed
            } else {
                M1QualificationCompactChoiceDisposition::ObservedAndPublished
            },
            next_input: if priming {
                M1QualificationNextInputPolicy::IndependentTeacherForcedPrompt
            } else {
                M1QualificationNextInputPolicy::Terminal
            },
            externally_emitted_output_count: if priming { 0 } else { 1 },
            capture_qualification_output: !priming,
        };
        assert(step.valid_at(ordinal));
        steps.push(step);
        ordinal += 1;
    }

    let plan = M1QualificationContextPlan {
        version: M1_QUALIFICATION_CONTEXT_PLAN_VERSION,
        plan_id: m1_qualification_context_plan_identity(grouping),
        grouping,
        tokens_per_lane: M1_QUALIFICATION_TOKENS_PER_LANE,
        steps,
    };
    assert(plan.every_step_is_valid()) by {
        reveal(M1QualificationContextPlan::every_step_is_valid);
    }
    proof {
        plan.exact_coverage_from_valid_steps();
    }
    plan
}

/// Fail-closed errors for the qualification-only context plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum M1QualificationContextPlanError {
    UnsupportedVersion,
    GroupingMismatch,
    PlanIdentityMismatch,
    TokensPerLane {
        expected_tokens: u32,
        actual_tokens: u32,
    },
    StepCount {
        expected_steps: usize,
        actual_steps: usize,
    },
    StepKind { ordinal: u32 },
    TokenCoverageStart { ordinal: u32, expected: u32, actual: u32 },
    TokenCoverageEnd { ordinal: u32, expected: u32, actual: u32 },
    PromptCommitMismatch { ordinal: u32 },
    CompactChoiceDisposition { ordinal: u32 },
    NextInputPolicy { ordinal: u32 },
    ExternallyEmittedOutputCount { ordinal: u32 },
    CapturePolicy { ordinal: u32 },
}

} // verus!

#[cfg(test)]
mod tests {
    use super::{
        m1_qualification_context_plan, m1_qualification_context_plan_identity,
        M1QualificationCompactChoiceDisposition, M1QualificationContextPlanError,
        M1QualificationContextStepKind, M1QualificationLaneGrouping,
        M1QualificationNextInputPolicy, M1_QUALIFICATION_CONTEXT_PLAN_STEPS,
        M1_QUALIFICATION_CONTEXT_PLAN_VERSION, M1_QUALIFICATION_PROMPT_CONTEXT_TOKENS,
        M1_QUALIFICATION_TOKENS_PER_LANE,
    };
    use crate::Identity;

    const GROUPINGS: [M1QualificationLaneGrouping; 3] = [
        M1QualificationLaneGrouping::S1,
        M1QualificationLaneGrouping::S8,
        M1QualificationLaneGrouping::S32,
    ];

    #[test]
    fn canonical_plans_commit_every_prompt_and_publish_only_terminal_output() {
        for grouping in GROUPINGS {
            let plan = m1_qualification_context_plan(grouping);
            assert_eq!(plan.validate(grouping), Ok(()));
            assert_eq!(plan.version, M1_QUALIFICATION_CONTEXT_PLAN_VERSION);
            assert_eq!(plan.tokens_per_lane, M1_QUALIFICATION_TOKENS_PER_LANE);
            assert_eq!(plan.steps.len(), M1_QUALIFICATION_CONTEXT_PLAN_STEPS);

            for ordinal in 0..M1_QUALIFICATION_PROMPT_CONTEXT_TOKENS as usize {
                let priming = plan.steps[ordinal];
                let token_index =
                    u32::try_from(ordinal).expect("qualification token index fits u32");
                assert_eq!(
                    priming.kind,
                    M1QualificationContextStepKind::TeacherForcedPromptContext
                );
                assert_eq!(priming.input_tokens.start, token_index);
                assert_eq!(priming.input_tokens.end, token_index + 1);
                assert_eq!(priming.prompt_context_commits, priming.input_tokens);
                assert_eq!(
                    priming.compact_choice,
                    M1QualificationCompactChoiceDisposition::ObservedButSuppressed
                );
                assert_eq!(
                    priming.next_input,
                    M1QualificationNextInputPolicy::IndependentTeacherForcedPrompt
                );
                assert_eq!(priming.externally_emitted_output_count, 0);
                assert!(!priming.capture_qualification_output);
            }

            let terminal = plan.steps[M1_QUALIFICATION_PROMPT_CONTEXT_TOKENS as usize];
            assert_eq!(terminal.kind, M1QualificationContextStepKind::FinalObserved);
            assert_eq!(terminal.input_tokens.start, 8_191);
            assert_eq!(terminal.input_tokens.end, 8_192);
            assert_eq!(terminal.prompt_context_commits, terminal.input_tokens);
            assert_eq!(
                terminal.compact_choice,
                M1QualificationCompactChoiceDisposition::ObservedAndPublished
            );
            assert_eq!(
                terminal.next_input,
                M1QualificationNextInputPolicy::Terminal
            );
            assert_eq!(terminal.externally_emitted_output_count, 1);
            assert!(terminal.capture_qualification_output);
        }
    }

    #[test]
    fn stable_plan_identities_are_group_specific_and_repeatable() {
        let expected = [
            [
                0xda, 0x4c, 0xe6, 0x39, 0xcc, 0xf6, 0xcb, 0x56, 0x9b, 0xdc, 0x77, 0x67, 0xd1, 0xfc,
                0x10, 0x1d, 0x6a, 0x03, 0xe3, 0x21, 0xcd, 0xf5, 0x89, 0x87, 0xef, 0x35, 0x61, 0xc1,
                0x93, 0xbf, 0x68, 0x9b,
            ],
            [
                0xa5, 0xa1, 0xe3, 0xd6, 0xfc, 0xd7, 0xce, 0xea, 0x02, 0x61, 0x15, 0xa9, 0x9d, 0x7f,
                0xf5, 0x4a, 0xf5, 0xe4, 0x90, 0x26, 0x3e, 0x59, 0x6e, 0x83, 0x03, 0x52, 0xe3, 0x8e,
                0x29, 0xd9, 0xe3, 0xe0,
            ],
            [
                0x7b, 0x43, 0x08, 0xbe, 0xb3, 0x88, 0x77, 0x43, 0x9b, 0xc6, 0xa1, 0xdc, 0x43, 0xf7,
                0x38, 0x53, 0x24, 0x52, 0xf6, 0x81, 0x76, 0xe6, 0xe1, 0xde, 0x91, 0x22, 0x00, 0x71,
                0x01, 0xc1, 0xd8, 0x91,
            ],
        ];

        for (index, grouping) in GROUPINGS.into_iter().enumerate() {
            let first = m1_qualification_context_plan(grouping);
            let second = m1_qualification_context_plan(grouping);
            assert_eq!(first.plan_id.as_bytes(), &expected[index]);
            assert!(first.plan_id.equals(&second.plan_id));
            assert!(first
                .plan_id
                .equals(&m1_qualification_context_plan_identity(grouping)));
        }
    }

    #[test]
    fn header_grouping_and_cardinality_mutations_fail_closed() {
        let grouping = M1QualificationLaneGrouping::S1;

        let mut changed = m1_qualification_context_plan(grouping);
        changed.version += 1;
        assert_eq!(
            changed.validate(grouping),
            Err(M1QualificationContextPlanError::UnsupportedVersion)
        );

        let mut changed = m1_qualification_context_plan(grouping);
        changed.plan_id = Identity::new([0; 32]);
        assert_eq!(
            changed.validate(grouping),
            Err(M1QualificationContextPlanError::PlanIdentityMismatch)
        );

        let changed = m1_qualification_context_plan(M1QualificationLaneGrouping::S8);
        assert_eq!(
            changed.validate(grouping),
            Err(M1QualificationContextPlanError::GroupingMismatch)
        );

        let mut changed = m1_qualification_context_plan(grouping);
        changed.tokens_per_lane -= 1;
        assert!(matches!(
            changed.validate(grouping),
            Err(M1QualificationContextPlanError::TokensPerLane { .. })
        ));

        let mut changed = m1_qualification_context_plan(grouping);
        changed.steps.pop();
        assert!(matches!(
            changed.validate(grouping),
            Err(M1QualificationContextPlanError::StepCount { .. })
        ));
    }

    #[test]
    fn gap_overlap_and_trailing_mutations_fail_closed() {
        let grouping = M1QualificationLaneGrouping::S1;

        let mut gap = m1_qualification_context_plan(grouping);
        gap.steps[4_096].input_tokens.start += 1;
        assert!(matches!(
            gap.validate(grouping),
            Err(M1QualificationContextPlanError::TokenCoverageStart { ordinal: 4_096, .. })
        ));

        let mut overlap = m1_qualification_context_plan(grouping);
        overlap.steps[8_190].input_tokens.start -= 1;
        assert!(matches!(
            overlap.validate(grouping),
            Err(M1QualificationContextPlanError::TokenCoverageStart { ordinal: 8_190, .. })
        ));

        let mut trailing = m1_qualification_context_plan(grouping);
        trailing.steps[8_191].input_tokens.end += 1;
        assert!(matches!(
            trailing.validate(grouping),
            Err(M1QualificationContextPlanError::TokenCoverageEnd { ordinal: 8_191, .. })
        ));
    }

    #[test]
    fn priming_choice_commit_emission_and_capture_mutations_fail_closed() {
        let grouping = M1QualificationLaneGrouping::S8;

        let mut commit_drift = m1_qualification_context_plan(grouping);
        commit_drift.steps[0].prompt_context_commits.end = 0;
        assert_eq!(
            commit_drift.validate(grouping),
            Err(M1QualificationContextPlanError::PromptCommitMismatch { ordinal: 0 })
        );

        let mut choice_published = m1_qualification_context_plan(grouping);
        choice_published.steps[17].compact_choice =
            M1QualificationCompactChoiceDisposition::ObservedAndPublished;
        assert_eq!(
            choice_published.validate(grouping),
            Err(M1QualificationContextPlanError::CompactChoiceDisposition { ordinal: 17 })
        );

        let mut choice_drives_prompt = m1_qualification_context_plan(grouping);
        choice_drives_prompt.steps[18].next_input =
            M1QualificationNextInputPolicy::CompactChoiceFeedback;
        assert_eq!(
            choice_drives_prompt.validate(grouping),
            Err(M1QualificationContextPlanError::NextInputPolicy { ordinal: 18 })
        );

        let mut priming_emits = m1_qualification_context_plan(grouping);
        priming_emits.steps[8_190].externally_emitted_output_count = 1;
        assert_eq!(
            priming_emits.validate(grouping),
            Err(M1QualificationContextPlanError::ExternallyEmittedOutputCount { ordinal: 8_190 })
        );

        let mut priming_captures = m1_qualification_context_plan(grouping);
        priming_captures.steps[1].capture_qualification_output = true;
        assert_eq!(
            priming_captures.validate(grouping),
            Err(M1QualificationContextPlanError::CapturePolicy { ordinal: 1 })
        );
    }

    #[test]
    fn terminal_choice_and_publication_mutations_fail_closed() {
        let grouping = M1QualificationLaneGrouping::S32;

        let mut terminal_choice = m1_qualification_context_plan(grouping);
        terminal_choice.steps[8_191].compact_choice =
            M1QualificationCompactChoiceDisposition::ObservedButSuppressed;
        assert_eq!(
            terminal_choice.validate(grouping),
            Err(M1QualificationContextPlanError::CompactChoiceDisposition { ordinal: 8_191 })
        );

        let mut terminal_suppressed = m1_qualification_context_plan(grouping);
        terminal_suppressed.steps[8_191].externally_emitted_output_count = 0;
        assert_eq!(
            terminal_suppressed.validate(grouping),
            Err(M1QualificationContextPlanError::ExternallyEmittedOutputCount { ordinal: 8_191 })
        );

        let mut terminal_not_captured = m1_qualification_context_plan(grouping);
        terminal_not_captured.steps[8_191].capture_qualification_output = false;
        assert_eq!(
            terminal_not_captured.validate(grouping),
            Err(M1QualificationContextPlanError::CapturePolicy { ordinal: 8_191 })
        );

        let mut phase_substitution = m1_qualification_context_plan(grouping);
        phase_substitution.steps[8_191].kind =
            M1QualificationContextStepKind::TeacherForcedPromptContext;
        assert_eq!(
            phase_substitution.validate(grouping),
            Err(M1QualificationContextPlanError::StepKind { ordinal: 8_191 })
        );
    }
}
