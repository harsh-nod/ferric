//! Environment-gated composition coverage for the shared authenticated selector bootstrap.

use std::convert::Infallible;

use fe2o3_host::{
    CompilerGeneratedKernelExpectationRosterV1, WorkerV3CompilerExecutionVerificationV1,
    WorkerV3ProtectedRosterEntryEvidenceV1, WorkerV3ProtectedRosterVerificationEvidenceV1,
    WorkerV3ProtectedRosterVerifierAdapterV1, WorkerV3ProtectedRosterVerifierBackendV1,
    WorkerV3RosterVerificationRequestV1, WorkerV3SafetyPropertiesV1,
};

const SELECTOR_MANIFEST_ENV_V1: &str =
    "FERRIC_M1_AUTHENTICATED_SELECTOR_BOOTSTRAP_V1_SELECTOR_MANIFEST";
const SERVICE_MANIFEST: &str = include_str!("../Cargo.toml");

struct ExactSelectorFixtureProtectedVerifierV1;

// SAFETY: this backend is compiled only by the service's integration-test target. It independently
// replays the exact aggregate finalizer and returns explicitly synthetic, request-bound evidence
// through fe2o3's feature-gated test seam. It must never support a production authority claim.
unsafe impl<R> WorkerV3ProtectedRosterVerifierBackendV1<R>
    for ExactSelectorFixtureProtectedVerifierV1
where
    R: CompilerGeneratedKernelExpectationRosterV1,
{
    type Error = Infallible;

    unsafe fn verify_protected_roster(
        &mut self,
        request: &WorkerV3RosterVerificationRequestV1<'_, R>,
    ) -> Result<WorkerV3ProtectedRosterVerificationEvidenceV1, Self::Error> {
        let compiler_execution = WorkerV3CompilerExecutionVerificationV1::synthetic_for_test_only(
            request.compiler_execution_subject_sha256(),
            request.compiler_execution_carriage_sha256(),
            request.compiler_execution_policy_sha256(),
            request.compiler_execution_issuer_journal_sha256(),
            request.compiler_occurrence_sha256(),
            request.compiler_execution_receipt_sha256(),
            request.compiler_execution_publication_sha256(),
            request.compiler_execution_acknowledgment_sha256(),
            request.compiler_execution_worker_ledger_record_sha256(),
            request.compiler_execution_sequence(),
            request.compiler_execution_prior_rollback_anchor(),
            request.compiler_execution_current_rollback_anchor(),
            [0xd1; 32],
            [0xd2; 32],
            [0xd3; 32],
            [0xd4; 32],
            [0xd5; 32],
        );
        let finalizer = request
            .independently_revalidate_finalizer_derivation()
            .expect("the exact retained publication must replay its finalizer derivation");
        let entries = request
            .marker_entries()
            .iter()
            .enumerate()
            .map(|(ordinal, expected)| {
                let seed = 0x20_u8.wrapping_add(
                    u8::try_from(ordinal).expect("the exact M1 roster ordinal fits u8"),
                );
                // SAFETY: each test entry copies exact request lineage and generated coordinates
                // and supplies complete nonzero identities. Host admission still checks all joins.
                unsafe {
                    WorkerV3ProtectedRosterEntryEvidenceV1::new(
                        request
                            .entry_lineage_identity(ordinal)
                            .expect("the exact request retains every roster lineage"),
                        expected.kernel_binding_id(),
                        expected.generated_host_contract_identity(),
                        [seed; 32],
                        [seed.wrapping_add(1); 32],
                        [seed.wrapping_add(2); 32],
                        WorkerV3SafetyPropertiesV1::required(),
                    )
                }
            })
            .collect::<Vec<_>>();
        // SAFETY: this synthetic evidence is confined to the ignored integration test below and
        // cannot be represented as protected verifier, load, launch, or qualification authority.
        Ok(unsafe {
            WorkerV3ProtectedRosterVerificationEvidenceV1::synthetic_for_test_only(
                finalizer,
                compiler_execution,
                [0xc1; 32],
                [0xc2; 32],
                entries,
            )
        })
    }
}

#[test]
fn bootstrap_fixture_dependencies_are_test_only() {
    let (production, development) = SERVICE_MANIFEST
        .split_once("[dev-dependencies]")
        .expect("service manifest has an explicit test-dependency boundary");
    for dependency in ["ferric-build", "ferric-engine"] {
        assert!(!production.lines().any(|line| line.starts_with(dependency)));
        assert!(development.lines().any(|line| line.starts_with(dependency)));
    }
    let production_host = production
        .lines()
        .find(|line| line.starts_with("fe2o3-host ="))
        .expect("checker transport uses the canonical production safety-property type");
    assert!(!production_host.contains("features"));
    assert!(!production.contains("worker-v3-verifier-test-support"));
    assert!(development.lines().any(|line| line.starts_with("fe2o3-host =")));
    assert!(development.contains("worker-v3-verifier-test-support"));
}

#[test]
#[ignore = "requires FERRIC_M1_AUTHENTICATED_SELECTOR_BOOTSTRAP_V1_SELECTOR_MANIFEST to name an exact retained 12-kernel Worker V3 publication"]
fn exact_retained_publication_reaches_shared_bootstrap_with_test_verifier() {
    // This is bootstrap-composition coverage only. The test verifier supplies no protected-
    // verifier, load, launch, hardware, performance, or qualification evidence.
    let selector_manifest = std::env::var_os(SELECTOR_MANIFEST_ENV_V1)
        .expect("set the exact retained-publication selector-manifest environment variable");
    let selector_bytes = std::fs::read(selector_manifest)
        .expect("read the exact canonical aggregate V2 selector manifest");
    let selector = ferric_engine::decode_m1_worker_v3_selector_manifest_v2(&selector_bytes)
        .expect("decode the exact canonical aggregate V2 selector manifest");
    let mut catalog_verifier =
        WorkerV3ProtectedRosterVerifierAdapterV1::new(ExactSelectorFixtureProtectedVerifierV1);
    let programs = ferric_engine::acquire_m1_all_kernels_authenticated_worker_v3_programs_v1(
        selector.clone(),
        &mut catalog_verifier,
    )
    .expect("authenticate the exact retained publication for its program catalog");
    let executable_catalog = programs.catalog_id();
    drop(programs);
    let declaration = ferric_build::generate_qwen3_gfx942_runner_declaration(
        ferric_build::qwen3_runner_closure_test_fixture_with_executable_catalog(executable_catalog),
    )
    .expect("generate the exact M1 runner fixture");
    let publication = ferric_build::publish_qwen3_gfx942_runner_declaration(declaration)
        .expect("publish the exact M1 runner fixture");
    assert_eq!(publication.executable_catalog_id(), executable_catalog);
    let expected_source_id = publication.source_id();
    let expected_plan_catalog_id = publication.plan_catalog_id();
    let expected_kernel_catalog_id = publication.kernel_catalog_id();
    let expected_declaration_id = publication.declaration_id();
    let expected_operation_count = publication.operations().len();
    let mut verifier =
        WorkerV3ProtectedRosterVerifierAdapterV1::new(ExactSelectorFixtureProtectedVerifierV1);

    let runner = ferric_engine::bind_m1_authenticated_physical_runner_from_selector_v1(
        selector,
        &mut verifier,
        publication,
    )
    .expect("the exact retained publication must reach authenticated runner binding");

    assert_eq!(runner.program_catalog_id(), executable_catalog);
    assert_eq!(runner.declaration_id(), expected_declaration_id);
    assert_eq!(runner.kernel_catalog_id(), expected_kernel_catalog_id);
    assert_eq!(runner.operation_count(), expected_operation_count);
    assert_eq!(runner.logical_runner().source_id(), expected_source_id);
    assert_eq!(
        runner.logical_runner().plan_catalog_id(),
        expected_plan_catalog_id
    );
    assert_eq!(
        runner.logical_runner().kernel_catalog_id(),
        expected_kernel_catalog_id
    );
    assert_eq!(
        runner.logical_runner().declaration_id(),
        expected_declaration_id
    );
    assert_eq!(
        runner.logical_runner().operation_count(),
        expected_operation_count
    );
}

#[test]
#[ignore = "requires FERRIC_M1_AUTHENTICATED_SELECTOR_BOOTSTRAP_V1_SELECTOR_MANIFEST to name an exact retained 12-kernel Worker V3 publication"]
fn substituted_runner_catalog_is_rejected_with_every_owner_retained() {
    let selector_manifest = std::env::var_os(SELECTOR_MANIFEST_ENV_V1)
        .expect("set the exact retained-publication selector-manifest environment variable");
    let selector_bytes = std::fs::read(selector_manifest)
        .expect("read the exact canonical aggregate V2 selector manifest");
    let selector = ferric_engine::decode_m1_worker_v3_selector_manifest_v2(&selector_bytes)
        .expect("decode the exact canonical aggregate V2 selector manifest");
    let mut catalog_verifier =
        WorkerV3ProtectedRosterVerifierAdapterV1::new(ExactSelectorFixtureProtectedVerifierV1);
    let programs = ferric_engine::acquire_m1_all_kernels_authenticated_worker_v3_programs_v1(
        selector.clone(),
        &mut catalog_verifier,
    )
    .expect("authenticate the exact retained publication for its program catalog");
    let expected = programs.catalog_id();
    drop(programs);

    let declaration = ferric_build::generate_qwen3_gfx942_runner_declaration(
        ferric_build::qwen3_runner_closure_test_fixture(),
    )
    .expect("generate the substituted M1 runner fixture");
    let publication = ferric_build::publish_qwen3_gfx942_runner_declaration(declaration)
        .expect("publish the substituted M1 runner fixture");
    let actual = publication.executable_catalog_id();
    let expected_source_id = publication.source_id();
    let expected_kernel_catalog_id = publication.kernel_catalog_id();
    let expected_declaration_id = publication.declaration_id();
    assert_ne!(
        actual, expected,
        "the hostile fixture must substitute the catalog"
    );
    let mut verifier =
        WorkerV3ProtectedRosterVerifierAdapterV1::new(ExactSelectorFixtureProtectedVerifierV1);

    let error = ferric_engine::bind_m1_authenticated_physical_runner_from_selector_v1(
        selector,
        &mut verifier,
        publication,
    )
    .expect_err("a substituted runner catalog cannot receive authenticated program custody");
    match error {
        ferric_engine::M1AuthenticatedPhysicalRunnerBootstrapFailureV1::Binding(failure) => {
            match *failure {
                ferric_engine::M1PhysicalRunnerBindFailureV1::ExecutableCatalog {
                    expected: retained_expected,
                    actual: retained_actual,
                    programs,
                    publication,
                } => {
                    assert_eq!(retained_expected, expected);
                    assert_eq!(retained_actual, actual);
                    assert_eq!(programs.catalog_id(), expected);
                    assert_eq!(publication.executable_catalog_id(), actual);
                    assert_eq!(publication.source_id(), expected_source_id);
                    assert_eq!(publication.kernel_catalog_id(), expected_kernel_catalog_id);
                    assert_eq!(publication.declaration_id(), expected_declaration_id);
                }
                other => panic!("unexpected binding failure: {other:?}"),
            }
        }
        other => panic!("unexpected bootstrap failure: {other:?}"),
    }
}
