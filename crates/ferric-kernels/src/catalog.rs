//! Exact finite mapping from sequential graph operations to reviewed K1-K7 sources.

use ferric_spec::{
    expected_step, Identity, Qwen3BufferKind, Qwen3ExecutionMode, Qwen3GeneratedPlan,
    Qwen3ModelRole, Qwen3Operator, Qwen3PlanBucket, Qwen3PlanBuffer, Qwen3PlanGeometry,
    Qwen3PlanSelection, Qwen3PlanShape, Qwen3PlanStep,
};

/// Canonical structural kernel-catalog record version.
pub const M1_KERNEL_CATALOG_VERSION: u32 = 1;
/// Exact number of target/draft B3 plans.
pub const M1_KERNEL_PLAN_COUNT: usize = 22;
/// Exact number of graph operations across all 22 plans.
pub const M1_KERNEL_OPERATION_BINDINGS: usize = 10_648;
/// Exact target processor required by every future executable candidate.
pub const GFX942_PROCESSOR: &str = "gfx942";
/// Exact target-feature policy required by every future executable candidate.
pub const GFX942_TARGET_FEATURES: &str = "+wavefrontsize64,-xnack";

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

/// Reviewed upstream source family owning one structural foundation.
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

/// Whether the reviewed upstream source exactly includes this graph boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KernelProfileDisposition {
    /// The upstream fixture/model declares the same finite operator boundary.
    ReviewedFoundation,
    /// A future upstream implementation must extend the reviewed foundation.
    RequiredExtension,
}

/// Exact reviewed, currently unmerged upstream source identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReviewedKernelSource {
    /// Stable K1-K7 family.
    pub family: KernelFamily,
    /// GitHub pull request number in `harsh-nod/fe2o3`.
    pub pull_request: u32,
    /// Exact head commit SHA-1.
    pub commit: &'static str,
    /// Exact Git tree SHA-1 for the head commit.
    pub tree: &'static str,
    /// Exact repository identity.
    pub repository: &'static str,
    /// Primary source path for the reviewed operator contract.
    pub source_path: &'static str,
}

/// K1-K7 reviewed source roster in family order.
pub const REVIEWED_KERNEL_SOURCES: [ReviewedKernelSource; 7] = [
    ReviewedKernelSource {
        family: KernelFamily::K1GemmGemv,
        pull_request: 191,
        commit: "1a6a76cbe5d17f8a9446bfa83ee49aa8cf9596ed",
        tree: "e6c12126caf499a8eb111b1a03e54e42512413d3",
        repository: "harsh-nod/fe2o3",
        source_path: "crates/fe2o3-llm-kernels/src/gemm.rs",
    },
    ReviewedKernelSource {
        family: KernelFamily::K2RmsNormResidual,
        pull_request: 186,
        commit: "88df011d87dc2c2e91b4963b010b97dd38fa015b",
        tree: "d1c4f1c69edc879e32a74ee74662f19cead45b40",
        repository: "harsh-nod/fe2o3",
        source_path: "examples/rmsnorm_residual_v1/src/contract.rs",
    },
    ReviewedKernelSource {
        family: KernelFamily::K3RopePagedKv,
        pull_request: 187,
        commit: "e805b51de645710d0504be36cc8782508cd36e75",
        tree: "cc78ee18bff42636e14d67a73d2d8a6cef88c2f9",
        repository: "harsh-nod/fe2o3",
        source_path: "crates/fe2o3-llm-kernels/src/rope_kv.rs",
    },
    ReviewedKernelSource {
        family: KernelFamily::K4GqaPrefill,
        pull_request: 188,
        commit: "2ee53195220c010699895d30ab5ad1b328073d9f",
        tree: "5b828c0bcd2e2c2e8a3314ac178067a0e9527a0b",
        repository: "harsh-nod/fe2o3",
        source_path: "examples/qwen3_gqa_prefill_v1/src/contract.rs",
    },
    ReviewedKernelSource {
        family: KernelFamily::K5PagedGqaDecode,
        pull_request: 189,
        commit: "4d1980df037fbd599ed113a93c413e937f40ddd7",
        tree: "c8a82349ca42b0b11dce08480f75b1eaaaad3995",
        repository: "harsh-nod/fe2o3",
        source_path: "examples/qwen3_paged_gqa_decode_v1/src/contract.rs",
    },
    ReviewedKernelSource {
        family: KernelFamily::K6SwiGlu,
        pull_request: 190,
        commit: "fc2578789dcf6ad2366f78b6cdf73077e354ddeb",
        tree: "7c0ccb6afe2e3af00529aced543ab00c313ed28f",
        repository: "harsh-nod/fe2o3",
        source_path: "examples/qwen3_swiglu_v1/src/contract.rs",
    },
    ReviewedKernelSource {
        family: KernelFamily::K7LogitsCompact,
        pull_request: 192,
        commit: "49193006a6bfb2daa7ffbcd698f00c12c3f20ecd",
        tree: "269ab71b2071f59959c3bd87fa5bc24be61a35b5",
        repository: "harsh-nod/fe2o3",
        source_path: "examples/qwen3_logits_compact_v1/src/contract.rs",
    },
];

/// Compiler/runtime identity required by the structural catalog.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KernelAuthorityComponent {
    /// Future exact aggregate fe2o3 source closure.
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
    /// Exact aggregate fe2o3 source closure.
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
    fn components(&self) -> [(KernelAuthorityComponent, Identity); 9] {
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

/// One exact graph operation and its finite reviewed-source profile.
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
    /// Whether this exact boundary exists in the reviewed fixture/model.
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
    /// The reviewed K1-K7 roster has the wrong length.
    ReviewedSourceCount {
        /// Exact required length.
        expected: usize,
        /// Observed length.
        actual: usize,
    },
    /// A K1-K7 PR, commit, tree, repository, or path identity drifted.
    ReviewedSourceDrift(KernelFamily),
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
    reviewed_sources: Box<[ReviewedKernelSource]>,
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

    /// Returns the exact reviewed K1-K7 source roster.
    #[must_use]
    pub fn reviewed_sources(&self) -> &[ReviewedKernelSource] {
        &self.reviewed_sources
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
/// Returns [`KernelCatalogError`] for any plan, profile, reviewed-source, or
/// compiler/runtime authority drift. Success does not grant any execution or
/// evidence authority.
pub fn build_structural_kernel_catalog(
    plans: &[Qwen3GeneratedPlan],
    plan_catalog_id: Identity,
    reviewed_sources: &[ReviewedKernelSource],
    authorities: KernelAuthorityRequirements,
) -> Result<StructuralKernelCatalog, KernelCatalogError> {
    if !plan_catalog_id.is_present() {
        return Err(KernelCatalogError::MissingPlanCatalogIdentity);
    }
    validate_authorities(&authorities)?;
    validate_reviewed_sources(reviewed_sources)?;
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
        reviewed_sources: reviewed_sources.to_vec().into_boxed_slice(),
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
    reviewed_sources: &[ReviewedKernelSource],
    authorities: KernelAuthorityRequirements,
) -> Result<(), KernelCatalogError> {
    let expected =
        build_structural_kernel_catalog(plans, plan_catalog_id, reviewed_sources, authorities)?;
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

fn family_for(
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
            KernelProfileDisposition::ReviewedFoundation,
        ),
        Qwen3Operator::InputRmsNorm
        | Qwen3Operator::QueryRmsNorm
        | Qwen3Operator::KeyRmsNorm
        | Qwen3Operator::PostAttentionRmsNorm
        | Qwen3Operator::FinalRmsNorm => (
            KernelFamily::K2RmsNormResidual,
            KernelProfileDisposition::RequiredExtension,
        ),
        Qwen3Operator::Rope | Qwen3Operator::KvWrite => (
            KernelFamily::K3RopePagedKv,
            KernelProfileDisposition::ReviewedFoundation,
        ),
        Qwen3Operator::Attention => match mode {
            Qwen3ExecutionMode::Prefill => (
                KernelFamily::K4GqaPrefill,
                KernelProfileDisposition::ReviewedFoundation,
            ),
            Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative => (
                KernelFamily::K5PagedGqaDecode,
                KernelProfileDisposition::ReviewedFoundation,
            ),
        },
        Qwen3Operator::SwiGlu => (
            KernelFamily::K6SwiGlu,
            KernelProfileDisposition::ReviewedFoundation,
        ),
        Qwen3Operator::ArgmaxCompactCompletion => (
            KernelFamily::K7LogitsCompact,
            KernelProfileDisposition::RequiredExtension,
        ),
    }
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

fn validate_reviewed_sources(
    reviewed_sources: &[ReviewedKernelSource],
) -> Result<(), KernelCatalogError> {
    if reviewed_sources.len() != REVIEWED_KERNEL_SOURCES.len() {
        return Err(KernelCatalogError::ReviewedSourceCount {
            expected: REVIEWED_KERNEL_SOURCES.len(),
            actual: reviewed_sources.len(),
        });
    }
    for (actual, expected) in reviewed_sources.iter().zip(REVIEWED_KERNEL_SOURCES) {
        if actual != &expected {
            return Err(KernelCatalogError::ReviewedSourceDrift(expected.family));
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
    record.extend_from_slice(&(REVIEWED_KERNEL_SOURCES.len() as u64).to_le_bytes());
    for source in REVIEWED_KERNEL_SOURCES {
        record.push(family_tag(source.family));
        record.extend_from_slice(&source.pull_request.to_le_bytes());
        push_bytes(&mut record, source.commit.as_bytes());
        push_bytes(&mut record, source.tree.as_bytes());
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
        KernelProfileDisposition::ReviewedFoundation => 1,
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
            &REVIEWED_KERNEL_SOURCES,
            authorities(),
        )
        .unwrap();
        assert_eq!(catalog.version(), M1_KERNEL_CATALOG_VERSION);
        assert_eq!(catalog.bindings().len(), M1_KERNEL_OPERATION_BINDINGS);
        assert_eq!(catalog.reviewed_sources(), REVIEWED_KERNEL_SOURCES);
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
                &REVIEWED_KERNEL_SOURCES,
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
                &REVIEWED_KERNEL_SOURCES,
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
                &REVIEWED_KERNEL_SOURCES,
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
                &REVIEWED_KERNEL_SOURCES,
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
            &REVIEWED_KERNEL_SOURCES,
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
            &REVIEWED_KERNEL_SOURCES,
            authorities(),
        )
        .unwrap();

        assert_eq!(
            validate_structural_kernel_catalog(
                &catalog,
                &plans,
                identity(201),
                &REVIEWED_KERNEL_SOURCES,
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
                    &REVIEWED_KERNEL_SOURCES,
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
                    &REVIEWED_KERNEL_SOURCES,
                    changed,
                ),
                Err(KernelCatalogError::CatalogDrift)
            );
        }
    }

    #[test]
    fn every_reviewed_pr_commit_tree_repository_and_path_is_exact() {
        for source_index in 0..REVIEWED_KERNEL_SOURCES.len() {
            for field in 0..5 {
                let mut sources = REVIEWED_KERNEL_SOURCES;
                match field {
                    0 => sources[source_index].pull_request += 100,
                    1 => sources[source_index].commit = "0000000000000000000000000000000000000001",
                    2 => sources[source_index].tree = "0000000000000000000000000000000000000002",
                    3 => sources[source_index].repository = "drift/fe2o3",
                    _ => sources[source_index].source_path = "drift.rs",
                }
                assert_eq!(
                    build_structural_kernel_catalog(
                        &plans(),
                        identity(200),
                        &sources,
                        authorities(),
                    ),
                    Err(KernelCatalogError::ReviewedSourceDrift(
                        REVIEWED_KERNEL_SOURCES[source_index].family
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
                &REVIEWED_KERNEL_SOURCES,
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
                &REVIEWED_KERNEL_SOURCES,
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
                &REVIEWED_KERNEL_SOURCES,
                authorities(),
            ),
            Err(KernelCatalogError::InvalidPlanIdentity { plan_index: 1 })
        );
    }
}
