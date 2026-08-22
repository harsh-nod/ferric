//! Deterministic expansion of the checked-in Qwen3 gfx942 runner template.
//!
//! The result is a declaration authority only. It retains authenticated model,
//! plan, kernel-catalog, and preliminary identity inputs, but contains no
//! executable artifact or runtime operation.

use super::{
    decode_bundle_admission_record, digest_bytes, expected_preliminary_kernel_catalog_identity,
    hash_field, sha256::Sha256, BundleAdmissionError, IdentityClosureError,
    PreliminaryIdentityClosure, SEQUENTIAL_PLAN_CATALOG_ENTRIES, SEQUENTIAL_PLAN_CATALOG_VERSION,
};
use ferric_generated_runner::{
    GeneratedPlanTemplate, RunnerPatchExtent, RunnerPatchKind, RunnerPatchScalarType,
    RunnerPatchSlotTemplate, GENERATED_PATCH_SLOTS, GENERATED_PLAN_TEMPLATES,
    GENERATED_RUNNER_OPERATION_COUNT, GENERATED_RUNNER_PLAN_COUNT, GENERATED_RUNNER_PROCESSOR,
    GENERATED_RUNNER_TARGET_FEATURES, GENERATED_RUNNER_TEMPLATE_VERSION,
};
use ferric_kernels::{
    validate_kernel_profile, validate_structural_kernel_catalog, KernelAuthorityRequirements,
    KernelCatalogError, KernelProfileDescriptor, FERRIC_KERNEL_SOURCE_DECLARATIONS,
    GFX942_PROCESSOR, GFX942_TARGET_FEATURES, M1_B3_PLAN_BUCKETS, M1_KERNEL_CATALOG_VERSION,
    M1_KERNEL_OPERATION_BINDINGS, M1_KERNEL_PLAN_COUNT,
};
use ferric_spec::{
    plan_step_count, Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket,
    Qwen3PlanSelection,
};
use std::fmt::Write;

/// Canonical complete runner-declaration record version.
pub const GENERATED_RUNNER_DECLARATION_VERSION: u32 = 1;
const GENERATED_SOURCE_DOMAIN: &[u8] = b"ferric.qwen3.gfx942.generated-runner-source-closure.v2";
const DECLARATION_DOMAIN: &[u8] = b"ferric.qwen3.gfx942.runner-declaration.v1";
const PRELIMINARY_CLOSURE_DOMAIN: &[u8] = b"ferric.qwen3.preliminary-identity-closure.v1";
const GENERATED_LIBRARY_SOURCE_PATH: &[u8] = b"src/lib.rs";
const GENERATED_VALIDATION_SOURCE_PATH: &[u8] = b"src/validation.rs";
const GENERATED_VALIDATION_SOURCE_BYTES: u64 = 31_470;
const GENERATED_VALIDATION_SOURCE_SHA256: [u8; 32] = [
    0xc7, 0xe8, 0x2b, 0x2e, 0x2f, 0x71, 0x22, 0x6a, 0x98, 0x47, 0xd6, 0x1f, 0x3d, 0x2d, 0xc7, 0x69,
    0x6c, 0x0b, 0x9f, 0x50, 0xf7, 0x59, 0xe8, 0xea, 0x99, 0xb2, 0xd9, 0x56, 0x65, 0x5e, 0xda, 0x50,
];

const EXPECTED_PATCH_SLOTS: [RunnerPatchSlotTemplate; 4] = [
    RunnerPatchSlotTemplate {
        slot_index: 0,
        kind: RunnerPatchKind::TokenIds,
        scalar_type: RunnerPatchScalarType::U32,
        extent: RunnerPatchExtent::ActiveTokens,
    },
    RunnerPatchSlotTemplate {
        slot_index: 1,
        kind: RunnerPatchKind::PositionIds,
        scalar_type: RunnerPatchScalarType::U32,
        extent: RunnerPatchExtent::ActiveTokens,
    },
    RunnerPatchSlotTemplate {
        slot_index: 2,
        kind: RunnerPatchKind::ActiveLengths,
        scalar_type: RunnerPatchScalarType::U32,
        extent: RunnerPatchExtent::Sequences,
    },
    RunnerPatchSlotTemplate {
        slot_index: 3,
        kind: RunnerPatchKind::ContextLengths,
        scalar_type: RunnerPatchScalarType::U32,
        extent: RunnerPatchExtent::Sequences,
    },
];

/// One exact finite plan and its contiguous flattened operation range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeneratedPlanDeclaration {
    /// Zero-based target-then-draft plan position.
    pub plan_index: u16,
    /// Complete plan identity constructed from authenticated inputs.
    pub plan_id: Identity,
    /// Exact role, execution mode, and B3 bucket.
    pub selection: Qwen3PlanSelection,
    /// First operation in the declaration's flattened operation sequence.
    pub operation_start: u32,
    /// Exact contiguous operation count.
    pub operation_count: u32,
}

/// One exact typed graph operation and its finite kernel profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeneratedOperationDeclaration {
    /// Zero-based position across all 22 exact plans.
    pub operation_index: u32,
    /// Zero-based target-then-draft plan position.
    pub plan_index: u16,
    /// Exact plan, selection, operator, geometry, buffers, shapes, and K1-K7 family.
    pub profile: KernelProfileDescriptor,
}

/// Retained authenticated inputs plus a complete inert runner declaration.
///
/// This non-clone value is not a compiled artifact and grants no allocation,
/// address, queue, load, launch, completion, proof, hardware, performance, or
/// qualification authority.
#[derive(Debug, Eq, PartialEq)]
pub struct GeneratedRunnerDeclaration {
    closure: PreliminaryIdentityClosure,
    source_id: Identity,
    admission_record_id: Identity,
    plan_catalog_id: Identity,
    kernel_catalog_id: Identity,
    closure_id: Identity,
    declaration_id: Identity,
    plans: Box<[GeneratedPlanDeclaration]>,
    operations: Box<[GeneratedOperationDeclaration]>,
    patch_slots: Box<[RunnerPatchSlotTemplate]>,
    canonical_bytes: Box<[u8]>,
}

impl GeneratedRunnerDeclaration {
    /// Returns the retained preliminary identity closure.
    #[must_use]
    pub const fn closure(&self) -> &PreliminaryIdentityClosure {
        &self.closure
    }

    /// Returns the byte identity of the complete checked-in runner source closure.
    #[must_use]
    pub const fn source_id(&self) -> Identity {
        self.source_id
    }

    /// Returns the retained authenticated bundle-admission record identity.
    #[must_use]
    pub const fn admission_record_id(&self) -> Identity {
        self.admission_record_id
    }

    /// Returns the exact sequential plan-catalog identity.
    #[must_use]
    pub const fn plan_catalog_id(&self) -> Identity {
        self.plan_catalog_id
    }

    /// Returns the exact structural K1-K7 catalog identity.
    #[must_use]
    pub const fn kernel_catalog_id(&self) -> Identity {
        self.kernel_catalog_id
    }

    /// Returns the complete retained preliminary closure identity.
    #[must_use]
    pub const fn closure_id(&self) -> Identity {
        self.closure_id
    }

    /// Returns the domain-separated identity of this complete declaration.
    #[must_use]
    pub const fn declaration_id(&self) -> Identity {
        self.declaration_id
    }

    /// Returns every exact target-then-draft plan declaration.
    #[must_use]
    pub fn plans(&self) -> &[GeneratedPlanDeclaration] {
        &self.plans
    }

    /// Returns all exact typed operations in plan/ordinal order.
    #[must_use]
    pub fn operations(&self) -> &[GeneratedOperationDeclaration] {
        &self.operations
    }

    /// Returns the request-independent logical scalar-input schema.
    #[must_use]
    pub fn patch_slots(&self) -> &[RunnerPatchSlotTemplate] {
        &self.patch_slots
    }

    /// Returns the complete canonical declaration record.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Returns the canonical declaration version.
    #[must_use]
    pub const fn version(&self) -> u32 {
        GENERATED_RUNNER_DECLARATION_VERSION
    }
}

/// One linearly published, fully retained generated-runner declaration.
///
/// This value can only be created by consuming a [`GeneratedRunnerDeclaration`]
/// through [`publish_qwen3_gfx942_runner_declaration`]. It deliberately keeps
/// the authenticated admission, exact prepacked manifests, plan catalog,
/// structural kernel catalog, preliminary closure, generated template, and
/// canonical declaration in build custody. It is not `Clone` and grants no
/// artifact, allocation, address, load, queue, launch, completion, hardware,
/// performance, graph-refinement, proof, or qualification authority.
#[derive(Debug, Eq, PartialEq)]
pub struct PublishedRunnerDeclaration {
    declaration: GeneratedRunnerDeclaration,
}

impl PublishedRunnerDeclaration {
    /// Returns the exact generated source-closure identity retained by publication.
    #[must_use]
    pub const fn source_id(&self) -> Identity {
        self.declaration.source_id
    }

    /// Returns the retained authenticated admission-record identity.
    #[must_use]
    pub const fn admission_record_id(&self) -> Identity {
        self.declaration.admission_record_id
    }

    /// Returns the exact admitted deployment identity.
    #[must_use]
    pub const fn bundle_id(&self) -> Identity {
        self.declaration.closure.catalog().deployment().bundle_id
    }

    /// Returns the authenticated target prepacked-manifest identity.
    #[must_use]
    pub const fn target_prepacked_id(&self) -> Identity {
        Identity::new(
            self.declaration
                .closure
                .catalog()
                .prepacked()
                .target_manifest()
                .aggregate_id(),
        )
    }

    /// Returns the authenticated draft prepacked-manifest identity.
    #[must_use]
    pub const fn draft_prepacked_id(&self) -> Identity {
        Identity::new(
            self.declaration
                .closure
                .catalog()
                .prepacked()
                .draft_manifest()
                .aggregate_id(),
        )
    }

    /// Returns the exact sequential plan-catalog identity.
    #[must_use]
    pub const fn plan_catalog_id(&self) -> Identity {
        self.declaration.plan_catalog_id
    }

    /// Returns the exact structural K1-K7 catalog identity.
    #[must_use]
    pub const fn kernel_catalog_id(&self) -> Identity {
        self.declaration.kernel_catalog_id
    }

    /// Returns the retained preliminary identity-closure identity.
    #[must_use]
    pub const fn closure_id(&self) -> Identity {
        self.declaration.closure_id
    }

    /// Returns the complete canonical generated-declaration identity.
    #[must_use]
    pub const fn declaration_id(&self) -> Identity {
        self.declaration.declaration_id
    }

    /// Returns the exact generated declaration format version.
    #[must_use]
    pub const fn declaration_version(&self) -> u32 {
        GENERATED_RUNNER_DECLARATION_VERSION
    }

    /// Returns the exact checked-in generated-template format version.
    #[must_use]
    pub const fn template_version(&self) -> u32 {
        GENERATED_RUNNER_TEMPLATE_VERSION
    }

    /// Returns every retained target-then-draft plan binding.
    #[must_use]
    pub fn plans(&self) -> &[GeneratedPlanDeclaration] {
        &self.declaration.plans
    }

    /// Returns all exact typed operations in plan/ordinal order.
    #[must_use]
    pub fn operations(&self) -> &[GeneratedOperationDeclaration] {
        &self.declaration.operations
    }

    /// Returns the exact logical request-input schema.
    #[must_use]
    pub fn patch_slots(&self) -> &[RunnerPatchSlotTemplate] {
        &self.declaration.patch_slots
    }

    /// Returns the canonical record retained by the declaration identity.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.declaration.canonical_bytes
    }
}

/// Fail-closed generated runner declaration error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GeneratedRunnerError {
    /// The checked-in generated declaration format or target tuple drifted.
    TemplateHeaderDrift,
    /// The exact generated plan count drifted.
    TemplatePlanCount {
        /// Required count.
        expected: usize,
        /// Observed count.
        actual: usize,
    },
    /// One plan selection, position, operation offset, or count drifted.
    TemplatePlanDrift {
        /// Position at which drift was observed.
        plan_index: usize,
    },
    /// The request-independent logical patch schema drifted.
    PatchSchemaDrift,
    /// The closure did not retain authenticated bundle admission.
    MissingAuthenticatedAdmission,
    /// The retained admission record failed canonical decoding.
    AdmissionRecord(BundleAdmissionError),
    /// The admission record and retained prepacked deployment differ.
    AdmissionDeploymentDrift,
    /// The plan catalog has an unsupported version, count, or identity.
    PlanCatalogDrift,
    /// The structural kernel catalog failed independent reconstruction.
    KernelCatalog(KernelCatalogError),
    /// The structural kernel catalog identity no longer matches the closure.
    KernelCatalogIdentityDrift,
    /// The preliminary closure's canonical bytes no longer match its identity.
    ClosureIdentityDrift,
    /// The closure's generated-source identity is not this exact two-file closure.
    SourceIdentityDrift,
    /// A flattened operation position or typed profile drifted.
    OperationDrift {
        /// Flattened operation position at which drift was observed.
        operation_index: usize,
    },
    /// The complete flattened operation count drifted.
    OperationCount {
        /// Required count.
        expected: usize,
        /// Observed count.
        actual: usize,
    },
    /// The retained declaration fields or canonical identity drifted.
    DeclarationDrift,
    /// A preliminary identity dependency failed closed.
    IdentityClosure(IdentityClosureError),
}

/// Renders the complete checked-in generated source deterministically.
///
/// The renderer is pure with respect to model requests and runtime state. Its
/// only output is Rust source for the fixed declaration table.
#[must_use]
pub fn render_qwen3_gfx942_runner_source() -> Vec<u8> {
    let header = String::from(
        "#![forbid(unsafe_code)]\n\n\
//! Generated, inert Qwen3 target/draft runner declarations for gfx942.\n//!\n\
//! Regenerate with `ferric_build::render_qwen3_gfx942_runner_source`. These\n\
//! declarations are request-independent data. They authorize no artifact,\n\
//! allocation, address, queue, load, launch, completion, hardware, proof,\n\
//! performance, or qualification action.\n\n\
use ferric_spec::{Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection};\n\
#[allow(unused_imports)]\n\
use vstd::prelude::*;\n\n\
/// Exact target processor named by the declaration template.\n\
pub const GENERATED_RUNNER_PROCESSOR: &str = \"gfx942\";\n\
/// Exact target features named by the declaration template.\n\
pub const GENERATED_RUNNER_TARGET_FEATURES: &str = \"+wavefrontsize64,-xnack\";\n\n\
verus! {\n\n\
/// Canonical generated runner declaration format.\n\
pub const GENERATED_RUNNER_TEMPLATE_VERSION: u32 = 1;\n\
/// Exact number of finite target/draft B3 plan declarations.\n\
pub const GENERATED_RUNNER_PLAN_COUNT: usize = 22;\n\
/// Exact number of ordered operation declarations across all plans.\n\
pub const GENERATED_RUNNER_OPERATION_COUNT: usize = 10_648;\n\n\
/// One exact plan position in the generated target-then-draft declaration.\n\
#[derive(Clone, Copy, Debug, Eq, PartialEq)]\n\
pub struct GeneratedPlanTemplate {\n\
    /// Zero-based target-then-draft plan index.\n\
    pub plan_index: u16,\n\
    /// Exact role, execution mode, and finite B3 bucket.\n\
    pub selection: Qwen3PlanSelection,\n\
    /// First operation in the flattened declaration sequence.\n\
    pub operation_start: u32,\n\
    /// Exact operation count for the selected model role.\n\
    pub operation_count: u32,\n\
}\n\n\
/// Logical scalar input whose value may vary between admitted requests.\n\
#[derive(Clone, Copy, Debug, Eq, PartialEq)]\n\
pub enum RunnerPatchKind {\n\
    /// Input token identifiers.\n\
    TokenIds,\n\
    /// Input position identifiers.\n\
    PositionIds,\n\
    /// Per-sequence active-token lengths.\n\
    ActiveLengths,\n\
    /// Per-sequence committed-context lengths.\n\
    ContextLengths,\n\
}\n\n\
/// Logical element type of a request-independent patch slot.\n\
#[derive(Clone, Copy, Debug, Eq, PartialEq)]\n\
pub enum RunnerPatchScalarType {\n\
    /// Unsigned 32-bit scalar.\n\
    U32,\n\
}\n\n\
/// Logical extent of a request-independent patch slot.\n\
#[derive(Clone, Copy, Debug, Eq, PartialEq)]\n\
pub enum RunnerPatchExtent {\n\
    /// One scalar for each active token in the selected finite bucket.\n\
    ActiveTokens,\n\
    /// One scalar for each sequence in the selected finite bucket.\n\
    Sequences,\n\
}\n\n\
/// One logical input schema entry, never a value, pointer, or device address.\n\
#[derive(Clone, Copy, Debug, Eq, PartialEq)]\n\
pub struct RunnerPatchSlotTemplate {\n\
    /// Stable zero-based schema position.\n\
    pub slot_index: u16,\n\
    /// Logical value supplied later by an independently checked runtime.\n\
    pub kind: RunnerPatchKind,\n\
    /// Exact scalar representation.\n\
    pub scalar_type: RunnerPatchScalarType,\n\
    /// Bucket-relative logical element count.\n\
    pub extent: RunnerPatchExtent,\n\
}\n\n\
/// Complete exact plan roster in target-then-draft B3 order.\n\
pub const GENERATED_PLAN_TEMPLATES: [GeneratedPlanTemplate; GENERATED_RUNNER_PLAN_COUNT] = [\n",
    );
    let mut source = indent_generated_declaration_fields(&header);
    for template in GENERATED_PLAN_TEMPLATES {
        write_plan_template(&mut source, template);
    }
    source.push_str(
        "];\n\n\
/// Complete request-independent logical input schema.\n\
pub const GENERATED_PATCH_SLOTS: [RunnerPatchSlotTemplate; 4] = [\n",
    );
    for slot in GENERATED_PATCH_SLOTS {
        write_patch_slot(&mut source, slot);
    }
    source.push_str(
        "];\n\n\
} // verus!\n\n\
mod validation;\n\n\
pub use validation::{\n\
\x20\x20\x20\x20generated_plan_template, validate_generated_runner_input, GeneratedRunnerIdentityInputs,\n\
\x20\x20\x20\x20GeneratedRunnerIdentityRole, GeneratedRunnerInput, GeneratedRunnerValidationError,\n\
\x20\x20\x20\x20ValidatedGeneratedRunnerInput,\n\
};\n",
    );
    source.into_bytes()
}

/// Renders the canonical two-file generated-runner source-closure record.
///
/// The supplied library bytes and validation length/digest are framed with
/// their exact workspace-relative paths. This function constructs data only
/// and does not validate or authenticate any input.
#[must_use]
pub fn render_qwen3_gfx942_runner_source_closure(
    library_source: &[u8],
    validation_source_bytes: u64,
    validation_source_sha256: [u8; 32],
) -> Vec<u8> {
    let mut closure = Vec::with_capacity(96 + library_source.len());
    closure.extend_from_slice(&2_u32.to_le_bytes());
    push_bytes(&mut closure, GENERATED_LIBRARY_SOURCE_PATH);
    push_bytes(&mut closure, library_source);
    push_bytes(&mut closure, GENERATED_VALIDATION_SOURCE_PATH);
    closure.extend_from_slice(&validation_source_bytes.to_le_bytes());
    closure.extend_from_slice(&validation_source_sha256);
    closure
}

/// Computes the domain-separated identity of the exact runner source closure.
#[must_use]
pub fn expected_qwen3_gfx942_runner_source_identity() -> Identity {
    let library_source = render_qwen3_gfx942_runner_source();
    let closure = render_qwen3_gfx942_runner_source_closure(
        &library_source,
        GENERATED_VALIDATION_SOURCE_BYTES,
        GENERATED_VALIDATION_SOURCE_SHA256,
    );
    identity_record(GENERATED_SOURCE_DOMAIN, &closure)
}

/// Validates the exact generated library and pinned validation-module digest.
///
/// # Errors
///
/// Returns [`GeneratedRunnerError::SourceIdentityDrift`] for any library or
/// validation-module digest or length drift. This equality check does not prove
/// SHA-256 collision resistance.
pub fn validate_qwen3_gfx942_runner_source_closure(
    library_source: &[u8],
    validation_source: &[u8],
) -> Result<Identity, GeneratedRunnerError> {
    let validation_digest = digest_bytes(validation_source);
    if library_source != render_qwen3_gfx942_runner_source()
        || validation_digest.byte_len != GENERATED_VALIDATION_SOURCE_BYTES
        || validation_digest.sha256 != GENERATED_VALIDATION_SOURCE_SHA256
    {
        return Err(GeneratedRunnerError::SourceIdentityDrift);
    }
    let closure = render_qwen3_gfx942_runner_source_closure(
        library_source,
        validation_digest.byte_len,
        validation_digest.sha256,
    );
    Ok(identity_record(GENERATED_SOURCE_DOMAIN, &closure))
}

/// Consumes the preliminary closure and builds the complete inert declaration.
///
/// # Errors
///
/// Returns [`GeneratedRunnerError`] for any generated source, authenticated
/// admission, plan, catalog, kernel, closure, operation, shape, buffer, or
/// logical patch-schema drift.
pub fn generate_qwen3_gfx942_runner_declaration(
    closure: PreliminaryIdentityClosure,
) -> Result<GeneratedRunnerDeclaration, GeneratedRunnerError> {
    validate_template_header()?;
    validate_plan_templates(&GENERATED_PLAN_TEMPLATES)?;
    validate_patch_slots(&GENERATED_PATCH_SLOTS)?;
    validate_closure(&closure)?;

    let source_id = expected_qwen3_gfx942_runner_source_identity();
    let admission_record_id = closure
        .catalog()
        .admission_record()
        .ok_or(GeneratedRunnerError::MissingAuthenticatedAdmission)?
        .record_id();
    let plan_catalog_id = closure.catalog().catalog_id();
    let kernel_catalog_id = closure.external().kernel_catalog;
    let closure_id = closure.closure_id();

    let plans = build_plan_declarations(&closure)?;
    let operations = build_operation_declarations(&closure)?;
    let patch_slots = GENERATED_PATCH_SLOTS.to_vec().into_boxed_slice();
    let canonical_bytes = canonical_record(
        &closure,
        source_id,
        admission_record_id,
        &plans,
        &operations,
        &patch_slots,
    );
    let declaration_id = identity_record(DECLARATION_DOMAIN, &canonical_bytes);
    let declaration = GeneratedRunnerDeclaration {
        closure,
        source_id,
        admission_record_id,
        plan_catalog_id,
        kernel_catalog_id,
        closure_id,
        declaration_id,
        plans,
        operations,
        patch_slots,
        canonical_bytes: canonical_bytes.into_boxed_slice(),
    };
    validate_qwen3_gfx942_runner_declaration(&declaration)?;
    Ok(declaration)
}

/// Consumes and linearly publishes an exact generated-runner declaration.
///
/// Publication revalidates the complete retained declaration immediately
/// before crossing into runtime custody. Failure consumes the candidate and
/// yields no published value.
///
/// # Errors
///
/// Returns [`GeneratedRunnerError`] for any authenticated admission, source,
/// prepacked manifest, plan, operation, kernel catalog, closure, template,
/// patch schema, canonical record, or identity drift.
pub fn publish_qwen3_gfx942_runner_declaration(
    declaration: GeneratedRunnerDeclaration,
) -> Result<PublishedRunnerDeclaration, GeneratedRunnerError> {
    validate_qwen3_gfx942_runner_declaration(&declaration)?;
    Ok(PublishedRunnerDeclaration { declaration })
}

/// Revalidates every retained generated declaration field independently.
///
/// # Errors
///
/// Returns [`GeneratedRunnerError`] if any authority, plan, operation, typed
/// buffer/shape profile, logical patch schema, canonical byte, or identity
/// field differs from the unique expected declaration.
pub fn validate_qwen3_gfx942_runner_declaration(
    declaration: &GeneratedRunnerDeclaration,
) -> Result<(), GeneratedRunnerError> {
    validate_template_header()?;
    validate_plan_templates(&GENERATED_PLAN_TEMPLATES)?;
    validate_patch_slots(&declaration.patch_slots)?;
    validate_closure(&declaration.closure)?;

    let expected_source_id = expected_qwen3_gfx942_runner_source_identity();
    let expected_admission_record_id = declaration
        .closure
        .catalog()
        .admission_record()
        .ok_or(GeneratedRunnerError::MissingAuthenticatedAdmission)?
        .record_id();
    let expected_plans = build_plan_declarations(&declaration.closure)?;
    let expected_operations = build_operation_declarations(&declaration.closure)?;
    if declaration.version() != GENERATED_RUNNER_DECLARATION_VERSION
        || declaration.source_id != expected_source_id
        || declaration.admission_record_id != expected_admission_record_id
        || declaration.plan_catalog_id != declaration.closure.catalog().catalog_id()
        || declaration.kernel_catalog_id != declaration.closure.external().kernel_catalog
        || declaration.closure_id != declaration.closure.closure_id()
        || declaration.plans.as_ref() != expected_plans.as_ref()
        || declaration.operations.as_ref() != expected_operations.as_ref()
        || declaration.patch_slots.as_ref() != GENERATED_PATCH_SLOTS
    {
        return Err(GeneratedRunnerError::DeclarationDrift);
    }
    let expected_bytes = canonical_record(
        &declaration.closure,
        expected_source_id,
        expected_admission_record_id,
        &expected_plans,
        &expected_operations,
        &declaration.patch_slots,
    );
    if declaration.canonical_bytes.as_ref() != expected_bytes
        || declaration.declaration_id != identity_record(DECLARATION_DOMAIN, &expected_bytes)
    {
        return Err(GeneratedRunnerError::DeclarationDrift);
    }
    Ok(())
}

fn validate_template_header() -> Result<(), GeneratedRunnerError> {
    if GENERATED_RUNNER_TEMPLATE_VERSION != 1
        || GENERATED_RUNNER_PROCESSOR != GFX942_PROCESSOR
        || GENERATED_RUNNER_TARGET_FEATURES != GFX942_TARGET_FEATURES
        || GENERATED_RUNNER_PLAN_COUNT != M1_KERNEL_PLAN_COUNT
        || GENERATED_RUNNER_OPERATION_COUNT != M1_KERNEL_OPERATION_BINDINGS
    {
        return Err(GeneratedRunnerError::TemplateHeaderDrift);
    }
    Ok(())
}

fn validate_plan_templates(
    templates: &[GeneratedPlanTemplate],
) -> Result<(), GeneratedRunnerError> {
    if templates.len() != SEQUENTIAL_PLAN_CATALOG_ENTRIES {
        return Err(GeneratedRunnerError::TemplatePlanCount {
            expected: SEQUENTIAL_PLAN_CATALOG_ENTRIES,
            actual: templates.len(),
        });
    }
    let mut operation_start = 0_u32;
    for (index, template) in templates.iter().copied().enumerate() {
        let role = if index < M1_B3_PLAN_BUCKETS.len() {
            Qwen3ModelRole::Target8B
        } else {
            Qwen3ModelRole::Draft06B
        };
        let (mode, bucket) = M1_B3_PLAN_BUCKETS[index % M1_B3_PLAN_BUCKETS.len()];
        let expected_selection = Qwen3PlanSelection { role, mode, bucket };
        let operation_count = plan_step_count(role);
        if usize::from(template.plan_index) != index
            || template.selection != expected_selection
            || template.operation_start != operation_start
            || template.operation_count != operation_count
        {
            return Err(GeneratedRunnerError::TemplatePlanDrift { plan_index: index });
        }
        operation_start = operation_start
            .checked_add(operation_count)
            .ok_or(GeneratedRunnerError::TemplatePlanDrift { plan_index: index })?;
    }
    if usize::try_from(operation_start).ok() != Some(GENERATED_RUNNER_OPERATION_COUNT) {
        return Err(GeneratedRunnerError::OperationCount {
            expected: GENERATED_RUNNER_OPERATION_COUNT,
            actual: usize::try_from(operation_start).unwrap_or(usize::MAX),
        });
    }
    Ok(())
}

fn validate_patch_slots(slots: &[RunnerPatchSlotTemplate]) -> Result<(), GeneratedRunnerError> {
    if slots != EXPECTED_PATCH_SLOTS {
        return Err(GeneratedRunnerError::PatchSchemaDrift);
    }
    Ok(())
}

fn validate_closure(closure: &PreliminaryIdentityClosure) -> Result<(), GeneratedRunnerError> {
    let catalog = closure.catalog();
    if closure.version() != super::PRELIMINARY_IDENTITY_CLOSURE_VERSION
        || catalog.version() != SEQUENTIAL_PLAN_CATALOG_VERSION
        || catalog.plans().len() != SEQUENTIAL_PLAN_CATALOG_ENTRIES
        || !catalog.catalog_id().is_present()
    {
        return Err(GeneratedRunnerError::PlanCatalogDrift);
    }
    let record = catalog
        .admission_record()
        .ok_or(GeneratedRunnerError::MissingAuthenticatedAdmission)?;
    let decoded = decode_bundle_admission_record(record.as_bytes())
        .map_err(GeneratedRunnerError::AdmissionRecord)?;
    if decoded.record_id != record.record_id() || decoded.deployment != *catalog.deployment() {
        return Err(GeneratedRunnerError::AdmissionDeploymentDrift);
    }
    if closure.external().generated_runner != expected_qwen3_gfx942_runner_source_identity() {
        return Err(GeneratedRunnerError::SourceIdentityDrift);
    }
    let expected_kernel_id =
        expected_preliminary_kernel_catalog_identity(catalog, &closure.external())
            .map_err(GeneratedRunnerError::IdentityClosure)?;
    if expected_kernel_id != closure.external().kernel_catalog {
        return Err(GeneratedRunnerError::KernelCatalogIdentityDrift);
    }
    validate_structural_kernel_catalog(
        closure.kernel_catalog(),
        catalog.plans(),
        catalog.catalog_id(),
        &FERRIC_KERNEL_SOURCE_DECLARATIONS,
        kernel_authorities(closure),
    )
    .map_err(GeneratedRunnerError::KernelCatalog)?;
    if closure.kernel_catalog().version() != M1_KERNEL_CATALOG_VERSION
        || closure.kernel_catalog().plan_catalog_id() != catalog.catalog_id()
    {
        return Err(GeneratedRunnerError::KernelCatalogIdentityDrift);
    }
    let closure_identity = identity_record(PRELIMINARY_CLOSURE_DOMAIN, closure.canonical_bytes());
    if closure_identity != closure.closure_id() {
        return Err(GeneratedRunnerError::ClosureIdentityDrift);
    }
    Ok(())
}

fn kernel_authorities(closure: &PreliminaryIdentityClosure) -> KernelAuthorityRequirements {
    let external = closure.external();
    KernelAuthorityRequirements {
        fe2o3_source: external.fe2o3_source,
        compiler: external.compiler,
        compiler_configuration: external.compiler_configuration,
        target_contract: external.target_contract,
        kernel_proof_set: external.kernel_proof_set,
        kernel_abi_catalog: external.kernel_abi_catalog,
        runtime_contract: external.runtime_contract,
        runtime_abi: external.runtime_abi,
        tcb_report: external.tcb_report,
    }
}

fn build_plan_declarations(
    closure: &PreliminaryIdentityClosure,
) -> Result<Box<[GeneratedPlanDeclaration]>, GeneratedRunnerError> {
    let mut plans = Vec::with_capacity(GENERATED_RUNNER_PLAN_COUNT);
    for (template, plan) in GENERATED_PLAN_TEMPLATES
        .iter()
        .copied()
        .zip(closure.catalog().plans())
    {
        let plan_index = usize::from(template.plan_index);
        if plan.selection != template.selection
            || plan.steps.len() != template.operation_count as usize
            || !plan.authority.plan_id.is_present()
            || plan.validate(plan.authority, template.selection).is_err()
        {
            return Err(GeneratedRunnerError::TemplatePlanDrift { plan_index });
        }
        plans.push(GeneratedPlanDeclaration {
            plan_index: template.plan_index,
            plan_id: plan.authority.plan_id,
            selection: plan.selection,
            operation_start: template.operation_start,
            operation_count: template.operation_count,
        });
    }
    if plans.len() != GENERATED_RUNNER_PLAN_COUNT {
        return Err(GeneratedRunnerError::TemplatePlanCount {
            expected: GENERATED_RUNNER_PLAN_COUNT,
            actual: plans.len(),
        });
    }
    Ok(plans.into_boxed_slice())
}

fn build_operation_declarations(
    closure: &PreliminaryIdentityClosure,
) -> Result<Box<[GeneratedOperationDeclaration]>, GeneratedRunnerError> {
    let catalog = closure.catalog();
    let bindings = closure.kernel_catalog().bindings();
    if bindings.len() != GENERATED_RUNNER_OPERATION_COUNT {
        return Err(GeneratedRunnerError::OperationCount {
            expected: GENERATED_RUNNER_OPERATION_COUNT,
            actual: bindings.len(),
        });
    }
    let mut operations = Vec::with_capacity(GENERATED_RUNNER_OPERATION_COUNT);
    let mut operation_index = 0_usize;
    for (plan_index, plan) in catalog.plans().iter().enumerate() {
        for step in &plan.steps {
            let binding = bindings
                .get(operation_index)
                .ok_or(GeneratedRunnerError::OperationDrift { operation_index })?;
            if usize::from(binding.plan_index) != plan_index
                || binding.profile.plan_id != plan.authority.plan_id
                || binding.profile.selection != plan.selection
                || binding.profile.step != *step
                || validate_kernel_profile(binding.profile, plan, step.ordinal).is_err()
            {
                return Err(GeneratedRunnerError::OperationDrift { operation_index });
            }
            operations.push(GeneratedOperationDeclaration {
                operation_index: u32::try_from(operation_index)
                    .map_err(|_| GeneratedRunnerError::OperationDrift { operation_index })?,
                plan_index: binding.plan_index,
                profile: binding.profile,
            });
            operation_index += 1;
        }
    }
    if operation_index != GENERATED_RUNNER_OPERATION_COUNT {
        return Err(GeneratedRunnerError::OperationCount {
            expected: GENERATED_RUNNER_OPERATION_COUNT,
            actual: operation_index,
        });
    }
    Ok(operations.into_boxed_slice())
}

fn canonical_record(
    closure: &PreliminaryIdentityClosure,
    source_id: Identity,
    admission_record_id: Identity,
    plans: &[GeneratedPlanDeclaration],
    operations: &[GeneratedOperationDeclaration],
    patch_slots: &[RunnerPatchSlotTemplate],
) -> Vec<u8> {
    let mut record = Vec::with_capacity(
        256 + plans.len() * 48
            + closure.kernel_catalog().canonical_bytes().len()
            + patch_slots.len() * 4,
    );
    record.extend_from_slice(&GENERATED_RUNNER_DECLARATION_VERSION.to_le_bytes());
    push_bytes(&mut record, GENERATED_RUNNER_PROCESSOR.as_bytes());
    push_bytes(&mut record, GENERATED_RUNNER_TARGET_FEATURES.as_bytes());
    for identity in [
        source_id,
        admission_record_id,
        closure.catalog().deployment().bundle_id,
        closure.catalog().catalog_id(),
        closure.external().kernel_catalog,
        closure.closure_id(),
    ] {
        record.extend_from_slice(identity.as_bytes());
    }
    record.extend_from_slice(&(plans.len() as u64).to_le_bytes());
    for plan in plans {
        record.extend_from_slice(&plan.plan_index.to_le_bytes());
        record.extend_from_slice(plan.plan_id.as_bytes());
        encode_selection(&mut record, plan.selection);
        record.extend_from_slice(&plan.operation_start.to_le_bytes());
        record.extend_from_slice(&plan.operation_count.to_le_bytes());
    }
    record.extend_from_slice(&(operations.len() as u64).to_le_bytes());
    push_bytes(&mut record, closure.kernel_catalog().canonical_bytes());
    record.extend_from_slice(&(patch_slots.len() as u64).to_le_bytes());
    for slot in patch_slots {
        record.extend_from_slice(&slot.slot_index.to_le_bytes());
        record.push(patch_kind_tag(slot.kind));
        record.push(patch_scalar_tag(slot.scalar_type));
        record.push(patch_extent_tag(slot.extent));
    }
    record
}

fn write_plan_template(source: &mut String, template: GeneratedPlanTemplate) {
    writeln!(source, "    GeneratedPlanTemplate {{").expect("writing to String cannot fail");
    writeln!(source, "        plan_index: {},", template.plan_index)
        .expect("writing to String cannot fail");
    writeln!(source, "        selection: Qwen3PlanSelection {{")
        .expect("writing to String cannot fail");
    writeln!(
        source,
        "            role: Qwen3ModelRole::{},",
        role_name(template.selection.role)
    )
    .expect("writing to String cannot fail");
    writeln!(
        source,
        "            mode: Qwen3ExecutionMode::{},",
        mode_name(template.selection.mode)
    )
    .expect("writing to String cannot fail");
    writeln!(
        source,
        "            bucket: Qwen3PlanBucket::{},",
        bucket_name(template.selection.bucket)
    )
    .expect("writing to String cannot fail");
    writeln!(source, "        }},").expect("writing to String cannot fail");
    writeln!(
        source,
        "        operation_start: {},",
        template.operation_start
    )
    .expect("writing to String cannot fail");
    writeln!(
        source,
        "        operation_count: {},",
        template.operation_count
    )
    .expect("writing to String cannot fail");
    writeln!(source, "    }},").expect("writing to String cannot fail");
}

fn indent_generated_declaration_fields(header: &str) -> String {
    let mut source = String::with_capacity(header.len() + 128);
    let mut declaration_depth = 0_u8;
    for line in header.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed == "}" {
            declaration_depth = declaration_depth.saturating_sub(1);
        }
        if declaration_depth != 0 && !trimmed.is_empty() {
            source.push_str("    ");
        }
        source.push_str(line);
        if trimmed.ends_with('{') && trimmed != "verus! {" {
            declaration_depth = declaration_depth.saturating_add(1);
        }
    }
    source
}

fn write_patch_slot(source: &mut String, slot: RunnerPatchSlotTemplate) {
    writeln!(source, "    RunnerPatchSlotTemplate {{").expect("writing to String cannot fail");
    writeln!(source, "        slot_index: {},", slot.slot_index)
        .expect("writing to String cannot fail");
    writeln!(
        source,
        "        kind: RunnerPatchKind::{},",
        patch_kind_name(slot.kind)
    )
    .expect("writing to String cannot fail");
    writeln!(
        source,
        "        scalar_type: RunnerPatchScalarType::{},",
        patch_scalar_name(slot.scalar_type)
    )
    .expect("writing to String cannot fail");
    writeln!(
        source,
        "        extent: RunnerPatchExtent::{},",
        patch_extent_name(slot.extent)
    )
    .expect("writing to String cannot fail");
    writeln!(source, "    }},").expect("writing to String cannot fail");
}

const fn role_name(role: Qwen3ModelRole) -> &'static str {
    match role {
        Qwen3ModelRole::Target8B => "Target8B",
        Qwen3ModelRole::Draft06B => "Draft06B",
    }
}

const fn mode_name(mode: Qwen3ExecutionMode) -> &'static str {
    match mode {
        Qwen3ExecutionMode::Prefill => "Prefill",
        Qwen3ExecutionMode::Decode => "Decode",
        Qwen3ExecutionMode::Speculative => "Speculative",
    }
}

const fn bucket_name(bucket: Qwen3PlanBucket) -> &'static str {
    match bucket {
        Qwen3PlanBucket::PrefillS1T128 => "PrefillS1T128",
        Qwen3PlanBucket::PrefillS8T128 => "PrefillS8T128",
        Qwen3PlanBucket::PrefillS1T512 => "PrefillS1T512",
        Qwen3PlanBucket::PrefillS1T2048 => "PrefillS1T2048",
        Qwen3PlanBucket::DecodeS1C8192 => "DecodeS1C8192",
        Qwen3PlanBucket::DecodeS8C8192 => "DecodeS8C8192",
        Qwen3PlanBucket::DecodeS32C8192 => "DecodeS32C8192",
        Qwen3PlanBucket::SpeculativeS1K4C8192 => "SpeculativeS1K4C8192",
        Qwen3PlanBucket::SpeculativeS8K4C8192 => "SpeculativeS8K4C8192",
        Qwen3PlanBucket::SpeculativeS1K8C8192 => "SpeculativeS1K8C8192",
        Qwen3PlanBucket::SpeculativeS1K16C8192 => "SpeculativeS1K16C8192",
    }
}

const fn patch_kind_name(kind: RunnerPatchKind) -> &'static str {
    match kind {
        RunnerPatchKind::TokenIds => "TokenIds",
        RunnerPatchKind::PositionIds => "PositionIds",
        RunnerPatchKind::ActiveLengths => "ActiveLengths",
        RunnerPatchKind::ContextLengths => "ContextLengths",
    }
}

const fn patch_scalar_name(scalar: RunnerPatchScalarType) -> &'static str {
    match scalar {
        RunnerPatchScalarType::U32 => "U32",
    }
}

const fn patch_extent_name(extent: RunnerPatchExtent) -> &'static str {
    match extent {
        RunnerPatchExtent::ActiveTokens => "ActiveTokens",
        RunnerPatchExtent::Sequences => "Sequences",
    }
}

const fn patch_kind_tag(kind: RunnerPatchKind) -> u8 {
    match kind {
        RunnerPatchKind::TokenIds => 1,
        RunnerPatchKind::PositionIds => 2,
        RunnerPatchKind::ActiveLengths => 3,
        RunnerPatchKind::ContextLengths => 4,
    }
}

const fn patch_scalar_tag(scalar: RunnerPatchScalarType) -> u8 {
    match scalar {
        RunnerPatchScalarType::U32 => 1,
    }
}

const fn patch_extent_tag(extent: RunnerPatchExtent) -> u8 {
    match extent {
        RunnerPatchExtent::ActiveTokens => 1,
        RunnerPatchExtent::Sequences => 2,
    }
}

fn encode_selection(record: &mut Vec<u8>, selection: Qwen3PlanSelection) {
    record.push(match selection.role {
        Qwen3ModelRole::Target8B => 1,
        Qwen3ModelRole::Draft06B => 2,
    });
    record.push(match selection.mode {
        Qwen3ExecutionMode::Prefill => 1,
        Qwen3ExecutionMode::Decode => 2,
        Qwen3ExecutionMode::Speculative => 3,
    });
    record.push(match selection.bucket {
        Qwen3PlanBucket::PrefillS1T128 => 1,
        Qwen3PlanBucket::PrefillS8T128 => 2,
        Qwen3PlanBucket::PrefillS1T512 => 3,
        Qwen3PlanBucket::PrefillS1T2048 => 4,
        Qwen3PlanBucket::DecodeS1C8192 => 5,
        Qwen3PlanBucket::DecodeS8C8192 => 6,
        Qwen3PlanBucket::DecodeS32C8192 => 7,
        Qwen3PlanBucket::SpeculativeS1K4C8192 => 8,
        Qwen3PlanBucket::SpeculativeS8K4C8192 => 9,
        Qwen3PlanBucket::SpeculativeS1K8C8192 => 10,
        Qwen3PlanBucket::SpeculativeS1K16C8192 => 11,
    });
}

fn push_bytes(record: &mut Vec<u8>, bytes: &[u8]) {
    record.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    record.extend_from_slice(bytes);
}

fn identity_record(domain: &[u8], bytes: &[u8]) -> Identity {
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, domain);
    hash_field(&mut hasher, bytes);
    Identity::new(hasher.finish())
}

/// Builds the compact sealed runner-closure fixture for cross-crate tests.
///
/// This helper exists only under the `test-fixtures` feature. Its in-crate
/// compact prepacked authorities do not authenticate deployed model bytes and
/// must not be used as production admission or qualification evidence.
#[cfg(feature = "test-fixtures")]
#[doc(hidden)]
#[must_use]
pub fn qwen3_runner_closure_test_fixture() -> PreliminaryIdentityClosure {
    use crate::{
        build_authenticated_sequential_plan_catalog, build_preliminary_identity_closure,
        build_prepacked_deployment_bundle, expected_preliminary_kernel_catalog_identity,
        seal_authenticated_bundle, tokenizer::test_fixtures::authenticated_assets,
        tokenizer::test_fixtures::test_tokenizer, weight_stream::test_fixtures::test_prepacked,
        ExternalIdentityClosureInputs,
    };
    use ferric_spec::Qwen3ModelRole;

    const fn fixture_identity(byte: u8) -> Identity {
        Identity::new([byte; 32])
    }

    let prepacked = build_prepacked_deployment_bundle(
        authenticated_assets(),
        test_tokenizer(Qwen3ModelRole::Target8B),
        test_tokenizer(Qwen3ModelRole::Draft06B),
        test_prepacked(Qwen3ModelRole::Target8B),
        test_prepacked(Qwen3ModelRole::Draft06B),
    )
    .expect("exact compact prepacked fixture");
    let admission = seal_authenticated_bundle(prepacked).expect("sealed compact fixture");
    let catalog = build_authenticated_sequential_plan_catalog(admission)
        .expect("authenticated compact plan fixture");
    let mut external = ExternalIdentityClosureInputs {
        ferric_source: fixture_identity(31),
        fe2o3_source: fixture_identity(32),
        compiler: fixture_identity(33),
        compiler_configuration: fixture_identity(34),
        target_contract: fixture_identity(35),
        kernel_catalog: fixture_identity(36),
        kernel_proof_set: fixture_identity(37),
        kernel_abi_catalog: fixture_identity(38),
        executable_catalog: fixture_identity(39),
        runtime_contract: fixture_identity(40),
        runtime_abi: fixture_identity(41),
        generated_runner: expected_qwen3_gfx942_runner_source_identity(),
        validator_registry: fixture_identity(43),
        qualification_protocol: fixture_identity(44),
        tcb_report: fixture_identity(45),
    };
    external.kernel_catalog = expected_preliminary_kernel_catalog_identity(&catalog, &external)
        .expect("structural compact kernel fixture");
    build_preliminary_identity_closure(catalog, external).expect("compact identity closure")
}

#[cfg(test)]
mod tests {
    use super::{
        expected_qwen3_gfx942_runner_source_identity, generate_qwen3_gfx942_runner_declaration,
        identity_record, publish_qwen3_gfx942_runner_declaration,
        render_qwen3_gfx942_runner_source, render_qwen3_gfx942_runner_source_closure,
        validate_patch_slots, validate_plan_templates, validate_qwen3_gfx942_runner_declaration,
        validate_qwen3_gfx942_runner_source_closure, GeneratedRunnerError, DECLARATION_DOMAIN,
        GENERATED_SOURCE_DOMAIN, GENERATED_VALIDATION_SOURCE_BYTES,
        GENERATED_VALIDATION_SOURCE_SHA256,
    };
    use crate::{
        build_authenticated_sequential_plan_catalog, build_preliminary_identity_closure,
        build_prepacked_deployment_bundle, build_sequential_plan_catalog, digest_bytes,
        expected_preliminary_kernel_catalog_identity, seal_authenticated_bundle,
        tokenizer::tests::{authenticated_assets, test_tokenizer},
        weight_stream::tests::test_prepacked,
        ExternalIdentityClosureInputs, PreliminaryIdentityClosure,
    };
    use ferric_generated_runner::{
        RunnerPatchExtent, RunnerPatchKind, RunnerPatchScalarType, GENERATED_PATCH_SLOTS,
        GENERATED_PLAN_TEMPLATES, GENERATED_RUNNER_OPERATION_COUNT, GENERATED_RUNNER_PLAN_COUNT,
    };
    use ferric_spec::{
        Identity, Qwen3BufferKind, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket,
    };

    const fn identity(byte: u8) -> Identity {
        Identity::new([byte; 32])
    }

    fn prepacked() -> crate::PrepackedDeploymentBundle {
        build_prepacked_deployment_bundle(
            authenticated_assets(),
            test_tokenizer(Qwen3ModelRole::Target8B),
            test_tokenizer(Qwen3ModelRole::Draft06B),
            test_prepacked(Qwen3ModelRole::Target8B),
            test_prepacked(Qwen3ModelRole::Draft06B),
        )
        .expect("complete prepacked deployment")
    }

    fn external() -> ExternalIdentityClosureInputs {
        ExternalIdentityClosureInputs {
            ferric_source: identity(31),
            fe2o3_source: identity(32),
            compiler: identity(33),
            compiler_configuration: identity(34),
            target_contract: identity(35),
            kernel_catalog: identity(36),
            kernel_proof_set: identity(37),
            kernel_abi_catalog: identity(38),
            executable_catalog: identity(39),
            runtime_contract: identity(40),
            runtime_abi: identity(41),
            generated_runner: expected_qwen3_gfx942_runner_source_identity(),
            validator_registry: identity(43),
            qualification_protocol: identity(44),
            tcb_report: identity(45),
        }
    }

    fn closure(authenticated: bool, source_id: Identity) -> PreliminaryIdentityClosure {
        let catalog = if authenticated {
            let admission = seal_authenticated_bundle(prepacked()).expect("sealed admission");
            build_authenticated_sequential_plan_catalog(admission).expect("authenticated plans")
        } else {
            build_sequential_plan_catalog(prepacked()).expect("raw prepacked plans")
        };
        let mut external = external();
        external.generated_runner = source_id;
        external.kernel_catalog = expected_preliminary_kernel_catalog_identity(&catalog, &external)
            .expect("structural kernel identity");
        build_preliminary_identity_closure(catalog, external).expect("preliminary closure")
    }

    fn exact_closure() -> PreliminaryIdentityClosure {
        closure(true, expected_qwen3_gfx942_runner_source_identity())
    }

    #[test]
    fn regeneration_and_complete_source_closure_are_byte_exact() {
        let generated = render_qwen3_gfx942_runner_source();
        let crate_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../ferric-generated-runner/src/lib.rs");
        let checked_in = std::fs::read(crate_path).expect("checked-in generated source");
        if generated != checked_in {
            let first = generated
                .iter()
                .zip(&checked_in)
                .position(|(left, right)| left != right)
                .unwrap_or(generated.len().min(checked_in.len()));
            panic!(
                "generated source drift at byte {first}: rendered_len={}, checked_in_len={}, rendered={:?}, checked_in={:?}",
                generated.len(),
                checked_in.len(),
                String::from_utf8_lossy(&generated[first.saturating_sub(40)..(first + 80).min(generated.len())]),
                String::from_utf8_lossy(&checked_in[first.saturating_sub(40)..(first + 80).min(checked_in.len())])
            );
        }
        let validation_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../ferric-generated-runner/src/validation.rs");
        let validation =
            std::fs::read(validation_path).expect("checked-in runner validation source");
        let validation_digest = digest_bytes(&validation);
        assert_eq!(
            validation_digest.byte_len,
            GENERATED_VALIDATION_SOURCE_BYTES
        );
        assert_eq!(validation_digest.sha256, GENERATED_VALIDATION_SOURCE_SHA256);
        assert_eq!(
            validate_qwen3_gfx942_runner_source_closure(&generated, &validation),
            Ok(expected_qwen3_gfx942_runner_source_identity())
        );
        assert_eq!(
            expected_qwen3_gfx942_runner_source_identity(),
            Identity::new([
                0x71, 0xcc, 0xfd, 0x00, 0xfa, 0x4c, 0xca, 0xf1, 0x7b, 0x66, 0x38, 0xe3, 0x63, 0x4d,
                0x69, 0xce, 0xb8, 0xdf, 0x44, 0x39, 0x9c, 0xe0, 0xa2, 0x7b, 0x5c, 0x31, 0x4d, 0x51,
                0x29, 0x4f, 0xba, 0x53,
            ])
        );
        for index in 0..generated.len() {
            let mut changed = generated.clone();
            changed[index] ^= 1;
            assert_eq!(
                validate_qwen3_gfx942_runner_source_closure(&changed, &validation),
                Err(GeneratedRunnerError::SourceIdentityDrift),
                "library source byte {index} was accepted"
            );
        }
        for index in 0..validation.len() {
            let mut changed = validation.clone();
            changed[index] ^= 1;
            assert_eq!(
                validate_qwen3_gfx942_runner_source_closure(&generated, &changed),
                Err(GeneratedRunnerError::SourceIdentityDrift),
                "validation source byte {index} was accepted"
            );
        }
        let mut trailing_library = generated.clone();
        trailing_library.push(0);
        assert_eq!(
            validate_qwen3_gfx942_runner_source_closure(&trailing_library, &validation),
            Err(GeneratedRunnerError::SourceIdentityDrift)
        );
        assert_eq!(
            validate_qwen3_gfx942_runner_source_closure(
                &generated[..generated.len() - 1],
                &validation,
            ),
            Err(GeneratedRunnerError::SourceIdentityDrift)
        );
        let mut trailing_validation = validation.clone();
        trailing_validation.push(0);
        assert_eq!(
            validate_qwen3_gfx942_runner_source_closure(&generated, &trailing_validation),
            Err(GeneratedRunnerError::SourceIdentityDrift)
        );
        assert_eq!(
            validate_qwen3_gfx942_runner_source_closure(
                &generated,
                &validation[..validation.len() - 1],
            ),
            Err(GeneratedRunnerError::SourceIdentityDrift)
        );

        let mut representative_validation_drift = validation.clone();
        representative_validation_drift[validation.len() / 2] ^= 1;
        let changed_validation_digest = digest_bytes(&representative_validation_drift);
        let changed_closure = render_qwen3_gfx942_runner_source_closure(
            &generated,
            changed_validation_digest.byte_len,
            changed_validation_digest.sha256,
        );
        assert_ne!(
            identity_record(GENERATED_SOURCE_DOMAIN, &changed_closure),
            expected_qwen3_gfx942_runner_source_identity()
        );
    }

    #[test]
    fn exact_declaration_retains_all_plans_operations_shapes_and_identities() {
        let declaration =
            generate_qwen3_gfx942_runner_declaration(exact_closure()).expect("exact declaration");
        assert_eq!(
            validate_qwen3_gfx942_runner_declaration(&declaration),
            Ok(())
        );
        assert_eq!(declaration.plans().len(), GENERATED_RUNNER_PLAN_COUNT);
        assert_eq!(
            declaration.operations().len(),
            GENERATED_RUNNER_OPERATION_COUNT
        );
        assert_eq!(declaration.patch_slots(), GENERATED_PATCH_SLOTS);
        assert!(declaration.source_id().is_present());
        assert!(declaration.admission_record_id().is_present());
        assert!(declaration.plan_catalog_id().is_present());
        assert!(declaration.kernel_catalog_id().is_present());
        assert!(declaration.closure_id().is_present());
        assert!(declaration.declaration_id().is_present());
        assert_eq!(
            declaration.declaration_id(),
            Identity::new([
                0xb5, 0xfb, 0x0e, 0xa8, 0xc0, 0xc9, 0x03, 0x29, 0x04, 0x41, 0x7e, 0x98, 0xae, 0x2c,
                0xfd, 0x49, 0x30, 0xc1, 0xee, 0xe7, 0x0c, 0x5d, 0x56, 0x46, 0x89, 0xf1, 0x79, 0x7f,
                0x74, 0x69, 0xc7, 0xca,
            ]),
            "actual declaration identity: {:02x?}",
            declaration.declaration_id().as_bytes(),
        );

        let catalog = declaration.closure().catalog();
        for (plan, template) in declaration.plans().iter().zip(GENERATED_PLAN_TEMPLATES) {
            let retained = &catalog.plans()[usize::from(plan.plan_index)];
            assert_eq!(plan.plan_id, retained.authority.plan_id);
            assert_eq!(plan.selection, retained.selection);
            assert_eq!(plan.selection, template.selection);
            let start = plan.operation_start as usize;
            let end = start + plan.operation_count as usize;
            for (local_index, operation) in declaration.operations()[start..end].iter().enumerate()
            {
                assert_eq!(operation.operation_index as usize, start + local_index);
                assert_eq!(operation.plan_index, plan.plan_index);
                assert_eq!(operation.profile.plan_id, plan.plan_id);
                assert_eq!(operation.profile.selection, plan.selection);
                assert_eq!(operation.profile.step, retained.steps[local_index]);
                for buffer in [
                    operation.profile.step.input_0,
                    operation.profile.step.input_1,
                    operation.profile.step.input_2,
                    operation.profile.step.output_0,
                    operation.profile.step.output_1,
                ] {
                    if buffer.kind == Qwen3BufferKind::Absent {
                        assert_eq!(buffer.shape.rank, 0);
                    } else {
                        assert!((2..=4).contains(&buffer.shape.rank));
                        assert!(buffer.shape.dimension_0 > 0);
                        assert!(buffer.shape.dimension_1 > 0);
                    }
                }
            }
        }
        assert_eq!(
            declaration.declaration_id(),
            identity_record(DECLARATION_DOMAIN, declaration.canonical_bytes())
        );

        let second =
            generate_qwen3_gfx942_runner_declaration(exact_closure()).expect("second declaration");
        assert_eq!(declaration.source_id(), second.source_id());
        assert_eq!(declaration.declaration_id(), second.declaration_id());
        assert_eq!(declaration.canonical_bytes(), second.canonical_bytes());
        assert_eq!(declaration.plans(), second.plans());
        assert_eq!(declaration.operations(), second.operations());
    }

    #[test]
    fn publication_consumes_and_retains_the_complete_exact_declaration() {
        let declaration =
            generate_qwen3_gfx942_runner_declaration(exact_closure()).expect("exact declaration");
        let source_id = declaration.source_id();
        let admission_record_id = declaration.admission_record_id();
        let bundle_id = declaration.closure().catalog().deployment().bundle_id;
        let target_prepacked_id = Identity::new(
            declaration
                .closure()
                .catalog()
                .prepacked()
                .target_manifest()
                .aggregate_id(),
        );
        let draft_prepacked_id = Identity::new(
            declaration
                .closure()
                .catalog()
                .prepacked()
                .draft_manifest()
                .aggregate_id(),
        );
        let plan_catalog_id = declaration.plan_catalog_id();
        let kernel_catalog_id = declaration.kernel_catalog_id();
        let closure_id = declaration.closure_id();
        let declaration_id = declaration.declaration_id();

        let published = publish_qwen3_gfx942_runner_declaration(declaration)
            .expect("exact declaration publishes");
        assert_eq!(published.source_id(), source_id);
        assert_eq!(published.admission_record_id(), admission_record_id);
        assert_eq!(published.bundle_id(), bundle_id);
        assert_eq!(published.target_prepacked_id(), target_prepacked_id);
        assert_eq!(published.draft_prepacked_id(), draft_prepacked_id);
        assert_eq!(published.plan_catalog_id(), plan_catalog_id);
        assert_eq!(published.kernel_catalog_id(), kernel_catalog_id);
        assert_eq!(published.closure_id(), closure_id);
        assert_eq!(published.declaration_id(), declaration_id);
        assert_eq!(published.plans().len(), GENERATED_RUNNER_PLAN_COUNT);
        assert_eq!(
            published.operations().len(),
            GENERATED_RUNNER_OPERATION_COUNT
        );
        assert_eq!(published.patch_slots(), GENERATED_PATCH_SLOTS);
        assert!(!published.canonical_bytes().is_empty());
    }

    #[test]
    fn publication_rejects_a_last_moment_declaration_mutation() {
        let mut declaration =
            generate_qwen3_gfx942_runner_declaration(exact_closure()).expect("exact declaration");
        declaration.operations[GENERATED_RUNNER_OPERATION_COUNT - 1]
            .profile
            .step
            .output_0
            .shape
            .dimension_1 += 1;
        assert_eq!(
            publish_qwen3_gfx942_runner_declaration(declaration),
            Err(GeneratedRunnerError::DeclarationDrift)
        );
    }

    #[test]
    fn every_plan_template_field_and_adjacent_shape_fail_closed() {
        assert_eq!(validate_plan_templates(&GENERATED_PLAN_TEMPLATES), Ok(()));
        assert!(matches!(
            validate_plan_templates(&GENERATED_PLAN_TEMPLATES[..21]),
            Err(GeneratedRunnerError::TemplatePlanCount { .. })
        ));
        let mut extra = GENERATED_PLAN_TEMPLATES.to_vec();
        extra.push(GENERATED_PLAN_TEMPLATES[21]);
        assert!(matches!(
            validate_plan_templates(&extra),
            Err(GeneratedRunnerError::TemplatePlanCount { .. })
        ));

        for index in 0..GENERATED_PLAN_TEMPLATES.len() {
            let exact = GENERATED_PLAN_TEMPLATES[index];
            let mut changed = GENERATED_PLAN_TEMPLATES;
            changed[index].plan_index = changed[index].plan_index.wrapping_add(1);
            assert_eq!(
                validate_plan_templates(&changed),
                Err(GeneratedRunnerError::TemplatePlanDrift { plan_index: index })
            );

            changed = GENERATED_PLAN_TEMPLATES;
            changed[index].selection.role = match exact.selection.role {
                Qwen3ModelRole::Target8B => Qwen3ModelRole::Draft06B,
                Qwen3ModelRole::Draft06B => Qwen3ModelRole::Target8B,
            };
            assert_eq!(
                validate_plan_templates(&changed),
                Err(GeneratedRunnerError::TemplatePlanDrift { plan_index: index })
            );

            changed = GENERATED_PLAN_TEMPLATES;
            changed[index].selection.mode = match exact.selection.mode {
                Qwen3ExecutionMode::Prefill => Qwen3ExecutionMode::Decode,
                Qwen3ExecutionMode::Decode => Qwen3ExecutionMode::Speculative,
                Qwen3ExecutionMode::Speculative => Qwen3ExecutionMode::Prefill,
            };
            assert_eq!(
                validate_plan_templates(&changed),
                Err(GeneratedRunnerError::TemplatePlanDrift { plan_index: index })
            );

            changed = GENERATED_PLAN_TEMPLATES;
            changed[index].selection.bucket = match exact.selection.bucket {
                Qwen3PlanBucket::PrefillS1T128 => Qwen3PlanBucket::PrefillS8T128,
                _ => Qwen3PlanBucket::PrefillS1T128,
            };
            assert_eq!(
                validate_plan_templates(&changed),
                Err(GeneratedRunnerError::TemplatePlanDrift { plan_index: index })
            );

            changed = GENERATED_PLAN_TEMPLATES;
            changed[index].operation_start = changed[index].operation_start.wrapping_add(1);
            assert_eq!(
                validate_plan_templates(&changed),
                Err(GeneratedRunnerError::TemplatePlanDrift { plan_index: index })
            );

            changed = GENERATED_PLAN_TEMPLATES;
            changed[index].operation_count = changed[index].operation_count.wrapping_add(1);
            assert_eq!(
                validate_plan_templates(&changed),
                Err(GeneratedRunnerError::TemplatePlanDrift { plan_index: index })
            );
        }
    }

    #[test]
    fn every_patch_schema_field_and_adjacent_shape_fail_closed() {
        assert_eq!(validate_patch_slots(&GENERATED_PATCH_SLOTS), Ok(()));
        assert_eq!(
            validate_patch_slots(&GENERATED_PATCH_SLOTS[..3]),
            Err(GeneratedRunnerError::PatchSchemaDrift)
        );
        let mut extra = GENERATED_PATCH_SLOTS.to_vec();
        extra.push(GENERATED_PATCH_SLOTS[3]);
        assert_eq!(
            validate_patch_slots(&extra),
            Err(GeneratedRunnerError::PatchSchemaDrift)
        );
        for index in 0..GENERATED_PATCH_SLOTS.len() {
            let mut changed = GENERATED_PATCH_SLOTS;
            changed[index].slot_index = changed[index].slot_index.wrapping_add(1);
            assert_eq!(
                validate_patch_slots(&changed),
                Err(GeneratedRunnerError::PatchSchemaDrift)
            );

            changed = GENERATED_PATCH_SLOTS;
            changed[index].kind = match changed[index].kind {
                RunnerPatchKind::TokenIds => RunnerPatchKind::PositionIds,
                _ => RunnerPatchKind::TokenIds,
            };
            assert_eq!(
                validate_patch_slots(&changed),
                Err(GeneratedRunnerError::PatchSchemaDrift)
            );

            changed = GENERATED_PATCH_SLOTS;
            changed[index].extent = match changed[index].extent {
                RunnerPatchExtent::ActiveTokens => RunnerPatchExtent::Sequences,
                RunnerPatchExtent::Sequences => RunnerPatchExtent::ActiveTokens,
            };
            assert_eq!(
                validate_patch_slots(&changed),
                Err(GeneratedRunnerError::PatchSchemaDrift)
            );

            changed = GENERATED_PATCH_SLOTS;
            changed[index].scalar_type = RunnerPatchScalarType::U32;
            changed.swap(index, (index + 1) % GENERATED_PATCH_SLOTS.len());
            assert_eq!(
                validate_patch_slots(&changed),
                Err(GeneratedRunnerError::PatchSchemaDrift)
            );
        }
    }

    #[test]
    fn unauthenticated_catalog_and_generated_source_drift_fail_closed() {
        let raw = closure(false, expected_qwen3_gfx942_runner_source_identity());
        assert_eq!(
            generate_qwen3_gfx942_runner_declaration(raw),
            Err(GeneratedRunnerError::MissingAuthenticatedAdmission)
        );

        let drifted = closure(true, identity(99));
        assert_eq!(
            generate_qwen3_gfx942_runner_declaration(drifted),
            Err(GeneratedRunnerError::SourceIdentityDrift)
        );
    }

    #[test]
    fn every_retained_declaration_identity_and_typed_operation_is_revalidated() {
        let mut declaration =
            generate_qwen3_gfx942_runner_declaration(exact_closure()).expect("exact declaration");
        let exact_source = declaration.source_id;
        declaration.source_id = identity(101);
        assert_eq!(
            validate_qwen3_gfx942_runner_declaration(&declaration),
            Err(GeneratedRunnerError::DeclarationDrift)
        );
        declaration.source_id = exact_source;

        let exact_admission = declaration.admission_record_id;
        declaration.admission_record_id = identity(102);
        assert_eq!(
            validate_qwen3_gfx942_runner_declaration(&declaration),
            Err(GeneratedRunnerError::DeclarationDrift)
        );
        declaration.admission_record_id = exact_admission;

        let exact_plan_catalog = declaration.plan_catalog_id;
        declaration.plan_catalog_id = identity(103);
        assert_eq!(
            validate_qwen3_gfx942_runner_declaration(&declaration),
            Err(GeneratedRunnerError::DeclarationDrift)
        );
        declaration.plan_catalog_id = exact_plan_catalog;

        let exact_kernel_catalog = declaration.kernel_catalog_id;
        declaration.kernel_catalog_id = identity(104);
        assert_eq!(
            validate_qwen3_gfx942_runner_declaration(&declaration),
            Err(GeneratedRunnerError::DeclarationDrift)
        );
        declaration.kernel_catalog_id = exact_kernel_catalog;

        let exact_closure = declaration.closure_id;
        declaration.closure_id = identity(105);
        assert_eq!(
            validate_qwen3_gfx942_runner_declaration(&declaration),
            Err(GeneratedRunnerError::DeclarationDrift)
        );
        declaration.closure_id = exact_closure;

        let exact_plan = declaration.plans[0];
        declaration.plans[0].operation_start += 1;
        assert_eq!(
            validate_qwen3_gfx942_runner_declaration(&declaration),
            Err(GeneratedRunnerError::DeclarationDrift)
        );
        declaration.plans[0] = exact_plan;

        let exact_operation = declaration.operations[0];
        declaration.operations[0]
            .profile
            .step
            .output_0
            .shape
            .dimension_1 += 1;
        assert_eq!(
            validate_qwen3_gfx942_runner_declaration(&declaration),
            Err(GeneratedRunnerError::DeclarationDrift)
        );
        declaration.operations[0] = exact_operation;

        let exact_bytes = declaration.canonical_bytes.clone();
        declaration.canonical_bytes[0] ^= 1;
        assert_eq!(
            validate_qwen3_gfx942_runner_declaration(&declaration),
            Err(GeneratedRunnerError::DeclarationDrift)
        );
        declaration.canonical_bytes = exact_bytes;

        declaration.declaration_id = identity(106);
        assert_eq!(
            validate_qwen3_gfx942_runner_declaration(&declaration),
            Err(GeneratedRunnerError::DeclarationDrift)
        );
    }
}
