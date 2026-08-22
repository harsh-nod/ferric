//! Verified, inert validation for one structural kernel-catalog operation.
//!
//! A separately supplied expectation selects one exact entry in the checked-in
//! 22-plan, 10,648-operation declaration. Its identity fields are data, not an
//! authentication root. Success retains catalog metadata only; this module
//! cannot compile, load, launch, complete, or otherwise execute a kernel.

use crate::{
    catalog::family_for, KernelAuthorityRequirements, KernelFamily, KernelProfileDescriptor,
    KernelProfileDisposition, M1_KERNEL_CATALOG_VERSION,
};
use ferric_generated_runner::{
    GeneratedPlanTemplate, GENERATED_PLAN_TEMPLATES, GENERATED_RUNNER_OPERATION_COUNT,
    GENERATED_RUNNER_PLAN_COUNT,
};
use ferric_spec::{expected_step, Identity};
use vstd::prelude::*;

verus! {

/// Exact processor bytes retained by the verified catalog handoff.
pub const VERIFIED_GFX942_PROCESSOR_BYTES: [u8; 6] = [
    103, 102, 120, 57, 52, 50,
];

/// Exact target-feature bytes retained by the verified catalog handoff.
pub const VERIFIED_GFX942_TARGET_FEATURE_BYTES: [u8; 23] = [
    43, 119, 97, 118, 101, 102, 114, 111, 110, 116, 115, 105, 122, 101, 54, 52, 44, 45, 120,
    110, 97, 99, 107,
];

/// Caller-supplied identity roles framed by one catalog handoff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KernelCatalogAuthorityRole {
    PlanCatalog,
    Plan,
    Fe2o3Source,
    Compiler,
    CompilerConfiguration,
    TargetContract,
    KernelProofSet,
    KernelAbiCatalog,
    RuntimeContract,
    RuntimeAbi,
    TcbReport,
}

/// Complete caller-supplied authority roster for one structural operation.
///
/// The validator checks presence, role ordering, distinctness, and exact
/// equality with the separately supplied expectation. It authenticates none
/// of these identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KernelCatalogAuthorityInputs {
    pub plan_catalog_id: Identity,
    pub plan_id: Identity,
    pub requirements: KernelAuthorityRequirements,
}

/// One structural catalog candidate or separately supplied expectation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KernelCatalogInput {
    pub catalog_version: u32,
    pub processor: [u8; 6],
    pub target_features: [u8; 23],
    pub plan_index: u16,
    pub operation_index: u32,
    pub profile: KernelProfileDescriptor,
    pub authorities: KernelCatalogAuthorityInputs,
}

/// Fail-closed rejection from structural catalog validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KernelCatalogValidationError {
    ExpectationVersionDrift,
    ExpectationTargetDrift,
    ExpectationPlanDrift,
    ExpectationOperationDrift,
    ExpectationProfileSelectionDrift,
    ExpectationProfileStepDrift,
    ExpectationProfileBoundsDrift,
    ExpectationProfileFamilyDrift,
    ExpectationMissingAuthority(KernelCatalogAuthorityRole),
    ExpectationReusedAuthority,
    CandidateFieldDrift,
    CandidateTargetDrift,
    CandidateProfileDrift,
    CandidateAuthorityDrift(KernelCatalogAuthorityRole),
}

/// Exact catalog metadata retained after validation.
///
/// This wrapper is intentionally not `Clone`, although its inert metadata is
/// copyable and available through read-only accessors. The wrapper is not a
/// linear resource and grants no artifact or execution authority.
#[derive(Debug, PartialEq, Eq)]
pub struct ValidatedKernelCatalogInput {
    input: KernelCatalogInput,
}

impl ValidatedKernelCatalogInput {
    pub closed spec fn input_spec(&self) -> KernelCatalogInput {
        self.input
    }

    /// Returns the complete retained structural input.
    #[must_use]
    pub const fn input(&self) -> (input: &KernelCatalogInput)
        ensures *input == self.input_spec(),
    {
        &self.input
    }

    /// Returns the exact retained flattened operation position.
    #[must_use]
    pub const fn operation_index(&self) -> (operation_index: u32)
        ensures operation_index == self.input_spec().operation_index,
    {
        self.input.operation_index
    }

    /// Returns the exact retained plan identity.
    #[must_use]
    pub const fn plan_id(&self) -> (plan_id: Identity)
        ensures plan_id == self.input_spec().authorities.plan_id,
    {
        self.input.authorities.plan_id
    }

    /// Returns the exact retained plan-catalog identity.
    #[must_use]
    pub const fn plan_catalog_id(&self) -> (plan_catalog_id: Identity)
        ensures plan_catalog_id == self.input_spec().authorities.plan_catalog_id,
    {
        self.input.authorities.plan_catalog_id
    }

    /// Returns the exact retained canonical operation profile.
    #[must_use]
    pub const fn profile(&self) -> (profile: KernelProfileDescriptor)
        ensures profile == self.input_spec().profile,
    {
        self.input.profile
    }

    /// Returns the exact retained caller-supplied authority requirements.
    #[must_use]
    pub const fn authority_requirements(&self) -> (requirements: KernelAuthorityRequirements)
        ensures requirements == self.input_spec().authorities.requirements,
    {
        self.input.authorities.requirements
    }
}

closed spec fn identity_present(identity: Identity) -> bool {
    exists|index: int|
        0 <= index < identity.bytes_spec().len()
            && identity.bytes_spec()[index] != 0
}

closed spec fn authority_sequence(inputs: KernelCatalogAuthorityInputs) -> Seq<Identity> {
    Seq::empty()
        .push(inputs.plan_catalog_id)
        .push(inputs.plan_id)
        .push(inputs.requirements.fe2o3_source)
        .push(inputs.requirements.compiler)
        .push(inputs.requirements.compiler_configuration)
        .push(inputs.requirements.target_contract)
        .push(inputs.requirements.kernel_proof_set)
        .push(inputs.requirements.kernel_abi_catalog)
        .push(inputs.requirements.runtime_contract)
        .push(inputs.requirements.runtime_abi)
        .push(inputs.requirements.tcb_report)
}

closed spec fn authorities_are_present_and_distinct(
    inputs: KernelCatalogAuthorityInputs,
) -> bool {
    let identities = authority_sequence(inputs);
    &&& forall|index: int|
        0 <= index < identities.len() ==> identity_present(identities[index])
    &&& forall|left: int, right: int|
        0 <= left < right < identities.len()
            ==> identities[left].bytes_spec() != identities[right].bytes_spec()
}

closed spec fn authorities_match_exactly(
    candidate: KernelCatalogAuthorityInputs,
    expected: KernelCatalogAuthorityInputs,
) -> bool {
    let candidate_identities = authority_sequence(candidate);
    let expected_identities = authority_sequence(expected);
    candidate_identities.len() == expected_identities.len()
        && forall|index: int|
            0 <= index < candidate_identities.len()
                ==> candidate_identities[index].bytes_spec()
                    == expected_identities[index].bytes_spec()
}

closed spec fn profile_matches_exactly(
    candidate: KernelProfileDescriptor,
    expected: KernelProfileDescriptor,
) -> bool {
    &&& candidate.plan_id.bytes_spec() == expected.plan_id.bytes_spec()
    &&& candidate.selection == expected.selection
    &&& candidate.step == expected.step
    &&& candidate.sequences == expected.sequences
    &&& candidate.active_tokens == expected.active_tokens
    &&& candidate.context_tokens == expected.context_tokens
    &&& candidate.family == expected.family
    &&& candidate.disposition == expected.disposition
}

closed spec fn target_is_exact(input: KernelCatalogInput) -> bool {
    input.processor@ == VERIFIED_GFX942_PROCESSOR_BYTES@
        && input.target_features@ == VERIFIED_GFX942_TARGET_FEATURE_BYTES@
}

closed spec fn expectation_matches_catalog(input: KernelCatalogInput) -> bool {
    &&& input.catalog_version == M1_KERNEL_CATALOG_VERSION
    &&& target_is_exact(input)
    &&& (input.plan_index as int) < GENERATED_RUNNER_PLAN_COUNT
    &&& {
        let template = GENERATED_PLAN_TEMPLATES@[input.plan_index as int];
        &&& template.plan_index == input.plan_index
        &&& template.selection == input.profile.selection
        &&& template.operation_start as int <= input.operation_index as int
        &&& (input.operation_index as int)
            < (template.operation_start as int) + (template.operation_count as int)
        &&& (input.operation_index as int) < GENERATED_RUNNER_OPERATION_COUNT
        &&& input.profile.plan_id.bytes_spec() == input.authorities.plan_id.bytes_spec()
        &&& ferric_spec::canonical_expected_step_spec(
            template.selection.role,
            template.selection.mode,
            template.selection.bucket,
            (input.operation_index - template.operation_start) as u32,
        ) == Some(input.profile.step)
        &&& match template.selection.bucket.dimensions_spec(
            template.selection.role,
            template.selection.mode,
        ) {
            Some(dimensions) => {
                &&& input.profile.sequences == dimensions.sequences
                &&& input.profile.active_tokens == dimensions.active_tokens
                &&& input.profile.context_tokens == dimensions.context_tokens
            },
            None => false,
        }
        &&& (input.profile.family, input.profile.disposition)
            == crate::catalog::family_for_spec(
                input.profile.step.operator,
                input.profile.selection.mode,
            )
    }
    &&& authorities_are_present_and_distinct(input.authorities)
}

closed spec fn input_matches_exactly(
    candidate: KernelCatalogInput,
    expected: KernelCatalogInput,
) -> bool {
    &&& candidate.catalog_version == expected.catalog_version
    &&& candidate.processor@ == expected.processor@
    &&& candidate.target_features@ == expected.target_features@
    &&& candidate.plan_index == expected.plan_index
    &&& candidate.operation_index == expected.operation_index
    &&& profile_matches_exactly(candidate.profile, expected.profile)
    &&& authorities_match_exactly(candidate.authorities, expected.authorities)
}

/// Exact mathematical acceptance relation for one inert catalog handoff.
pub closed spec fn kernel_catalog_input_matches_exactly(
    candidate: KernelCatalogInput,
    expected: KernelCatalogInput,
) -> bool {
    expectation_matches_catalog(expected) && input_matches_exactly(candidate, expected)
}

/// Exposes the exact generated-position and logical-profile consequences used
/// by cross-crate graph composition proofs.
///
/// The catalog acceptance predicate remains closed in this owning module. No
/// compiler, artifact, address, dispatch, or execution authority follows from
/// this lemma.
pub proof fn kernel_catalog_match_exposes_plan_operation(
    candidate: KernelCatalogInput,
    expected: KernelCatalogInput,
)
    requires kernel_catalog_input_matches_exactly(candidate, expected),
    ensures
        candidate.catalog_version == expected.catalog_version,
        candidate.plan_index == expected.plan_index,
        candidate.operation_index == expected.operation_index,
        candidate.profile.selection == expected.profile.selection,
        candidate.profile.step == expected.profile.step,
        candidate.profile.sequences == expected.profile.sequences,
        candidate.profile.active_tokens == expected.profile.active_tokens,
        candidate.profile.context_tokens == expected.profile.context_tokens,
        candidate.profile.family == expected.profile.family,
        candidate.profile.disposition == expected.profile.disposition,
        candidate.profile.plan_id.bytes_spec() == expected.profile.plan_id.bytes_spec(),
        candidate.authorities.plan_id.bytes_spec()
            == expected.authorities.plan_id.bytes_spec(),
        candidate.authorities.plan_catalog_id.bytes_spec()
            == expected.authorities.plan_catalog_id.bytes_spec(),
        (expected.plan_index as int) < GENERATED_RUNNER_PLAN_COUNT,
        expected.plan_index
            == GENERATED_PLAN_TEMPLATES@[expected.plan_index as int].plan_index,
        expected.profile.selection
            == GENERATED_PLAN_TEMPLATES@[expected.plan_index as int].selection,
        ferric_spec::canonical_expected_step_spec(
            expected.profile.selection.role,
            expected.profile.selection.mode,
            expected.profile.selection.bucket,
            expected.profile.step.ordinal,
        ) == Some(expected.profile.step),
{
    reveal(kernel_catalog_input_matches_exactly);
    reveal(expectation_matches_catalog);
    reveal(input_matches_exactly);
    reveal(profile_matches_exactly);
    reveal(authorities_match_exactly);
    assert(authority_sequence(candidate.authorities)[0].bytes_spec()
        == authority_sequence(expected.authorities)[0].bytes_spec());
    assert(authority_sequence(candidate.authorities)[1].bytes_spec()
        == authority_sequence(expected.authorities)[1].bytes_spec());
    reveal(authority_sequence);
    reveal(ferric_spec::canonical_expected_step_spec);
}

impl ValidatedKernelCatalogInput {
    /// Exact independently supplied expectation accepted for this retained
    /// structural kernel operation.
    pub closed spec fn matches_expected_spec(&self, expected: KernelCatalogInput) -> bool {
        kernel_catalog_input_matches_exactly(self.input, expected)
    }

    /// Exposes exact generated-position and logical-profile consequences
    /// without opening the owning module's acceptance relation.
    pub proof fn expose_plan_operation(&self, expected: KernelCatalogInput)
        requires self.matches_expected_spec(expected),
        ensures
            self.input_spec().catalog_version == expected.catalog_version,
            self.input_spec().plan_index == expected.plan_index,
            self.input_spec().operation_index == expected.operation_index,
            self.input_spec().profile.selection == expected.profile.selection,
            self.input_spec().profile.step == expected.profile.step,
            self.input_spec().profile.sequences == expected.profile.sequences,
            self.input_spec().profile.active_tokens == expected.profile.active_tokens,
            self.input_spec().profile.context_tokens == expected.profile.context_tokens,
            self.input_spec().profile.family == expected.profile.family,
            self.input_spec().profile.disposition == expected.profile.disposition,
            self.input_spec().profile.plan_id.bytes_spec()
                == expected.profile.plan_id.bytes_spec(),
            self.input_spec().authorities.plan_id.bytes_spec()
                == expected.authorities.plan_id.bytes_spec(),
            self.input_spec().authorities.plan_catalog_id.bytes_spec()
                == expected.authorities.plan_catalog_id.bytes_spec(),
            (expected.plan_index as int) < GENERATED_RUNNER_PLAN_COUNT,
            expected.plan_index
                == GENERATED_PLAN_TEMPLATES@[expected.plan_index as int].plan_index,
            expected.profile.selection
                == GENERATED_PLAN_TEMPLATES@[expected.plan_index as int].selection,
            ferric_spec::canonical_expected_step_spec(
                expected.profile.selection.role,
                expected.profile.selection.mode,
                expected.profile.selection.bucket,
                expected.profile.step.ordinal,
            ) == Some(expected.profile.step),
    {
        reveal(ValidatedKernelCatalogInput::matches_expected_spec);
        kernel_catalog_match_exposes_plan_operation(self.input, expected);
    }

    /// Exposes the exact M1 logical envelope and finite K1-K7 family bound for
    /// one independently accepted catalog operation.
    ///
    /// This lemma does not expose schedule implementation, memory allocation,
    /// compiler, object, driver, hardware, numerical, or performance facts.
    pub proof fn expose_m1_finite_profile(&self, expected: KernelCatalogInput)
        requires self.matches_expected_spec(expected),
        ensures
            crate::m1_kernel_profile_is_finite(self.input_spec().profile),
            crate::m1_kernel_profile_is_finite(expected.profile),
    {
        self.expose_plan_operation(expected);
        reveal(kernel_catalog_input_matches_exactly);
        reveal(expectation_matches_catalog);
        let dimensions = expected.profile.selection.bucket.dimensions_spec(
            expected.profile.selection.role,
            expected.profile.selection.mode,
        ).unwrap();
        ferric_spec::qwen3_m1_plan_dimensions_are_bounded(
            expected.profile.selection.bucket,
            expected.profile.selection.role,
            expected.profile.selection.mode,
            dimensions,
        );
        crate::catalog::family_for_is_declared_foundation(
            expected.profile.step.operator,
            expected.profile.selection.mode,
        );
        reveal(crate::m1_kernel_profile_is_finite);
        reveal(crate::catalog::m1_kernel_profile_is_finite);
    }
}

/// Returns one exact plan position, or `None` outside the finite roster.
#[must_use]
pub fn kernel_catalog_plan(
    plan_index: u16,
) -> (template: Option<GeneratedPlanTemplate>)
    ensures
        template == if (plan_index as int) < GENERATED_RUNNER_PLAN_COUNT {
            Some(GENERATED_PLAN_TEMPLATES@[plan_index as int])
        } else {
            None
        },
{
    if (plan_index as usize) < GENERATED_RUNNER_PLAN_COUNT {
        Some(GENERATED_PLAN_TEMPLATES[plan_index as usize])
    } else {
        None
    }
}

fn processor_bytes_match(left: [u8; 6], right: [u8; 6]) -> (matches: bool)
    ensures matches == (left@ == right@),
{
    let mut index = 0usize;
    while index < left.len()
        invariant
            left@.len() == right@.len(),
            index <= left@.len(),
            forall|prior: int| 0 <= prior < index ==> left@[prior] == right@[prior],
        decreases left@.len() - index,
    {
        if left[index] != right[index] {
            assert(left@ != right@);
            return false;
        }
        index += 1;
    }
    assert(left@ =~= right@);
    true
}

fn target_feature_bytes_match(left: &[u8; 23], right: &[u8; 23]) -> (matches: bool)
    ensures matches == (left@ == right@),
{
    let mut index = 0usize;
    while index < left.len()
        invariant
            left@.len() == right@.len(),
            index <= left@.len(),
            forall|prior: int| 0 <= prior < index ==> left@[prior] == right@[prior],
        decreases left@.len() - index,
    {
        if left[index] != right[index] {
            assert(left@ != right@);
            return false;
        }
        index += 1;
    }
    assert(left@ =~= right@);
    true
}

fn family_matches(left: KernelFamily, right: KernelFamily) -> (matches: bool)
    ensures matches == (left == right),
{
    matches!((left, right),
        (KernelFamily::K1GemmGemv, KernelFamily::K1GemmGemv)
            | (KernelFamily::K2RmsNormResidual, KernelFamily::K2RmsNormResidual)
            | (KernelFamily::K3RopePagedKv, KernelFamily::K3RopePagedKv)
            | (KernelFamily::K4GqaPrefill, KernelFamily::K4GqaPrefill)
            | (KernelFamily::K5PagedGqaDecode, KernelFamily::K5PagedGqaDecode)
            | (KernelFamily::K6SwiGlu, KernelFamily::K6SwiGlu)
            | (KernelFamily::K7LogitsCompact, KernelFamily::K7LogitsCompact)
    )
}

fn disposition_matches(
    left: KernelProfileDisposition,
    right: KernelProfileDisposition,
) -> (matches: bool)
    ensures matches == (left == right),
{
    matches!((left, right),
        (
            KernelProfileDisposition::DeclaredFoundation,
            KernelProfileDisposition::DeclaredFoundation,
        ) | (
            KernelProfileDisposition::RequiredExtension,
            KernelProfileDisposition::RequiredExtension,
        )
    )
}

fn profile_matches(
    candidate: &KernelProfileDescriptor,
    expected: &KernelProfileDescriptor,
) -> (matches: bool)
    ensures matches == profile_matches_exactly(*candidate, *expected),
{
    candidate.plan_id.equals(&expected.plan_id)
        && candidate.selection.matches(expected.selection)
        && candidate.step.matches(expected.step)
        && candidate.sequences == expected.sequences
        && candidate.active_tokens == expected.active_tokens
        && candidate.context_tokens == expected.context_tokens
        && family_matches(candidate.family, expected.family)
        && disposition_matches(candidate.disposition, expected.disposition)
}

fn authority_values(inputs: &KernelCatalogAuthorityInputs) -> (values: [Identity; 11])
    ensures values@ == authority_sequence(*inputs),
{
    [
        inputs.plan_catalog_id,
        inputs.plan_id,
        inputs.requirements.fe2o3_source,
        inputs.requirements.compiler,
        inputs.requirements.compiler_configuration,
        inputs.requirements.target_contract,
        inputs.requirements.kernel_proof_set,
        inputs.requirements.kernel_abi_catalog,
        inputs.requirements.runtime_contract,
        inputs.requirements.runtime_abi,
        inputs.requirements.tcb_report,
    ]
}

const fn authority_role(index: usize) -> KernelCatalogAuthorityRole {
    match index {
        0 => KernelCatalogAuthorityRole::PlanCatalog,
        1 => KernelCatalogAuthorityRole::Plan,
        2 => KernelCatalogAuthorityRole::Fe2o3Source,
        3 => KernelCatalogAuthorityRole::Compiler,
        4 => KernelCatalogAuthorityRole::CompilerConfiguration,
        5 => KernelCatalogAuthorityRole::TargetContract,
        6 => KernelCatalogAuthorityRole::KernelProofSet,
        7 => KernelCatalogAuthorityRole::KernelAbiCatalog,
        8 => KernelCatalogAuthorityRole::RuntimeContract,
        9 => KernelCatalogAuthorityRole::RuntimeAbi,
        _ => KernelCatalogAuthorityRole::TcbReport,
    }
}

fn validate_authority_expectation(
    inputs: &KernelCatalogAuthorityInputs,
) -> (result: Result<(), KernelCatalogValidationError>)
    ensures result.is_ok() == authorities_are_present_and_distinct(*inputs),
{
    let values = authority_values(inputs);
    let mut index = 0usize;
    while index < values.len()
        invariant
            values@ == authority_sequence(*inputs),
            index <= values@.len(),
            forall|prior: int| 0 <= prior < index ==> identity_present(values@[prior]),
            forall|left: int, right: int|
                0 <= left < right < index
                    ==> values@[left].bytes_spec() != values@[right].bytes_spec(),
        decreases values@.len() - index,
    {
        if !values[index].is_present() {
            assert(!authorities_are_present_and_distinct(*inputs)) by {
                reveal(authorities_are_present_and_distinct);
                assert(!identity_present(authority_sequence(*inputs)[index as int]));
            }
            return Err(KernelCatalogValidationError::ExpectationMissingAuthority(
                authority_role(index),
            ));
        }
        let mut prior = 0usize;
        while prior < index
            invariant
                values@ == authority_sequence(*inputs),
                prior <= index,
                index < values@.len(),
                forall|previous: int| 0 <= previous < index ==> identity_present(values@[previous]),
                identity_present(values@[index as int]),
                forall|left: int, right: int|
                    0 <= left < right < index
                        ==> values@[left].bytes_spec() != values@[right].bytes_spec(),
                forall|previous: int| 0 <= previous < prior
                    ==> values@[previous].bytes_spec() != values@[index as int].bytes_spec(),
            decreases index - prior,
        {
            if values[index].equals(&values[prior]) {
                assert(!authorities_are_present_and_distinct(*inputs)) by {
                    reveal(authorities_are_present_and_distinct);
                }
                return Err(KernelCatalogValidationError::ExpectationReusedAuthority);
            }
            prior += 1;
        }
        assert forall|left: int, right: int| 0 <= left < right < index + 1
            implies values@[left].bytes_spec() != values@[right].bytes_spec() by {
            if right < index {
                assert(values@[left].bytes_spec() != values@[right].bytes_spec());
            } else {
                assert(right == index);
                assert(values@[left].bytes_spec() != values@[index as int].bytes_spec());
            }
        }
        index += 1;
    }
    assert(authorities_are_present_and_distinct(*inputs)) by {
        reveal(authorities_are_present_and_distinct);
    }
    Ok(())
}

fn validate_authority_match(
    candidate: &KernelCatalogAuthorityInputs,
    expected: &KernelCatalogAuthorityInputs,
) -> (result: Result<(), KernelCatalogValidationError>)
    ensures result.is_ok() == authorities_match_exactly(*candidate, *expected),
{
    let candidate_values = authority_values(candidate);
    let expected_values = authority_values(expected);
    let mut index = 0usize;
    while index < candidate_values.len()
        invariant
            candidate_values@ == authority_sequence(*candidate),
            expected_values@ == authority_sequence(*expected),
            candidate_values@.len() == expected_values@.len(),
            index <= candidate_values@.len(),
            forall|prior: int| 0 <= prior < index
                ==> candidate_values@[prior].bytes_spec()
                    == expected_values@[prior].bytes_spec(),
        decreases candidate_values@.len() - index,
    {
        if !candidate_values[index].equals(&expected_values[index]) {
            assert(!authorities_match_exactly(*candidate, *expected)) by {
                reveal(authorities_match_exactly);
            }
            return Err(KernelCatalogValidationError::CandidateAuthorityDrift(
                authority_role(index),
            ));
        }
        index += 1;
    }
    assert(authorities_match_exactly(*candidate, *expected)) by {
        reveal(authorities_match_exactly);
        assert forall|position: int| 0 <= position < authority_sequence(*candidate).len()
            implies authority_sequence(*candidate)[position].bytes_spec()
                == authority_sequence(*expected)[position].bytes_spec() by {}
    }
    Ok(())
}

fn validate_expectation(
    expected: &KernelCatalogInput,
) -> (result: Result<(), KernelCatalogValidationError>)
    ensures result.is_ok() == expectation_matches_catalog(*expected),
{
    if expected.catalog_version != M1_KERNEL_CATALOG_VERSION {
        return Err(KernelCatalogValidationError::ExpectationVersionDrift);
    }
    if !processor_bytes_match(expected.processor, VERIFIED_GFX942_PROCESSOR_BYTES)
        || !target_feature_bytes_match(
            &expected.target_features,
            &VERIFIED_GFX942_TARGET_FEATURE_BYTES,
        )
    {
        return Err(KernelCatalogValidationError::ExpectationTargetDrift);
    }
    let template = match kernel_catalog_plan(expected.plan_index) {
        Some(template) => template,
        None => return Err(KernelCatalogValidationError::ExpectationPlanDrift),
    };
    if expected.plan_index != template.plan_index
        || !expected.profile.selection.matches(template.selection)
    {
        return Err(KernelCatalogValidationError::ExpectationPlanDrift);
    }
    if expected.operation_index < template.operation_start {
        return Err(KernelCatalogValidationError::ExpectationOperationDrift);
    }
    let ordinal = expected.operation_index - template.operation_start;
    if ordinal >= template.operation_count
        || (expected.operation_index as usize) >= GENERATED_RUNNER_OPERATION_COUNT
    {
        return Err(KernelCatalogValidationError::ExpectationOperationDrift);
    }
    if !expected
        .profile
        .plan_id
        .equals(&expected.authorities.plan_id)
    {
        return Err(KernelCatalogValidationError::ExpectationProfileSelectionDrift);
    }
    let step = match expected_step(
        template.selection.role,
        template.selection.mode,
        template.selection.bucket,
        ordinal,
    ) {
        Some(step) => step,
        None => return Err(KernelCatalogValidationError::ExpectationProfileStepDrift),
    };
    if !expected.profile.step.matches(step) {
        return Err(KernelCatalogValidationError::ExpectationProfileStepDrift);
    }
    let dimensions = match template.selection.bucket.dimensions(
        template.selection.role,
        template.selection.mode,
    ) {
        Some(dimensions) => dimensions,
        None => return Err(KernelCatalogValidationError::ExpectationProfileBoundsDrift),
    };
    if expected.profile.sequences != dimensions.sequences
        || expected.profile.active_tokens != dimensions.active_tokens
        || expected.profile.context_tokens != dimensions.context_tokens
    {
        return Err(KernelCatalogValidationError::ExpectationProfileBoundsDrift);
    }
    let (family, disposition) = family_for(
        expected.profile.step.operator,
        expected.profile.selection.mode,
    );
    if !family_matches(expected.profile.family, family)
        || !disposition_matches(expected.profile.disposition, disposition)
    {
        return Err(KernelCatalogValidationError::ExpectationProfileFamilyDrift);
    }
    validate_authority_expectation(&expected.authorities)?;
    Ok(())
}

/// Validates and retains one exact inert catalog operation.
///
/// # Errors
///
/// Returns [`KernelCatalogValidationError`] unless the expectation selects one
/// exact generated plan/operation, its full canonical graph step, dimensions,
/// family/disposition, target tuple, and authority framing are exact, and the
/// candidate matches that expectation field for field.
///
/// Success authenticates no identity and grants no artifact, compilation,
/// machine-code refinement, allocation, launch, completion, hardware,
/// performance, or qualification authority.
pub fn validate_kernel_catalog_input(
    candidate: KernelCatalogInput,
    expected: KernelCatalogInput,
) -> (result: Result<ValidatedKernelCatalogInput, KernelCatalogValidationError>)
    ensures
        result.is_ok() == kernel_catalog_input_matches_exactly(candidate, expected),
        match result {
            Ok(validated) => {
                &&& validated.input_spec() == candidate
                &&& validated.matches_expected_spec(expected)
            },
            Err(_) => true,
        },
{
    validate_expectation(&expected)?;
    if candidate.catalog_version != expected.catalog_version
        || candidate.plan_index != expected.plan_index
        || candidate.operation_index != expected.operation_index
    {
        return Err(KernelCatalogValidationError::CandidateFieldDrift);
    }
    if !processor_bytes_match(candidate.processor, expected.processor)
        || !target_feature_bytes_match(&candidate.target_features, &expected.target_features)
    {
        return Err(KernelCatalogValidationError::CandidateTargetDrift);
    }
    if !profile_matches(&candidate.profile, &expected.profile) {
        return Err(KernelCatalogValidationError::CandidateProfileDrift);
    }
    validate_authority_match(&candidate.authorities, &expected.authorities)?;
    Ok(ValidatedKernelCatalogInput { input: candidate })
}

} // verus!

#[cfg(test)]
mod tests {
    use super::*;
    use ferric_spec::{Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket};

    fn identity(seed: u8) -> Identity {
        let mut bytes = [seed; 32];
        bytes[31] = seed.wrapping_add(1);
        Identity::new(bytes)
    }

    fn authorities(plan_id: Identity) -> KernelCatalogAuthorityInputs {
        KernelCatalogAuthorityInputs {
            plan_catalog_id: identity(1),
            plan_id,
            requirements: KernelAuthorityRequirements {
                fe2o3_source: identity(3),
                compiler: identity(4),
                compiler_configuration: identity(5),
                target_contract: identity(6),
                kernel_proof_set: identity(7),
                kernel_abi_catalog: identity(8),
                runtime_contract: identity(9),
                runtime_abi: identity(10),
                tcb_report: identity(11),
            },
        }
    }

    fn input(plan_index: usize, local_ordinal: u32) -> KernelCatalogInput {
        let template = GENERATED_PLAN_TEMPLATES[plan_index];
        let step = expected_step(
            template.selection.role,
            template.selection.mode,
            template.selection.bucket,
            local_ordinal,
        )
        .expect("canonical operation exists");
        let dimensions = template
            .selection
            .bucket
            .dimensions(template.selection.role, template.selection.mode)
            .expect("canonical dimensions exist");
        let (family, disposition) = family_for(step.operator, template.selection.mode);
        let plan_id = identity(u8::try_from(20 + plan_index).expect("plan index fits u8"));
        KernelCatalogInput {
            catalog_version: M1_KERNEL_CATALOG_VERSION,
            processor: VERIFIED_GFX942_PROCESSOR_BYTES,
            target_features: VERIFIED_GFX942_TARGET_FEATURE_BYTES,
            plan_index: template.plan_index,
            operation_index: template.operation_start + local_ordinal,
            profile: KernelProfileDescriptor {
                plan_id,
                selection: template.selection,
                step,
                sequences: dimensions.sequences,
                active_tokens: dimensions.active_tokens,
                context_tokens: dimensions.context_tokens,
                family,
                disposition,
            },
            authorities: authorities(plan_id),
        }
    }

    #[test]
    fn every_plan_accepts_first_and_last_exact_operation() {
        for (plan_index, template) in GENERATED_PLAN_TEMPLATES.iter().enumerate() {
            for ordinal in [0, template.operation_count - 1] {
                let expected = input(plan_index, ordinal);
                let validated = validate_kernel_catalog_input(expected, expected).unwrap();
                assert_eq!(validated.operation_index(), expected.operation_index);
                assert_eq!(validated.plan_id(), expected.authorities.plan_id);
                assert_eq!(
                    validated.plan_catalog_id(),
                    expected.authorities.plan_catalog_id
                );
                assert_eq!(validated.profile(), expected.profile);
                assert_eq!(
                    validated.authority_requirements(),
                    expected.authorities.requirements
                );
                assert_eq!(validated.input(), &expected);
            }
        }
    }

    #[test]
    fn first_and_last_plan_or_operation_drift_fail_closed() {
        let first = input(0, 0);
        let mut changed = first;
        changed.profile.selection.role = Qwen3ModelRole::Draft06B;
        assert_eq!(
            validate_kernel_catalog_input(changed, changed),
            Err(KernelCatalogValidationError::ExpectationPlanDrift)
        );
        let mut changed = first;
        changed.operation_index = 544;
        assert_eq!(
            validate_kernel_catalog_input(changed, changed),
            Err(KernelCatalogValidationError::ExpectationOperationDrift)
        );

        let last = input(21, GENERATED_PLAN_TEMPLATES[21].operation_count - 1);
        let mut changed = last;
        changed.plan_index = 22;
        assert_eq!(
            validate_kernel_catalog_input(changed, changed),
            Err(KernelCatalogValidationError::ExpectationPlanDrift)
        );
        let mut changed = last;
        changed.operation_index += 1;
        assert_eq!(
            validate_kernel_catalog_input(changed, changed),
            Err(KernelCatalogValidationError::ExpectationOperationDrift)
        );
    }

    #[test]
    fn family_disposition_geometry_and_bounds_drift_fail_closed() {
        let exact = input(0, 0);
        let mut changed = exact;
        changed.profile.family = KernelFamily::K7LogitsCompact;
        assert_eq!(
            validate_kernel_catalog_input(changed, changed),
            Err(KernelCatalogValidationError::ExpectationProfileFamilyDrift)
        );
        let mut changed = exact;
        changed.profile.disposition = KernelProfileDisposition::RequiredExtension;
        assert_eq!(
            validate_kernel_catalog_input(changed, changed),
            Err(KernelCatalogValidationError::ExpectationProfileFamilyDrift)
        );
        let mut changed = exact;
        changed.profile.step.geometry.hidden_size += 1;
        assert_eq!(
            validate_kernel_catalog_input(changed, changed),
            Err(KernelCatalogValidationError::ExpectationProfileStepDrift)
        );
        let mut changed = exact;
        changed.profile.context_tokens += 1;
        assert_eq!(
            validate_kernel_catalog_input(changed, changed),
            Err(KernelCatalogValidationError::ExpectationProfileBoundsDrift)
        );
    }

    #[test]
    fn target_feature_and_authority_drift_fail_closed() {
        let exact = input(21, 423);
        let mut changed = exact;
        changed.processor[0] = b'G';
        assert_eq!(
            validate_kernel_catalog_input(changed, changed),
            Err(KernelCatalogValidationError::ExpectationTargetDrift)
        );
        let mut changed = exact;
        changed.target_features[22] = b'K';
        assert_eq!(
            validate_kernel_catalog_input(changed, changed),
            Err(KernelCatalogValidationError::ExpectationTargetDrift)
        );
        let mut changed = exact;
        changed.authorities.requirements.runtime_abi = Identity::new([0; 32]);
        assert_eq!(
            validate_kernel_catalog_input(changed, changed),
            Err(KernelCatalogValidationError::ExpectationMissingAuthority(
                KernelCatalogAuthorityRole::RuntimeAbi
            ))
        );
        let mut changed = exact;
        changed.authorities.requirements.runtime_abi = changed.authorities.requirements.compiler;
        assert_eq!(
            validate_kernel_catalog_input(changed, changed),
            Err(KernelCatalogValidationError::ExpectationReusedAuthority)
        );
    }

    #[test]
    fn exact_expectation_rejects_candidate_profile_target_and_authority_drift() {
        let expected = input(10, 543);
        let mut candidate = expected;
        candidate.operation_index -= 1;
        assert_eq!(
            validate_kernel_catalog_input(candidate, expected),
            Err(KernelCatalogValidationError::CandidateFieldDrift)
        );
        let mut candidate = expected;
        candidate.target_features[0] = b'-';
        assert_eq!(
            validate_kernel_catalog_input(candidate, expected),
            Err(KernelCatalogValidationError::CandidateTargetDrift)
        );
        let mut candidate = expected;
        candidate.profile.step.output_0.shape.dimension_0 += 1;
        assert_eq!(
            validate_kernel_catalog_input(candidate, expected),
            Err(KernelCatalogValidationError::CandidateProfileDrift)
        );
        let mut candidate = expected;
        candidate.authorities.requirements.target_contract = identity(200);
        assert_eq!(
            validate_kernel_catalog_input(candidate, expected),
            Err(KernelCatalogValidationError::CandidateAuthorityDrift(
                KernelCatalogAuthorityRole::TargetContract
            ))
        );
    }

    #[test]
    fn hostile_mode_bucket_pair_cannot_select_an_adjacent_plan() {
        let exact = input(7, 0);
        let mut changed = exact;
        changed.profile.selection.mode = Qwen3ExecutionMode::Decode;
        changed.profile.selection.bucket = Qwen3PlanBucket::DecodeS1C8192;
        assert_eq!(
            validate_kernel_catalog_input(changed, changed),
            Err(KernelCatalogValidationError::ExpectationPlanDrift)
        );
    }
}
