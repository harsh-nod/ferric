#![forbid(unsafe_code)]

//! Offline admission of the pinned Qwen3 M1 deployment pair.
//!
//! This crate owns byte parsing and identity construction. The resulting
//! value is still admitted by the executable contract in `ferric-spec`.
//! Configuration, tokenizer metadata, tokenizer vocabulary, and safetensors
//! authentication remain separate sealed stages until the final bundle path
//! consumes their authorities.

mod auth;
/// Canonical M1 deployment-bundle wire format and verifier-visible relations.
pub mod bundle;
mod identity_closure;
mod json;
mod kernel_artifact_manifest;
mod kernel_artifact_policy;
mod kernel_artifacts;
mod memory_layout;
mod model;
mod model_layout;
mod plan;
mod runner;
mod safetensors;
mod sha256;
mod step_workspace;
mod tokenizer;
mod tokenizer_execution;
mod weight_stream;

pub use auth::{
    decode_bundle_admission_record, seal_authenticated_bundle, AuthenticatedBundleAdmission,
    BundleAdmissionDescriptor, BundleAdmissionError, BundleAdmissionRecord, ManifestCommitment,
    BUNDLE_ADMISSION_RECORD_BYTES, BUNDLE_ADMISSION_RECORD_VERSION, MANIFEST_COMMITMENT_BYTES,
};
pub use bundle::{
    decode_canonical_deployment_bundle, encode_canonical_deployment_bundle, CanonicalBundleError,
    CanonicalDeploymentBundle, CANONICAL_DEPLOYMENT_BUNDLE_BYTES,
    CANONICAL_DEPLOYMENT_BUNDLE_VERSION,
};
pub use identity_closure::{
    build_preliminary_identity_closure, expected_preliminary_kernel_catalog_identity,
    ExternalIdentityClosureInputs, IdentityClosureComponent, IdentityClosureError,
    PreliminaryIdentityClosure, PRELIMINARY_IDENTITY_CLOSURE_VERSION,
};
pub use kernel_artifact_manifest::{
    decode_m1_kernel_artifact_manifest_v1, M1KernelArtifactEntryV1, M1KernelArtifactFamilyV1,
    M1KernelArtifactManifestErrorV1, M1KernelArtifactManifestV1, M1KernelArtifactProgramV1,
    M1KernelProfileCatalogV1, M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1,
    M1_KERNEL_ARTIFACT_MANIFEST_VERSION_V1, M1_PHYSICAL_PROGRAM_COUNT_V1,
};
pub use kernel_artifact_policy::{
    M1_KERNEL_WORKER_BUILD_IDENTITY_V1, M1_KERNEL_WORKER_EXECUTABLE_BYTES_V1,
    M1_KERNEL_WORKER_EXECUTABLE_SHA256_V1, M1_KERNEL_WORKER_LLVM_BUILD_IDENTITY_V1,
};
pub use kernel_artifacts::{
    build_and_publish_m1_kernel_artifacts_v1, BuiltAndInspectedM1KernelArtifactsV1,
    M1KernelArtifactBuildErrorV1, M1KernelArtifactBuildStageV1,
    M1KernelArtifactPublicationStatusV1, M1_KERNEL_ARTIFACT_MANIFEST_FILENAME_V1,
};
#[cfg(feature = "test-fixtures")]
#[doc(hidden)]
pub use memory_layout::qwen3_model_memory_plan_test_fixture;
pub use memory_layout::{
    plan_authenticated_model_memory, qwen3_kv_arena_bytes, AddresslessModelMemoryPlan,
    DeclaredDeviceAllocation, DeclaredMemoryRange, KvCacheComponent, ModelKvPageMemoryBinding,
    ModelMemoryAllocationKind, ModelMemoryAllocationSet, ModelMemoryPlanError,
    ModelMemoryPlanFailure, ModelMemoryPlanOutcome, ModelWeightMemoryBinding,
    QWEN3_KV_ARENA_ALIGNMENT_V1, QWEN3_KV_LAYER_BYTES_V1, QWEN3_KV_PAGE_BYTES_V1,
    QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
};
pub use model::{
    build_authenticated_deployment_bundle, build_preliminary_deployment_bundle,
    build_prepacked_deployment_bundle, build_weight_authenticated_deployment_bundle, digest_bytes,
    ArtifactDigest, AuthenticatedDeploymentAssets, AuthenticatedModelAssets, BuildError,
    DeploymentAssets, ModelAssets, PrepackedDeploymentBundle, WeightAuthenticatedDeploymentAssets,
    WeightAuthenticatedModelAssets, WeightDescriptor, DRAFT_REPOSITORY, DRAFT_REVISION,
    QWEN3_DRAFT_MODEL_ID, QWEN3_DRAFT_TENSOR_DATA_BYTES, QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
    QWEN3_DRAFT_WEIGHT_SHA256, QWEN3_TARGET_MODEL_ID, QWEN3_TARGET_TENSOR_DATA_BYTES,
    QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES, QWEN3_TARGET_WEIGHT_SET_SHA256, QWEN3_TOKENIZER_BYTES,
    QWEN3_TOKENIZER_SHA256, TARGET_REPOSITORY, TARGET_REVISION,
};
pub use model_layout::{
    build_authenticated_model_weight_layout, AuthenticatedModelWeightLayout, ModelWeightBinding,
    ModelWeightLayoutError,
};
pub use plan::{
    build_authenticated_sequential_plan_catalog, build_sequential_plan_catalog,
    SequentialPlanCatalog, SequentialPlanError, SEQUENTIAL_PLAN_CATALOG_ENTRIES,
    SEQUENTIAL_PLAN_CATALOG_VERSION,
};
#[cfg(feature = "test-fixtures")]
#[doc(hidden)]
pub use runner::qwen3_runner_closure_test_fixture;
pub use runner::{
    expected_qwen3_gfx942_runner_source_identity, generate_qwen3_gfx942_runner_declaration,
    publish_qwen3_gfx942_runner_declaration, render_qwen3_gfx942_runner_source,
    render_qwen3_gfx942_runner_source_closure, validate_qwen3_gfx942_runner_declaration,
    validate_qwen3_gfx942_runner_source_closure, GeneratedOperationDeclaration,
    GeneratedPlanDeclaration, GeneratedRunnerDeclaration, GeneratedRunnerError,
    PublishedRunnerDeclaration, GENERATED_RUNNER_DECLARATION_VERSION,
};
pub use safetensors::{
    authenticate_qwen3_draft_weights, authenticate_qwen3_target_weights, AuthenticatedWeightSet,
    SafetensorsError, SafetensorsSource,
};
pub use step_workspace::{
    m1_step_workspace_requirements, plan_addressless_m1_step_workspace,
    AddresslessM1StepWorkspacePlan, AvailableM1StepWorkspace, DeclaredM1StepWorkspaceAllocation,
    M1StepWorkspaceDeclaration, M1StepWorkspaceMemoryBinding, M1StepWorkspacePlanError,
    M1StepWorkspacePlanFailure, M1StepWorkspacePlanOutcome, M1StepWorkspaceRange,
    M1StepWorkspaceRangeRole, M1StepWorkspaceRequirements,
    M1_STEP_WORKSPACE_ALLOCATION_ALIGNMENT_V1, M1_STEP_WORKSPACE_LAYOUT_VERSION,
};
pub use tokenizer::{authenticate_qwen3_tokenizer, AuthenticatedTokenizer, TokenizerError};
pub use tokenizer_execution::{
    SpecialTokenDecodePolicy, SpecialTokenEncodePolicy, TokenizerExecutionError,
    TokenizerExecutionLimits, MAX_TOKENIZER_INPUT_BYTES, MAX_TOKENIZER_OUTPUT_BYTES,
    MAX_TOKENIZER_OUTPUT_TOKENS, MAX_TOKENIZER_WORK,
};
pub use weight_stream::{
    prepack_qwen3_draft_weights, prepack_qwen3_target_weights, reopen_persisted_qwen3_weights,
    PersistedPrepackedWeightError, PrepackedWeightSet, WeightSection, WeightSectionManifest,
    WeightStreamError, WeightTransform, PREPACKED_WEIGHT_MANIFEST_VERSION,
    QWEN3_DRAFT_PREPACKED_MANIFEST_BYTES, QWEN3_DRAFT_PREPACKED_MANIFEST_SHA256,
    QWEN3_TARGET_PREPACKED_MANIFEST_BYTES, QWEN3_TARGET_PREPACKED_MANIFEST_SHA256,
};

pub(crate) use model::{
    bundle_identity, decode_hex_32, encode_u32_big_endian, encode_u64_big_endian, hash_field,
    model_identity, ADDED_TOKENS, DRAFT_CONFIG_SHA256, TARGET_CONFIG_SHA256,
    TOKENIZER_METADATA_SHA256,
};
