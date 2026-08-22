//! Exact finite mapping from sequential graph operations to Ferric-owned K1-K7 sources.

use ferric_spec::{
    expected_step, Identity, Qwen3BufferKind, Qwen3ExecutionMode, Qwen3GeneratedPlan,
    Qwen3ModelRole, Qwen3Operator, Qwen3PlanBucket, Qwen3PlanBuffer, Qwen3PlanGeometry,
    Qwen3PlanSelection, Qwen3PlanShape, Qwen3PlanStep,
};
use vstd::prelude::*;

verus! {

/// Canonical structural kernel-catalog record version.
pub const M1_KERNEL_CATALOG_VERSION: u32 = 2;
/// Exact number of target/draft B3 plans.
pub const M1_KERNEL_PLAN_COUNT: usize = 22;
/// Exact number of graph operations across all 22 plans.
pub const M1_KERNEL_OPERATION_BINDINGS: usize = 10_648;
/// All and only finite B3 mode/bucket pairs, in catalog order.
pub const M1_B3_PLAN_BUCKETS: [(Qwen3ExecutionMode, Qwen3PlanBucket); 11] = [
    (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128),
    (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS8T128),
    (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T512),
    (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T2048),
    (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192),
    (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS8C8192),
    (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS32C8192),
    (
        Qwen3ExecutionMode::Speculative,
        Qwen3PlanBucket::SpeculativeS1K4C8192,
    ),
    (
        Qwen3ExecutionMode::Speculative,
        Qwen3PlanBucket::SpeculativeS8K4C8192,
    ),
    (
        Qwen3ExecutionMode::Speculative,
        Qwen3PlanBucket::SpeculativeS1K8C8192,
    ),
    (
        Qwen3ExecutionMode::Speculative,
        Qwen3PlanBucket::SpeculativeS1K16C8192,
    ),
];

/// Ferric-owned kernel family named by one structural declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KernelFamily {
    /// K1 B3 GEMM/GEMV linear specialization.
    K1GemmGemv,
    /// K2 `RMSNorm` plus residual foundation.
    K2RmsNormResidual,
    /// K3 Qwen3 `RoPE` and exclusive paged-KV foundation.
    K3RopePagedKv,
    /// K4 causal GQA prefill foundation.
    K4GqaPrefill,
    /// K5 paged causal GQA decode/speculative foundation.
    K5PagedGqaDecode,
    /// K6 `SwiGLU` foundation.
    K6SwiGlu,
    /// K7 logits, lowest-ID argmax, and compact-record foundation.
    K7LogitsCompact,
}

/// Whether the declared Ferric source is intended to cover this graph boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KernelProfileDisposition {
    /// The Ferric source declaration names the same finite operator boundary.
    DeclaredFoundation,
    /// The Ferric implementation must extend the currently declared boundary.
    RequiredExtension,
}

/// Compiler/runtime identity required by the structural catalog.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KernelAuthorityComponent {
    /// Future exact generic fe2o3 compiler/runtime dependency closure.
    Fe2o3Source,
    /// Future exact compiler implementation.
    Compiler,
    /// Future exact compiler invocation and target configuration.
    CompilerConfiguration,
    /// Future independent target contract.
    TargetContract,
    /// Future exact kernel proof set.
    KernelProofSet,
    /// Future exact kernel ABI catalog.
    KernelAbiCatalog,
    /// Future reviewed runtime contract.
    RuntimeContract,
    /// Future exact runtime ABI and queue protocol.
    RuntimeAbi,
    /// Future explicit compiler/runtime/hardware TCB report.
    TcbReport,
}

/// Caller-supplied future authorities bound into every catalog identity.
///
/// Presence is structural only. No field is authenticated by this crate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelAuthorityRequirements {
    /// Exact generic fe2o3 compiler/runtime dependency closure.
    ///
    /// This identity does not include or grant ownership of Ferric kernel
    /// source. Model-specific kernel declarations are bound separately.
    pub fe2o3_source: Identity,
    /// Exact compiler implementation.
    pub compiler: Identity,
    /// Exact compiler invocation and target configuration.
    pub compiler_configuration: Identity,
    /// Independent gfx942 target contract.
    pub target_contract: Identity,
    /// Exact kernel proof set.
    pub kernel_proof_set: Identity,
    /// Exact kernel ABI catalog.
    pub kernel_abi_catalog: Identity,
    /// Reviewed runtime contract.
    pub runtime_contract: Identity,
    /// Exact runtime ABI and queue protocol.
    pub runtime_abi: Identity,
    /// Explicit compiler/runtime/hardware TCB report.
    pub tcb_report: Identity,
}

impl KernelAuthorityRequirements {
    pub(crate) closed spec fn components_spec(&self) -> Seq<(KernelAuthorityComponent, Identity)> {
        Seq::empty()
            .push((KernelAuthorityComponent::Fe2o3Source, self.fe2o3_source))
            .push((KernelAuthorityComponent::Compiler, self.compiler))
            .push((
                KernelAuthorityComponent::CompilerConfiguration,
                self.compiler_configuration,
            ))
            .push((
                KernelAuthorityComponent::TargetContract,
                self.target_contract,
            ))
            .push((
                KernelAuthorityComponent::KernelProofSet,
                self.kernel_proof_set,
            ))
            .push((
                KernelAuthorityComponent::KernelAbiCatalog,
                self.kernel_abi_catalog,
            ))
            .push((
                KernelAuthorityComponent::RuntimeContract,
                self.runtime_contract,
            ))
            .push((KernelAuthorityComponent::RuntimeAbi, self.runtime_abi))
            .push((KernelAuthorityComponent::TcbReport, self.tcb_report))
    }

    pub(crate) fn components(&self) -> (components: [(KernelAuthorityComponent, Identity); 9])
        ensures components@ == self.components_spec(),
    {
        [
            (KernelAuthorityComponent::Fe2o3Source, self.fe2o3_source),
            (KernelAuthorityComponent::Compiler, self.compiler),
            (
                KernelAuthorityComponent::CompilerConfiguration,
                self.compiler_configuration,
            ),
            (
                KernelAuthorityComponent::TargetContract,
                self.target_contract,
            ),
            (
                KernelAuthorityComponent::KernelProofSet,
                self.kernel_proof_set,
            ),
            (
                KernelAuthorityComponent::KernelAbiCatalog,
                self.kernel_abi_catalog,
            ),
            (
                KernelAuthorityComponent::RuntimeContract,
                self.runtime_contract,
            ),
            (KernelAuthorityComponent::RuntimeAbi, self.runtime_abi),
            (KernelAuthorityComponent::TcbReport, self.tcb_report),
        ]
    }
}

/// One exact graph operation and its finite source-declaration profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelProfileDescriptor {
    /// Exact plan identity containing this operation.
    pub plan_id: Identity,
    /// Exact role, execution mode, and B3 bucket.
    pub selection: Qwen3PlanSelection,
    /// Exact graph step, including all shapes and buffer edges.
    pub step: Qwen3PlanStep,
    /// Exact bucket sequence count.
    pub sequences: u32,
    /// Exact role-dependent active-token count per sequence.
    pub active_tokens: u32,
    /// Exact bucket context-token bound.
    pub context_tokens: u32,
    /// K1-K7 family selected for this operation.
    pub family: KernelFamily,
    /// Whether the Ferric declaration covers this boundary or needs extension.
    pub disposition: KernelProfileDisposition,
}

/// Stable plan/operation position plus its exact profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelOperationBinding {
    /// Zero-based position in the exact target-then-draft plan catalog.
    pub plan_index: u16,
    /// Exact operation profile.
    pub profile: KernelProfileDescriptor,
}

pub(crate) closed spec fn family_for_spec(
    operator: Qwen3Operator,
    mode: Qwen3ExecutionMode,
) -> (KernelFamily, KernelProfileDisposition) {
    match operator {
        Qwen3Operator::TokenEmbedding => (
            KernelFamily::K1GemmGemv,
            KernelProfileDisposition::RequiredExtension,
        ),
        Qwen3Operator::QueryProjection
        | Qwen3Operator::KeyProjection
        | Qwen3Operator::ValueProjection
        | Qwen3Operator::AttentionOutputResidual
        | Qwen3Operator::GateProjection
        | Qwen3Operator::UpProjection
        | Qwen3Operator::DownResidual
        | Qwen3Operator::LogitsProjection => (
            KernelFamily::K1GemmGemv,
            KernelProfileDisposition::DeclaredFoundation,
        ),
        Qwen3Operator::InputRmsNorm
        | Qwen3Operator::QueryRmsNorm
        | Qwen3Operator::KeyRmsNorm
        | Qwen3Operator::PostAttentionRmsNorm
        | Qwen3Operator::FinalRmsNorm => (
            KernelFamily::K2RmsNormResidual,
            KernelProfileDisposition::DeclaredFoundation,
        ),
        Qwen3Operator::Rope | Qwen3Operator::KvWrite => (
            KernelFamily::K3RopePagedKv,
            KernelProfileDisposition::DeclaredFoundation,
        ),
        Qwen3Operator::Attention => match mode {
            Qwen3ExecutionMode::Prefill => (
                KernelFamily::K4GqaPrefill,
                KernelProfileDisposition::DeclaredFoundation,
            ),
            Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative => (
                KernelFamily::K5PagedGqaDecode,
                KernelProfileDisposition::DeclaredFoundation,
            ),
        },
        Qwen3Operator::SwiGlu => (
            KernelFamily::K6SwiGlu,
            KernelProfileDisposition::DeclaredFoundation,
        ),
        Qwen3Operator::ArgmaxCompactCompletion => (
            KernelFamily::K7LogitsCompact,
            KernelProfileDisposition::DeclaredFoundation,
        ),
    }
}

pub(crate) fn family_for(
    operator: Qwen3Operator,
    mode: Qwen3ExecutionMode,
) -> (result: (KernelFamily, KernelProfileDisposition))
    ensures result == family_for_spec(operator, mode),
{
    match operator {
        Qwen3Operator::TokenEmbedding => (
            KernelFamily::K1GemmGemv,
            KernelProfileDisposition::RequiredExtension,
        ),
        Qwen3Operator::QueryProjection
        | Qwen3Operator::KeyProjection
        | Qwen3Operator::ValueProjection
        | Qwen3Operator::AttentionOutputResidual
        | Qwen3Operator::GateProjection
        | Qwen3Operator::UpProjection
        | Qwen3Operator::DownResidual
        | Qwen3Operator::LogitsProjection => (
            KernelFamily::K1GemmGemv,
            KernelProfileDisposition::DeclaredFoundation,
        ),
        Qwen3Operator::InputRmsNorm
        | Qwen3Operator::QueryRmsNorm
        | Qwen3Operator::KeyRmsNorm
        | Qwen3Operator::PostAttentionRmsNorm
        | Qwen3Operator::FinalRmsNorm => (
            KernelFamily::K2RmsNormResidual,
            KernelProfileDisposition::DeclaredFoundation,
        ),
        Qwen3Operator::Rope | Qwen3Operator::KvWrite => (
            KernelFamily::K3RopePagedKv,
            KernelProfileDisposition::DeclaredFoundation,
        ),
        Qwen3Operator::Attention => match mode {
            Qwen3ExecutionMode::Prefill => (
                KernelFamily::K4GqaPrefill,
                KernelProfileDisposition::DeclaredFoundation,
            ),
            Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative => (
                KernelFamily::K5PagedGqaDecode,
                KernelProfileDisposition::DeclaredFoundation,
            ),
        },
        Qwen3Operator::SwiGlu => (
            KernelFamily::K6SwiGlu,
            KernelProfileDisposition::DeclaredFoundation,
        ),
        Qwen3Operator::ArgmaxCompactCompletion => (
            KernelFamily::K7LogitsCompact,
            KernelProfileDisposition::DeclaredFoundation,
        ),
    }
}

} // verus!

/// Exact target processor required by every future executable candidate.
pub const GFX942_PROCESSOR: &str = "gfx942";
/// Exact target-feature policy required by every future executable candidate.
pub const GFX942_TARGET_FEATURES: &str = "+wavefrontsize64,-xnack";

/// One Ferric-owned kernel-family source declaration.
///
/// This record names an ownership path. It is not evidence that the path
/// exists, has been reviewed, implements the family, or satisfies an M1
/// obligation. Exact source identity remains a separate closure input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelSourceDeclaration {
    /// Stable K1-K7 family.
    pub family: KernelFamily,
    /// Repository owning the model-specific implementation.
    pub repository: &'static str,
    /// Primary Ferric source path for the operator declaration.
    pub source_path: &'static str,
}

/// K1-K7 Ferric source declarations in family order.
///
/// Missing files remain explicit future obligations in
/// `proofs/M1_REQUIREMENTS.json`; this roster does not claim availability.
pub const FERRIC_KERNEL_SOURCE_DECLARATIONS: [KernelSourceDeclaration; 7] = [
    KernelSourceDeclaration {
        family: KernelFamily::K1GemmGemv,
        repository: "harsh-nod/ferric",
        source_path: "crates/ferric-qwen-kernels/src/gemm.rs",
    },
    KernelSourceDeclaration {
        family: KernelFamily::K2RmsNormResidual,
        repository: "harsh-nod/ferric",
        source_path: "crates/ferric-qwen-kernels/src/rmsnorm.rs",
    },
    KernelSourceDeclaration {
        family: KernelFamily::K3RopePagedKv,
        repository: "harsh-nod/ferric",
        source_path: "crates/ferric-qwen-kernels/src/rope_kv.rs",
    },
    KernelSourceDeclaration {
        family: KernelFamily::K4GqaPrefill,
        repository: "harsh-nod/ferric",
        source_path: "crates/ferric-qwen-kernels/src/prefill.rs",
    },
    KernelSourceDeclaration {
        family: KernelFamily::K5PagedGqaDecode,
        repository: "harsh-nod/ferric",
        source_path: "crates/ferric-qwen-kernels/src/paged_decode.rs",
    },
    KernelSourceDeclaration {
        family: KernelFamily::K6SwiGlu,
        repository: "harsh-nod/ferric",
        source_path: "crates/ferric-qwen-kernels/src/swiglu.rs",
    },
    KernelSourceDeclaration {
        family: KernelFamily::K7LogitsCompact,
        repository: "harsh-nod/ferric",
        source_path: "crates/ferric-qwen-kernels/src/logits.rs",
    },
];

/// Fail-closed structural kernel-catalog error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KernelCatalogError {
    /// The plan-catalog identity is absent.
    MissingPlanCatalogIdentity,
    /// One compiler/runtime requirement is absent.
    MissingAuthority(KernelAuthorityComponent),
    /// Two independent authority components reused an identity.
    ReusedAuthority {
        /// Earlier component.
        first: KernelAuthorityComponent,
        /// Later component.
        second: KernelAuthorityComponent,
    },
    /// The declared K1-K7 source roster has the wrong length.
    SourceDeclarationCount {
        /// Exact required length.
        expected: usize,
        /// Observed length.
        actual: usize,
    },
    /// A K1-K7 repository or path declaration drifted.
    SourceDeclarationDrift(KernelFamily),
    /// The plan input has the wrong finite length.
    PlanCount {
        /// Exact required length.
        expected: usize,
        /// Observed length.
        actual: usize,
    },
    /// A plan is not at its exact role/mode/bucket position.
    PlanSelection { plan_index: usize },
    /// A plan fails its exact sequential graph contract.
    InvalidPlan { plan_index: usize },
    /// A plan identity is absent or reused.
    InvalidPlanIdentity { plan_index: usize },
    /// A profile does not name the expected plan identity or selection.
    ProfileSelection,
    /// A profile step, geometry, buffer edge, or shape is not exact.
    ProfileStep,
    /// A profile uses the wrong K1-K7 family or extension disposition.
    ProfileFamily,
    /// A catalog construction bug produced the wrong operation count.
    OperationCount {
        /// Exact required count.
        expected: usize,
        /// Observed count.
        actual: usize,
    },
    /// A retained catalog differs from independently supplied exact inputs.
    CatalogDrift,
}

/// Structural, inert finite kernel catalog.
///
/// This value is not an executable catalog. Its canonical bytes require a
/// separate domain-separated identity in `ferric-build`, and that identity is
/// still preliminary until independent evidence authenticates every input.
#[derive(Debug, Eq, PartialEq)]
pub struct StructuralKernelCatalog {
    plan_catalog_id: Identity,
    authorities: KernelAuthorityRequirements,
    source_declarations: Box<[KernelSourceDeclaration]>,
    bindings: Box<[KernelOperationBinding]>,
    canonical_bytes: Box<[u8]>,
}

impl StructuralKernelCatalog {
    /// Returns the exact plan-catalog identity bound by this record.
    #[must_use]
    pub const fn plan_catalog_id(&self) -> Identity {
        self.plan_catalog_id
    }

    /// Returns the caller-supplied future authority requirements.
    #[must_use]
    pub const fn authorities(&self) -> KernelAuthorityRequirements {
        self.authorities
    }

    /// Returns the exact Ferric-owned K1-K7 source declarations.
    #[must_use]
    pub fn source_declarations(&self) -> &[KernelSourceDeclaration] {
        &self.source_declarations
    }

    /// Returns every plan operation in exact plan/ordinal order.
    #[must_use]
    pub fn bindings(&self) -> &[KernelOperationBinding] {
        &self.bindings
    }

    /// Returns the canonical bytes for external domain-separated hashing.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Returns the canonical format version.
    #[must_use]
    pub const fn version(&self) -> u32 {
        M1_KERNEL_CATALOG_VERSION
    }
}

/// Builds the exact inert structural catalog from a sequential plan catalog.
///
/// # Errors
///
/// Returns [`KernelCatalogError`] for any plan, profile, source-declaration, or
/// compiler/runtime authority drift. Success does not grant any execution or
/// evidence authority.
pub fn build_structural_kernel_catalog(
    plans: &[Qwen3GeneratedPlan],
    plan_catalog_id: Identity,
    source_declarations: &[KernelSourceDeclaration],
    authorities: KernelAuthorityRequirements,
) -> Result<StructuralKernelCatalog, KernelCatalogError> {
    if !plan_catalog_id.is_present() {
        return Err(KernelCatalogError::MissingPlanCatalogIdentity);
    }
    validate_authorities(&authorities)?;
    validate_source_declarations(source_declarations)?;
    if plans.len() != M1_KERNEL_PLAN_COUNT {
        return Err(KernelCatalogError::PlanCount {
            expected: M1_KERNEL_PLAN_COUNT,
            actual: plans.len(),
        });
    }

    let mut bindings = Vec::with_capacity(M1_KERNEL_OPERATION_BINDINGS);
    for (plan_index, plan) in plans.iter().enumerate() {
        let expected_selection = expected_selection(plan_index);
        if plan.selection != expected_selection {
            return Err(KernelCatalogError::PlanSelection { plan_index });
        }
        if !plan.authority.plan_id.is_present()
            || plans[..plan_index]
                .iter()
                .any(|prior| prior.authority.plan_id == plan.authority.plan_id)
        {
            return Err(KernelCatalogError::InvalidPlanIdentity { plan_index });
        }
        if plan.validate(plan.authority, expected_selection).is_err() {
            return Err(KernelCatalogError::InvalidPlan { plan_index });
        }
        for step in &plan.steps {
            let profile = canonical_profile(plan, *step)?;
            let plan_index =
                u16::try_from(plan_index).map_err(|_| KernelCatalogError::PlanCount {
                    expected: M1_KERNEL_PLAN_COUNT,
                    actual: plans.len(),
                })?;
            bindings.push(KernelOperationBinding {
                plan_index,
                profile,
            });
        }
    }
    if bindings.len() != M1_KERNEL_OPERATION_BINDINGS {
        return Err(KernelCatalogError::OperationCount {
            expected: M1_KERNEL_OPERATION_BINDINGS,
            actual: bindings.len(),
        });
    }
    let canonical_bytes = encode_catalog(plan_catalog_id, &authorities, plans, &bindings);
    Ok(StructuralKernelCatalog {
        plan_catalog_id,
        authorities,
        source_declarations: source_declarations.to_vec().into_boxed_slice(),
        bindings: bindings.into_boxed_slice(),
        canonical_bytes: canonical_bytes.into_boxed_slice(),
    })
}

/// Revalidates a retained structural catalog against independent exact inputs.
///
/// # Errors
///
/// Returns the specific input validation error, or [`KernelCatalogError::CatalogDrift`]
/// if valid expected inputs construct a different canonical catalog.
pub fn validate_structural_kernel_catalog(
    catalog: &StructuralKernelCatalog,
    plans: &[Qwen3GeneratedPlan],
    plan_catalog_id: Identity,
    source_declarations: &[KernelSourceDeclaration],
    authorities: KernelAuthorityRequirements,
) -> Result<(), KernelCatalogError> {
    let expected =
        build_structural_kernel_catalog(plans, plan_catalog_id, source_declarations, authorities)?;
    if catalog != &expected {
        return Err(KernelCatalogError::CatalogDrift);
    }
    Ok(())
}

/// Validates one profile against its independently supplied exact plan step.
///
/// # Errors
///
/// Returns [`KernelCatalogError`] for role, mode, bucket, identity, operator,
/// family, geometry, buffer-edge, or shape drift.
pub fn validate_kernel_profile(
    profile: KernelProfileDescriptor,
    plan: &Qwen3GeneratedPlan,
    ordinal: u32,
) -> Result<(), KernelCatalogError> {
    if profile.plan_id != plan.authority.plan_id || profile.selection != plan.selection {
        return Err(KernelCatalogError::ProfileSelection);
    }
    let Some(expected) = expected_step(
        plan.selection.role,
        plan.selection.mode,
        plan.selection.bucket,
        ordinal,
    ) else {
        return Err(KernelCatalogError::ProfileStep);
    };
    if profile.step != expected || profile.step.ordinal != ordinal {
        return Err(KernelCatalogError::ProfileStep);
    }
    let Some(dimensions) = plan
        .selection
        .bucket
        .dimensions(plan.selection.role, plan.selection.mode)
    else {
        return Err(KernelCatalogError::ProfileSelection);
    };
    if profile.sequences != dimensions.sequences
        || profile.active_tokens != dimensions.active_tokens
        || profile.context_tokens != dimensions.context_tokens
    {
        return Err(KernelCatalogError::ProfileStep);
    }
    let (family, disposition) = family_for(profile.step.operator, profile.selection.mode);
    if profile.family != family || profile.disposition != disposition {
        return Err(KernelCatalogError::ProfileFamily);
    }
    Ok(())
}

fn canonical_profile(
    plan: &Qwen3GeneratedPlan,
    step: Qwen3PlanStep,
) -> Result<KernelProfileDescriptor, KernelCatalogError> {
    let Some(dimensions) = plan
        .selection
        .bucket
        .dimensions(plan.selection.role, plan.selection.mode)
    else {
        return Err(KernelCatalogError::ProfileSelection);
    };
    let (family, disposition) = family_for(step.operator, plan.selection.mode);
    let profile = KernelProfileDescriptor {
        plan_id: plan.authority.plan_id,
        selection: plan.selection,
        step,
        sequences: dimensions.sequences,
        active_tokens: dimensions.active_tokens,
        context_tokens: dimensions.context_tokens,
        family,
        disposition,
    };
    validate_kernel_profile(profile, plan, step.ordinal)?;
    Ok(profile)
}

fn validate_authorities(
    authorities: &KernelAuthorityRequirements,
) -> Result<(), KernelCatalogError> {
    let components = authorities.components();
    for (index, (component, identity)) in components.iter().copied().enumerate() {
        if !identity.is_present() {
            return Err(KernelCatalogError::MissingAuthority(component));
        }
        for (prior_component, prior_identity) in components[..index].iter().copied() {
            if identity == prior_identity {
                return Err(KernelCatalogError::ReusedAuthority {
                    first: prior_component,
                    second: component,
                });
            }
        }
    }
    Ok(())
}

fn validate_source_declarations(
    source_declarations: &[KernelSourceDeclaration],
) -> Result<(), KernelCatalogError> {
    if source_declarations.len() != FERRIC_KERNEL_SOURCE_DECLARATIONS.len() {
        return Err(KernelCatalogError::SourceDeclarationCount {
            expected: FERRIC_KERNEL_SOURCE_DECLARATIONS.len(),
            actual: source_declarations.len(),
        });
    }
    for (actual, expected) in source_declarations
        .iter()
        .zip(FERRIC_KERNEL_SOURCE_DECLARATIONS)
    {
        if actual != &expected {
            return Err(KernelCatalogError::SourceDeclarationDrift(expected.family));
        }
    }
    Ok(())
}

fn expected_selection(plan_index: usize) -> Qwen3PlanSelection {
    let role = if plan_index < M1_B3_PLAN_BUCKETS.len() {
        Qwen3ModelRole::Target8B
    } else {
        Qwen3ModelRole::Draft06B
    };
    let (mode, bucket) = M1_B3_PLAN_BUCKETS[plan_index % M1_B3_PLAN_BUCKETS.len()];
    Qwen3PlanSelection { role, mode, bucket }
}

fn encode_catalog(
    plan_catalog_id: Identity,
    authorities: &KernelAuthorityRequirements,
    plans: &[Qwen3GeneratedPlan],
    bindings: &[KernelOperationBinding],
) -> Vec<u8> {
    let mut record = Vec::with_capacity(1_500_000);
    record.extend_from_slice(&M1_KERNEL_CATALOG_VERSION.to_le_bytes());
    push_bytes(&mut record, GFX942_PROCESSOR.as_bytes());
    push_bytes(&mut record, GFX942_TARGET_FEATURES.as_bytes());
    record.extend_from_slice(plan_catalog_id.as_bytes());
    for (_, identity) in authorities.components() {
        record.extend_from_slice(identity.as_bytes());
    }
    record.extend_from_slice(&(FERRIC_KERNEL_SOURCE_DECLARATIONS.len() as u64).to_le_bytes());
    for source in FERRIC_KERNEL_SOURCE_DECLARATIONS {
        record.push(family_tag(source.family));
        push_bytes(&mut record, source.repository.as_bytes());
        push_bytes(&mut record, source.source_path.as_bytes());
    }
    record.extend_from_slice(&(plans.len() as u64).to_le_bytes());
    for plan in plans {
        record.extend_from_slice(plan.authority.plan_id.as_bytes());
        encode_selection(&mut record, plan.selection);
        record.extend_from_slice(&(plan.steps.len() as u64).to_le_bytes());
    }
    record.extend_from_slice(&(bindings.len() as u64).to_le_bytes());
    for binding in bindings {
        record.extend_from_slice(&binding.plan_index.to_le_bytes());
        encode_profile(&mut record, binding.profile);
    }
    record
}

fn push_bytes(record: &mut Vec<u8>, bytes: &[u8]) {
    record.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    record.extend_from_slice(bytes);
}

fn encode_profile(record: &mut Vec<u8>, profile: KernelProfileDescriptor) {
    record.extend_from_slice(profile.plan_id.as_bytes());
    encode_selection(record, profile.selection);
    record.extend_from_slice(&profile.sequences.to_le_bytes());
    record.extend_from_slice(&profile.active_tokens.to_le_bytes());
    record.extend_from_slice(&profile.context_tokens.to_le_bytes());
    record.push(family_tag(profile.family));
    record.push(match profile.disposition {
        KernelProfileDisposition::DeclaredFoundation => 1,
        KernelProfileDisposition::RequiredExtension => 2,
    });
    encode_step(record, profile.step);
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
    record.push(bucket_tag(selection.bucket));
}

fn encode_step(record: &mut Vec<u8>, step: Qwen3PlanStep) {
    record.extend_from_slice(&step.ordinal.to_le_bytes());
    record.extend_from_slice(&step.layer.to_le_bytes());
    record.push(operator_tag(step.operator));
    encode_geometry(record, step.geometry);
    for buffer in [
        step.input_0,
        step.input_1,
        step.input_2,
        step.output_0,
        step.output_1,
    ] {
        encode_buffer(record, buffer);
    }
}

fn encode_geometry(record: &mut Vec<u8>, geometry: Qwen3PlanGeometry) {
    for value in [
        geometry.hidden_size,
        geometry.intermediate_size,
        geometry.query_heads,
        geometry.kv_heads,
        geometry.head_dim,
        geometry.gqa_group_size,
    ] {
        record.extend_from_slice(&value.to_le_bytes());
    }
}

fn encode_buffer(record: &mut Vec<u8>, buffer: Qwen3PlanBuffer) {
    record.push(buffer_tag(buffer.kind));
    record.extend_from_slice(&buffer.layer.to_le_bytes());
    encode_shape(record, buffer.shape);
}

fn encode_shape(record: &mut Vec<u8>, shape: Qwen3PlanShape) {
    for value in [
        shape.rank,
        shape.dimension_0,
        shape.dimension_1,
        shape.dimension_2,
        shape.dimension_3,
    ] {
        record.extend_from_slice(&value.to_le_bytes());
    }
}

const fn family_tag(family: KernelFamily) -> u8 {
    match family {
        KernelFamily::K1GemmGemv => 1,
        KernelFamily::K2RmsNormResidual => 2,
        KernelFamily::K3RopePagedKv => 3,
        KernelFamily::K4GqaPrefill => 4,
        KernelFamily::K5PagedGqaDecode => 5,
        KernelFamily::K6SwiGlu => 6,
        KernelFamily::K7LogitsCompact => 7,
    }
}

const fn bucket_tag(bucket: Qwen3PlanBucket) -> u8 {
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

const fn operator_tag(operator: Qwen3Operator) -> u8 {
    match operator {
        Qwen3Operator::TokenEmbedding => 1,
        Qwen3Operator::InputRmsNorm => 2,
        Qwen3Operator::QueryProjection => 3,
        Qwen3Operator::KeyProjection => 4,
        Qwen3Operator::ValueProjection => 5,
        Qwen3Operator::QueryRmsNorm => 6,
        Qwen3Operator::KeyRmsNorm => 7,
        Qwen3Operator::Rope => 8,
        Qwen3Operator::KvWrite => 9,
        Qwen3Operator::Attention => 10,
        Qwen3Operator::AttentionOutputResidual => 11,
        Qwen3Operator::PostAttentionRmsNorm => 12,
        Qwen3Operator::GateProjection => 13,
        Qwen3Operator::UpProjection => 14,
        Qwen3Operator::SwiGlu => 15,
        Qwen3Operator::DownResidual => 16,
        Qwen3Operator::FinalRmsNorm => 17,
        Qwen3Operator::LogitsProjection => 18,
        Qwen3Operator::ArgmaxCompactCompletion => 19,
    }
}

const fn buffer_tag(kind: Qwen3BufferKind) -> u8 {
    match kind {
        Qwen3BufferKind::Absent => 0,
        Qwen3BufferKind::TokenIds => 1,
        Qwen3BufferKind::PositionIds => 2,
        Qwen3BufferKind::Hidden => 3,
        Qwen3BufferKind::NormalizedHidden => 4,
        Qwen3BufferKind::Query => 5,
        Qwen3BufferKind::Key => 6,
        Qwen3BufferKind::Value => 7,
        Qwen3BufferKind::NormalizedQuery => 8,
        Qwen3BufferKind::NormalizedKey => 9,
        Qwen3BufferKind::RotatedQuery => 10,
        Qwen3BufferKind::RotatedKey => 11,
        Qwen3BufferKind::KvKeys => 12,
        Qwen3BufferKind::KvValues => 13,
        Qwen3BufferKind::AttentionOutput => 14,
        Qwen3BufferKind::HiddenAfterAttention => 15,
        Qwen3BufferKind::PostAttentionNormalized => 16,
        Qwen3BufferKind::Gate => 17,
        Qwen3BufferKind::Up => 18,
        Qwen3BufferKind::Activated => 19,
        Qwen3BufferKind::FinalNormalized => 20,
        Qwen3BufferKind::Logits => 21,
        Qwen3BufferKind::CompactCompletion => 22,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferric_spec::{plan_step_count, Qwen3PlanAuthority};

    fn identity(seed: u8) -> Identity {
        let mut bytes = [seed; 32];
        bytes[31] = seed.wrapping_add(1);
        Identity::new(bytes)
    }

    fn authorities() -> KernelAuthorityRequirements {
        KernelAuthorityRequirements {
            fe2o3_source: identity(1),
            compiler: identity(2),
            compiler_configuration: identity(3),
            target_contract: identity(4),
            kernel_proof_set: identity(5),
            kernel_abi_catalog: identity(6),
            runtime_contract: identity(7),
            runtime_abi: identity(8),
            tcb_report: identity(9),
        }
    }

    fn plans() -> Vec<Qwen3GeneratedPlan> {
        (0..M1_KERNEL_PLAN_COUNT)
            .map(|plan_index| {
                let selection = expected_selection(plan_index);
                let steps = (0..plan_step_count(selection.role))
                    .map(|ordinal| {
                        expected_step(selection.role, selection.mode, selection.bucket, ordinal)
                            .expect("exact plan step exists")
                    })
                    .collect();
                Qwen3GeneratedPlan {
                    authority: Qwen3PlanAuthority {
                        bundle_id: identity(20),
                        model_id: identity(match selection.role {
                            Qwen3ModelRole::Target8B => 21,
                            Qwen3ModelRole::Draft06B => 22,
                        }),
                        config_id: identity(match selection.role {
                            Qwen3ModelRole::Target8B => 23,
                            Qwen3ModelRole::Draft06B => 24,
                        }),
                        graph_id: identity(u8::try_from(40 + plan_index).expect("plan index fits")),
                        plan_id: identity(u8::try_from(80 + plan_index).expect("plan index fits")),
                        revision: 1,
                    },
                    selection,
                    steps,
                }
            })
            .collect()
    }

    #[test]
    fn exact_catalog_binds_every_plan_operation_and_family() {
        let plans = plans();
        let catalog = build_structural_kernel_catalog(
            &plans,
            identity(200),
            &FERRIC_KERNEL_SOURCE_DECLARATIONS,
            authorities(),
        )
        .unwrap();
        assert_eq!(catalog.version(), M1_KERNEL_CATALOG_VERSION);
        assert_eq!(catalog.bindings().len(), M1_KERNEL_OPERATION_BINDINGS);
        assert_eq!(
            catalog.source_declarations(),
            FERRIC_KERNEL_SOURCE_DECLARATIONS
        );
        assert!(!catalog.canonical_bytes().is_empty());

        let mut family_seen = [false; 7];
        for binding in catalog.bindings() {
            let plan = &plans[usize::from(binding.plan_index)];
            validate_kernel_profile(binding.profile, plan, binding.profile.step.ordinal).unwrap();
            family_seen[usize::from(family_tag(binding.profile.family) - 1)] = true;
        }
        assert_eq!(family_seen, [true; 7]);
    }

    #[test]
    fn adjacent_role_bucket_operator_and_shape_drift_fail_closed() {
        let base = plans();

        let mut changed = base.clone();
        changed[0].selection.role = Qwen3ModelRole::Draft06B;
        assert_eq!(
            build_structural_kernel_catalog(
                &changed,
                identity(200),
                &FERRIC_KERNEL_SOURCE_DECLARATIONS,
                authorities(),
            ),
            Err(KernelCatalogError::PlanSelection { plan_index: 0 })
        );

        let mut changed = base.clone();
        changed[0].selection.bucket = Qwen3PlanBucket::PrefillS1T512;
        assert_eq!(
            build_structural_kernel_catalog(
                &changed,
                identity(200),
                &FERRIC_KERNEL_SOURCE_DECLARATIONS,
                authorities(),
            ),
            Err(KernelCatalogError::PlanSelection { plan_index: 0 })
        );

        let mut changed = base.clone();
        changed[0].steps[0].operator = Qwen3Operator::QueryProjection;
        assert_eq!(
            build_structural_kernel_catalog(
                &changed,
                identity(200),
                &FERRIC_KERNEL_SOURCE_DECLARATIONS,
                authorities(),
            ),
            Err(KernelCatalogError::InvalidPlan { plan_index: 0 })
        );

        let mut changed = base;
        changed[0].steps[0].output_0.shape.dimension_1 += 1;
        assert_eq!(
            build_structural_kernel_catalog(
                &changed,
                identity(200),
                &FERRIC_KERNEL_SOURCE_DECLARATIONS,
                authorities(),
            ),
            Err(KernelCatalogError::InvalidPlan { plan_index: 0 })
        );
    }

    #[test]
    fn wrong_operator_family_and_profile_selection_are_rejected() {
        let plans = plans();
        let catalog = build_structural_kernel_catalog(
            &plans,
            identity(200),
            &FERRIC_KERNEL_SOURCE_DECLARATIONS,
            authorities(),
        )
        .unwrap();
        let mut profile = catalog.bindings()[0].profile;
        profile.family = KernelFamily::K7LogitsCompact;
        assert_eq!(
            validate_kernel_profile(profile, &plans[0], 0),
            Err(KernelCatalogError::ProfileFamily)
        );

        let mut profile = catalog.bindings()[0].profile;
        profile.selection.mode = Qwen3ExecutionMode::Decode;
        assert_eq!(
            validate_kernel_profile(profile, &plans[0], 0),
            Err(KernelCatalogError::ProfileSelection)
        );
    }

    #[test]
    fn every_plan_and_authority_identity_is_bound_against_drift() {
        let plans = plans();
        let catalog = build_structural_kernel_catalog(
            &plans,
            identity(200),
            &FERRIC_KERNEL_SOURCE_DECLARATIONS,
            authorities(),
        )
        .unwrap();

        assert_eq!(
            validate_structural_kernel_catalog(
                &catalog,
                &plans,
                identity(201),
                &FERRIC_KERNEL_SOURCE_DECLARATIONS,
                authorities(),
            ),
            Err(KernelCatalogError::CatalogDrift)
        );
        for plan_index in 0..M1_KERNEL_PLAN_COUNT {
            let mut changed = plans.clone();
            changed[plan_index].authority.plan_id =
                identity(u8::try_from(120 + plan_index).expect("bounded plan index fits u8"));
            assert_eq!(
                validate_structural_kernel_catalog(
                    &catalog,
                    &changed,
                    identity(200),
                    &FERRIC_KERNEL_SOURCE_DECLARATIONS,
                    authorities(),
                ),
                Err(KernelCatalogError::CatalogDrift)
            );
        }

        for component in 0..9 {
            let mut changed = authorities();
            let replacement = identity(u8::try_from(220 + component).expect("component fits u8"));
            match component {
                0 => changed.fe2o3_source = replacement,
                1 => changed.compiler = replacement,
                2 => changed.compiler_configuration = replacement,
                3 => changed.target_contract = replacement,
                4 => changed.kernel_proof_set = replacement,
                5 => changed.kernel_abi_catalog = replacement,
                6 => changed.runtime_contract = replacement,
                7 => changed.runtime_abi = replacement,
                _ => changed.tcb_report = replacement,
            }
            assert_eq!(
                validate_structural_kernel_catalog(
                    &catalog,
                    &plans,
                    identity(200),
                    &FERRIC_KERNEL_SOURCE_DECLARATIONS,
                    changed,
                ),
                Err(KernelCatalogError::CatalogDrift)
            );
        }
    }

    #[test]
    fn every_ferric_repository_and_path_declaration_is_exact() {
        for source_index in 0..FERRIC_KERNEL_SOURCE_DECLARATIONS.len() {
            for field in 0..2 {
                let mut sources = FERRIC_KERNEL_SOURCE_DECLARATIONS;
                match field {
                    0 => sources[source_index].repository = "drift/ferric",
                    _ => sources[source_index].source_path = "drift.rs",
                }
                assert_eq!(
                    build_structural_kernel_catalog(
                        &plans(),
                        identity(200),
                        &sources,
                        authorities(),
                    ),
                    Err(KernelCatalogError::SourceDeclarationDrift(
                        FERRIC_KERNEL_SOURCE_DECLARATIONS[source_index].family
                    ))
                );
            }
        }
    }

    #[test]
    fn missing_reused_and_duplicate_identities_fail_closed() {
        let plans = plans();
        let mut missing = authorities();
        missing.runtime_abi = Identity::new([0; 32]);
        assert_eq!(
            build_structural_kernel_catalog(
                &plans,
                identity(200),
                &FERRIC_KERNEL_SOURCE_DECLARATIONS,
                missing,
            ),
            Err(KernelCatalogError::MissingAuthority(
                KernelAuthorityComponent::RuntimeAbi
            ))
        );

        let mut reused = authorities();
        reused.runtime_abi = reused.compiler;
        assert_eq!(
            build_structural_kernel_catalog(
                &plans,
                identity(200),
                &FERRIC_KERNEL_SOURCE_DECLARATIONS,
                reused,
            ),
            Err(KernelCatalogError::ReusedAuthority {
                first: KernelAuthorityComponent::Compiler,
                second: KernelAuthorityComponent::RuntimeAbi,
            })
        );

        let mut duplicate = plans;
        duplicate[1].authority.plan_id = duplicate[0].authority.plan_id;
        assert_eq!(
            build_structural_kernel_catalog(
                &duplicate,
                identity(200),
                &FERRIC_KERNEL_SOURCE_DECLARATIONS,
                authorities(),
            ),
            Err(KernelCatalogError::InvalidPlanIdentity { plan_index: 1 })
        );
    }
}
