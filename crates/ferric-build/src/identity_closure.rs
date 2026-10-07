//! Preliminary identity closure for an exact offline plan catalog.
//!
//! This module binds the complete identity roster required by a future M1
//! executable, but it does not authenticate any caller-supplied external
//! identity. The result is therefore deliberately named preliminary and has
//! no artifact, proof, load, launch, or qualification authority.

use super::{hash_field, sha256::Sha256, SequentialPlanCatalog, SEQUENTIAL_PLAN_CATALOG_ENTRIES};
use ferric_kernels::{
    build_structural_kernel_catalog, KernelAuthorityRequirements, KernelCatalogError,
    StructuralKernelCatalog, FERRIC_KERNEL_SOURCE_DECLARATIONS,
};
use ferric_spec::Identity;

/// Canonical format version for the preliminary identity-closure record.
pub const PRELIMINARY_IDENTITY_CLOSURE_VERSION: u32 = 1;
const EXTERNAL_COMPONENT_COUNT: usize = 15;

/// One independently domain-separated external identity required at closure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityClosureComponent {
    /// Exact Ferric qualified source closure.
    FerricSource,
    /// Exact generic fe2o3 compiler/runtime dependency closure.
    Fe2o3Source,
    /// Rust/fe2o3 compiler implementation identity.
    Compiler,
    /// Exact compiler invocation and target-feature identity.
    CompilerConfiguration,
    /// Independent gfx942 target-conformance contract identity.
    TargetContract,
    /// Finite operator schedule catalog identity.
    KernelCatalog,
    /// Exact kernel proof-set identity.
    KernelProofSet,
    /// Complete kernel ABI catalog identity.
    KernelAbiCatalog,
    /// Finalized target/draft executable catalog identity.
    ExecutableCatalog,
    /// Reviewed runtime contract identity.
    RuntimeContract,
    /// Exact runtime ABI and queue protocol identity.
    RuntimeAbi,
    /// Generated runner source identity.
    GeneratedRunner,
    /// Independent validator roster identity.
    ValidatorRegistry,
    /// Qualification protocol identity.
    QualificationProtocol,
    /// Explicit compiler/runtime/hardware TCB report identity.
    TcbReport,
}

/// Caller-supplied identities that only later independent validators may
/// authenticate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExternalIdentityClosureInputs {
    /// Exact Ferric qualified source closure.
    pub ferric_source: Identity,
    /// Exact generic fe2o3 compiler/runtime dependency closure.
    pub fe2o3_source: Identity,
    /// Rust/fe2o3 compiler implementation identity.
    pub compiler: Identity,
    /// Exact compiler invocation and target-feature identity.
    pub compiler_configuration: Identity,
    /// Independent gfx942 target-conformance contract identity.
    pub target_contract: Identity,
    /// Finite operator schedule catalog identity.
    pub kernel_catalog: Identity,
    /// Exact kernel proof-set identity.
    pub kernel_proof_set: Identity,
    /// Complete kernel ABI catalog identity.
    pub kernel_abi_catalog: Identity,
    /// Finalized target/draft executable catalog identity.
    pub executable_catalog: Identity,
    /// Reviewed runtime contract identity.
    pub runtime_contract: Identity,
    /// Exact runtime ABI and queue protocol identity.
    pub runtime_abi: Identity,
    /// Generated runner source identity.
    pub generated_runner: Identity,
    /// Independent validator roster identity.
    pub validator_registry: Identity,
    /// Qualification protocol identity.
    pub qualification_protocol: Identity,
    /// Explicit compiler/runtime/hardware TCB report identity.
    pub tcb_report: Identity,
}

impl ExternalIdentityClosureInputs {
    fn components(&self) -> [(IdentityClosureComponent, Identity); EXTERNAL_COMPONENT_COUNT] {
        [
            (IdentityClosureComponent::FerricSource, self.ferric_source),
            (IdentityClosureComponent::Fe2o3Source, self.fe2o3_source),
            (IdentityClosureComponent::Compiler, self.compiler),
            (
                IdentityClosureComponent::CompilerConfiguration,
                self.compiler_configuration,
            ),
            (
                IdentityClosureComponent::TargetContract,
                self.target_contract,
            ),
            (IdentityClosureComponent::KernelCatalog, self.kernel_catalog),
            (
                IdentityClosureComponent::KernelProofSet,
                self.kernel_proof_set,
            ),
            (
                IdentityClosureComponent::KernelAbiCatalog,
                self.kernel_abi_catalog,
            ),
            (
                IdentityClosureComponent::ExecutableCatalog,
                self.executable_catalog,
            ),
            (
                IdentityClosureComponent::RuntimeContract,
                self.runtime_contract,
            ),
            (IdentityClosureComponent::RuntimeAbi, self.runtime_abi),
            (
                IdentityClosureComponent::GeneratedRunner,
                self.generated_runner,
            ),
            (
                IdentityClosureComponent::ValidatorRegistry,
                self.validator_registry,
            ),
            (
                IdentityClosureComponent::QualificationProtocol,
                self.qualification_protocol,
            ),
            (IdentityClosureComponent::TcbReport, self.tcb_report),
        ]
    }
}

/// Fail-closed preliminary closure construction error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityClosureError {
    /// The retained plan catalog no longer has its exact finite shape.
    InvalidPlanCatalog,
    /// One externally supplied identity is absent.
    MissingExternalIdentity(IdentityClosureComponent),
    /// Two independently domain-separated external components reused one ID.
    ReusedExternalIdentity {
        /// First component using the identity.
        first: IdentityClosureComponent,
        /// Later component reusing it.
        second: IdentityClosureComponent,
    },
    /// The exact plan/operator catalog or its compiler/runtime requirements drifted.
    KernelCatalog(KernelCatalogError),
    /// The caller-supplied kernel-catalog identity is not the canonical record identity.
    KernelCatalogIdentityDrift,
}

/// Exact offline catalog plus its structurally complete identity roster.
///
/// This value retains the consumed prepacked deployment through its plan
/// catalog. It is not `Clone` and cannot be converted into runtime or artifact
/// authority. External identities remain caller assertions until future
/// independent validators authenticate the canonical record.
#[derive(Debug, Eq, PartialEq)]
pub struct PreliminaryIdentityClosure {
    catalog: SequentialPlanCatalog,
    kernel_catalog: StructuralKernelCatalog,
    external: ExternalIdentityClosureInputs,
    closure_id: Identity,
    canonical_bytes: Box<[u8]>,
}

impl PreliminaryIdentityClosure {
    /// Returns the retained exact sequential plan catalog.
    #[must_use]
    pub const fn catalog(&self) -> &SequentialPlanCatalog {
        &self.catalog
    }

    /// Returns the retained exact structural K1-K7 operation catalog.
    #[must_use]
    pub const fn kernel_catalog(&self) -> &StructuralKernelCatalog {
        &self.kernel_catalog
    }

    /// Returns the complete caller-supplied external identity roster.
    #[must_use]
    pub const fn external(&self) -> ExternalIdentityClosureInputs {
        self.external
    }

    /// Returns the domain-separated aggregate preliminary closure identity.
    #[must_use]
    pub const fn closure_id(&self) -> Identity {
        self.closure_id
    }

    /// Returns the canonical byte record hashed by [`Self::closure_id`].
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Returns the canonical preliminary format version.
    #[must_use]
    pub const fn version(&self) -> u32 {
        PRELIMINARY_IDENTITY_CLOSURE_VERSION
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CatalogIdentityInputs {
    bundle_id: Identity,
    target_model_id: Identity,
    target_config_id: Identity,
    target_tokenizer_id: Identity,
    target_vocabulary_id: Identity,
    target_weights_id: Identity,
    draft_model_id: Identity,
    draft_config_id: Identity,
    draft_tokenizer_id: Identity,
    draft_vocabulary_id: Identity,
    draft_weights_id: Identity,
    target_prepacked_id: [u8; 32],
    draft_prepacked_id: [u8; 32],
    plan_catalog_id: Identity,
    plan_ids: Vec<Identity>,
}

/// Consumes the exact sequential catalog and binds a complete preliminary
/// identity roster.
///
/// # Errors
///
/// Returns [`IdentityClosureError`] unless the catalog remains exact and every
/// external identity is present and distinct. The supplied `kernel_catalog`
/// identity must equal [`expected_preliminary_kernel_catalog_identity`] for the
/// retained plans and compiler/runtime requirements. Success does not
/// authenticate the external inputs or grant artifact, proof, load, launch, or
/// qualification authority.
pub fn build_preliminary_identity_closure(
    catalog: SequentialPlanCatalog,
    external: ExternalIdentityClosureInputs,
) -> Result<PreliminaryIdentityClosure, IdentityClosureError> {
    let catalog_inputs = catalog_inputs(&catalog)?;
    validate_external(&external)?;
    let kernel_catalog = structural_kernel_catalog(&catalog, &external)?;
    let expected_kernel_catalog_id = kernel_catalog_identity(&kernel_catalog);
    if external.kernel_catalog != expected_kernel_catalog_id {
        return Err(IdentityClosureError::KernelCatalogIdentityDrift);
    }
    let canonical_bytes = canonical_record(&catalog_inputs, &external);
    let closure_id = identity_record(&canonical_bytes);
    Ok(PreliminaryIdentityClosure {
        catalog,
        kernel_catalog,
        external,
        closure_id,
        canonical_bytes: canonical_bytes.into_boxed_slice(),
    })
}

/// Computes the canonical preliminary kernel-catalog identity for an exact
/// sequential catalog and future compiler/runtime authority requirements.
///
/// The `kernel_catalog` field in `external` is ignored so the caller can derive
/// it before constructing the complete preliminary closure. No external field
/// is authenticated and no execution or evidence authority is granted.
///
/// # Errors
///
/// Returns [`IdentityClosureError`] if the plan catalog or any bound
/// compiler/runtime authority requirement is structurally invalid.
pub fn expected_preliminary_kernel_catalog_identity(
    catalog: &SequentialPlanCatalog,
    external: &ExternalIdentityClosureInputs,
) -> Result<Identity, IdentityClosureError> {
    catalog_inputs(catalog)?;
    let kernel_catalog = structural_kernel_catalog(catalog, external)?;
    Ok(kernel_catalog_identity(&kernel_catalog))
}

fn structural_kernel_catalog(
    catalog: &SequentialPlanCatalog,
    external: &ExternalIdentityClosureInputs,
) -> Result<StructuralKernelCatalog, IdentityClosureError> {
    build_structural_kernel_catalog(
        catalog.plans(),
        catalog.catalog_id(),
        &FERRIC_KERNEL_SOURCE_DECLARATIONS,
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
        },
    )
    .map_err(IdentityClosureError::KernelCatalog)
}

fn kernel_catalog_identity(catalog: &StructuralKernelCatalog) -> Identity {
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, b"ferric.qwen3.structural-kernel-catalog.v2");
    hash_field(&mut hasher, catalog.canonical_bytes());
    Identity::new(hasher.finish())
}

fn catalog_inputs(
    catalog: &SequentialPlanCatalog,
) -> Result<CatalogIdentityInputs, IdentityClosureError> {
    if catalog.version() != super::SEQUENTIAL_PLAN_CATALOG_VERSION
        || catalog.plans().len() != SEQUENTIAL_PLAN_CATALOG_ENTRIES
        || !catalog.catalog_id().is_present()
        || catalog
            .plans()
            .iter()
            .any(|plan| !plan.authority.plan_id.is_present())
        || catalog.deployment().validate().is_err()
    {
        return Err(IdentityClosureError::InvalidPlanCatalog);
    }
    let deployment = catalog.deployment();
    Ok(CatalogIdentityInputs {
        bundle_id: deployment.bundle_id,
        target_model_id: deployment.target_model.config.model_id,
        target_config_id: deployment.target_model.config.config_id,
        target_tokenizer_id: deployment.target_model.tokenizer.tokenizer_id,
        target_vocabulary_id: deployment.target_model.tokenizer.vocabulary_id,
        target_weights_id: deployment.target_model.weights.weights_id,
        draft_model_id: deployment.draft_model.config.model_id,
        draft_config_id: deployment.draft_model.config.config_id,
        draft_tokenizer_id: deployment.draft_model.tokenizer.tokenizer_id,
        draft_vocabulary_id: deployment.draft_model.tokenizer.vocabulary_id,
        draft_weights_id: deployment.draft_model.weights.weights_id,
        target_prepacked_id: catalog.prepacked().target_manifest().aggregate_id(),
        draft_prepacked_id: catalog.prepacked().draft_manifest().aggregate_id(),
        plan_catalog_id: catalog.catalog_id(),
        plan_ids: catalog
            .plans()
            .iter()
            .map(|plan| plan.authority.plan_id)
            .collect(),
    })
}

fn validate_external(external: &ExternalIdentityClosureInputs) -> Result<(), IdentityClosureError> {
    let components = external.components();
    for (index, (component, identity)) in components.iter().copied().enumerate() {
        if !identity.is_present() {
            return Err(IdentityClosureError::MissingExternalIdentity(component));
        }
        for (prior_component, prior_identity) in components[..index].iter().copied() {
            if identity == prior_identity {
                return Err(IdentityClosureError::ReusedExternalIdentity {
                    first: prior_component,
                    second: component,
                });
            }
        }
    }
    Ok(())
}

fn canonical_record(
    catalog: &CatalogIdentityInputs,
    external: &ExternalIdentityClosureInputs,
) -> Vec<u8> {
    let identity_count = 14 + catalog.plan_ids.len() + EXTERNAL_COMPONENT_COUNT;
    let mut record = Vec::with_capacity(16 + identity_count * 32);
    record.extend_from_slice(&PRELIMINARY_IDENTITY_CLOSURE_VERSION.to_le_bytes());
    for identity in [
        catalog.bundle_id,
        catalog.target_model_id,
        catalog.target_config_id,
        catalog.target_tokenizer_id,
        catalog.target_vocabulary_id,
        catalog.target_weights_id,
        catalog.draft_model_id,
        catalog.draft_config_id,
        catalog.draft_tokenizer_id,
        catalog.draft_vocabulary_id,
        catalog.draft_weights_id,
    ] {
        record.extend_from_slice(identity.as_bytes());
    }
    record.extend_from_slice(&catalog.target_prepacked_id);
    record.extend_from_slice(&catalog.draft_prepacked_id);
    record.extend_from_slice(catalog.plan_catalog_id.as_bytes());
    record.extend_from_slice(
        &u64::try_from(catalog.plan_ids.len())
            .expect("bounded plan count fits u64")
            .to_le_bytes(),
    );
    for plan_id in &catalog.plan_ids {
        record.extend_from_slice(plan_id.as_bytes());
    }
    for (_, identity) in external.components() {
        record.extend_from_slice(identity.as_bytes());
    }
    record
}

fn identity_record(record: &[u8]) -> Identity {
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, b"ferric.qwen3.preliminary-identity-closure.v1");
    hash_field(&mut hasher, record);
    Identity::new(hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_record, identity_record, validate_external, CatalogIdentityInputs,
        ExternalIdentityClosureInputs, IdentityClosureComponent, IdentityClosureError,
        PRELIMINARY_IDENTITY_CLOSURE_VERSION,
    };
    use ferric_spec::Identity;

    const fn identity(byte: u8) -> Identity {
        Identity::new([byte; 32])
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
            generated_runner: identity(42),
            validator_registry: identity(43),
            qualification_protocol: identity(44),
            tcb_report: identity(45),
        }
    }

    fn catalog() -> CatalogIdentityInputs {
        CatalogIdentityInputs {
            bundle_id: identity(1),
            target_model_id: identity(2),
            target_config_id: identity(3),
            target_tokenizer_id: identity(4),
            target_vocabulary_id: identity(5),
            target_weights_id: identity(6),
            draft_model_id: identity(7),
            draft_config_id: identity(8),
            draft_tokenizer_id: identity(4),
            draft_vocabulary_id: identity(5),
            draft_weights_id: identity(9),
            target_prepacked_id: [10; 32],
            draft_prepacked_id: [11; 32],
            plan_catalog_id: identity(12),
            plan_ids: (0..22).map(|index| identity(index + 60)).collect(),
        }
    }

    #[test]
    fn complete_roster_has_one_deterministic_identity() {
        let record = canonical_record(&catalog(), &external());
        assert_eq!(
            &record[..4],
            &PRELIMINARY_IDENTITY_CLOSURE_VERSION.to_le_bytes()
        );
        assert_eq!(record.len(), 4 + (14 + 22 + 15) * 32 + 8);
        let closure_id = identity_record(&record);
        assert_eq!(
            closure_id,
            Identity::new([
                0x91, 0x40, 0x53, 0x13, 0x4e, 0xd4, 0x80, 0x2f, 0xa6, 0xd8, 0x81, 0x69, 0x69, 0xa2,
                0x15, 0x7d, 0xdb, 0xa0, 0xd1, 0xf0, 0xaa, 0xb0, 0x94, 0x73, 0xe7, 0x11, 0x2a, 0x58,
                0xe2, 0x65, 0x3f, 0xf8,
            ])
        );
        assert!(closure_id.is_present());
    }

    #[test]
    fn every_external_identity_is_required() {
        let exact = external();
        let components = exact.components();
        for (index, (expected, _)) in components.iter().copied().enumerate() {
            let mut changed = exact;
            let replacement = Identity::new([0; 32]);
            match index {
                0 => changed.ferric_source = replacement,
                1 => changed.fe2o3_source = replacement,
                2 => changed.compiler = replacement,
                3 => changed.compiler_configuration = replacement,
                4 => changed.target_contract = replacement,
                5 => changed.kernel_catalog = replacement,
                6 => changed.kernel_proof_set = replacement,
                7 => changed.kernel_abi_catalog = replacement,
                8 => changed.executable_catalog = replacement,
                9 => changed.runtime_contract = replacement,
                10 => changed.runtime_abi = replacement,
                11 => changed.generated_runner = replacement,
                12 => changed.validator_registry = replacement,
                13 => changed.qualification_protocol = replacement,
                14 => changed.tcb_report = replacement,
                _ => unreachable!(),
            }
            assert_eq!(
                validate_external(&changed),
                Err(IdentityClosureError::MissingExternalIdentity(expected))
            );
        }
    }

    #[test]
    fn cross_domain_identity_reuse_is_rejected() {
        let mut changed = external();
        changed.runtime_abi = changed.compiler;
        assert_eq!(
            validate_external(&changed),
            Err(IdentityClosureError::ReusedExternalIdentity {
                first: IdentityClosureComponent::Compiler,
                second: IdentityClosureComponent::RuntimeAbi,
            })
        );
    }

    #[test]
    fn every_catalog_and_external_field_rekeys_the_record() {
        let base_catalog = catalog();
        let base_external = external();
        let base = identity_record(&canonical_record(&base_catalog, &base_external));

        for index in 0..14 {
            let mut changed = base_catalog.clone();
            let replacement = identity(130 + u8::try_from(index).unwrap());
            match index {
                0 => changed.bundle_id = replacement,
                1 => changed.target_model_id = replacement,
                2 => changed.target_config_id = replacement,
                3 => changed.target_tokenizer_id = replacement,
                4 => changed.target_vocabulary_id = replacement,
                5 => changed.target_weights_id = replacement,
                6 => changed.draft_model_id = replacement,
                7 => changed.draft_config_id = replacement,
                8 => changed.draft_tokenizer_id = replacement,
                9 => changed.draft_vocabulary_id = replacement,
                10 => changed.draft_weights_id = replacement,
                11 => changed.target_prepacked_id = *replacement.as_bytes(),
                12 => changed.draft_prepacked_id = *replacement.as_bytes(),
                13 => changed.plan_catalog_id = replacement,
                _ => unreachable!(),
            }
            assert_ne!(
                base,
                identity_record(&canonical_record(&changed, &base_external))
            );
        }

        for index in 0..base_catalog.plan_ids.len() {
            let mut changed = base_catalog.clone();
            changed.plan_ids[index] = identity(200 + u8::try_from(index).unwrap());
            assert_ne!(
                base,
                identity_record(&canonical_record(&changed, &base_external))
            );
        }

        for index in 0..15 {
            let mut changed = base_external;
            let replacement = identity(160 + u8::try_from(index).unwrap());
            match index {
                0 => changed.ferric_source = replacement,
                1 => changed.fe2o3_source = replacement,
                2 => changed.compiler = replacement,
                3 => changed.compiler_configuration = replacement,
                4 => changed.target_contract = replacement,
                5 => changed.kernel_catalog = replacement,
                6 => changed.kernel_proof_set = replacement,
                7 => changed.kernel_abi_catalog = replacement,
                8 => changed.executable_catalog = replacement,
                9 => changed.runtime_contract = replacement,
                10 => changed.runtime_abi = replacement,
                11 => changed.generated_runner = replacement,
                12 => changed.validator_registry = replacement,
                13 => changed.qualification_protocol = replacement,
                14 => changed.tcb_report = replacement,
                _ => unreachable!(),
            }
            assert_ne!(
                base,
                identity_record(&canonical_record(&base_catalog, &changed))
            );
        }
    }
}
