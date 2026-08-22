//! Inert binding from the published logical operation roster to Qwen kernel declarations.
//!
//! The build-published runner supplies the exact ordered graph operations and
//! structural K1-K7 catalog identity. Ferric's finite Qwen catalogs supply the
//! exact family profile identity for each operation. Caller declarations name
//! a family build, artifact, and ABI-layout identity, but this module does not
//! authenticate those names or the bytes they purport to identify.
//! The eight canonical catalogs contain 440 profiles. Generated graph
//! operations select 418 of them; the remaining 22 are the explicitly
//! auxiliary residual-fused `RMSNorm` profiles.
//!
//! This bridge performs no compilation, allocation, loading, residency check,
//! queue action, launch, readback, inference, numerical refinement, hardware
//! interaction, performance measurement, or M1 qualification. A future
//! artifact authority must authenticate the retained declarations before a
//! physical runner can use them.

use std::collections::HashSet;

use ferric_build::GeneratedOperationDeclaration;
use ferric_kernels::{
    KernelFamily, KernelProfileDisposition, M1_B3_PLAN_BUCKETS, M1_KERNEL_OPERATION_BINDINGS,
    M1_KERNEL_PLAN_COUNT,
};
use ferric_qwen_kernels::{gemm, logits, paged_decode, prefill, rmsnorm, rope_kv, swiglu};
use ferric_spec::{
    Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3Operator, Qwen3PlanSelection,
};

use crate::{LogicalRunnerDeclaration, LogicalRunnerError};

const FAMILY_COUNT: usize = 7;

const FAMILIES: [KernelFamily; FAMILY_COUNT] = [
    KernelFamily::K1GemmGemv,
    KernelFamily::K2RmsNormResidual,
    KernelFamily::K3RopePagedKv,
    KernelFamily::K4GqaPrefill,
    KernelFamily::K5PagedGqaDecode,
    KernelFamily::K6SwiGlu,
    KernelFamily::K7LogitsCompact,
];

/// One caller-declared family build and artifact identity tuple.
///
/// These are structural labels only. Construction does not inspect, hash,
/// authenticate, load, or execute any artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeclaredKernelFamilyArtifact {
    family: KernelFamily,
    build_id: Identity,
    artifact_id: Identity,
    abi_layout_id: Identity,
}

impl DeclaredKernelFamilyArtifact {
    /// Constructs one inert family declaration.
    #[must_use]
    pub const fn new(
        family: KernelFamily,
        build_id: Identity,
        artifact_id: Identity,
        abi_layout_id: Identity,
    ) -> Self {
        Self {
            family,
            build_id,
            artifact_id,
            abi_layout_id,
        }
    }

    /// Declared K1-K7 family.
    #[must_use]
    pub const fn family(self) -> KernelFamily {
        self.family
    }

    /// Caller-declared family compiler-build identity.
    #[must_use]
    pub const fn build_id(self) -> Identity {
        self.build_id
    }

    /// Caller-declared family artifact identity.
    #[must_use]
    pub const fn artifact_id(self) -> Identity {
        self.artifact_id
    }

    /// Caller-declared family ABI-layout identity.
    #[must_use]
    pub const fn abi_layout_id(self) -> Identity {
        self.abi_layout_id
    }

    /// A declaration alone does not authenticate the named artifact.
    #[must_use]
    pub const fn authenticates_artifact(self) -> bool {
        false
    }
}

/// Exact generated and Qwen profile identities declared for one operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeclaredOperationIdentity {
    plan: Identity,
    runner_declaration: Identity,
    kernel_catalog: Identity,
    profile_catalog: Identity,
    profile: Identity,
}

impl DeclaredOperationIdentity {
    /// Groups the exact plan, runner, catalog, and Qwen profile identities.
    #[must_use]
    pub const fn new(
        plan_id: Identity,
        runner_declaration_id: Identity,
        kernel_catalog_id: Identity,
        profile_catalog_id: Identity,
        profile_id: Identity,
    ) -> Self {
        Self {
            plan: plan_id,
            runner_declaration: runner_declaration_id,
            kernel_catalog: kernel_catalog_id,
            profile_catalog: profile_catalog_id,
            profile: profile_id,
        }
    }
}

/// Exact declared kernel binding for one generated operation position.
///
/// The profile and profile-catalog identities are checked against Ferric's
/// canonical Qwen catalogs. Build, artifact, and ABI-layout identities are
/// checked only against the corresponding caller-declared family tuple.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeclaredOperationKernelBinding {
    operation_index: u32,
    plan_index: u16,
    family: KernelFamily,
    plan_id: Identity,
    runner_declaration_id: Identity,
    kernel_catalog_id: Identity,
    profile_catalog_id: Identity,
    profile_id: Identity,
    family_build_id: Identity,
    artifact_id: Identity,
    abi_layout_id: Identity,
}

impl DeclaredOperationKernelBinding {
    /// Constructs one inert operation declaration.
    #[must_use]
    pub const fn new(
        operation_index: u32,
        plan_index: u16,
        identity: DeclaredOperationIdentity,
        family_artifact: DeclaredKernelFamilyArtifact,
    ) -> Self {
        Self {
            operation_index,
            plan_index,
            family: family_artifact.family,
            plan_id: identity.plan,
            runner_declaration_id: identity.runner_declaration,
            kernel_catalog_id: identity.kernel_catalog,
            profile_catalog_id: identity.profile_catalog,
            profile_id: identity.profile,
            family_build_id: family_artifact.build_id,
            artifact_id: family_artifact.artifact_id,
            abi_layout_id: family_artifact.abi_layout_id,
        }
    }

    /// Global generated-operation position.
    #[must_use]
    pub const fn operation_index(self) -> u32 {
        self.operation_index
    }

    /// Target-then-draft generated-plan position.
    #[must_use]
    pub const fn plan_index(self) -> u16 {
        self.plan_index
    }

    /// Exact K1-K7 family.
    #[must_use]
    pub const fn family(self) -> KernelFamily {
        self.family
    }

    /// Build-published plan identity retained by the generated operation.
    #[must_use]
    pub const fn plan_id(self) -> Identity {
        self.plan_id
    }

    /// Exact published generated-runner declaration identity.
    #[must_use]
    pub const fn runner_declaration_id(self) -> Identity {
        self.runner_declaration_id
    }

    /// Exact published structural K1-K7 catalog identity.
    #[must_use]
    pub const fn kernel_catalog_id(self) -> Identity {
        self.kernel_catalog_id
    }

    /// Exact canonical Ferric Qwen family-profile catalog identity.
    #[must_use]
    pub const fn profile_catalog_id(self) -> Identity {
        self.profile_catalog_id
    }

    /// Exact canonical Ferric Qwen operation-profile identity.
    #[must_use]
    pub const fn profile_id(self) -> Identity {
        self.profile_id
    }

    /// Caller-declared family compiler-build identity.
    #[must_use]
    pub const fn family_build_id(self) -> Identity {
        self.family_build_id
    }

    /// Caller-declared family artifact identity.
    #[must_use]
    pub const fn artifact_id(self) -> Identity {
        self.artifact_id
    }

    /// Caller-declared family ABI-layout identity.
    #[must_use]
    pub const fn abi_layout_id(self) -> Identity {
        self.abi_layout_id
    }
}

/// Identity role whose caller declaration was absent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationKernelIdentityComponent {
    /// Family compiler-build declaration.
    FamilyBuild,
    /// Family artifact declaration.
    Artifact,
    /// Family ABI-layout declaration.
    AbiLayout,
}

/// Fail-closed structural operation/kernel planning error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationKernelPlanError {
    /// The family artifact roster has the wrong size.
    FamilyCount {
        /// Required K1-K7 declaration count.
        expected: usize,
        /// Observed declaration count.
        actual: usize,
    },
    /// A family appears more than once.
    DuplicateFamily(KernelFamily),
    /// The family roster is not in exact K1-K7 order.
    FamilyOrder {
        /// Zero-based roster position.
        index: usize,
        /// Required family.
        expected: KernelFamily,
        /// Observed family.
        actual: KernelFamily,
    },
    /// A caller-declared family identity is all zero.
    MissingFamilyIdentity {
        /// Family containing the absent identity.
        family: KernelFamily,
        /// Semantic identity role.
        component: OperationKernelIdentityComponent,
    },
    /// The published runner has the wrong plan count.
    PublishedPlanCount {
        /// Required exact target/draft plan count.
        expected: usize,
        /// Observed count.
        actual: usize,
    },
    /// The operation declaration roster has the wrong size.
    OperationCount {
        /// Required published operation count.
        expected: usize,
        /// Observed declaration count.
        actual: usize,
    },
    /// An operation index appears more than once.
    DuplicateOperationIndex(u32),
    /// A published plan could not be selected or retained a bad range.
    Runner(LogicalRunnerError),
    /// A generated plan is not in exact target-then-draft order.
    PublishedPlanOrder {
        /// Required plan index.
        expected: u16,
        /// Observed plan index.
        actual: u16,
    },
    /// The declared operation position differs from the generated position.
    OperationOrder {
        /// Roster position being checked.
        position: usize,
        /// Required generated operation index.
        expected: u32,
        /// Observed declared operation index.
        actual: u32,
    },
    /// The declared operation names the wrong generated plan.
    PlanIndex(u32),
    /// The generated disposition or declared K1-K7 family is wrong.
    Family(u32),
    /// The declared operation names the wrong exact plan identity.
    PlanIdentity(u32),
    /// The declared operation names the wrong generated declaration.
    RunnerDeclarationIdentity(u32),
    /// The declared operation names the wrong structural K1-K7 catalog.
    KernelCatalogIdentity(u32),
    /// The generated operation cannot resolve into an exact Qwen profile.
    CanonicalProfile(u32),
    /// One canonical Qwen profile catalog could not be constructed.
    CanonicalCatalog,
    /// The declared operation names the wrong Qwen profile catalog.
    ProfileCatalogIdentity(u32),
    /// The declared operation names the wrong exact Qwen profile.
    ProfileIdentity(u32),
    /// The declared operation names the wrong family compiler build.
    FamilyBuildIdentity(u32),
    /// The declared operation names the wrong family artifact.
    ArtifactIdentity(u32),
    /// The declared operation names the wrong family ABI layout.
    AbiLayoutIdentity(u32),
}

/// Retry-safe rejection retaining every exact inert input.
///
/// This owner is intentionally not `Clone`.
///
/// ```compile_fail
/// use ferric_engine::OperationKernelPlanFailure;
/// fn require_clone<T: Clone>() {}
/// require_clone::<OperationKernelPlanFailure>();
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct OperationKernelPlanFailure {
    error: OperationKernelPlanError,
    runner: LogicalRunnerDeclaration,
    families: Box<[DeclaredKernelFamilyArtifact]>,
    operations: Box<[DeclaredOperationKernelBinding]>,
}

impl OperationKernelPlanFailure {
    /// Returns the diagnostic without consuming retained custody.
    #[must_use]
    pub const fn error(&self) -> OperationKernelPlanError {
        self.error
    }

    /// Recovers every exact unchanged input for correction or retry.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        OperationKernelPlanError,
        LogicalRunnerDeclaration,
        Box<[DeclaredKernelFamilyArtifact]>,
        Box<[DeclaredOperationKernelBinding]>,
    ) {
        (self.error, self.runner, self.families, self.operations)
    }
}

/// Linear engine custody of one exact inert operation/kernel plan.
///
/// This type is intentionally not `Clone` and exposes no execution operation.
///
/// ```compile_fail
/// use ferric_engine::DeclaredOperationKernelPlan;
/// fn require_clone<T: Clone>() {}
/// require_clone::<DeclaredOperationKernelPlan>();
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct DeclaredOperationKernelPlan {
    runner: LogicalRunnerDeclaration,
    families: Box<[DeclaredKernelFamilyArtifact]>,
    operations: Box<[DeclaredOperationKernelBinding]>,
}

impl DeclaredOperationKernelPlan {
    /// Exact generated-runner declaration identity retained in custody.
    #[must_use]
    pub const fn runner_declaration_id(&self) -> Identity {
        self.runner.declaration_id()
    }

    /// Exact structural K1-K7 catalog identity retained in custody.
    #[must_use]
    pub const fn kernel_catalog_id(&self) -> Identity {
        self.runner.kernel_catalog_id()
    }

    /// Exact K1-K7 family build/artifact/layout declarations.
    #[must_use]
    pub fn families(&self) -> &[DeclaredKernelFamilyArtifact] {
        &self.families
    }

    /// Every operation binding in exact generated order.
    #[must_use]
    pub fn operations(&self) -> &[DeclaredOperationKernelBinding] {
        &self.operations
    }

    /// Returns bindings for one exact role/mode/bucket selection.
    ///
    /// # Errors
    ///
    /// Returns [`LogicalRunnerError`] if the retained published plan is absent
    /// or its operation range no longer fits the retained declaration roster.
    pub fn operations_for(
        &self,
        selection: Qwen3PlanSelection,
    ) -> Result<&[DeclaredOperationKernelBinding], LogicalRunnerError> {
        self.operation_declarations_for(selection)
            .map(|(_, bindings)| bindings)
    }

    /// Returns the exact generated operations and their checked bindings.
    ///
    /// Both slices have the same nonzero length and position `i` in the first
    /// slice is structurally bound to position `i` in the second. The returned
    /// declarations remain inert: they do not authenticate artifacts or grant
    /// allocation, address, packet, queue, or launch authority.
    ///
    /// # Errors
    ///
    /// Returns [`LogicalRunnerError`] if the retained published plan is absent
    /// or either operation range no longer fits its retained roster.
    pub fn operation_declarations_for(
        &self,
        selection: Qwen3PlanSelection,
    ) -> Result<
        (
            &[GeneratedOperationDeclaration],
            &[DeclaredOperationKernelBinding],
        ),
        LogicalRunnerError,
    > {
        let generated = self.runner.operations_for(selection)?;
        let Some(first) = generated.first() else {
            return Err(LogicalRunnerError::OperationRangeDrift);
        };
        let start = usize::try_from(first.operation_index)
            .map_err(|_| LogicalRunnerError::OperationRangeDrift)?;
        let end = start
            .checked_add(generated.len())
            .ok_or(LogicalRunnerError::OperationRangeDrift)?;
        let bindings = self
            .operations
            .get(start..end)
            .ok_or(LogicalRunnerError::OperationRangeDrift)?;
        if bindings.len() != generated.len() {
            return Err(LogicalRunnerError::OperationRangeDrift);
        }
        Ok((generated, bindings))
    }

    /// Recovers the inert runner and declarations without creating authority.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        LogicalRunnerDeclaration,
        Box<[DeclaredKernelFamilyArtifact]>,
        Box<[DeclaredOperationKernelBinding]>,
    ) {
        (self.runner, self.families, self.operations)
    }

    /// Structural declarations do not authenticate artifact bytes.
    #[must_use]
    pub const fn authenticates_artifacts(&self) -> bool {
        false
    }

    /// Structural declarations grant no physical execution authority.
    #[must_use]
    pub const fn grants_execution_authority(&self) -> bool {
        false
    }

    /// Structural declarations establish no numerical or operator refinement.
    #[must_use]
    pub const fn proves_refinement(&self) -> bool {
        false
    }
}

/// Exact linear outcome of one structural planning attempt.
#[must_use]
#[derive(Debug, Eq, PartialEq)]
pub enum OperationKernelPlanOutcome {
    /// Every generated operation matched its exact structural declaration.
    Bound(DeclaredOperationKernelPlan),
    /// The unchanged inputs are retained with a fail-closed diagnostic.
    Rejected(OperationKernelPlanFailure),
}

/// Binds a published runner to exact declared Ferric Qwen kernel identities.
///
/// The function consumes runner custody. It traverses every exact published
/// plan through [`LogicalRunnerDeclaration::operations_for`], derives the
/// canonical Qwen profile/catalog identity, and compares every caller field.
/// Rejection retains all inputs unchanged. Success remains inert and does not
/// authenticate the declared build, artifact, or ABI-layout identities.
pub fn bind_declared_operation_kernel_plan(
    runner: LogicalRunnerDeclaration,
    families: Box<[DeclaredKernelFamilyArtifact]>,
    operations: Box<[DeclaredOperationKernelBinding]>,
) -> OperationKernelPlanOutcome {
    match validate_plan(&runner, &families, &operations) {
        Ok(()) => OperationKernelPlanOutcome::Bound(DeclaredOperationKernelPlan {
            runner,
            families,
            operations,
        }),
        Err(error) => OperationKernelPlanOutcome::Rejected(OperationKernelPlanFailure {
            error,
            runner,
            families,
            operations,
        }),
    }
}

struct CanonicalCatalogs {
    gemm: gemm::Qwen3GemmProfileCatalogV1,
    embedding: gemm::Qwen3TokenEmbeddingProfileCatalogV1,
    rmsnorm: rmsnorm::Qwen3RmsNormProfileCatalogV1,
    rope_kv: rope_kv::Qwen3RopeKvProfileCatalogV1,
    prefill: prefill::Qwen3PrefillProfileCatalogV1,
    paged_decode: paged_decode::Qwen3PagedDecodeProfileCatalogV1,
    swiglu: swiglu::Qwen3SwiGluProfileCatalogV1,
    logits: logits::Qwen3LogitsProfileCatalogV1,
}

impl CanonicalCatalogs {
    fn build() -> Result<Self, OperationKernelPlanError> {
        Ok(Self {
            gemm: gemm::Qwen3GemmProfileCatalogV1::canonical()
                .map_err(|_| OperationKernelPlanError::CanonicalCatalog)?,
            embedding: gemm::Qwen3TokenEmbeddingProfileCatalogV1::canonical()
                .map_err(|_| OperationKernelPlanError::CanonicalCatalog)?,
            rmsnorm: rmsnorm::Qwen3RmsNormProfileCatalogV1::canonical()
                .map_err(|_| OperationKernelPlanError::CanonicalCatalog)?,
            rope_kv: rope_kv::Qwen3RopeKvProfileCatalogV1::canonical()
                .map_err(|_| OperationKernelPlanError::CanonicalCatalog)?,
            prefill: prefill::Qwen3PrefillProfileCatalogV1::canonical()
                .map_err(|_| OperationKernelPlanError::CanonicalCatalog)?,
            paged_decode: paged_decode::Qwen3PagedDecodeProfileCatalogV1::canonical()
                .map_err(|_| OperationKernelPlanError::CanonicalCatalog)?,
            swiglu: swiglu::Qwen3SwiGluProfileCatalogV1::canonical()
                .map_err(|_| OperationKernelPlanError::CanonicalCatalog)?,
            logits: logits::Qwen3LogitsProfileCatalogV1::canonical()
                .map_err(|_| OperationKernelPlanError::CanonicalCatalog)?,
        })
    }
}

#[derive(Clone, Copy)]
struct ExpectedProfile {
    catalog_id: Identity,
    profile_id: Identity,
}

fn validate_plan(
    runner: &LogicalRunnerDeclaration,
    families: &[DeclaredKernelFamilyArtifact],
    candidates: &[DeclaredOperationKernelBinding],
) -> Result<(), OperationKernelPlanError> {
    if runner.plan_count() != M1_KERNEL_PLAN_COUNT {
        return Err(OperationKernelPlanError::PublishedPlanCount {
            expected: M1_KERNEL_PLAN_COUNT,
            actual: runner.plan_count(),
        });
    }
    if runner.operation_count() != M1_KERNEL_OPERATION_BINDINGS {
        return Err(OperationKernelPlanError::OperationCount {
            expected: M1_KERNEL_OPERATION_BINDINGS,
            actual: runner.operation_count(),
        });
    }
    let mut generated = Vec::with_capacity(runner.operation_count());
    let mut expected_plan_index = 0_u16;
    for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
        for (mode, bucket) in M1_B3_PLAN_BUCKETS {
            let selection = Qwen3PlanSelection { role, mode, bucket };
            let plan = runner
                .plan(selection)
                .map_err(OperationKernelPlanError::Runner)?;
            if plan.plan_index != expected_plan_index {
                return Err(OperationKernelPlanError::PublishedPlanOrder {
                    expected: expected_plan_index,
                    actual: plan.plan_index,
                });
            }
            let plan_operations = runner
                .operations_for(selection)
                .map_err(OperationKernelPlanError::Runner)?;
            generated.extend_from_slice(plan_operations);
            expected_plan_index += 1;
        }
    }
    validate_operation_sequence(
        &generated,
        runner.declaration_id(),
        runner.kernel_catalog_id(),
        families,
        candidates,
    )
}

fn validate_operation_sequence(
    generated: &[GeneratedOperationDeclaration],
    runner_declaration_id: Identity,
    kernel_catalog_id: Identity,
    families: &[DeclaredKernelFamilyArtifact],
    candidates: &[DeclaredOperationKernelBinding],
) -> Result<(), OperationKernelPlanError> {
    let catalogs = CanonicalCatalogs::build()?;
    validate_operation_sequence_with_catalogs(
        generated,
        runner_declaration_id,
        kernel_catalog_id,
        families,
        candidates,
        &catalogs,
    )
}

fn validate_operation_sequence_with_catalogs(
    generated: &[GeneratedOperationDeclaration],
    runner_declaration_id: Identity,
    kernel_catalog_id: Identity,
    families: &[DeclaredKernelFamilyArtifact],
    candidates: &[DeclaredOperationKernelBinding],
    catalogs: &CanonicalCatalogs,
) -> Result<(), OperationKernelPlanError> {
    validate_families(families)?;
    if generated.len() != M1_KERNEL_OPERATION_BINDINGS {
        return Err(OperationKernelPlanError::OperationCount {
            expected: M1_KERNEL_OPERATION_BINDINGS,
            actual: generated.len(),
        });
    }
    if candidates.len() != generated.len() {
        return Err(OperationKernelPlanError::OperationCount {
            expected: generated.len(),
            actual: candidates.len(),
        });
    }
    validate_unique_operation_indices(candidates)?;
    let context = ValidationContext {
        runner_declaration_id,
        kernel_catalog_id,
        families,
        catalogs,
    };
    for (position, (operation, candidate)) in generated.iter().zip(candidates).enumerate() {
        validate_operation(position, operation, candidate, &context)?;
    }
    Ok(())
}

fn validate_families(
    families: &[DeclaredKernelFamilyArtifact],
) -> Result<(), OperationKernelPlanError> {
    if families.len() != FAMILY_COUNT {
        return Err(OperationKernelPlanError::FamilyCount {
            expected: FAMILY_COUNT,
            actual: families.len(),
        });
    }
    for (index, family) in families.iter().enumerate() {
        if families[..index]
            .iter()
            .any(|prior| prior.family == family.family)
        {
            return Err(OperationKernelPlanError::DuplicateFamily(family.family));
        }
        if family.family != FAMILIES[index] {
            return Err(OperationKernelPlanError::FamilyOrder {
                index,
                expected: FAMILIES[index],
                actual: family.family,
            });
        }
        for (component, identity) in [
            (
                OperationKernelIdentityComponent::FamilyBuild,
                family.build_id,
            ),
            (
                OperationKernelIdentityComponent::Artifact,
                family.artifact_id,
            ),
            (
                OperationKernelIdentityComponent::AbiLayout,
                family.abi_layout_id,
            ),
        ] {
            if !identity.is_present() {
                return Err(OperationKernelPlanError::MissingFamilyIdentity {
                    family: family.family,
                    component,
                });
            }
        }
    }
    Ok(())
}

fn validate_unique_operation_indices(
    candidates: &[DeclaredOperationKernelBinding],
) -> Result<(), OperationKernelPlanError> {
    let mut seen = HashSet::with_capacity(candidates.len());
    for candidate in candidates {
        if !seen.insert(candidate.operation_index) {
            return Err(OperationKernelPlanError::DuplicateOperationIndex(
                candidate.operation_index,
            ));
        }
    }
    Ok(())
}

fn validate_operation(
    position: usize,
    generated: &GeneratedOperationDeclaration,
    candidate: &DeclaredOperationKernelBinding,
    context: &ValidationContext<'_>,
) -> Result<(), OperationKernelPlanError> {
    if usize::try_from(generated.operation_index).ok() != Some(position)
        || candidate.operation_index != generated.operation_index
    {
        return Err(OperationKernelPlanError::OperationOrder {
            position,
            expected: generated.operation_index,
            actual: candidate.operation_index,
        });
    }
    let operation_index = generated.operation_index;
    if candidate.plan_index != generated.plan_index {
        return Err(OperationKernelPlanError::PlanIndex(operation_index));
    }
    if generated.profile.disposition != KernelProfileDisposition::DeclaredFoundation
        || generated.profile.family
            != expected_family(
                generated.profile.step.operator,
                generated.profile.selection.mode,
            )
        || candidate.family != generated.profile.family
    {
        return Err(OperationKernelPlanError::Family(operation_index));
    }
    if candidate.plan_id != generated.profile.plan_id {
        return Err(OperationKernelPlanError::PlanIdentity(operation_index));
    }
    if candidate.runner_declaration_id != context.runner_declaration_id {
        return Err(OperationKernelPlanError::RunnerDeclarationIdentity(
            operation_index,
        ));
    }
    if candidate.kernel_catalog_id != context.kernel_catalog_id {
        return Err(OperationKernelPlanError::KernelCatalogIdentity(
            operation_index,
        ));
    }
    let expected = resolve_profile(generated, context.catalogs)
        .ok_or(OperationKernelPlanError::CanonicalProfile(operation_index))?;
    if candidate.profile_catalog_id != expected.catalog_id {
        return Err(OperationKernelPlanError::ProfileCatalogIdentity(
            operation_index,
        ));
    }
    if candidate.profile_id != expected.profile_id {
        return Err(OperationKernelPlanError::ProfileIdentity(operation_index));
    }
    let family = context
        .families
        .iter()
        .find(|family| family.family == generated.profile.family)
        .ok_or(OperationKernelPlanError::Family(operation_index))?;
    if candidate.family_build_id != family.build_id {
        return Err(OperationKernelPlanError::FamilyBuildIdentity(
            operation_index,
        ));
    }
    if candidate.artifact_id != family.artifact_id {
        return Err(OperationKernelPlanError::ArtifactIdentity(operation_index));
    }
    if candidate.abi_layout_id != family.abi_layout_id {
        return Err(OperationKernelPlanError::AbiLayoutIdentity(operation_index));
    }
    Ok(())
}

fn expected_family(operator: Qwen3Operator, mode: Qwen3ExecutionMode) -> KernelFamily {
    match operator {
        Qwen3Operator::TokenEmbedding
        | Qwen3Operator::QueryProjection
        | Qwen3Operator::KeyProjection
        | Qwen3Operator::ValueProjection
        | Qwen3Operator::AttentionOutputResidual
        | Qwen3Operator::GateProjection
        | Qwen3Operator::UpProjection
        | Qwen3Operator::DownResidual
        | Qwen3Operator::LogitsProjection => KernelFamily::K1GemmGemv,
        Qwen3Operator::InputRmsNorm
        | Qwen3Operator::QueryRmsNorm
        | Qwen3Operator::KeyRmsNorm
        | Qwen3Operator::PostAttentionRmsNorm
        | Qwen3Operator::FinalRmsNorm => KernelFamily::K2RmsNormResidual,
        Qwen3Operator::Rope | Qwen3Operator::KvWrite => KernelFamily::K3RopePagedKv,
        Qwen3Operator::Attention => match mode {
            Qwen3ExecutionMode::Prefill => KernelFamily::K4GqaPrefill,
            Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative => {
                KernelFamily::K5PagedGqaDecode
            }
        },
        Qwen3Operator::SwiGlu => KernelFamily::K6SwiGlu,
        Qwen3Operator::ArgmaxCompactCompletion => KernelFamily::K7LogitsCompact,
    }
}

struct ValidationContext<'a> {
    runner_declaration_id: Identity,
    kernel_catalog_id: Identity,
    families: &'a [DeclaredKernelFamilyArtifact],
    catalogs: &'a CanonicalCatalogs,
}

fn resolve_profile(
    operation: &GeneratedOperationDeclaration,
    catalogs: &CanonicalCatalogs,
) -> Option<ExpectedProfile> {
    let role_index = match operation.profile.selection.role {
        Qwen3ModelRole::Target8B => 0,
        Qwen3ModelRole::Draft06B => 1,
    };
    let bucket_index = M1_B3_PLAN_BUCKETS.iter().position(|&(mode, bucket)| {
        operation.profile.selection.mode == mode && operation.profile.selection.bucket == bucket
    })?;
    let operator = operation.profile.step.operator;
    match operator {
        Qwen3Operator::TokenEmbedding => profile_at(
            &catalogs.embedding,
            role_index * 11 + bucket_index,
            |profile| {
                usize::from(profile.bucket().role() as u8) == role_index + 1
                    && usize::from(profile.bucket().kind() as u8) == bucket_index + 1
            },
            |catalog| *catalog.identity().as_bytes(),
            |profile| *profile.identity().as_bytes(),
        ),
        Qwen3Operator::QueryProjection
        | Qwen3Operator::KeyProjection
        | Qwen3Operator::ValueProjection
        | Qwen3Operator::AttentionOutputResidual
        | Qwen3Operator::GateProjection
        | Qwen3Operator::UpProjection
        | Qwen3Operator::DownResidual
        | Qwen3Operator::LogitsProjection => {
            let operation_offset = match operator {
                Qwen3Operator::QueryProjection => 0,
                Qwen3Operator::KeyProjection => 1,
                Qwen3Operator::ValueProjection => 2,
                Qwen3Operator::AttentionOutputResidual => 3,
                Qwen3Operator::GateProjection => 4,
                Qwen3Operator::UpProjection => 5,
                Qwen3Operator::DownResidual => 6,
                Qwen3Operator::LogitsProjection => 7,
                _ => return None,
            };
            profile_at(
                &catalogs.gemm,
                (role_index * 11 + bucket_index) * 8 + operation_offset,
                |profile| {
                    usize::from(profile.bucket().role() as u8) == role_index + 1
                        && usize::from(profile.bucket().kind() as u8) == bucket_index + 1
                        && usize::from(profile.operation() as u8) == operation_offset + 1
                },
                |catalog| *catalog.identity().as_bytes(),
                |profile| *profile.identity().as_bytes(),
            )
        }
        Qwen3Operator::InputRmsNorm
        | Qwen3Operator::QueryRmsNorm
        | Qwen3Operator::KeyRmsNorm
        | Qwen3Operator::PostAttentionRmsNorm
        | Qwen3Operator::FinalRmsNorm => {
            let operation_offset = match operator {
                Qwen3Operator::InputRmsNorm => 0,
                Qwen3Operator::QueryRmsNorm => 1,
                Qwen3Operator::KeyRmsNorm => 2,
                Qwen3Operator::PostAttentionRmsNorm => 3,
                Qwen3Operator::FinalRmsNorm => 4,
                _ => return None,
            };
            profile_at(
                &catalogs.rmsnorm,
                (role_index * 11 + bucket_index) * 6 + operation_offset,
                |profile| {
                    usize::from(profile.bucket().role() as u8) == role_index + 1
                        && usize::from(profile.bucket().kind() as u8) == bucket_index + 1
                        && usize::from(profile.operation() as u8) == operation_offset + 1
                },
                |catalog| *catalog.identity().as_bytes(),
                |profile| *profile.identity().as_bytes(),
            )
        }
        Qwen3Operator::Rope | Qwen3Operator::KvWrite => {
            let operation_offset = usize::from(operator == Qwen3Operator::KvWrite);
            profile_at(
                &catalogs.rope_kv,
                (role_index * 11 + bucket_index) * 2 + operation_offset,
                |profile| {
                    usize::from(profile.bucket().role() as u8) == role_index + 1
                        && usize::from(profile.bucket().kind() as u8) == bucket_index + 1
                        && usize::from(profile.operation() as u8) == operation_offset + 1
                },
                |catalog| *catalog.identity().as_bytes(),
                |profile| *profile.identity().as_bytes(),
            )
        }
        Qwen3Operator::Attention => match operation.profile.selection.mode {
            Qwen3ExecutionMode::Prefill => profile_at(
                &catalogs.prefill,
                role_index * 4 + bucket_index,
                |profile| {
                    usize::from(profile.role() as u8) == role_index + 1
                        && usize::from(profile.bucket() as u8) == bucket_index + 1
                },
                |catalog| *catalog.identity().as_bytes(),
                |profile| *profile.identity().as_bytes(),
            ),
            Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative => profile_at(
                &catalogs.paged_decode,
                role_index * 7 + bucket_index.checked_sub(4)?,
                |profile| {
                    usize::from(profile.role() as u8) == role_index + 1
                        && usize::from(profile.bucket() as u8) == bucket_index - 3
                },
                |catalog| *catalog.identity().as_bytes(),
                |profile| *profile.identity().as_bytes(),
            ),
        },
        Qwen3Operator::SwiGlu => profile_at(
            &catalogs.swiglu,
            role_index * 11 + bucket_index,
            |profile| {
                usize::from(profile.role() as u8) == role_index + 1
                    && usize::from(profile.bucket() as u8) == bucket_index + 1
            },
            |catalog| *catalog.identity().as_bytes(),
            |profile| *profile.identity().as_bytes(),
        ),
        Qwen3Operator::ArgmaxCompactCompletion => profile_at(
            &catalogs.logits,
            role_index * 11 + bucket_index,
            |profile| {
                usize::from(profile.bucket().role() as u8) == role_index + 1
                    && usize::from(profile.bucket().kind() as u8) == bucket_index + 1
            },
            |catalog| *catalog.identity().as_bytes(),
            |profile| *profile.identity().as_bytes(),
        ),
    }
}

trait ProfileCatalog {
    type Profile: Copy;

    fn profiles(&self) -> &[Self::Profile];
}

impl ProfileCatalog for gemm::Qwen3GemmProfileCatalogV1 {
    type Profile = gemm::Qwen3GemmProfileV1;

    fn profiles(&self) -> &[Self::Profile] {
        self.profiles()
    }
}

impl ProfileCatalog for gemm::Qwen3TokenEmbeddingProfileCatalogV1 {
    type Profile = gemm::Qwen3TokenEmbeddingProfileV1;

    fn profiles(&self) -> &[Self::Profile] {
        self.profiles()
    }
}

impl ProfileCatalog for rmsnorm::Qwen3RmsNormProfileCatalogV1 {
    type Profile = rmsnorm::Qwen3RmsNormProfileV1;

    fn profiles(&self) -> &[Self::Profile] {
        self.profiles()
    }
}

impl ProfileCatalog for rope_kv::Qwen3RopeKvProfileCatalogV1 {
    type Profile = rope_kv::Qwen3RopeKvProfileV1;

    fn profiles(&self) -> &[Self::Profile] {
        self.profiles()
    }
}

impl ProfileCatalog for prefill::Qwen3PrefillProfileCatalogV1 {
    type Profile = prefill::Qwen3PrefillProfileV1;

    fn profiles(&self) -> &[Self::Profile] {
        self.profiles()
    }
}

impl ProfileCatalog for paged_decode::Qwen3PagedDecodeProfileCatalogV1 {
    type Profile = paged_decode::Qwen3PagedDecodeProfileV1;

    fn profiles(&self) -> &[Self::Profile] {
        self.profiles()
    }
}

impl ProfileCatalog for swiglu::Qwen3SwiGluProfileCatalogV1 {
    type Profile = swiglu::Qwen3SwiGluProfileV1;

    fn profiles(&self) -> &[Self::Profile] {
        self.profiles()
    }
}

impl ProfileCatalog for logits::Qwen3LogitsProfileCatalogV1 {
    type Profile = logits::Qwen3LogitsProfileV1;

    fn profiles(&self) -> &[Self::Profile] {
        self.profiles()
    }
}

fn profile_at<C, M, F, G>(
    catalog: &C,
    index: usize,
    matches: M,
    catalog_identity: F,
    profile_identity: G,
) -> Option<ExpectedProfile>
where
    C: ProfileCatalog,
    M: FnOnce(C::Profile) -> bool,
    F: FnOnce(&C) -> [u8; 32],
    G: FnOnce(C::Profile) -> [u8; 32],
{
    let profile = *catalog.profiles().get(index)?;
    if !matches(profile) {
        return None;
    }
    Some(ExpectedProfile {
        catalog_id: Identity::new(catalog_identity(catalog)),
        profile_id: Identity::new(profile_identity(profile)),
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::HashSet;

    use ferric_build::{
        generate_qwen3_gfx942_runner_declaration, publish_qwen3_gfx942_runner_declaration,
        qwen3_runner_closure_test_fixture, GeneratedOperationDeclaration,
        GeneratedRunnerDeclaration,
    };
    use ferric_kernels::{KernelFamily, KernelProfileDescriptor, KernelProfileDisposition};
    use ferric_qwen_kernels::{gemm, logits, paged_decode, prefill, rmsnorm, rope_kv, swiglu};
    use ferric_spec::{
        expected_step, Identity, Qwen3ModelRole, Qwen3Operator, Qwen3PlanBucket, Qwen3PlanSelection,
    };

    use super::{
        bind_declared_operation_kernel_plan, expected_family, resolve_profile, validate_families,
        validate_operation_sequence_with_catalogs, CanonicalCatalogs, DeclaredKernelFamilyArtifact,
        DeclaredOperationIdentity, DeclaredOperationKernelBinding, DeclaredOperationKernelPlan,
        LogicalRunnerDeclaration, OperationKernelIdentityComponent, OperationKernelPlanError,
        OperationKernelPlanOutcome, FAMILIES, M1_B3_PLAN_BUCKETS, M1_KERNEL_OPERATION_BINDINGS,
    };

    const TARGET_OPERATIONS: usize = 11 * 544;
    const DRAFT_OPERATIONS: usize = 11 * 424;
    const CANONICAL_QWEN_PROFILES: usize = 440;
    const GRAPH_USED_QWEN_PROFILES: usize = 418;
    const RUNNER_DECLARATION_ID: Identity = Identity::new([91; 32]);
    const KERNEL_CATALOG_ID: Identity = Identity::new([92; 32]);

    fn identity(seed: u32) -> Identity {
        let mut bytes = [0_u8; 32];
        bytes[..4].copy_from_slice(&seed.to_le_bytes());
        bytes[31] = 1;
        Identity::new(bytes)
    }

    fn family_artifacts() -> Box<[DeclaredKernelFamilyArtifact]> {
        FAMILIES
            .iter()
            .copied()
            .enumerate()
            .map(|(index, family)| {
                let index = u32::try_from(index).expect("seven families fit u32");
                DeclaredKernelFamilyArtifact::new(
                    family,
                    identity(100 + index),
                    identity(200 + index),
                    identity(300 + index),
                )
            })
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    fn generated_operations() -> Box<[GeneratedOperationDeclaration]> {
        let mut operations = Vec::with_capacity(M1_KERNEL_OPERATION_BINDINGS);
        let mut plan_index = 0_u16;
        for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
            for (mode, bucket) in M1_B3_PLAN_BUCKETS {
                let selection = Qwen3PlanSelection { role, mode, bucket };
                let dimensions = bucket
                    .dimensions(role, mode)
                    .expect("finite exact selection has dimensions");
                let plan_id = identity(1_000 + u32::from(plan_index));
                let mut ordinal = 0_u32;
                while let Some(step) = expected_step(role, mode, bucket, ordinal) {
                    let operation_index =
                        u32::try_from(operations.len()).expect("M1 operation count fits u32");
                    operations.push(GeneratedOperationDeclaration {
                        operation_index,
                        plan_index,
                        profile: KernelProfileDescriptor {
                            plan_id,
                            selection,
                            step,
                            sequences: dimensions.sequences,
                            active_tokens: dimensions.active_tokens,
                            context_tokens: dimensions.context_tokens,
                            family: expected_family(step.operator, mode),
                            disposition: KernelProfileDisposition::DeclaredFoundation,
                        },
                    });
                    ordinal += 1;
                }
                plan_index += 1;
            }
        }
        assert_eq!(operations.len(), M1_KERNEL_OPERATION_BINDINGS);
        operations.into_boxed_slice()
    }

    fn declared_operations(
        generated: &[GeneratedOperationDeclaration],
        families: &[DeclaredKernelFamilyArtifact],
        catalogs: &CanonicalCatalogs,
    ) -> Box<[DeclaredOperationKernelBinding]> {
        generated
            .iter()
            .map(|operation| {
                let expected = resolve_profile(operation, catalogs).expect("canonical profile");
                let family = families
                    .iter()
                    .copied()
                    .find(|family| family.family == operation.profile.family)
                    .expect("exact family declaration");
                DeclaredOperationKernelBinding::new(
                    operation.operation_index,
                    operation.plan_index,
                    DeclaredOperationIdentity::new(
                        operation.profile.plan_id,
                        RUNNER_DECLARATION_ID,
                        KERNEL_CATALOG_ID,
                        expected.catalog_id,
                        expected.profile_id,
                    ),
                    family,
                )
            })
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    fn validate(
        generated: &[GeneratedOperationDeclaration],
        families: &[DeclaredKernelFamilyArtifact],
        candidates: &[DeclaredOperationKernelBinding],
        catalogs: &CanonicalCatalogs,
    ) -> Result<(), OperationKernelPlanError> {
        validate_operation_sequence_with_catalogs(
            generated,
            RUNNER_DECLARATION_ID,
            KERNEL_CATALOG_ID,
            families,
            candidates,
            catalogs,
        )
    }

    #[derive(Clone, Copy)]
    struct PublicProfileFixture {
        role_tag: u8,
        bucket_tag: u8,
        operator: Qwen3Operator,
        catalog_id: Identity,
        profile_id: Identity,
    }

    fn insert_public_profile(
        fixtures: &mut Vec<PublicProfileFixture>,
        fixture: PublicProfileFixture,
    ) {
        assert!(!fixtures.iter().any(|prior| {
            prior.role_tag == fixture.role_tag
                && prior.bucket_tag == fixture.bucket_tag
                && prior.operator == fixture.operator
        }));
        fixtures.push(fixture);
    }

    const fn public_gemm_operator(operation: gemm::Qwen3GemmOperationV1) -> Qwen3Operator {
        match operation {
            gemm::Qwen3GemmOperationV1::QueryProjection => Qwen3Operator::QueryProjection,
            gemm::Qwen3GemmOperationV1::KeyProjection => Qwen3Operator::KeyProjection,
            gemm::Qwen3GemmOperationV1::ValueProjection => Qwen3Operator::ValueProjection,
            gemm::Qwen3GemmOperationV1::AttentionOutputResidual => {
                Qwen3Operator::AttentionOutputResidual
            }
            gemm::Qwen3GemmOperationV1::GateProjection => Qwen3Operator::GateProjection,
            gemm::Qwen3GemmOperationV1::UpProjection => Qwen3Operator::UpProjection,
            gemm::Qwen3GemmOperationV1::DownResidual => Qwen3Operator::DownResidual,
            gemm::Qwen3GemmOperationV1::LogitsProjection => Qwen3Operator::LogitsProjection,
        }
    }

    const fn public_rmsnorm_operator(
        operation: rmsnorm::Qwen3RmsNormOperationV1,
    ) -> Option<Qwen3Operator> {
        match operation {
            rmsnorm::Qwen3RmsNormOperationV1::InputRmsNorm => Some(Qwen3Operator::InputRmsNorm),
            rmsnorm::Qwen3RmsNormOperationV1::QueryRmsNorm => Some(Qwen3Operator::QueryRmsNorm),
            rmsnorm::Qwen3RmsNormOperationV1::KeyRmsNorm => Some(Qwen3Operator::KeyRmsNorm),
            rmsnorm::Qwen3RmsNormOperationV1::PostAttentionRmsNorm => {
                Some(Qwen3Operator::PostAttentionRmsNorm)
            }
            rmsnorm::Qwen3RmsNormOperationV1::FinalRmsNorm => Some(Qwen3Operator::FinalRmsNorm),
            rmsnorm::Qwen3RmsNormOperationV1::ResidualFusedHidden => None,
        }
    }

    const fn public_rope_kv_operator(operation: rope_kv::Qwen3RopeKvOperationV1) -> Qwen3Operator {
        match operation {
            rope_kv::Qwen3RopeKvOperationV1::Rope => Qwen3Operator::Rope,
            rope_kv::Qwen3RopeKvOperationV1::PagedKvWrite => Qwen3Operator::KvWrite,
        }
    }

    fn public_profile_fixtures() -> Vec<PublicProfileFixture> {
        let gemm = gemm::Qwen3GemmProfileCatalogV1::canonical().expect("public GEMM catalog");
        let embedding = gemm::Qwen3TokenEmbeddingProfileCatalogV1::canonical()
            .expect("public embedding catalog");
        let rmsnorm =
            rmsnorm::Qwen3RmsNormProfileCatalogV1::canonical().expect("public RMSNorm catalog");
        let rope_kv =
            rope_kv::Qwen3RopeKvProfileCatalogV1::canonical().expect("public RoPE/KV catalog");
        let prefill =
            prefill::Qwen3PrefillProfileCatalogV1::canonical().expect("public prefill catalog");
        let paged_decode = paged_decode::Qwen3PagedDecodeProfileCatalogV1::canonical()
            .expect("public decode catalog");
        let swiglu =
            swiglu::Qwen3SwiGluProfileCatalogV1::canonical().expect("public SwiGLU catalog");
        let logits =
            logits::Qwen3LogitsProfileCatalogV1::canonical().expect("public logits catalog");
        let catalog_counts = [
            gemm.profiles().len(),
            embedding.profiles().len(),
            rmsnorm.profiles().len(),
            rope_kv.profiles().len(),
            prefill.profiles().len(),
            paged_decode.profiles().len(),
            swiglu.profiles().len(),
            logits.profiles().len(),
        ];
        assert_eq!(catalog_counts, [176, 22, 132, 44, 8, 14, 22, 22]);
        assert_eq!(
            catalog_counts.into_iter().sum::<usize>(),
            CANONICAL_QWEN_PROFILES
        );

        let mut fixtures = Vec::with_capacity(GRAPH_USED_QWEN_PROFILES);
        for profile in gemm.profiles() {
            insert_public_profile(
                &mut fixtures,
                PublicProfileFixture {
                    role_tag: profile.bucket().role() as u8,
                    bucket_tag: profile.bucket().kind() as u8,
                    operator: public_gemm_operator(profile.operation()),
                    catalog_id: Identity::new(*gemm.identity().as_bytes()),
                    profile_id: Identity::new(*profile.identity().as_bytes()),
                },
            );
        }
        for profile in embedding.profiles() {
            insert_public_profile(
                &mut fixtures,
                PublicProfileFixture {
                    role_tag: profile.bucket().role() as u8,
                    bucket_tag: profile.bucket().kind() as u8,
                    operator: Qwen3Operator::TokenEmbedding,
                    catalog_id: Identity::new(*embedding.identity().as_bytes()),
                    profile_id: Identity::new(*profile.identity().as_bytes()),
                },
            );
        }
        for profile in rmsnorm.profiles() {
            let Some(operator) = public_rmsnorm_operator(profile.operation()) else {
                continue;
            };
            insert_public_profile(
                &mut fixtures,
                PublicProfileFixture {
                    role_tag: profile.bucket().role() as u8,
                    bucket_tag: profile.bucket().kind() as u8,
                    operator,
                    catalog_id: Identity::new(*rmsnorm.identity().as_bytes()),
                    profile_id: Identity::new(*profile.identity().as_bytes()),
                },
            );
        }
        for profile in rope_kv.profiles() {
            insert_public_profile(
                &mut fixtures,
                PublicProfileFixture {
                    role_tag: profile.bucket().role() as u8,
                    bucket_tag: profile.bucket().kind() as u8,
                    operator: public_rope_kv_operator(profile.operation()),
                    catalog_id: Identity::new(*rope_kv.identity().as_bytes()),
                    profile_id: Identity::new(*profile.identity().as_bytes()),
                },
            );
        }
        for profile in prefill.profiles() {
            insert_public_profile(
                &mut fixtures,
                PublicProfileFixture {
                    role_tag: profile.role() as u8,
                    bucket_tag: profile.bucket() as u8,
                    operator: Qwen3Operator::Attention,
                    catalog_id: Identity::new(*prefill.identity().as_bytes()),
                    profile_id: Identity::new(*profile.identity().as_bytes()),
                },
            );
        }
        for profile in paged_decode.profiles() {
            insert_public_profile(
                &mut fixtures,
                PublicProfileFixture {
                    role_tag: profile.role() as u8,
                    bucket_tag: profile.bucket() as u8 + 4,
                    operator: Qwen3Operator::Attention,
                    catalog_id: Identity::new(*paged_decode.identity().as_bytes()),
                    profile_id: Identity::new(*profile.identity().as_bytes()),
                },
            );
        }
        for profile in swiglu.profiles() {
            insert_public_profile(
                &mut fixtures,
                PublicProfileFixture {
                    role_tag: profile.role() as u8,
                    bucket_tag: profile.bucket() as u8,
                    operator: Qwen3Operator::SwiGlu,
                    catalog_id: Identity::new(*swiglu.identity().as_bytes()),
                    profile_id: Identity::new(*profile.identity().as_bytes()),
                },
            );
        }
        for profile in logits.profiles() {
            insert_public_profile(
                &mut fixtures,
                PublicProfileFixture {
                    role_tag: profile.bucket().role() as u8,
                    bucket_tag: profile.bucket().kind() as u8,
                    operator: Qwen3Operator::ArgmaxCompactCompletion,
                    catalog_id: Identity::new(*logits.identity().as_bytes()),
                    profile_id: Identity::new(*profile.identity().as_bytes()),
                },
            );
        }
        assert_eq!(fixtures.len(), GRAPH_USED_QWEN_PROFILES);
        fixtures
    }

    const fn public_model_role_tag(role: Qwen3ModelRole) -> u8 {
        match role {
            Qwen3ModelRole::Target8B => 1,
            Qwen3ModelRole::Draft06B => 2,
        }
    }

    const fn public_plan_bucket_tag(bucket: Qwen3PlanBucket) -> u8 {
        match bucket {
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
        }
    }

    fn public_operation_bindings(
        declaration: &GeneratedRunnerDeclaration,
        families: &[DeclaredKernelFamilyArtifact],
        fixtures: &[PublicProfileFixture],
    ) -> Box<[DeclaredOperationKernelBinding]> {
        declaration
            .operations()
            .iter()
            .map(|operation| {
                let profile = fixtures
                    .iter()
                    .find(|fixture| {
                        fixture.role_tag == public_model_role_tag(operation.profile.selection.role)
                            && fixture.bucket_tag
                                == public_plan_bucket_tag(operation.profile.selection.bucket)
                            && fixture.operator == operation.profile.step.operator
                    })
                    .expect("generated operation has one public Qwen profile");
                let family = families
                    .iter()
                    .copied()
                    .find(|family| family.family() == operation.profile.family)
                    .expect("generated operation has one family declaration");
                DeclaredOperationKernelBinding::new(
                    operation.operation_index,
                    operation.plan_index,
                    DeclaredOperationIdentity::new(
                        operation.profile.plan_id,
                        declaration.declaration_id(),
                        declaration.kernel_catalog_id(),
                        profile.catalog_id,
                        profile.profile_id,
                    ),
                    family,
                )
            })
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    fn public_runner_fixture() -> (
        LogicalRunnerDeclaration,
        Box<[DeclaredKernelFamilyArtifact]>,
        Box<[DeclaredOperationKernelBinding]>,
    ) {
        let declaration =
            generate_qwen3_gfx942_runner_declaration(qwen3_runner_closure_test_fixture())
                .expect("generated runner from compact sealed fixture");
        let families = family_artifacts();
        let profiles = public_profile_fixtures();
        let operations = public_operation_bindings(&declaration, &families, &profiles);
        let publication = publish_qwen3_gfx942_runner_declaration(declaration)
            .expect("published runner from compact sealed fixture");
        (
            LogicalRunnerDeclaration::from_published(publication),
            families,
            operations,
        )
    }

    pub(crate) fn public_operation_kernel_plan_fixture() -> DeclaredOperationKernelPlan {
        let (runner, families, operations) = public_runner_fixture();
        let OperationKernelPlanOutcome::Bound(plan) =
            bind_declared_operation_kernel_plan(runner, families, operations)
        else {
            panic!("exact published runner must bind");
        };
        plan
    }

    #[test]
    fn real_publication_binds_all_plans_and_rejection_recovers_exact_inputs() {
        let (runner, families, operations) = public_runner_fixture();
        let outcome = bind_declared_operation_kernel_plan(runner, families, operations);
        let OperationKernelPlanOutcome::Bound(plan) = outcome else {
            panic!("exact published runner must bind");
        };
        let mut plan_count = 0_usize;
        let mut operation_count = 0_usize;
        for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
            for (mode, bucket) in M1_B3_PLAN_BUCKETS {
                let selection = Qwen3PlanSelection { role, mode, bucket };
                let (generated, operations) = plan
                    .operation_declarations_for(selection)
                    .expect("published plan range");
                assert_eq!(
                    operations.len(),
                    match role {
                        Qwen3ModelRole::Target8B => 544,
                        Qwen3ModelRole::Draft06B => 424,
                    }
                );
                assert_eq!(generated.len(), operations.len());
                for (generated, binding) in generated.iter().zip(operations) {
                    assert_eq!(generated.operation_index, binding.operation_index());
                    assert_eq!(generated.plan_index, binding.plan_index());
                    assert_eq!(generated.profile.selection, selection);
                    assert_eq!(generated.profile.plan_id, binding.plan_id());
                    assert_eq!(generated.profile.family, binding.family());
                }
                assert_eq!(
                    usize::try_from(operations[0].operation_index()).expect("u32 fits usize"),
                    operation_count
                );
                operation_count += operations.len();
                plan_count += 1;
            }
        }
        assert_eq!(plan_count, 22);
        assert_eq!(operation_count, M1_KERNEL_OPERATION_BINDINGS);
        assert_eq!(plan.operations().len(), M1_KERNEL_OPERATION_BINDINGS);

        let (runner, families, mut operations) = public_runner_fixture();
        let retained_runner = (
            runner.source_id(),
            runner.admission_record_id(),
            runner.bundle_id(),
            runner.target_prepacked_id(),
            runner.draft_prepacked_id(),
            runner.plan_catalog_id(),
            runner.kernel_catalog_id(),
            runner.closure_id(),
            runner.declaration_id(),
            runner.plan_count(),
            runner.operation_count(),
        );
        let retained_families = families.clone();
        let first = operations[0];
        let family = families
            .iter()
            .copied()
            .find(|family| family.family() == first.family())
            .expect("first operation family");
        operations[0] = DeclaredOperationKernelBinding::new(
            first.operation_index(),
            first.plan_index(),
            DeclaredOperationIdentity::new(
                identity(99),
                first.runner_declaration_id(),
                first.kernel_catalog_id(),
                first.profile_catalog_id(),
                first.profile_id(),
            ),
            family,
        );
        let retained_operations = operations.clone();
        let outcome = bind_declared_operation_kernel_plan(runner, families, operations);
        let OperationKernelPlanOutcome::Rejected(failure) = outcome else {
            panic!("hostile plan identity must fail closed");
        };
        assert_eq!(failure.error(), OperationKernelPlanError::PlanIdentity(0));
        let (error, runner, families, operations) = failure.into_parts();
        assert_eq!(error, OperationKernelPlanError::PlanIdentity(0));
        assert_eq!(families, retained_families);
        assert_eq!(operations, retained_operations);
        assert_eq!(
            (
                runner.source_id(),
                runner.admission_record_id(),
                runner.bundle_id(),
                runner.target_prepacked_id(),
                runner.draft_prepacked_id(),
                runner.plan_catalog_id(),
                runner.kernel_catalog_id(),
                runner.closure_id(),
                runner.declaration_id(),
                runner.plan_count(),
                runner.operation_count(),
            ),
            retained_runner
        );
    }

    #[test]
    fn all_target_and_draft_operations_bind_to_the_exact_qwen_profile_roster() {
        let catalogs = CanonicalCatalogs::build().expect("canonical catalogs");
        let families = family_artifacts();
        let generated = generated_operations();
        let candidates = declared_operations(&generated, &families, &catalogs);

        let catalog_counts = [
            catalogs.gemm.profiles().len(),
            catalogs.embedding.profiles().len(),
            catalogs.rmsnorm.profiles().len(),
            catalogs.rope_kv.profiles().len(),
            catalogs.prefill.profiles().len(),
            catalogs.paged_decode.profiles().len(),
            catalogs.swiglu.profiles().len(),
            catalogs.logits.profiles().len(),
        ];
        assert_eq!(catalog_counts, [176, 22, 132, 44, 8, 14, 22, 22]);
        assert_eq!(
            catalog_counts.into_iter().sum::<usize>(),
            CANONICAL_QWEN_PROFILES
        );
        assert_eq!(
            validate(&generated, &families, &candidates, &catalogs),
            Ok(())
        );
        assert_eq!(
            generated
                .iter()
                .filter(|operation| {
                    operation.profile.selection.role == Qwen3ModelRole::Target8B
                })
                .count(),
            TARGET_OPERATIONS
        );
        assert_eq!(
            generated
                .iter()
                .filter(|operation| {
                    operation.profile.selection.role == Qwen3ModelRole::Draft06B
                })
                .count(),
            DRAFT_OPERATIONS
        );
        let profiles: HashSet<_> = candidates
            .iter()
            .map(|candidate| (candidate.profile_catalog_id, candidate.profile_id))
            .collect();
        assert_eq!(profiles.len(), GRAPH_USED_QWEN_PROFILES);
        assert!(candidates.iter().all(|candidate| {
            candidate.plan_id.is_present()
                && candidate.runner_declaration_id == RUNNER_DECLARATION_ID
                && candidate.kernel_catalog_id == KERNEL_CATALOG_ID
                && candidate.profile_catalog_id.is_present()
                && candidate.profile_id.is_present()
        }));
        for family in FAMILIES {
            assert!(candidates
                .iter()
                .any(|candidate| candidate.family == family));
        }
    }

    #[test]
    fn family_roster_rejects_count_duplicate_order_and_every_absent_identity_role() {
        let exact = family_artifacts();
        assert_eq!(validate_families(&exact), Ok(()));
        assert_eq!(
            validate_families(&exact[..6]),
            Err(OperationKernelPlanError::FamilyCount {
                expected: 7,
                actual: 6,
            })
        );

        let mut changed = exact.to_vec();
        changed[1].family = changed[0].family;
        assert_eq!(
            validate_families(&changed),
            Err(OperationKernelPlanError::DuplicateFamily(
                KernelFamily::K1GemmGemv
            ))
        );

        changed = exact.to_vec();
        changed.swap(0, 1);
        assert_eq!(
            validate_families(&changed),
            Err(OperationKernelPlanError::FamilyOrder {
                index: 0,
                expected: KernelFamily::K1GemmGemv,
                actual: KernelFamily::K2RmsNormResidual,
            })
        );

        for component in [
            OperationKernelIdentityComponent::FamilyBuild,
            OperationKernelIdentityComponent::Artifact,
            OperationKernelIdentityComponent::AbiLayout,
        ] {
            changed = exact.to_vec();
            match component {
                OperationKernelIdentityComponent::FamilyBuild => {
                    changed[0].build_id = Identity::new([0; 32]);
                }
                OperationKernelIdentityComponent::Artifact => {
                    changed[0].artifact_id = Identity::new([0; 32]);
                }
                OperationKernelIdentityComponent::AbiLayout => {
                    changed[0].abi_layout_id = Identity::new([0; 32]);
                }
            }
            assert_eq!(
                validate_families(&changed),
                Err(OperationKernelPlanError::MissingFamilyIdentity {
                    family: KernelFamily::K1GemmGemv,
                    component,
                })
            );
        }
    }

    #[test]
    fn hostile_operation_order_count_and_duplicate_indices_fail_closed() {
        let catalogs = CanonicalCatalogs::build().expect("canonical catalogs");
        let families = family_artifacts();
        let generated = generated_operations();
        let exact = declared_operations(&generated, &families, &catalogs);

        assert_eq!(
            validate(&generated, &families, &exact[..exact.len() - 1], &catalogs),
            Err(OperationKernelPlanError::OperationCount {
                expected: M1_KERNEL_OPERATION_BINDINGS,
                actual: M1_KERNEL_OPERATION_BINDINGS - 1,
            })
        );

        let mut changed = exact.to_vec();
        changed[0].operation_index = u32::MAX;
        assert_eq!(
            validate(&generated, &families, &changed, &catalogs),
            Err(OperationKernelPlanError::OperationOrder {
                position: 0,
                expected: 0,
                actual: u32::MAX,
            })
        );

        changed = exact.to_vec();
        changed[1].operation_index = changed[0].operation_index;
        assert_eq!(
            validate(&generated, &families, &changed, &catalogs),
            Err(OperationKernelPlanError::DuplicateOperationIndex(0))
        );

        changed = exact.to_vec();
        changed.swap(0, 1);
        assert_eq!(
            validate(&generated, &families, &changed, &catalogs),
            Err(OperationKernelPlanError::OperationOrder {
                position: 0,
                expected: 0,
                actual: 1,
            })
        );
    }

    #[test]
    fn every_operation_identity_boundary_rejects_hostile_substitution() {
        let catalogs = CanonicalCatalogs::build().expect("canonical catalogs");
        let families = family_artifacts();
        let generated = generated_operations();
        let exact = declared_operations(&generated, &families, &catalogs);

        let mut changed = exact.to_vec();
        changed[0].plan_index = 1;
        assert_eq!(
            validate(&generated, &families, &changed, &catalogs),
            Err(OperationKernelPlanError::PlanIndex(0))
        );

        changed = exact.to_vec();
        changed[0].family = KernelFamily::K2RmsNormResidual;
        assert_eq!(
            validate(&generated, &families, &changed, &catalogs),
            Err(OperationKernelPlanError::Family(0))
        );

        for (replacement, error) in [
            (identity(401), OperationKernelPlanError::PlanIdentity(0)),
            (
                identity(402),
                OperationKernelPlanError::RunnerDeclarationIdentity(0),
            ),
            (
                identity(403),
                OperationKernelPlanError::KernelCatalogIdentity(0),
            ),
            (
                identity(404),
                OperationKernelPlanError::ProfileCatalogIdentity(0),
            ),
            (identity(405), OperationKernelPlanError::ProfileIdentity(0)),
            (
                identity(406),
                OperationKernelPlanError::FamilyBuildIdentity(0),
            ),
            (identity(407), OperationKernelPlanError::ArtifactIdentity(0)),
            (
                identity(408),
                OperationKernelPlanError::AbiLayoutIdentity(0),
            ),
        ] {
            changed = exact.to_vec();
            match error {
                OperationKernelPlanError::PlanIdentity(_) => changed[0].plan_id = replacement,
                OperationKernelPlanError::RunnerDeclarationIdentity(_) => {
                    changed[0].runner_declaration_id = replacement;
                }
                OperationKernelPlanError::KernelCatalogIdentity(_) => {
                    changed[0].kernel_catalog_id = replacement;
                }
                OperationKernelPlanError::ProfileCatalogIdentity(_) => {
                    changed[0].profile_catalog_id = replacement;
                }
                OperationKernelPlanError::ProfileIdentity(_) => {
                    changed[0].profile_id = replacement;
                }
                OperationKernelPlanError::FamilyBuildIdentity(_) => {
                    changed[0].family_build_id = replacement;
                }
                OperationKernelPlanError::ArtifactIdentity(_) => {
                    changed[0].artifact_id = replacement;
                }
                OperationKernelPlanError::AbiLayoutIdentity(_) => {
                    changed[0].abi_layout_id = replacement;
                }
                _ => unreachable!("exact hostile identity error roster"),
            }
            assert_eq!(
                validate(&generated, &families, &changed, &catalogs),
                Err(error)
            );
        }
    }

    #[test]
    fn generated_wrong_family_or_disposition_cannot_cross_into_qwen_binding() {
        let catalogs = CanonicalCatalogs::build().expect("canonical catalogs");
        let families = family_artifacts();
        let generated = generated_operations();
        let exact = declared_operations(&generated, &families, &catalogs);

        let mut changed_generated = generated.to_vec();
        changed_generated[0].profile.family = KernelFamily::K2RmsNormResidual;
        let mut changed_candidates = exact.to_vec();
        changed_candidates[0].family = KernelFamily::K2RmsNormResidual;
        assert_eq!(
            validate(
                &changed_generated,
                &families,
                &changed_candidates,
                &catalogs,
            ),
            Err(OperationKernelPlanError::Family(0))
        );

        changed_generated = generated.to_vec();
        changed_generated[0].profile.disposition = KernelProfileDisposition::RequiredExtension;
        assert_eq!(
            validate(&changed_generated, &families, &exact, &catalogs),
            Err(OperationKernelPlanError::Family(0))
        );
    }
}
