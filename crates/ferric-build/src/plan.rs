use super::{
    hash_field, sha256::Sha256, AuthenticatedBundleAdmission, BundleAdmissionRecord,
    PrepackedDeploymentBundle,
};
use ferric_spec::{
    expected_step, plan_step_count, DeploymentBundle, Identity, Qwen3BufferKind,
    Qwen3ExecutionMode, Qwen3GeneratedPlan, Qwen3ModelRole, Qwen3Operator, Qwen3PlanAuthority,
    Qwen3PlanBucket, Qwen3PlanBuffer, Qwen3PlanError, Qwen3PlanSelection, Qwen3PlanShape,
    Qwen3PlanStep, SpecError,
};

/// Canonical format version for the offline sequential plan catalog.
pub const SEQUENTIAL_PLAN_CATALOG_VERSION: u32 = 1;
/// Exact target/draft role and finite-bucket combinations in the M1 catalog.
pub const SEQUENTIAL_PLAN_CATALOG_ENTRIES: usize = 22;
const PLAN_REVISION: u64 = 1;

const BUCKETS: [(Qwen3ExecutionMode, Qwen3PlanBucket); 11] = [
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

/// Exact offline graph plans retained with their consumed prepacked authority.
///
/// This is a sequential graph-input catalog. It does not bind kernel
/// schedules, ABI layouts, compiler artifacts, machine code, or runtime
/// dispatch authority and therefore is not a qualified executable plan.
#[derive(Debug, PartialEq, Eq)]
pub struct SequentialPlanCatalog {
    prepacked: PrepackedDeploymentBundle,
    admission_record: Option<BundleAdmissionRecord>,
    plans: Vec<Qwen3GeneratedPlan>,
    catalog_id: Identity,
}

impl SequentialPlanCatalog {
    /// Returns the exact admitted deployment retained by this catalog.
    #[must_use]
    pub const fn deployment(&self) -> &DeploymentBundle {
        self.prepacked.deployment()
    }

    /// Returns the retained target and draft prepacked authority.
    #[must_use]
    pub const fn prepacked(&self) -> &PrepackedDeploymentBundle {
        &self.prepacked
    }

    /// Returns the retained authenticated admission commitment, when the
    /// catalog was constructed through the authenticated path.
    #[must_use]
    pub const fn admission_record(&self) -> Option<&BundleAdmissionRecord> {
        self.admission_record.as_ref()
    }

    /// Returns all plans in target-then-draft, finite-bucket order.
    #[must_use]
    pub fn plans(&self) -> &[Qwen3GeneratedPlan] {
        &self.plans
    }

    /// Returns the identity of the exact ordered plan catalog.
    #[must_use]
    pub const fn catalog_id(&self) -> Identity {
        self.catalog_id
    }

    /// Returns the canonical catalog format version.
    #[must_use]
    pub const fn version(&self) -> u32 {
        SEQUENTIAL_PLAN_CATALOG_VERSION
    }
}

/// Fail-closed sequential graph-plan generation error.
#[derive(Debug, PartialEq, Eq)]
pub enum SequentialPlanError {
    /// The retained deployment no longer satisfies its executable contract.
    InvalidDeployment(SpecError),
    /// A retained weight manifest has the wrong canonical format version.
    ManifestVersion {
        /// Expected target or draft role.
        role: Qwen3ModelRole,
        /// Observed manifest version.
        actual: u32,
    },
    /// A retained weight manifest was routed to the wrong model role.
    ManifestRole {
        /// Required role for this manifest position.
        expected: Qwen3ModelRole,
        /// Role recorded in the manifest.
        actual: Qwen3ModelRole,
    },
    /// The manifest source identity differs from the admitted source weights.
    ManifestSourceIdentity(Qwen3ModelRole),
    /// The manifest source length differs from the admitted source artifact.
    ManifestSourceBytes(Qwen3ModelRole),
    /// The foundation prepack is not the exact tensor-byte identity layout.
    ManifestOutputLayout(Qwen3ModelRole),
    /// The verified graph specification omitted a required ordinal.
    MissingStep {
        /// Role being generated.
        role: Qwen3ModelRole,
        /// Bucket being generated.
        bucket: Qwen3PlanBucket,
        /// Required zero-based step ordinal.
        ordinal: u32,
    },
    /// The generated plan did not validate against its exact authority.
    InvalidGeneratedPlan(Qwen3PlanError),
    /// A catalog construction bug produced the wrong number of plans.
    PlanCount {
        /// Exact required count.
        expected: usize,
        /// Observed generated count.
        actual: usize,
    },
}

/// Consumes a prepacked deployment and builds all exact sequential M1 plans.
///
/// The returned authority retains both prepacked weight manifests and binds
/// their aggregate identities into every plan and the ordered catalog ID.
/// It does not authorize kernel selection, compilation, loading, or launch.
///
/// # Errors
///
/// Returns [`SequentialPlanError`] for stale deployment/manifest authority,
/// an incomplete graph specification, or a generated plan that fails its
/// independently checked exact-plan contract.
pub fn build_sequential_plan_catalog(
    prepacked: PrepackedDeploymentBundle,
) -> Result<SequentialPlanCatalog, SequentialPlanError> {
    build_catalog(prepacked, None)
}

/// Consumes authenticated prepacked admission and builds every exact M1 plan.
///
/// The returned catalog retains both the prepacked authorities and their
/// canonical authenticated-admission record. It grants no kernel, artifact,
/// load, launch, runtime, or qualification authority.
///
/// # Errors
///
/// Returns [`SequentialPlanError`] for stale deployment/manifest authority,
/// an incomplete graph specification, or plan identity drift.
pub fn build_authenticated_sequential_plan_catalog(
    admission: AuthenticatedBundleAdmission,
) -> Result<SequentialPlanCatalog, SequentialPlanError> {
    let (prepacked, record) = admission.into_parts();
    build_catalog(prepacked, Some(record))
}

fn build_catalog(
    prepacked: PrepackedDeploymentBundle,
    admission_record: Option<BundleAdmissionRecord>,
) -> Result<SequentialPlanCatalog, SequentialPlanError> {
    validate_prepacked(&prepacked)?;
    let target_manifest_id = prepacked.target_manifest().aggregate_id();
    let draft_manifest_id = prepacked.draft_manifest().aggregate_id();
    let (plans, catalog_id) = build_catalog_parts(
        prepacked.deployment(),
        target_manifest_id,
        draft_manifest_id,
    )?;
    Ok(SequentialPlanCatalog {
        prepacked,
        admission_record,
        plans,
        catalog_id,
    })
}

fn validate_prepacked(prepacked: &PrepackedDeploymentBundle) -> Result<(), SequentialPlanError> {
    let deployment = prepacked.deployment();
    deployment
        .validate()
        .map_err(SequentialPlanError::InvalidDeployment)?;
    validate_manifest(
        prepacked.target_manifest(),
        Qwen3ModelRole::Target8B,
        deployment.target_model.weights.weights_id,
        deployment.target_model.weights.total_bytes,
    )?;
    validate_manifest(
        prepacked.draft_manifest(),
        Qwen3ModelRole::Draft06B,
        deployment.draft_model.weights.weights_id,
        deployment.draft_model.weights.total_bytes,
    )
}

fn validate_manifest(
    manifest: &super::WeightSectionManifest,
    expected_role: Qwen3ModelRole,
    expected_source_id: Identity,
    expected_source_bytes: u64,
) -> Result<(), SequentialPlanError> {
    if manifest.version() != super::PREPACKED_WEIGHT_MANIFEST_VERSION {
        return Err(SequentialPlanError::ManifestVersion {
            role: expected_role,
            actual: manifest.version(),
        });
    }
    if manifest.role() != expected_role {
        return Err(SequentialPlanError::ManifestRole {
            expected: expected_role,
            actual: manifest.role(),
        });
    }
    if manifest.source_weights_id() != *expected_source_id.as_bytes() {
        return Err(SequentialPlanError::ManifestSourceIdentity(expected_role));
    }
    if manifest.source_artifact_bytes() != expected_source_bytes {
        return Err(SequentialPlanError::ManifestSourceBytes(expected_role));
    }
    if manifest.tensor_data_bytes() != expected_role.tensor_data_bytes()
        || manifest.output_bytes() != manifest.tensor_data_bytes()
        || manifest.sections().len() != expected_role.tensor_count() as usize
        || manifest.canonical_bytes().is_empty()
        || super::sha256::digest(manifest.canonical_bytes()) != manifest.aggregate_id()
    {
        return Err(SequentialPlanError::ManifestOutputLayout(expected_role));
    }
    Ok(())
}

fn build_catalog_parts(
    deployment: &DeploymentBundle,
    target_manifest_id: [u8; 32],
    draft_manifest_id: [u8; 32],
) -> Result<(Vec<Qwen3GeneratedPlan>, Identity), SequentialPlanError> {
    deployment
        .validate()
        .map_err(SequentialPlanError::InvalidDeployment)?;
    let mut plans = Vec::with_capacity(SEQUENTIAL_PLAN_CATALOG_ENTRIES);
    for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
        for (mode, bucket) in BUCKETS {
            let selection = Qwen3PlanSelection { role, mode, bucket };
            let steps = build_steps(selection)?;
            let graph_id = graph_identity(selection, &steps);
            let model = match role {
                Qwen3ModelRole::Target8B => deployment.target_model,
                Qwen3ModelRole::Draft06B => deployment.draft_model,
            };
            let authority_without_plan = Qwen3PlanAuthority {
                bundle_id: deployment.bundle_id,
                model_id: model.config.model_id,
                config_id: model.config.config_id,
                graph_id,
                plan_id: Identity::new([0; 32]),
                revision: PLAN_REVISION,
            };
            let plan_id = plan_identity(
                authority_without_plan,
                selection,
                target_manifest_id,
                draft_manifest_id,
            );
            let authority = Qwen3PlanAuthority {
                plan_id,
                ..authority_without_plan
            };
            let plan = Qwen3GeneratedPlan {
                authority,
                selection,
                steps,
            };
            plan.validate(authority, selection)
                .map_err(SequentialPlanError::InvalidGeneratedPlan)?;
            plans.push(plan);
        }
    }
    if plans.len() != SEQUENTIAL_PLAN_CATALOG_ENTRIES {
        return Err(SequentialPlanError::PlanCount {
            expected: SEQUENTIAL_PLAN_CATALOG_ENTRIES,
            actual: plans.len(),
        });
    }
    let catalog_id = catalog_identity(&plans, target_manifest_id, draft_manifest_id);
    Ok((plans, catalog_id))
}

fn build_steps(selection: Qwen3PlanSelection) -> Result<Vec<Qwen3PlanStep>, SequentialPlanError> {
    let count = plan_step_count(selection.role);
    let mut steps = Vec::with_capacity(count as usize);
    for ordinal in 0..count {
        let Some(step) = expected_step(selection.role, selection.mode, selection.bucket, ordinal)
        else {
            return Err(SequentialPlanError::MissingStep {
                role: selection.role,
                bucket: selection.bucket,
                ordinal,
            });
        };
        steps.push(step);
    }
    Ok(steps)
}

fn graph_identity(selection: Qwen3PlanSelection, steps: &[Qwen3PlanStep]) -> Identity {
    let mut record = Vec::with_capacity(64 + steps.len() * 180);
    record.extend_from_slice(&SEQUENTIAL_PLAN_CATALOG_VERSION.to_le_bytes());
    encode_selection(&mut record, selection);
    record.extend_from_slice(&(steps.len() as u64).to_le_bytes());
    for step in steps {
        encode_step(&mut record, *step);
    }
    identity_record(b"ferric.qwen3.sequential-graph.v1", &record)
}

fn plan_identity(
    authority: Qwen3PlanAuthority,
    selection: Qwen3PlanSelection,
    target_manifest_id: [u8; 32],
    draft_manifest_id: [u8; 32],
) -> Identity {
    let mut record = Vec::with_capacity(192);
    record.extend_from_slice(&SEQUENTIAL_PLAN_CATALOG_VERSION.to_le_bytes());
    record.extend_from_slice(authority.bundle_id.as_bytes());
    record.extend_from_slice(authority.model_id.as_bytes());
    record.extend_from_slice(authority.config_id.as_bytes());
    record.extend_from_slice(authority.graph_id.as_bytes());
    record.extend_from_slice(&authority.revision.to_le_bytes());
    record.extend_from_slice(&target_manifest_id);
    record.extend_from_slice(&draft_manifest_id);
    encode_selection(&mut record, selection);
    identity_record(b"ferric.qwen3.sequential-plan.v1", &record)
}

fn catalog_identity(
    plans: &[Qwen3GeneratedPlan],
    target_manifest_id: [u8; 32],
    draft_manifest_id: [u8; 32],
) -> Identity {
    let mut record = Vec::with_capacity(80 + plans.len() * 32);
    record.extend_from_slice(&SEQUENTIAL_PLAN_CATALOG_VERSION.to_le_bytes());
    record.extend_from_slice(&target_manifest_id);
    record.extend_from_slice(&draft_manifest_id);
    record.extend_from_slice(&(plans.len() as u64).to_le_bytes());
    for plan in plans {
        record.extend_from_slice(plan.authority.plan_id.as_bytes());
    }
    identity_record(b"ferric.qwen3.sequential-plan-catalog.v1", &record)
}

fn identity_record(domain: &[u8], record: &[u8]) -> Identity {
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, domain);
    hash_field(&mut hasher, record);
    Identity::new(hasher.finish())
}

fn encode_selection(record: &mut Vec<u8>, selection: Qwen3PlanSelection) {
    record.push(role_tag(selection.role));
    record.push(mode_tag(selection.mode));
    record.push(bucket_tag(selection.bucket));
}

fn encode_step(record: &mut Vec<u8>, step: Qwen3PlanStep) {
    record.extend_from_slice(&step.ordinal.to_le_bytes());
    record.extend_from_slice(&step.layer.to_le_bytes());
    record.push(operator_tag(step.operator));
    record.extend_from_slice(&step.geometry.hidden_size.to_le_bytes());
    record.extend_from_slice(&step.geometry.intermediate_size.to_le_bytes());
    record.extend_from_slice(&step.geometry.query_heads.to_le_bytes());
    record.extend_from_slice(&step.geometry.kv_heads.to_le_bytes());
    record.extend_from_slice(&step.geometry.head_dim.to_le_bytes());
    record.extend_from_slice(&step.geometry.gqa_group_size.to_le_bytes());
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

fn encode_buffer(record: &mut Vec<u8>, buffer: Qwen3PlanBuffer) {
    record.push(buffer_tag(buffer.kind));
    record.extend_from_slice(&buffer.layer.to_le_bytes());
    encode_shape(record, buffer.shape);
}

fn encode_shape(record: &mut Vec<u8>, shape: Qwen3PlanShape) {
    record.extend_from_slice(&shape.rank.to_le_bytes());
    record.extend_from_slice(&shape.dimension_0.to_le_bytes());
    record.extend_from_slice(&shape.dimension_1.to_le_bytes());
    record.extend_from_slice(&shape.dimension_2.to_le_bytes());
    record.extend_from_slice(&shape.dimension_3.to_le_bytes());
}

const fn role_tag(role: Qwen3ModelRole) -> u8 {
    match role {
        Qwen3ModelRole::Target8B => 0,
        Qwen3ModelRole::Draft06B => 1,
    }
}

const fn mode_tag(mode: Qwen3ExecutionMode) -> u8 {
    match mode {
        Qwen3ExecutionMode::Prefill => 0,
        Qwen3ExecutionMode::Decode => 1,
        Qwen3ExecutionMode::Speculative => 2,
    }
}

const fn bucket_tag(bucket: Qwen3PlanBucket) -> u8 {
    match bucket {
        Qwen3PlanBucket::PrefillS1T128 => 0,
        Qwen3PlanBucket::PrefillS8T128 => 1,
        Qwen3PlanBucket::PrefillS1T512 => 2,
        Qwen3PlanBucket::PrefillS1T2048 => 3,
        Qwen3PlanBucket::DecodeS1C8192 => 4,
        Qwen3PlanBucket::DecodeS8C8192 => 5,
        Qwen3PlanBucket::DecodeS32C8192 => 6,
        Qwen3PlanBucket::SpeculativeS1K4C8192 => 7,
        Qwen3PlanBucket::SpeculativeS8K4C8192 => 8,
        Qwen3PlanBucket::SpeculativeS1K8C8192 => 9,
        Qwen3PlanBucket::SpeculativeS1K16C8192 => 10,
    }
}

const fn operator_tag(operator: Qwen3Operator) -> u8 {
    match operator {
        Qwen3Operator::TokenEmbedding => 0,
        Qwen3Operator::InputRmsNorm => 1,
        Qwen3Operator::QueryProjection => 2,
        Qwen3Operator::KeyProjection => 3,
        Qwen3Operator::ValueProjection => 4,
        Qwen3Operator::QueryRmsNorm => 5,
        Qwen3Operator::KeyRmsNorm => 6,
        Qwen3Operator::Rope => 7,
        Qwen3Operator::KvWrite => 8,
        Qwen3Operator::Attention => 9,
        Qwen3Operator::AttentionOutputResidual => 10,
        Qwen3Operator::PostAttentionRmsNorm => 11,
        Qwen3Operator::GateProjection => 12,
        Qwen3Operator::UpProjection => 13,
        Qwen3Operator::SwiGlu => 14,
        Qwen3Operator::DownResidual => 15,
        Qwen3Operator::FinalRmsNorm => 16,
        Qwen3Operator::LogitsProjection => 17,
        Qwen3Operator::ArgmaxCompactCompletion => 18,
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
    use super::{
        build_catalog_parts, graph_identity, identity_record, BUCKETS,
        SEQUENTIAL_PLAN_CATALOG_ENTRIES,
    };
    use ferric_spec::{
        DeploymentBundle, EngineLimits, Identity, ModelArtifact, ModelConfig, NumericalPolicy,
        Qwen3ModelRole, Qwen3PlanError, Target, TokenizerConfig, WeightManifest,
        QWEN3_END_OF_TEXT_TOKEN, QWEN3_IM_END_TOKEN, QWEN3_IM_START_TOKEN, QWEN3_VOCABULARY_SIZE,
    };

    const fn identity(byte: u8) -> Identity {
        Identity::new([byte; 32])
    }

    const fn tokenizer() -> TokenizerConfig {
        TokenizerConfig {
            tokenizer_id: identity(8),
            vocabulary_id: identity(9),
            vocabulary_size: QWEN3_VOCABULARY_SIZE,
            end_of_text_token: QWEN3_END_OF_TEXT_TOKEN,
            im_start_token: QWEN3_IM_START_TOKEN,
            im_end_token: QWEN3_IM_END_TOKEN,
        }
    }

    const fn model(role: Qwen3ModelRole) -> ModelArtifact {
        let (layers, hidden_size, intermediate_size, query_heads, tie_word_embeddings, id) =
            match role {
                Qwen3ModelRole::Target8B => (36, 4_096, 12_288, 32, false, 2),
                Qwen3ModelRole::Draft06B => (28, 1_024, 3_072, 16, true, 4),
            };
        ModelArtifact {
            config: ModelConfig {
                role,
                model_id: identity(id),
                config_id: identity(id + 1),
                vocabulary_size: QWEN3_VOCABULARY_SIZE,
                layers,
                hidden_size,
                intermediate_size,
                query_heads,
                kv_heads: 8,
                head_dim: 128,
                max_position_embeddings: 40_960,
                rope_theta: 1_000_000,
                tie_word_embeddings,
            },
            tokenizer: tokenizer(),
            weights: WeightManifest {
                weights_id: identity(id + 10),
                total_bytes: 1_024,
                sections: 1,
            },
        }
    }

    const fn deployment() -> DeploymentBundle {
        DeploymentBundle {
            bundle_id: identity(1),
            target: Target::Gfx942XnackMinus,
            numerical_policy: NumericalPolicy::Bf16ParametersFp32Accumulation,
            limits: EngineLimits {
                max_context_tokens: 8_192,
                max_active_sequences: 32,
                kv_page_tokens: 256,
                max_draft_tokens: 16,
            },
            target_model: model(Qwen3ModelRole::Target8B),
            draft_model: model(Qwen3ModelRole::Draft06B),
        }
    }

    #[test]
    fn generates_every_exact_target_and_draft_bucket_deterministically() {
        let first = build_catalog_parts(&deployment(), [21; 32], [22; 32]).unwrap();
        let second = build_catalog_parts(&deployment(), [21; 32], [22; 32]).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first.1,
            Identity::new([
                0x1e, 0xff, 0xa3, 0x41, 0x24, 0x0f, 0x2d, 0x4f, 0xdb, 0xe0, 0x93, 0xfc, 0xa9, 0xd4,
                0x93, 0x8d, 0x3f, 0x82, 0x3f, 0x01, 0xdb, 0xe7, 0x2f, 0xd1, 0x7f, 0x99, 0x8c, 0xe8,
                0x00, 0x2f, 0xff, 0xcd,
            ])
        );
        assert_eq!(first.0.len(), SEQUENTIAL_PLAN_CATALOG_ENTRIES);
        assert!(first.1.is_present());
        assert_eq!(
            first.0[0].authority.graph_id,
            Identity::new([
                0xf5, 0xf4, 0x0e, 0x63, 0xad, 0xd9, 0x35, 0x1c, 0xfd, 0xba, 0xb2, 0x67, 0x4d, 0xa5,
                0xfd, 0x4a, 0x08, 0x85, 0x1b, 0xd6, 0x64, 0x2d, 0x43, 0xbc, 0x8e, 0x5d, 0x94, 0x00,
                0x97, 0x33, 0x24, 0x02,
            ])
        );
        assert_eq!(
            first.0[0].authority.plan_id,
            Identity::new([
                0xab, 0xca, 0x11, 0x90, 0xf4, 0xe8, 0x4d, 0xcd, 0x89, 0x72, 0x84, 0x9f, 0x9d, 0x1a,
                0x87, 0x77, 0x38, 0x6a, 0xbb, 0xce, 0xdf, 0x0c, 0xbe, 0x07, 0x89, 0x0b, 0x1f, 0x3b,
                0x20, 0x31, 0xf0, 0x3c,
            ])
        );
        for (role_index, role) in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B]
            .into_iter()
            .enumerate()
        {
            for (bucket_index, (mode, bucket)) in BUCKETS.into_iter().enumerate() {
                let plan = &first.0[role_index * BUCKETS.len() + bucket_index];
                assert_eq!(plan.selection.role, role);
                assert_eq!(plan.selection.mode, mode);
                assert_eq!(plan.selection.bucket, bucket);
                assert_eq!(plan.validate(plan.authority, plan.selection), Ok(()));
            }
        }
    }

    #[test]
    fn manifest_and_bundle_identity_changes_rekey_the_catalog() {
        let base = build_catalog_parts(&deployment(), [21; 32], [22; 32]).unwrap();
        let target_changed = build_catalog_parts(&deployment(), [23; 32], [22; 32]).unwrap();
        let draft_changed = build_catalog_parts(&deployment(), [21; 32], [24; 32]).unwrap();
        let mut different_bundle = deployment();
        different_bundle.bundle_id = identity(31);
        let bundle_changed = build_catalog_parts(&different_bundle, [21; 32], [22; 32]).unwrap();
        assert_ne!(base.1, target_changed.1);
        assert_ne!(base.1, draft_changed.1);
        assert_ne!(base.1, bundle_changed.1);
        assert_ne!(
            base.0[0].authority.plan_id,
            target_changed.0[0].authority.plan_id
        );
        assert_ne!(
            base.0[0].authority.plan_id,
            draft_changed.0[0].authority.plan_id
        );
        assert_ne!(
            base.0[0].authority.plan_id,
            bundle_changed.0[0].authority.plan_id
        );
    }

    #[test]
    fn step_or_selection_substitution_is_detected() {
        let (mut plans, _) = build_catalog_parts(&deployment(), [21; 32], [22; 32]).unwrap();
        let first = &mut plans[0];
        first.steps.swap(0, 1);
        assert_eq!(
            first.validate(first.authority, first.selection),
            Err(Qwen3PlanError::StepMismatch { ordinal: 0 })
        );

        let (mut plans, _) = build_catalog_parts(&deployment(), [21; 32], [22; 32]).unwrap();
        let first = &mut plans[0];
        let expected = first.selection;
        first.selection.bucket = ferric_spec::Qwen3PlanBucket::PrefillS1T512;
        assert_eq!(
            first.validate(first.authority, expected),
            Err(Qwen3PlanError::SelectionMismatch)
        );
    }

    #[test]
    fn graph_record_binds_every_step_field() {
        let (plans, _) = build_catalog_parts(&deployment(), [21; 32], [22; 32]).unwrap();
        let plan = &plans[0];
        let base = graph_identity(plan.selection, &plan.steps);
        let mut changed = plan.steps.clone();
        changed[1].output_0.shape.dimension_1 += 1;
        assert_ne!(base, graph_identity(plan.selection, &changed));
        assert_ne!(
            identity_record(b"ferric.qwen3.sequential-graph.v1", b"record"),
            identity_record(b"ferric.qwen3.sequential-plan.v1", b"record")
        );
    }
}
