//! Verified, inert handoff validation for one generated runner operation.
//!
//! The separately supplied expectation is data, not an authentication root.
//! Acceptance proves only that the candidate exactly matches that expectation,
//! names one operation in the checked-in generated plan table, carries the
//! exact four logical patch slots, and retains a complete present/distinct
//! identity roster. No value in this module can allocate, patch an address,
//! construct a packet, launch work, observe completion, or publish a result.

use crate::{
    GeneratedPlanTemplate, RunnerPatchExtent, RunnerPatchKind, RunnerPatchScalarType,
    RunnerPatchSlotTemplate, GENERATED_PATCH_SLOTS, GENERATED_PLAN_TEMPLATES,
    GENERATED_RUNNER_PLAN_COUNT, GENERATED_RUNNER_TEMPLATE_VERSION,
};
use ferric_spec::{Identity, Qwen3PlanSelection};
use vstd::prelude::*;

verus! {

/// Exact identity inputs retained for one logical generated operation.
///
/// These values are compared with a separately supplied expectation. Their
/// bytes are not authenticated by this crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeneratedRunnerIdentityInputs {
    pub source_id: Identity,
    pub admission_record_id: Identity,
    pub bundle_id: Identity,
    pub plan_catalog_id: Identity,
    pub kernel_catalog_id: Identity,
    pub closure_id: Identity,
    pub declaration_id: Identity,
    pub plan_id: Identity,
    pub operation_id: Identity,
    pub kernel_contract_id: Identity,
    pub artifact_id: Identity,
    pub descriptor_id: Identity,
    pub geometry_id: Identity,
    pub kernarg_layout_id: Identity,
    pub buffer_layout_id: Identity,
    pub effect_contract_id: Identity,
}

/// One operation handoff candidate or separately supplied exact expectation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeneratedRunnerInput {
    pub template_version: u32,
    pub plan_index: u16,
    pub selection: Qwen3PlanSelection,
    pub operation_start: u32,
    pub operation_count: u32,
    pub operation_index: u32,
    pub patch_slots: [RunnerPatchSlotTemplate; 4],
    pub identities: GeneratedRunnerIdentityInputs,
}

/// Identity role used by fail-closed diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneratedRunnerIdentityRole {
    Source,
    AdmissionRecord,
    Bundle,
    PlanCatalog,
    KernelCatalog,
    Closure,
    Declaration,
    Plan,
    Operation,
    KernelContract,
    Artifact,
    Descriptor,
    Geometry,
    KernargLayout,
    BufferLayout,
    EffectContract,
}

/// Fail-closed rejection from the inert generated-runner handoff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneratedRunnerValidationError {
    ExpectationTemplateDrift,
    ExpectationPlanDrift,
    ExpectationOperationDrift,
    ExpectationPatchSchemaDrift,
    ExpectationMissingIdentity(GeneratedRunnerIdentityRole),
    ExpectationReusedIdentity,
    CandidateFieldDrift,
    CandidatePatchSchemaDrift,
    CandidateIdentityDrift(GeneratedRunnerIdentityRole),
}

/// Exact generated input retained after inert validation.
///
/// This type is intentionally not `Clone`. It exposes the retained declaration,
/// plan, operation, and buffer-layout identities, but no execution operation.
#[derive(Debug, PartialEq, Eq)]
pub struct ValidatedGeneratedRunnerInput {
    input: GeneratedRunnerInput,
}

impl ValidatedGeneratedRunnerInput {
    pub closed spec fn input_spec(&self) -> GeneratedRunnerInput {
        self.input
    }

    /// Returns the complete retained input.
    #[must_use]
    pub const fn input(&self) -> (input: &GeneratedRunnerInput)
        ensures *input == self.input_spec(),
    {
        &self.input
    }

    /// Returns the exact retained declaration identity.
    #[must_use]
    pub const fn declaration_id(&self) -> (identity: Identity)
        ensures identity == self.input_spec().identities.declaration_id,
    {
        self.input.identities.declaration_id
    }

    /// Returns the exact retained plan identity.
    #[must_use]
    pub const fn plan_id(&self) -> (identity: Identity)
        ensures identity == self.input_spec().identities.plan_id,
    {
        self.input.identities.plan_id
    }

    /// Returns the exact retained operation identity.
    #[must_use]
    pub const fn operation_id(&self) -> (identity: Identity)
        ensures identity == self.input_spec().identities.operation_id,
    {
        self.input.identities.operation_id
    }

    /// Returns the exact retained logical buffer-layout identity.
    #[must_use]
    pub const fn buffer_layout_id(&self) -> (identity: Identity)
        ensures identity == self.input_spec().identities.buffer_layout_id,
    {
        self.input.identities.buffer_layout_id
    }
}

closed spec fn identity_present(identity: Identity) -> bool {
    exists|index: int|
        0 <= index < identity.bytes_spec().len()
            && identity.bytes_spec()[index] != 0
}

closed spec fn identity_sequence(inputs: GeneratedRunnerIdentityInputs) -> Seq<Identity> {
    Seq::empty()
        .push(inputs.source_id)
        .push(inputs.admission_record_id)
        .push(inputs.bundle_id)
        .push(inputs.plan_catalog_id)
        .push(inputs.kernel_catalog_id)
        .push(inputs.closure_id)
        .push(inputs.declaration_id)
        .push(inputs.plan_id)
        .push(inputs.operation_id)
        .push(inputs.kernel_contract_id)
        .push(inputs.artifact_id)
        .push(inputs.descriptor_id)
        .push(inputs.geometry_id)
        .push(inputs.kernarg_layout_id)
        .push(inputs.buffer_layout_id)
        .push(inputs.effect_contract_id)
}

closed spec fn identities_are_present_and_distinct(inputs: GeneratedRunnerIdentityInputs) -> bool {
    let identities = identity_sequence(inputs);
    &&& forall|index: int|
        0 <= index < identities.len() ==> identity_present(identities[index])
    &&& forall|left: int, right: int|
        0 <= left < right < identities.len()
            ==> identities[left].bytes_spec() != identities[right].bytes_spec()
}

closed spec fn patch_schema_matches(slots: [RunnerPatchSlotTemplate; 4]) -> bool {
    slots@ == GENERATED_PATCH_SLOTS@
}

closed spec fn expectation_matches_generated_table(expected: GeneratedRunnerInput) -> bool {
    &&& expected.template_version == GENERATED_RUNNER_TEMPLATE_VERSION
    &&& (expected.plan_index as int) < GENERATED_RUNNER_PLAN_COUNT
    &&& {
        let template = GENERATED_PLAN_TEMPLATES@[expected.plan_index as int];
        &&& expected.plan_index == template.plan_index
        &&& expected.selection == template.selection
        &&& expected.operation_start == template.operation_start
        &&& expected.operation_count == template.operation_count
        &&& template.operation_start as int <= expected.operation_index as int
        &&& (expected.operation_index as int)
            < (template.operation_start as int) + (template.operation_count as int)
    }
    &&& patch_schema_matches(expected.patch_slots)
    &&& identities_are_present_and_distinct(expected.identities)
}

closed spec fn identity_inputs_match_exactly(
    candidate: GeneratedRunnerIdentityInputs,
    expected: GeneratedRunnerIdentityInputs,
) -> bool {
    let candidate_identities = identity_sequence(candidate);
    let expected_identities = identity_sequence(expected);
    candidate_identities.len() == expected_identities.len()
        && forall|index: int|
            0 <= index < candidate_identities.len()
                ==> candidate_identities[index].bytes_spec()
                    == expected_identities[index].bytes_spec()
}

closed spec fn input_fields_match_exactly(
    candidate: GeneratedRunnerInput,
    expected: GeneratedRunnerInput,
) -> bool {
    &&& candidate.template_version == expected.template_version
    &&& candidate.plan_index == expected.plan_index
    &&& candidate.selection == expected.selection
    &&& candidate.operation_start == expected.operation_start
    &&& candidate.operation_count == expected.operation_count
    &&& candidate.operation_index == expected.operation_index
    &&& candidate.patch_slots@ == expected.patch_slots@
    &&& identity_inputs_match_exactly(candidate.identities, expected.identities)
}

/// Exact mathematical acceptance relation for the inert handoff.
pub closed spec fn generated_runner_input_matches_exactly(
    candidate: GeneratedRunnerInput,
    expected: GeneratedRunnerInput,
) -> bool {
    expectation_matches_generated_table(expected)
        && input_fields_match_exactly(candidate, expected)
}

/// Returns one exact generated plan position, or `None` outside the roster.
#[must_use]
pub fn generated_plan_template(
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

fn patch_kind_matches(left: RunnerPatchKind, right: RunnerPatchKind) -> (matches: bool)
    ensures matches == (left == right),
{
    matches!(
        (left, right),
        (RunnerPatchKind::TokenIds, RunnerPatchKind::TokenIds)
            | (RunnerPatchKind::PositionIds, RunnerPatchKind::PositionIds)
            | (RunnerPatchKind::ActiveLengths, RunnerPatchKind::ActiveLengths)
            | (RunnerPatchKind::ContextLengths, RunnerPatchKind::ContextLengths)
    )
}

fn patch_scalar_matches(
    left: RunnerPatchScalarType,
    right: RunnerPatchScalarType,
) -> (matches: bool)
    ensures matches == (left == right),
{
    matches!((left, right), (RunnerPatchScalarType::U32, RunnerPatchScalarType::U32))
}

fn patch_extent_matches(left: RunnerPatchExtent, right: RunnerPatchExtent) -> (matches: bool)
    ensures matches == (left == right),
{
    matches!(
        (left, right),
        (RunnerPatchExtent::ActiveTokens, RunnerPatchExtent::ActiveTokens)
            | (RunnerPatchExtent::Sequences, RunnerPatchExtent::Sequences)
    )
}

fn patch_slot_matches(
    left: RunnerPatchSlotTemplate,
    right: RunnerPatchSlotTemplate,
) -> (matches: bool)
    ensures matches == (left == right),
{
    left.slot_index == right.slot_index
        && patch_kind_matches(left.kind, right.kind)
        && patch_scalar_matches(left.scalar_type, right.scalar_type)
        && patch_extent_matches(left.extent, right.extent)
}

fn patch_slots_match(
    left: &[RunnerPatchSlotTemplate; 4],
    right: &[RunnerPatchSlotTemplate; 4],
) -> (matches: bool)
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
        if !patch_slot_matches(left[index], right[index]) {
            assert(left@ != right@) by {
                if left@ == right@ {
                    assert(left@[index as int] == right@[index as int]);
                    assert(false);
                }
            }
            return false;
        }
        index += 1;
    }
    assert(left@ =~= right@) by {
        assert forall|position: int| 0 <= position < left@.len()
            implies left@[position] == right@[position] by {}
    }
    true
}

fn identity_values(inputs: &GeneratedRunnerIdentityInputs) -> (values: [Identity; 16])
    ensures values@ == identity_sequence(*inputs),
{
    [
        inputs.source_id,
        inputs.admission_record_id,
        inputs.bundle_id,
        inputs.plan_catalog_id,
        inputs.kernel_catalog_id,
        inputs.closure_id,
        inputs.declaration_id,
        inputs.plan_id,
        inputs.operation_id,
        inputs.kernel_contract_id,
        inputs.artifact_id,
        inputs.descriptor_id,
        inputs.geometry_id,
        inputs.kernarg_layout_id,
        inputs.buffer_layout_id,
        inputs.effect_contract_id,
    ]
}

const fn identity_role(index: usize) -> GeneratedRunnerIdentityRole {
    match index {
        0 => GeneratedRunnerIdentityRole::Source,
        1 => GeneratedRunnerIdentityRole::AdmissionRecord,
        2 => GeneratedRunnerIdentityRole::Bundle,
        3 => GeneratedRunnerIdentityRole::PlanCatalog,
        4 => GeneratedRunnerIdentityRole::KernelCatalog,
        5 => GeneratedRunnerIdentityRole::Closure,
        6 => GeneratedRunnerIdentityRole::Declaration,
        7 => GeneratedRunnerIdentityRole::Plan,
        8 => GeneratedRunnerIdentityRole::Operation,
        9 => GeneratedRunnerIdentityRole::KernelContract,
        10 => GeneratedRunnerIdentityRole::Artifact,
        11 => GeneratedRunnerIdentityRole::Descriptor,
        12 => GeneratedRunnerIdentityRole::Geometry,
        13 => GeneratedRunnerIdentityRole::KernargLayout,
        14 => GeneratedRunnerIdentityRole::BufferLayout,
        _ => GeneratedRunnerIdentityRole::EffectContract,
    }
}

fn validate_identity_expectation(
    inputs: &GeneratedRunnerIdentityInputs,
) -> (result: Result<(), GeneratedRunnerValidationError>)
    ensures result.is_ok() == identities_are_present_and_distinct(*inputs),
{
    let values = identity_values(inputs);
    let mut index = 0usize;
    while index < values.len()
        invariant
            values@ == identity_sequence(*inputs),
            index <= values@.len(),
            forall|prior: int|
                0 <= prior < index ==> identity_present(values@[prior]),
            forall|left: int, right: int|
                0 <= left < right < index
                    ==> values@[left].bytes_spec() != values@[right].bytes_spec(),
        decreases values@.len() - index,
    {
        if !values[index].is_present() {
            assert(!identities_are_present_and_distinct(*inputs)) by {
                reveal(identities_are_present_and_distinct);
                assert(!identity_present(identity_sequence(*inputs)[index as int]));
            }
            return Err(GeneratedRunnerValidationError::ExpectationMissingIdentity(
                identity_role(index),
            ));
        }
        let mut prior = 0usize;
        while prior < index
            invariant
                values@ == identity_sequence(*inputs),
                prior <= index,
                index < values@.len(),
                forall|previous: int|
                    0 <= previous < index ==> identity_present(values@[previous]),
                identity_present(values@[index as int]),
                forall|left: int, right: int|
                    0 <= left < right < index
                        ==> values@[left].bytes_spec() != values@[right].bytes_spec(),
                forall|previous: int|
                    0 <= previous < prior
                        ==> values@[previous].bytes_spec()
                            != values@[index as int].bytes_spec(),
            decreases index - prior,
        {
            if values[index].equals(&values[prior]) {
                assert(!identities_are_present_and_distinct(*inputs)) by {
                    reveal(identities_are_present_and_distinct);
                }
                return Err(GeneratedRunnerValidationError::ExpectationReusedIdentity);
            }
            prior += 1;
        }
        assert forall|left: int, right: int|
            0 <= left < right < index + 1
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
    assert(identities_are_present_and_distinct(*inputs)) by {
        reveal(identities_are_present_and_distinct);
    }
    Ok(())
}

fn validate_identity_match(
    candidate: &GeneratedRunnerIdentityInputs,
    expected: &GeneratedRunnerIdentityInputs,
) -> (result: Result<(), GeneratedRunnerValidationError>)
    ensures result.is_ok() == identity_inputs_match_exactly(*candidate, *expected),
{
    let candidate_values = identity_values(candidate);
    let expected_values = identity_values(expected);
    let mut index = 0usize;
    while index < candidate_values.len()
        invariant
            candidate_values@ == identity_sequence(*candidate),
            expected_values@ == identity_sequence(*expected),
            candidate_values@.len() == expected_values@.len(),
            index <= candidate_values@.len(),
            forall|prior: int|
                0 <= prior < index
                    ==> candidate_values@[prior].bytes_spec()
                        == expected_values@[prior].bytes_spec(),
        decreases candidate_values@.len() - index,
    {
        if !candidate_values[index].equals(&expected_values[index]) {
            assert(!identity_inputs_match_exactly(*candidate, *expected)) by {
                reveal(identity_inputs_match_exactly);
            }
            return Err(GeneratedRunnerValidationError::CandidateIdentityDrift(
                identity_role(index),
            ));
        }
        index += 1;
    }
    assert(identity_inputs_match_exactly(*candidate, *expected)) by {
        reveal(identity_inputs_match_exactly);
        assert forall|position: int|
            0 <= position < identity_sequence(*candidate).len()
                implies identity_sequence(*candidate)[position].bytes_spec()
                    == identity_sequence(*expected)[position].bytes_spec() by {}
    }
    Ok(())
}

fn validate_expectation(
    expected: &GeneratedRunnerInput,
) -> (result: Result<(), GeneratedRunnerValidationError>)
    ensures result.is_ok() == expectation_matches_generated_table(*expected),
{
    if expected.template_version != GENERATED_RUNNER_TEMPLATE_VERSION {
        return Err(GeneratedRunnerValidationError::ExpectationTemplateDrift);
    }
    let template = match generated_plan_template(expected.plan_index) {
        Some(template) => template,
        None => return Err(GeneratedRunnerValidationError::ExpectationPlanDrift),
    };
    if expected.plan_index != template.plan_index
        || !expected.selection.matches(template.selection)
        || expected.operation_start != template.operation_start
        || expected.operation_count != template.operation_count
    {
        return Err(GeneratedRunnerValidationError::ExpectationPlanDrift);
    }
    let operation_end = expected.operation_start + expected.operation_count;
    if expected.operation_index < expected.operation_start
        || expected.operation_index >= operation_end
    {
        return Err(GeneratedRunnerValidationError::ExpectationOperationDrift);
    }
    if !patch_slots_match(&expected.patch_slots, &GENERATED_PATCH_SLOTS) {
        return Err(GeneratedRunnerValidationError::ExpectationPatchSchemaDrift);
    }
    validate_identity_expectation(&expected.identities)?;
    Ok(())
}

/// Validates and retains one exact inert generated-runner operation handoff.
///
/// # Errors
///
/// Returns [`GeneratedRunnerValidationError`] unless the expectation names an
/// exact checked-in plan and operation range, its logical patch schema is
/// canonical, all identities are present and role-distinct, and every candidate
/// field exactly matches that expectation.
///
/// Success authenticates no identity and grants no physical execution,
/// completion, publication, hardware, performance, or qualification authority.
pub fn validate_generated_runner_input(
    candidate: GeneratedRunnerInput,
    expected: GeneratedRunnerInput,
) -> (result: Result<ValidatedGeneratedRunnerInput, GeneratedRunnerValidationError>)
    ensures
        result.is_ok() == generated_runner_input_matches_exactly(candidate, expected),
        match result {
            Ok(validated) => validated.input_spec() == candidate,
            Err(_) => true,
        },
{
    validate_expectation(&expected)?;
    if candidate.template_version != expected.template_version
        || candidate.plan_index != expected.plan_index
        || !candidate.selection.matches(expected.selection)
        || candidate.operation_start != expected.operation_start
        || candidate.operation_count != expected.operation_count
        || candidate.operation_index != expected.operation_index
    {
        return Err(GeneratedRunnerValidationError::CandidateFieldDrift);
    }
    if !patch_slots_match(&candidate.patch_slots, &expected.patch_slots) {
        return Err(GeneratedRunnerValidationError::CandidatePatchSchemaDrift);
    }
    validate_identity_match(&candidate.identities, &expected.identities)?;
    Ok(ValidatedGeneratedRunnerInput { input: candidate })
}

} // verus!

#[cfg(test)]
mod tests {
    use super::{
        generated_plan_template, validate_generated_runner_input, GeneratedRunnerIdentityInputs,
        GeneratedRunnerIdentityRole, GeneratedRunnerInput, GeneratedRunnerValidationError,
    };
    use crate::{
        RunnerPatchExtent, RunnerPatchKind, GENERATED_PATCH_SLOTS, GENERATED_PLAN_TEMPLATES,
        GENERATED_RUNNER_TEMPLATE_VERSION,
    };
    use ferric_spec::{Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket};

    const fn identity(seed: u8) -> Identity {
        Identity::new([seed; 32])
    }

    const fn identities() -> GeneratedRunnerIdentityInputs {
        GeneratedRunnerIdentityInputs {
            source_id: identity(1),
            admission_record_id: identity(2),
            bundle_id: identity(3),
            plan_catalog_id: identity(4),
            kernel_catalog_id: identity(5),
            closure_id: identity(6),
            declaration_id: identity(7),
            plan_id: identity(8),
            operation_id: identity(9),
            kernel_contract_id: identity(10),
            artifact_id: identity(11),
            descriptor_id: identity(12),
            geometry_id: identity(13),
            kernarg_layout_id: identity(14),
            buffer_layout_id: identity(15),
            effect_contract_id: identity(16),
        }
    }

    fn exact_input(plan_index: usize, operation_index: u32) -> GeneratedRunnerInput {
        let template = GENERATED_PLAN_TEMPLATES[plan_index];
        GeneratedRunnerInput {
            template_version: GENERATED_RUNNER_TEMPLATE_VERSION,
            plan_index: template.plan_index,
            selection: template.selection,
            operation_start: template.operation_start,
            operation_count: template.operation_count,
            operation_index,
            patch_slots: GENERATED_PATCH_SLOTS,
            identities: identities(),
        }
    }

    #[test]
    fn every_generated_plan_accepts_its_first_and_last_operation() {
        for (plan_index, template) in GENERATED_PLAN_TEMPLATES.iter().copied().enumerate() {
            for operation_index in [
                template.operation_start,
                template.operation_start + template.operation_count - 1,
            ] {
                let input = exact_input(plan_index, operation_index);
                let validated = validate_generated_runner_input(input, input).unwrap();
                assert_eq!(validated.input(), &input);
                assert_eq!(validated.declaration_id(), input.identities.declaration_id);
                assert_eq!(validated.plan_id(), input.identities.plan_id);
                assert_eq!(validated.operation_id(), input.identities.operation_id);
                assert_eq!(
                    validated.buffer_layout_id(),
                    input.identities.buffer_layout_id
                );
            }
        }
        assert_eq!(generated_plan_template(22), None);
    }

    #[test]
    fn expectation_rejects_static_plan_operation_and_patch_drift() {
        let exact = exact_input(0, 0);

        let mut changed = exact;
        changed.template_version += 1;
        assert_eq!(
            validate_generated_runner_input(exact, changed),
            Err(GeneratedRunnerValidationError::ExpectationTemplateDrift)
        );

        changed = exact;
        changed.plan_index = u16::MAX;
        assert_eq!(
            validate_generated_runner_input(exact, changed),
            Err(GeneratedRunnerValidationError::ExpectationPlanDrift)
        );

        changed = exact;
        changed.selection.mode = Qwen3ExecutionMode::Decode;
        assert_eq!(
            validate_generated_runner_input(exact, changed),
            Err(GeneratedRunnerValidationError::ExpectationPlanDrift)
        );

        changed = exact;
        changed.selection.role = Qwen3ModelRole::Draft06B;
        assert_eq!(
            validate_generated_runner_input(exact, changed),
            Err(GeneratedRunnerValidationError::ExpectationPlanDrift)
        );

        changed = exact;
        changed.selection.bucket = Qwen3PlanBucket::PrefillS8T128;
        assert_eq!(
            validate_generated_runner_input(exact, changed),
            Err(GeneratedRunnerValidationError::ExpectationPlanDrift)
        );

        changed = exact;
        changed.operation_start += 1;
        assert_eq!(
            validate_generated_runner_input(exact, changed),
            Err(GeneratedRunnerValidationError::ExpectationPlanDrift)
        );

        changed = exact;
        changed.operation_count -= 1;
        assert_eq!(
            validate_generated_runner_input(exact, changed),
            Err(GeneratedRunnerValidationError::ExpectationPlanDrift)
        );

        changed = exact;
        changed.operation_index = exact.operation_start + exact.operation_count;
        assert_eq!(
            validate_generated_runner_input(exact, changed),
            Err(GeneratedRunnerValidationError::ExpectationOperationDrift)
        );

        changed = exact;
        changed.patch_slots[0].kind = RunnerPatchKind::PositionIds;
        assert_eq!(
            validate_generated_runner_input(exact, changed),
            Err(GeneratedRunnerValidationError::ExpectationPatchSchemaDrift)
        );

        changed = exact;
        changed.patch_slots[3].extent = RunnerPatchExtent::ActiveTokens;
        assert_eq!(
            validate_generated_runner_input(exact, changed),
            Err(GeneratedRunnerValidationError::ExpectationPatchSchemaDrift)
        );
    }

    #[test]
    fn expectation_rejects_absent_and_reused_identity_authority() {
        let exact = exact_input(0, 0);
        let mut missing = exact;
        missing.identities.source_id = identity(0);
        assert_eq!(
            validate_generated_runner_input(exact, missing),
            Err(GeneratedRunnerValidationError::ExpectationMissingIdentity(
                GeneratedRunnerIdentityRole::Source
            ))
        );

        let mut reused = exact;
        reused.identities.buffer_layout_id = reused.identities.plan_id;
        assert_eq!(
            validate_generated_runner_input(exact, reused),
            Err(GeneratedRunnerValidationError::ExpectationReusedIdentity)
        );
    }

    #[test]
    fn candidate_rejects_every_identity_substitution() {
        let exact = exact_input(21, 10_647);
        let replacements = [
            GeneratedRunnerIdentityRole::Source,
            GeneratedRunnerIdentityRole::AdmissionRecord,
            GeneratedRunnerIdentityRole::Bundle,
            GeneratedRunnerIdentityRole::PlanCatalog,
            GeneratedRunnerIdentityRole::KernelCatalog,
            GeneratedRunnerIdentityRole::Closure,
            GeneratedRunnerIdentityRole::Declaration,
            GeneratedRunnerIdentityRole::Plan,
            GeneratedRunnerIdentityRole::Operation,
            GeneratedRunnerIdentityRole::KernelContract,
            GeneratedRunnerIdentityRole::Artifact,
            GeneratedRunnerIdentityRole::Descriptor,
            GeneratedRunnerIdentityRole::Geometry,
            GeneratedRunnerIdentityRole::KernargLayout,
            GeneratedRunnerIdentityRole::BufferLayout,
            GeneratedRunnerIdentityRole::EffectContract,
        ];
        for (index, role) in replacements.into_iter().enumerate() {
            let mut changed = exact;
            let replacement = identity(64 + u8::try_from(index).expect("16 identity roles"));
            match role {
                GeneratedRunnerIdentityRole::Source => changed.identities.source_id = replacement,
                GeneratedRunnerIdentityRole::AdmissionRecord => {
                    changed.identities.admission_record_id = replacement;
                }
                GeneratedRunnerIdentityRole::Bundle => changed.identities.bundle_id = replacement,
                GeneratedRunnerIdentityRole::PlanCatalog => {
                    changed.identities.plan_catalog_id = replacement;
                }
                GeneratedRunnerIdentityRole::KernelCatalog => {
                    changed.identities.kernel_catalog_id = replacement;
                }
                GeneratedRunnerIdentityRole::Closure => {
                    changed.identities.closure_id = replacement;
                }
                GeneratedRunnerIdentityRole::Declaration => {
                    changed.identities.declaration_id = replacement;
                }
                GeneratedRunnerIdentityRole::Plan => changed.identities.plan_id = replacement,
                GeneratedRunnerIdentityRole::Operation => {
                    changed.identities.operation_id = replacement;
                }
                GeneratedRunnerIdentityRole::KernelContract => {
                    changed.identities.kernel_contract_id = replacement;
                }
                GeneratedRunnerIdentityRole::Artifact => {
                    changed.identities.artifact_id = replacement;
                }
                GeneratedRunnerIdentityRole::Descriptor => {
                    changed.identities.descriptor_id = replacement;
                }
                GeneratedRunnerIdentityRole::Geometry => {
                    changed.identities.geometry_id = replacement;
                }
                GeneratedRunnerIdentityRole::KernargLayout => {
                    changed.identities.kernarg_layout_id = replacement;
                }
                GeneratedRunnerIdentityRole::BufferLayout => {
                    changed.identities.buffer_layout_id = replacement;
                }
                GeneratedRunnerIdentityRole::EffectContract => {
                    changed.identities.effect_contract_id = replacement;
                }
            }
            assert_eq!(
                validate_generated_runner_input(changed, exact),
                Err(GeneratedRunnerValidationError::CandidateIdentityDrift(role))
            );
        }
    }

    #[test]
    fn candidate_rejects_plan_operation_and_patch_substitution() {
        let exact = exact_input(0, 0);
        let mut changed = exact;
        changed.operation_index += 1;
        assert_eq!(
            validate_generated_runner_input(changed, exact),
            Err(GeneratedRunnerValidationError::CandidateFieldDrift)
        );

        changed = exact;
        changed.patch_slots.swap(0, 1);
        assert_eq!(
            validate_generated_runner_input(changed, exact),
            Err(GeneratedRunnerValidationError::CandidatePatchSchemaDrift)
        );
    }
}
