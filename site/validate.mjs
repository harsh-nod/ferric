import { readFile, stat } from "node:fs/promises";
import { dirname, join, normalize, relative } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const siteRoot = dirname(fileURLToPath(import.meta.url));
const allowedStates = new Set([
  "implemented",
  "integration",
  "observed",
  "verified",
  "qualified",
  "open",
]);
const expectedCurrent = Object.freeze({
  siteRefreshBase: "09678a2d013287da4c36ece506166e1baa94b96f",
  integrationCommit: "30af5c2012850afa525539a7e50f8a3b92497f50",
  integrationTree: "841865566065230e28856ecf835475180a36d87e",
  ferricCheckpointCommit: "9431cb4638401718b8085bcf258a390650c719ef",
  ferricCheckpointTree: "09c48165b19883aebbebd2e77b827794cd039164",
  ferricFeatureBranch: "origin/codex/fe2o3-current-main-repin-v1",
  ferricCheckpointStatus: "public-feature-branch-not-main-not-final-all-m1-gates-open",
  aggregateBuildConfigFormat: "fe2o3-production-build-config-v2",
  aggregateBuildObservation: "source-isa-summary-v1",
  aggregateKernelModuleCount: 7,
  aggregatePrebuiltKernelDependency: false,
  aggregateVendorKernelDependency: false,
  implementationCommit: "7f516e073b8759eb012c998bc9df2eb101d0c7ab",
  authenticatedR32Commit: "d67fae3b063b1997aaa92b0cbc6f4c960c3b010b",
  aggregateSelectionCommit: "eceffdf00c1ec0f7241be95d6b636fa1ea69a46d",
  aggregateSelectionStatus: "noncurrent-candidate",
  pendingVerifierProjectionCommit: "75c5f724fbc7928bf1b231a86aec0f1d5fdcc3f9",
  commonCustodyPreflightCommit: "e187ca52dfdaee79fdc17921c9acffebeed6ca96",
  associationPreflightCommit: "eb3b1937ec509cb6ecea080a25965dd3e8bc5457",
  finalizedHsacoReinspectionCommit: "749324c9e287aaec688c8733c88becddc539b12e",
  fe2o3EngineeringSchemaCommit: "5099cf38c7bee0aa513a8cf9d5ce4efb56a0ffa8",
  fe2o3EngineeringSchemaTree: "e089a7e95eb4c103e61e973321ed79a7b1233364",
  fe2o3CompilerCandidate: "5f46dd23cfa3cf58118a93ee2bf5e1252903366c",
  fe2o3CompilerCandidateTree: "52ec0a254d626102cbfa37b79a906722490cc9d8",
  fe2o3CompilerQualificationBase: "ce44dc0342003d6325e18f0a83eb27a61601282c",
  fe2o3LatestMain: "5f46dd23cfa3cf58118a93ee2bf5e1252903366c",
  fe2o3CheckedComponentProjectionCommit: "b4f600ae92288ccf1662ddff40d71fefba631777",
  fe2o3Llvm22WorkerSwitchCommit: "ce44dc0342003d6325e18f0a83eb27a61601282c",
  fe2o3ConnectedPathTransportCommit: "1167b9cfeb28d36ede1ca5ad5f2a71cb5a8719ee",
  fe2o3BoundedQueueWaitCommit: "5f46dd23cfa3cf58118a93ee2bf5e1252903366c",
  fe2o3GuardedSubtractionCommit: "e745bc75c",
  fe2o3CompilerCandidateStatus: "public-main-consumed-by-local-ferric-integration",
  productionSpeculativeExecutorCandidate: "0c2b73bfb8d4e62c100c42a125171c271c8850d8",
  productionSpeculativeExecutorTree: "00c4b8a04aab2f52af0f43de8a26a7e9564c5568",
  productionSpeculativeExecutorIntegrationCommit: "867f863e223d00e3b304d324e89146e27d2c5c28",
  productionSpeculativeExecutorStatus: "independent-go-integrated",
  engineeringAggregateLoaderCandidate: "c9072b0de61a27be917020baf5eecb4b743734f0",
  engineeringAggregateLoaderTree: "c725eb6e3e6f470fa327f94289509fe910eb83ef",
  engineeringAggregateLoaderIntegrationCommit: "99cf0d514feb7fccb916f066c645c3a1cf831a0c",
  engineeringAggregateLoaderStatus: "independent-go-integrated",
  engineeringAggregateHsacoStatus: "not-produced",
  engineeringAggregateAttemptOuterExitCode: 1,
  engineeringAggregateAttemptBoundary: "v20-prefill-cache-length-semantic-value-mismatch-before-hsaco",
  engineeringAggregateAttemptConnectCount: 0,
  engineeringAggregateAttemptOutputCount: 0,
  engineeringAggregateAttemptStatus:
    "exact-5f-aggregate-v20-stopped-at-prefill-distinct-cache-length-semantic-values-no-hsaco",
  followupProofFixStatus: "focused-v20-pass-v21-single-cache-length-ssa-local-fix-active",
  fe2o3CurrentnessStatus: "public-main-5f46dd2-consumed-by-local-integration-30af5c2",
  fe2o3IsFiniteRemediationStatus: "independent-source-go",
  targetEngineeringSmokeCandidate: "951d48ac119089a62546cb6f96f324feaad013af",
  targetEngineeringSmokeTree: "ffad404f1bce2ee8c55d94b226d9d54dcd8fc62c",
  targetEngineeringSmokeIntegrationCommit: "a2bb2dc9f0087d4573d58b7c0f5b15aee3b3245b",
  targetEngineeringSmokeStatus: "event-timed-feature-branch-not-executed",
  targetEngineeringSmokeEngineTests: 511,
  targetEngineeringSmokeCaptureTests: 84,
  targetEngineeringSmokeDoctests: 145,
  targetEngineeringSmokeExactFinalPinStatus: "open",
  targetEngineeringSmokeHardwareStatus: "not-run",
  servingComparisonR33V3IntegrationCommit: "a2bb2dc9f0087d4573d58b7c0f5b15aee3b3245b",
  servingComparisonR33V3Status: "event-backed-feature-branch-not-run",
  gpuAvailabilityStatus: "not-revalidated-at-this-checkpoint",
  baselineAuditStatus: "not-revalidated-at-this-checkpoint",
  comparisonStatus: "not-run",
  protectedVerifierServiceLocalCandidate: "9a435522a4a88d55108f7c6a4cb493aabb01ad93",
  protectedVerifierServiceStatus:
    "one-shot-listener-integrated-not-deployed-no-checker-signer-supervisor-or-r33-backend",
  verifierBinderCandidate: "6846d9282f858c80dd2b0b4abfe247dc89e9d8f8",
  verifierBinderCandidateTree: "4690d8c9e502de18a947d6def2f8c09d4f153ea1",
  verifierBinderIntegrationCommit: "ed708de7fc906926091be29ff118af95ee50a42b",
  verifierBinderStatus: "qualified-go-local-integration",
  authenticatedTargetRolloverCommit: "047ee32f6d0bb1861adb211c9ced1f403a22514c",
  authenticatedTargetRolloverStatus: "implemented-integrated-not-run",
  authenticatedTargetServingBridgeCommit: "f8f8ce60e23a1331a83606029fca2d0958bd3157",
  authenticatedTargetServingBridgeStatus:
    "new-window-control-transaction-public-lower-physical-rebind-unavailable",
  swigluCheckedBlockCommit: "ff2ca043600550ec72957ac99bae64251f2e4184",
  healthyAllTerminalShutdownCommit: "9eba94342a99f55a3d38294ae34341f37fe03767",
  daemonLifecycleProofCommit: "3f5b4982fa08c4488ff13bb7db37bcbb0ab8368f",
  daemonLifecycleProofTree: "cb79327ebd365d396f388c7a55023c4557a2b0d8",
  daemonLifecycleProofStatus:
    "public-feature-branch-verified-authority-free-no-service-conformance",
  daemonLifecycleOrderedMeasures: 20,
  daemonLifecyclePositiveRows: 4,
  daemonLifecycleHostileMutations: 2,
  supervisedR33WireCommit: "66b8547ab7ce1572d29557d2e9c16d58b2b6f41f",
  supervisedR33WireTree: "8e7348bcd1b99cffa488da44a77813e44bbb3bc7",
  supervisedR33WireStatus: "public-feature-branch-authority-free-no-serving",
  localIntegrationCandidate: "30af5c2012850afa525539a7e50f8a3b92497f50",
  localIntegrationCandidateStatus:
    "local-unpublished-non-final-latest-fe2-integration-no-release-qualification",
  localBoundedQueueWaitCandidate: "9f3618bba902b6e5468ea017e1fdef826f89f130",
  localBoundedQueueWaitCandidateStatus:
    "local-focused-qualified-integrated-as-e3cfc62-in-30af5c2-stack",
  catalogBindingIntegrationCommit: "5b23a36c04057850fa3382db05e2eff905f41213",
  descriptorOrderIntegrationCommit: "e0e79fe9980a5a302a8b2bd18eef0f9e13161a3c",
  boundedQueueWaitIntegrationCommit: "e3cfc62e14d0ac35a6b48e6e754e5165f8cb0b68",
  combinedPolicyIntegrationCommit: "c29c52cf5e05e298bf9b52af204d0881e7fa3cdf",
  connectedPathIntegrationCommit: "7f4a251707d2b0d8811184533b1d8f805a745726",
  protectedVerifierListenerIntegrationCommit: "64ab4fc98ecc64e78b4591197a47674f27bc992e",
  boundedProductionQueueWaitIntegrationCommit: "1e7695d9517a9711389a907c1ebbc2a6bc76249a",
  combinedEngineTestsPassed: 576,
  combinedEngineTestsFailed: 0,
  combinedEngineEnvGatedIgnored: 5,
  boundedWaitHarnessTestsPassed: 11,
  boundedWaitPacketTestsPassed: 2,
  boundedWaitQualificationTestsPassed: 75,
  boundedWaitQualificationTestsIgnored: 2,
  boundedWaitPreflightTestsPassed: 2,
  boundedWaitDoctestsPassed: 148,
  boundedWaitAdapterTestsPassed: 8,
  boundedWaitAdapterDoctestsPassed: 3,
  boundedWaitClippyStatus: "pass",
  combinedSourceGateModules: 162,
  combinedSourceGateBodies: 7533,
  combinedSourceGateTestsPassed: 28,
  combinedSourceGateTestsFailed: 0,
  boundedWaitHostilePolicyStatus: "pass",
  connectedPathAdapterTestsPassed: 88,
  connectedPathServiceTestsPassed: 46,
  connectedPathIntentionalIgnores: 3,
  connectedPathAdapterClippyStatus: "pass",
  connectedPathServiceClippyStatus: "pass",
  protectedListenerSourceGateTestsPassed: 28,
  combinedSourceInventoryDiscovered: 6951,
  combinedSourceInventoryAdmitted: 6854,
  combinedSourceInventoryUnadmitted: 97,
  combinedSourceInventoryStale: 0,
  combinedFormalRegenerationStatus:
    "canonical-current-inventory-regenerated-source-gate-28-pass-runtime-instant-kfd-explicitly-unverified",
  kernelFocusedV19LogSha256:
    "ec13dc73ca35d705268567e72a3caa1c1c0deedac8947297b150424eab139ad3",
  kernelExactV19FailureLogSha256:
    "751a189b8db108cd3c8c52ce0e39a7fde28ccd20415890832dfbc6024cb99365",
  kernelExactV19StatusSha256:
    "4355a46b19d348dc2f57c046f8ef63d4538ebb936000f3c9ee954a27460dd865",
  kernelFocusedV20LogSha256:
    "389630423eb58c7de402fbc772c0643a604e7734162f1b7909e220b11e13144d",
  kernelFocusedV20StatusSha256:
    "9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa",
  kernelExactV20FailureLogSha256:
    "3ebf0b9e5d73758c114b14a48c66fbf0abf7b6febb9344ee5fd87047768b9c18",
  kernelExactV20ExitStatus: 1,
  authenticatedNewWindowJoinCandidate: "3e4f5d60db5b69a6335e08b1e44066b9f2a86cd2",
  authenticatedNewWindowJoinStatus:
    "dco-candidate-integrated-at-30af5c2-full-post-export-gates-green-independent-reviews-go",
  protectedDeploymentStatus:
    "open-one-shot-listener-integrated-no-supervisor-concrete-checker-signer-or-production-r33-backend",
  localPolicyCandidate: "829eb98ba11c645958ef762a01da4c81c4bf795b",
  localPolicyCandidateStatus:
    "local-unpublished-non-final-v5-qualified-exact-829-on-ce44-only-no-combined-or-m1-authority",
  localPolicyClosureSha256:
    "d52d2b2954fa103bf67f5a9fc72eebb4b315bbc0e1226424480b6412999d8714",
  localPolicyReceiptSha256:
    "2ab2b3e87a3d32e84d18b9fc1f840a05c6dac415a244ad19bf07e08f619e6641",
  localSelectorCandidate: "e18eb0e06473fabc6dbfe4cef80b692e34b04a22",
  localSelectorCandidateStatus: "local-unpublished-non-final-focused-checks-only",
  localPhysicalNewWindowProofCandidate: "a18de34ac7ffb39d8eaa632e1deb07b3dd1df274",
  localPhysicalNewWindowProofCandidateStatus:
    "local-unpublished-non-final-focused-proof-and-tests-only",
  localVariableCardinalityPlannerCandidate: "c3fe1f076e6ceded02633fa1e6c6973b5afb4d75",
  variableCardinalityPlannerStatus:
    "local-unpublished-non-final-focused-arithmetic-and-proof-gates-only-no-runtime-wiring-or-atomicity",
  localDirectDataIndexCandidate: "c72c2a8ffa47fcc1521ec7f1d2d02ae02f987046",
  localDirectDataIndexCandidateStatus: "local-unpublished-non-final-focused-checks-only",
  vectorGemmReductionCommit: "720c9bcf0e8522eb574f289c9e60a91c2f5e2be0",
  canonicalPrepackBundleIdentity:
    "6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b",
  canonicalPrepackAdmissionIdentity:
    "6a396e95e715d1be16bbc27b8c762a9308e40e5355c5bd89b9fc28fb06a1dd16",
  canonicalPrepackStatus: "real-qwen3-8b-and-06b-verified-non-execution",
  vectorGemmSsaWorkObserved: 94192887,
  vectorGemmSsaWorkLimit: 67108864,
  priorAggregateStorageObserved: 3188438,
  priorAggregateStorageLimit: 2097152,
  aggregateRerunStatus:
    "focused-v20-pass-exact-v20-stopped-at-distinct-cache-length-values-v21-active-no-hsaco",
  aggregateSourceCommit: "5514afe176a090aa3f1da9e5354799bb4ca5a8b3",
  aggregateProducerCommit: "e57c42523050922ad76538150df691cc5ab975a7",
  aggregateKernelCount: 12,
  diagnosticBridgeCommit: "24748e11358db7ad3ab5fe35992cff354896e607",
  diagnosticStatus: "partial-non-evidence",
  diagnosticDispatchGeneration: 1,
  diagnosticCopyCount: 5,
  proofQueries: 1493,
  directVerifiedBodies: 666,
  proofErrors: 0,
  proofPackages: 8,
  actualBodyHostileMutations: 37,
  sourceQualityPassMarkers: 13,
  sourceGateModules: 160,
  admittedUnverifiedBodies: 6807,
  sourceGateBodies: 7473,
  sourceClosureFiles: 603,
  openM1Gates: 33,
  openAssuranceProperties: 17,
});
const expectedProof = Object.freeze({
  source: "7f516e073b8759eb012c998bc9df2eb101d0c7ab",
  closureSha256:
    "f8c4a39eb4d81c61d95f7db50e380eb7b33c63c21375e693311c54cf4ee433f4",
  receiptSha256:
    "44a1710a26b2cb51889f536461d023dbc874b7bc274fb0feb4a1ded615ca4821",
  logSha256:
    "2335372df19fd103d387d8ca24a2ebaac73f177c1d0274e17544d683404cc7bd",
});

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function assertState(state, location) {
  assert(allowedStates.has(state), `${location} has unknown state ${state}`);
}

function assertCommit(commit, location) {
  assert(
    typeof commit === "string" && /^[0-9a-f]{7,40}$/.test(commit),
    `${location} must be a 7-40 character lowercase Git commit`,
  );
}

function assertExactKeys(value, expected, location) {
  const actualKeys = Reflect.ownKeys(value)
    .map((key) => (typeof key === "symbol" ? key.toString() : key))
    .sort();
  const expectedKeys = Object.keys(expected).sort();
  assert(
    JSON.stringify(actualKeys) === JSON.stringify(expectedKeys),
    `${location} keys must exactly match the reviewed schema`,
  );
}

const dataSource = await readFile(join(siteRoot, "data/project.js"), "utf8");
const context = { window: {} };
vm.runInNewContext(dataSource, context, { filename: "site/data/project.js" });
const project = context.window.FERRIC_PROJECT;

assert(project && typeof project === "object", "FERRIC_PROJECT must be defined");
assert(/^\d{4}-\d{2}-\d{2}$/.test(project.updated), "updated must use YYYY-MM-DD");
assert(
  /^https:\/\/github\.com\/[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(
    project.repository,
  ),
  "repository must be a GitHub repository URL",
);
assert(
  /^https:\/\/github\.com\/[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(
    project.fe2o3Repository,
  ),
  "fe2o3Repository must be a GitHub repository URL",
);
assert(project.current && typeof project.current === "object", "current status is missing");
assertExactKeys(project.current, expectedCurrent, "current");
assertCommit(project.current.siteRefreshBase, "current.siteRefreshBase");
assertCommit(project.current.integrationCommit, "current.integrationCommit");
assertCommit(project.current.integrationTree, "current.integrationTree");
assertCommit(project.current.ferricCheckpointCommit, "current.ferricCheckpointCommit");
assertCommit(project.current.ferricCheckpointTree, "current.ferricCheckpointTree");
assertCommit(
  project.current.fe2o3GuardedSubtractionCommit,
  "current.fe2o3GuardedSubtractionCommit",
);
assertCommit(project.current.implementationCommit, "current.implementationCommit");
assertCommit(project.current.authenticatedR32Commit, "current.authenticatedR32Commit");
assertCommit(project.current.aggregateSelectionCommit, "current.aggregateSelectionCommit");
assertCommit(
  project.current.pendingVerifierProjectionCommit,
  "current.pendingVerifierProjectionCommit",
);
assertCommit(
  project.current.commonCustodyPreflightCommit,
  "current.commonCustodyPreflightCommit",
);
assertCommit(project.current.associationPreflightCommit, "current.associationPreflightCommit");
assertCommit(
  project.current.finalizedHsacoReinspectionCommit,
  "current.finalizedHsacoReinspectionCommit",
);
assertCommit(project.current.fe2o3EngineeringSchemaCommit, "current.fe2o3EngineeringSchemaCommit");
assertCommit(project.current.fe2o3EngineeringSchemaTree, "current.fe2o3EngineeringSchemaTree");
assertCommit(project.current.fe2o3CompilerCandidate, "current.fe2o3CompilerCandidate");
assertCommit(project.current.fe2o3CompilerCandidateTree, "current.fe2o3CompilerCandidateTree");
assertCommit(
  project.current.fe2o3CompilerQualificationBase,
  "current.fe2o3CompilerQualificationBase",
);
assertCommit(project.current.fe2o3LatestMain, "current.fe2o3LatestMain");
assertCommit(
  project.current.fe2o3CheckedComponentProjectionCommit,
  "current.fe2o3CheckedComponentProjectionCommit",
);
assertCommit(
  project.current.fe2o3Llvm22WorkerSwitchCommit,
  "current.fe2o3Llvm22WorkerSwitchCommit",
);
assertCommit(
  project.current.fe2o3ConnectedPathTransportCommit,
  "current.fe2o3ConnectedPathTransportCommit",
);
assertCommit(
  project.current.fe2o3BoundedQueueWaitCommit,
  "current.fe2o3BoundedQueueWaitCommit",
);
assertCommit(
  project.current.productionSpeculativeExecutorCandidate,
  "current.productionSpeculativeExecutorCandidate",
);
assertCommit(
  project.current.productionSpeculativeExecutorTree,
  "current.productionSpeculativeExecutorTree",
);
assertCommit(
  project.current.productionSpeculativeExecutorIntegrationCommit,
  "current.productionSpeculativeExecutorIntegrationCommit",
);
assertCommit(
  project.current.engineeringAggregateLoaderCandidate,
  "current.engineeringAggregateLoaderCandidate",
);
assertCommit(
  project.current.engineeringAggregateLoaderTree,
  "current.engineeringAggregateLoaderTree",
);
assertCommit(
  project.current.engineeringAggregateLoaderIntegrationCommit,
  "current.engineeringAggregateLoaderIntegrationCommit",
);
assertCommit(project.current.targetEngineeringSmokeCandidate, "current.targetEngineeringSmokeCandidate");
assertCommit(project.current.targetEngineeringSmokeTree, "current.targetEngineeringSmokeTree");
assertCommit(
  project.current.targetEngineeringSmokeIntegrationCommit,
  "current.targetEngineeringSmokeIntegrationCommit",
);
assertCommit(
  project.current.servingComparisonR33V3IntegrationCommit,
  "current.servingComparisonR33V3IntegrationCommit",
);
assertCommit(
  project.current.protectedVerifierServiceLocalCandidate,
  "current.protectedVerifierServiceLocalCandidate",
);
assertCommit(
  project.current.verifierBinderCandidate,
  "current.verifierBinderCandidate",
);
assertCommit(
  project.current.verifierBinderCandidateTree,
  "current.verifierBinderCandidateTree",
);
assertCommit(
  project.current.verifierBinderIntegrationCommit,
  "current.verifierBinderIntegrationCommit",
);
assertCommit(
  project.current.authenticatedTargetRolloverCommit,
  "current.authenticatedTargetRolloverCommit",
);
assertCommit(
  project.current.authenticatedTargetServingBridgeCommit,
  "current.authenticatedTargetServingBridgeCommit",
);
assertCommit(project.current.swigluCheckedBlockCommit, "current.swigluCheckedBlockCommit");
assertCommit(
  project.current.healthyAllTerminalShutdownCommit,
  "current.healthyAllTerminalShutdownCommit",
);
assertCommit(project.current.daemonLifecycleProofCommit, "current.daemonLifecycleProofCommit");
assertCommit(project.current.daemonLifecycleProofTree, "current.daemonLifecycleProofTree");
assertCommit(
  project.current.supervisedR33WireCommit,
  "current.supervisedR33WireCommit",
);
assertCommit(
  project.current.supervisedR33WireTree,
  "current.supervisedR33WireTree",
);
assertCommit(project.current.localIntegrationCandidate, "current.localIntegrationCandidate");
assertCommit(
  project.current.authenticatedNewWindowJoinCandidate,
  "current.authenticatedNewWindowJoinCandidate",
);
assertCommit(
  project.current.localBoundedQueueWaitCandidate,
  "current.localBoundedQueueWaitCandidate",
);
assertCommit(
  project.current.catalogBindingIntegrationCommit,
  "current.catalogBindingIntegrationCommit",
);
assertCommit(
  project.current.descriptorOrderIntegrationCommit,
  "current.descriptorOrderIntegrationCommit",
);
assertCommit(
  project.current.boundedQueueWaitIntegrationCommit,
  "current.boundedQueueWaitIntegrationCommit",
);
assertCommit(
  project.current.combinedPolicyIntegrationCommit,
  "current.combinedPolicyIntegrationCommit",
);
assertCommit(
  project.current.connectedPathIntegrationCommit,
  "current.connectedPathIntegrationCommit",
);
assertCommit(
  project.current.protectedVerifierListenerIntegrationCommit,
  "current.protectedVerifierListenerIntegrationCommit",
);
assertCommit(
  project.current.boundedProductionQueueWaitIntegrationCommit,
  "current.boundedProductionQueueWaitIntegrationCommit",
);
assertCommit(project.current.localPolicyCandidate, "current.localPolicyCandidate");
assertCommit(project.current.localSelectorCandidate, "current.localSelectorCandidate");
assertCommit(
  project.current.localPhysicalNewWindowProofCandidate,
  "current.localPhysicalNewWindowProofCandidate",
);
assertCommit(
  project.current.localVariableCardinalityPlannerCandidate,
  "current.localVariableCardinalityPlannerCandidate",
);
assertCommit(
  project.current.localDirectDataIndexCandidate,
  "current.localDirectDataIndexCandidate",
);
assertCommit(project.current.vectorGemmReductionCommit, "current.vectorGemmReductionCommit");
assert(
  /^[0-9a-f]{64}$/.test(project.current.localPolicyClosureSha256) &&
    /^[0-9a-f]{64}$/.test(project.current.localPolicyReceiptSha256) &&
    /^[0-9a-f]{64}$/.test(project.current.kernelFocusedV19LogSha256) &&
    /^[0-9a-f]{64}$/.test(project.current.kernelExactV19FailureLogSha256) &&
    /^[0-9a-f]{64}$/.test(project.current.kernelExactV19StatusSha256) &&
    /^[0-9a-f]{64}$/.test(project.current.kernelFocusedV20LogSha256) &&
    /^[0-9a-f]{64}$/.test(project.current.kernelFocusedV20StatusSha256) &&
    /^[0-9a-f]{64}$/.test(project.current.kernelExactV20FailureLogSha256) &&
    project.current.kernelExactV20ExitStatus === 1,
  "local policy and focused-kernel identities must be lowercase SHA-256 digests",
);
assert(
  /^[0-9a-f]{64}$/.test(project.current.canonicalPrepackBundleIdentity) &&
    /^[0-9a-f]{64}$/.test(project.current.canonicalPrepackAdmissionIdentity),
  "canonical prepack identities must be lowercase SHA-256 digests",
);
assertCommit(project.current.aggregateSourceCommit, "current.aggregateSourceCommit");
assertCommit(project.current.aggregateProducerCommit, "current.aggregateProducerCommit");
assertCommit(project.current.diagnosticBridgeCommit, "current.diagnosticBridgeCommit");
for (const [key, expected] of Object.entries(expectedCurrent)) {
  const actual = project.current[key];
  assert(
    JSON.stringify(actual) === JSON.stringify(expected),
    `current.${key} must match the selected implementation status`,
  );
}
assert(
  project.current.fe2o3LatestMain === expectedCurrent.fe2o3BoundedQueueWaitCommit &&
    project.current.fe2o3CompilerQualificationBase ===
      expectedCurrent.fe2o3Llvm22WorkerSwitchCommit &&
    project.current.fe2o3LatestMain !== project.current.fe2o3CompilerQualificationBase &&
    project.current.fe2o3CompilerCandidateStatus ===
      "public-main-consumed-by-local-ferric-integration",
  "latest public fe2o3 main must be consumed locally and remain distinct from the older policy base",
);
assertState(project.milestone.state, "milestone");

const expectedEnvelopeTerms = [
  "Target",
  "Draft",
  "Device",
  "Precision",
  "Context",
  "Concurrency",
  "Pages refresh base",
  "Ferric implementation checkpoint",
  "fe2o3 public main",
  "Local Ferric integration",
  "Local validation",
  "Speculative executor",
  "Engineering aggregate loader",
  "Engineering aggregate output",
  "Canonical Qwen prepack",
  "Authenticated target rollover",
  "R33 wire and lifecycle proof",
  "Target-only engineering smoke",
  "R33 V3 serving comparison",
  "Baseline comparison",
  "Protected verifier status",
  "Aggregate device source",
  "Aggregate source-pin policy",
  "Aggregate build producer",
  "Authenticated R32 capture",
  "Aggregate selection candidate",
  "Pending verifier projection",
  "Aggregate verifier preflight",
  "Strict proof release",
  "Protected acceptance",
  "Historical protected artifact",
  "Current authority",
];
assert(
  Array.isArray(project.envelope) && project.envelope.length === expectedEnvelopeTerms.length,
  "envelope must contain exactly the reviewed rows",
);
project.envelope.forEach((entry, index) => {
  assert(
    Array.isArray(entry) &&
      entry.length === 2 &&
      entry.every((value) => typeof value === "string" && value.length > 0),
    `envelope[${index}] must be a non-empty term and definition pair`,
  );
});
const envelope = new Map(project.envelope);
assert(
  envelope.size === expectedEnvelopeTerms.length &&
    expectedEnvelopeTerms.every((term) => envelope.has(term)),
  "envelope terms must exactly match the reviewed rows",
);
assert(
  envelope.get("Ferric implementation checkpoint")?.includes(
    expectedCurrent.ferricCheckpointCommit,
  ) &&
    envelope.get("Ferric implementation checkpoint")?.includes(expectedCurrent.ferricCheckpointTree) &&
    envelope.get("Ferric implementation checkpoint")?.includes(
      expectedCurrent.ferricFeatureBranch,
    ) &&
    envelope.get("Ferric implementation checkpoint")?.includes(
      "not on Ferric main or final",
    ) &&
    envelope.get("Ferric implementation checkpoint")?.includes(
      "all 33 M1 gates remain open",
    ) &&
    envelope.get("fe2o3 public main")?.includes(expectedCurrent.fe2o3LatestMain) &&
    envelope.get("fe2o3 public main")?.includes(expectedCurrent.fe2o3CompilerCandidateTree) &&
    envelope.get("fe2o3 public main")?.includes(
      "compiler, runtime, KFD, and generic verification transport ownership remains in fe2o3",
    ) &&
    envelope.get("fe2o3 public main")?.includes(
      "model kernels, inference, and Ferric policy remain in Ferric",
    ),
  "envelope must expose the durable Ferric and fe2o3 checkpoints without current authority",
);
assert(
  envelope.get("Local Ferric integration")?.includes(expectedCurrent.localIntegrationCandidate) &&
    envelope.get("Local Ferric integration")?.includes(expectedCurrent.integrationTree) &&
    envelope
      .get("Local Ferric integration")
      ?.includes(expectedCurrent.catalogBindingIntegrationCommit.slice(0, 7)) &&
    envelope
      .get("Local Ferric integration")
      ?.includes(expectedCurrent.descriptorOrderIntegrationCommit.slice(0, 7)) &&
    envelope
      .get("Local Ferric integration")
      ?.includes(expectedCurrent.boundedQueueWaitIntegrationCommit.slice(0, 7)) &&
    envelope
      .get("Local Ferric integration")
      ?.includes(expectedCurrent.protectedVerifierListenerIntegrationCommit.slice(0, 7)) &&
    envelope
      .get("Local Ferric integration")
      ?.includes(expectedCurrent.boundedProductionQueueWaitIntegrationCommit.slice(0, 7)) &&
    envelope.get("Local Ferric integration")?.includes("local, unpublished, non-final") &&
    envelope.get("Local Ferric integration")?.includes("no current artifact, Qwen"),
  "envelope must identify the local combined Ferric head without public or runtime authority",
);
assert(
  envelope.get("Local validation")?.includes(expectedCurrent.localIntegrationCandidate.slice(0, 7)) &&
    envelope.get("Local validation")?.includes("576 library tests passed with 5 intentional ignores") &&
    envelope.get("Local validation")?.includes("158 doctests") &&
    envelope.get("Local validation")?.includes("all-target strict Clippy PASS") &&
    envelope.get("Local validation")?.includes("25 targeted tests") &&
    envelope.get("Local validation")?.includes("13 capture tests") &&
    envelope.get("Local validation")?.includes("28 hostile tests") &&
    envelope.get("Local validation")?.includes("6 source-policy tests") &&
    envelope.get("Local validation")?.includes("12 rollover tests") &&
    envelope.get("Local validation")?.includes("token, history, and API source reviews returned GO") &&
    envelope.get("Local validation")?.includes("11 harness tests") &&
    envelope.get("Local validation")?.includes("75 qualification tests with 2 intentional ignores") &&
    envelope.get("Local validation")?.includes("148 doctests") &&
    envelope.get("Local validation")?.includes("46 service tests with 3 intentional ignores") &&
    envelope.get("Local validation")?.includes("88 adapter tests") &&
    envelope.get("Local validation")?.includes("28 source-gate tests") &&
    envelope.get("Local validation")?.includes("failed closed on unsupported #[allow(clippy::too_many_lines)]") &&
    envelope.get("Local validation")?.includes("narrow closed-allowlist synchronization") &&
    envelope.get("Local validation")?.includes("source gate passes 28/28") &&
    envelope.get("Local validation")?.includes("6,951 discovered versus 6,854 admitted") &&
    envelope.get("Local validation")?.includes("97 unadmitted, and 0 stale") &&
    envelope.get("Local validation")?.includes("Runtime, Instant, and KFD bodies remain explicitly unverified") &&
    envelope.get("Local validation")?.includes("not a release qualification") &&
    envelope.get("Local validation")?.includes("closed M1 gate"),
  "envelope must bind exact combined validation evidence without release authority",
);
assert(
  envelope.get("Speculative executor")?.includes(
    expectedCurrent.productionSpeculativeExecutorCandidate,
  ) &&
    envelope.get("Speculative executor")?.includes(
      expectedCurrent.productionSpeculativeExecutorTree,
    ) &&
    envelope.get("Speculative executor")?.includes(
      expectedCurrent.productionSpeculativeExecutorIntegrationCommit,
    ) &&
    envelope.get("Speculative executor")?.includes("independent review GO"),
  "envelope must expose the exact integrated speculative executor and independent GO",
);
assert(
  envelope.get("Engineering aggregate loader")?.includes(
    expectedCurrent.engineeringAggregateLoaderCandidate,
  ) &&
    envelope.get("Engineering aggregate loader")?.includes(
      expectedCurrent.engineeringAggregateLoaderTree,
    ) &&
    envelope.get("Engineering aggregate loader")?.includes("independent review GO") &&
    envelope.get("Engineering aggregate loader")?.includes("non-authoritative"),
  "envelope must expose the exact engineering loader, independent GO, and nonauthority",
);
assert(
  envelope.get("Engineering aggregate output")?.includes("public fe2o3 5f46dd2") &&
    envelope.get("Engineering aggregate output")?.includes("Focused kernel v20 is fully green") &&
    envelope.get("Engineering aggregate output")?.includes("prefill profiles 4/4") &&
    envelope.get("Engineering aggregate output")?.includes("paged-decode profiles 4/4") &&
    envelope.get("Engineering aggregate output")?.includes(expectedCurrent.kernelFocusedV20LogSha256) &&
    envelope.get("Engineering aggregate output")?.includes(expectedCurrent.kernelFocusedV20StatusSha256) &&
    envelope.get("Engineering aggregate output")?.includes("All four frozen SHAs remain intact") &&
    envelope.get("Engineering aggregate output")?.includes("Exact v20 produced no HSACO") &&
    envelope.get("Engineering aggregate output")?.includes("two k.len() evaluations lowered to distinct semantic values") &&
    envelope.get("Engineering aggregate output")?.includes(expectedCurrent.kernelExactV20FailureLogSha256) &&
    envelope.get("Engineering aggregate output")?.includes("exit status 1") &&
    envelope.get("Engineering aggregate output")?.includes("active v21 change") &&
    envelope.get("Engineering aggregate output")?.includes("immutable cache_len = k.len() once") &&
    envelope.get("Engineering aggregate output")?.includes("external proof-shape edits were preserved") &&
    envelope.get("Engineering aggregate output")?.includes("neither fact implies correctness") &&
    envelope.get("Engineering aggregate output")?.includes("kernel work remains in progress") &&
    envelope.get("Engineering aggregate output")?.includes("No aggregate Worker V3 HSACO") &&
    envelope.get("Engineering aggregate output")?.includes("Qwen serving") &&
    envelope.get("Engineering aggregate output")?.includes(
      "artifact authority, or execution authority",
    ),
  "envelope must expose the current aggregate compile boundary and execution limits",
);
assert(
  envelope.get("Aggregate device source")?.includes("12 Ferric-local Rust M1 kernel roots") &&
    envelope.get("Aggregate device source")?.includes("seven canonical source modules") &&
    envelope.get("Aggregate device source")?.includes("no prebuilt or vendor kernel dependency"),
  "envelope must expose the local aggregate source roster and dependency boundary",
);
assert(
  envelope.get("Aggregate build producer")?.includes(
    expectedCurrent.aggregateBuildConfigFormat,
  ) &&
    envelope.get("Aggregate build producer")?.includes(
      expectedCurrent.aggregateBuildObservation,
    ),
  "aggregate build producer must require production config V2 observation",
);
assert(
  envelope.get("Target-only engineering smoke")?.includes("gfx942 binding") &&
  envelope.get("Target-only engineering smoke")?.includes("physical KV partitioning") &&
    envelope.get("Target-only engineering smoke")?.includes("545-packet target batch") &&
    envelope.get("Target-only engineering smoke")?.includes("monotonic-raw single-request timing") &&
    envelope.get("Target-only engineering smoke")?.includes(
      "not a continuous-serving or HTTP endpoint",
    ),
  "envelope must expose the physical target runner without claiming execution or timing",
);
assert(
  envelope.get("Canonical Qwen prepack")?.includes(
    expectedCurrent.canonicalPrepackBundleIdentity,
  ) &&
    envelope.get("Canonical Qwen prepack")?.includes(
      expectedCurrent.canonicalPrepackAdmissionIdentity,
    ) &&
    envelope.get("Canonical Qwen prepack")?.includes("validates input packaging") &&
    envelope.get("Canonical Qwen prepack")?.includes("not a protected artifact, GPU run, token"),
  "envelope must bind the canonical prepack identities without execution authority",
);
assert(
  envelope.get("Authenticated target rollover")?.includes(
    expectedCurrent.authenticatedTargetRolloverCommit,
  ) &&
    envelope.get("Authenticated target rollover")?.includes(
      expectedCurrent.localIntegrationCandidate,
    ) &&
    envelope.get("Authenticated target rollover")?.includes(
      expectedCurrent.authenticatedNewWindowJoinCandidate,
    ) &&
    envelope
      .get("Authenticated target rollover")
      ?.includes(expectedCurrent.protectedVerifierListenerIntegrationCommit.slice(0, 7)) &&
    envelope
      .get("Authenticated target rollover")
      ?.includes(expectedCurrent.boundedProductionQueueWaitIntegrationCommit.slice(0, 7)) &&
    envelope.get("Authenticated target rollover")?.includes("exact-slot reincarnation") &&
    envelope.get("Authenticated target rollover")?.includes("integrated locally") &&
    envelope.get("Authenticated target rollover")?.includes("exact all-terminal speculative execution") &&
    envelope.get("Authenticated target rollover")?.includes("private move-only whole-page-set token") &&
    envelope.get("Authenticated target rollover")?.includes("archives predecessor physical history inertly") &&
    envelope.get("Authenticated target rollover")?.includes("resetting active history") &&
    envelope.get("Authenticated target rollover")?.includes("Published->Observed->Released") &&
    envelope.get("Authenticated target rollover")?.includes("576 passed with 5 ignored") &&
    envelope.get("Authenticated target rollover")?.includes("158 doctests") &&
    envelope.get("Authenticated target rollover")?.includes("targeted 25/25") &&
    envelope.get("Authenticated target rollover")?.includes("capture 13/13") &&
    envelope.get("Authenticated target rollover")?.includes("hostile 28/28") &&
    envelope.get("Authenticated target rollover")?.includes("source policy 6/6") &&
    envelope.get("Authenticated target rollover")?.includes("rollover 12/12") &&
    envelope.get("Authenticated target rollover")?.includes("token, history, and API source reviews returned GO") &&
    envelope.get("Authenticated target rollover")?.includes("local integration is unpublished, non-final") &&
    envelope.get("Authenticated target rollover")?.includes("release-qualified") &&
    envelope.get("Authenticated target rollover")?.includes("evidence of Qwen"),
  "envelope must distinguish public rollover from the unpublished local engine candidate",
);
assert(
  envelope.get("R33 wire and lifecycle proof")?.includes(
    expectedCurrent.daemonLifecycleProofCommit,
  ) &&
    envelope.get("R33 wire and lifecycle proof")?.includes(
      expectedCurrent.supervisedR33WireCommit,
    ) &&
    envelope.get("R33 wire and lifecycle proof")?.includes("Four exact positive rows") &&
    envelope.get("R33 wire and lifecycle proof")?.includes("two hostile actual-body mutations") &&
    envelope.get("R33 wire and lifecycle proof")?.includes("independent of the service implementation") &&
    envelope.get("R33 wire and lifecycle proof")?.includes("no adapter conformance") &&
    envelope.get("R33 wire and lifecycle proof")?.includes("protected authority"),
  "envelope must bind integrated wire and lifecycle proof without service or runtime authority",
);
assert(
  envelope.get("R33 V3 serving comparison")?.includes(
    expectedCurrent.servingComparisonR33V3IntegrationCommit,
  ) &&
    envelope.get("R33 V3 serving comparison")?.includes("paired per-request") &&
    envelope.get("R33 V3 serving comparison")?.includes("p50/p90/p99") &&
    envelope.get("R33 V3 serving comparison")?.includes("TTFT") &&
    envelope.get("R33 V3 serving comparison")?.includes("TPOT") &&
    envelope.get("R33 V3 serving comparison")?.includes("no Qwen measurement") &&
    envelope.get("R33 V3 serving comparison")?.includes("vLLM/SGLang baseline"),
  "envelope must retain the event-backed, unrun R33 V3 comparison surface",
);
assert(
  envelope.get("Baseline comparison")?.includes("No Ferric Qwen timing") &&
    envelope.get("Baseline comparison")?.includes("comparison not run"),
  "envelope must avoid asserting stale baseline infrastructure availability",
);
assert(
  envelope.get("Protected verifier status")?.includes(
    expectedCurrent.protectedVerifierListenerIntegrationCommit.slice(0, 7),
  ) &&
    envelope.get("Protected verifier status")?.includes("one-shot protected-verifier listener") &&
    envelope.get("Protected verifier status")?.includes("production R33 backend remain absent") &&
    envelope.get("Protected verifier status")?.includes("does not create a deployed verifier"),
  "envelope must expose the missing protected deployment inputs",
);
assert(
  envelope.get("Ferric implementation checkpoint")?.includes(
    expectedCurrent.ferricCheckpointCommit,
  ) &&
    envelope.get("Ferric implementation checkpoint")?.includes(expectedCurrent.ferricCheckpointTree),
  "envelope must expose the exact durable Ferric checkpoint and tree",
);
assert(
  envelope.get("Aggregate verifier preflight")?.includes(
    "sole terminal result MissingProtectedVerificationReceipt",
  ),
  "envelope must expose the reject-only aggregate verifier preflight",
);
assert(Array.isArray(project.readiness) && project.readiness.length > 0, "readiness is empty");
project.readiness.forEach((item, index) =>
  assertState(item.state, `readiness[${index}]`),
);
const r32Readiness = project.readiness.find(
  (item) => item.label === "Authenticated R32 first-publication capture vertical",
);
assert(r32Readiness?.state === "integration", "R32 vertical must remain in integration");
assert(
  r32Readiness.detail.includes("partial-non-evidence") &&
    r32Readiness.detail.includes("cannot pass its protected-verifier boundary today"),
  "R32 readiness must retain its fail-closed partial nonclaim",
);
const selectionReadiness = project.readiness.find(
  (item) => item.label === "Aggregate publication-selection candidate",
);
assert(
  selectionReadiness?.state === "integration" &&
    selectionReadiness.detail.includes("explicitly noncurrent"),
  "aggregate selection candidate must remain noncurrent integration",
);
const projectionReadiness = project.readiness.find(
  (item) => item.label === "Aggregate pending-verifier projection",
);
assert(
  projectionReadiness?.state === "integration" &&
    projectionReadiness.detail.includes("private, reject-only projection") &&
    projectionReadiness.detail.includes("remain Option") &&
    projectionReadiness.detail.includes("cannot leave the rejection path"),
  "aggregate pending-verifier projection must remain private, optional, and reject-only",
);
const verifierPreflightReadiness = project.readiness.find(
  (item) => item.label === "Reject-only aggregate verifier preflight",
);
assert(
  verifierPreflightReadiness?.state === "integration" &&
    verifierPreflightReadiness.detail.includes(
      "call the pinned finalized-HSACO verifier exactly once on the request bytes",
    ) &&
    verifierPreflightReadiness.detail.includes(
      "validate common multi-root compiler proof inputs",
    ) &&
    verifierPreflightReadiness.detail.includes(
      "unique 12-entry export-plus-descriptor-symbol permutation",
    ) &&
    verifierPreflightReadiness.detail.includes("same-process descriptive integrity") &&
    verifierPreflightReadiness.detail.includes("not independent verifier authority") &&
    verifierPreflightReadiness.detail.includes("MissingProtectedVerificationReceipt") &&
    verifierPreflightReadiness.detail.includes("grants no protected, load, launch, hardware, Qwen"),
  "aggregate verifier preflight must preserve reinspection, associations, rejection, and nonclaims",
);
const protectedAcceptance = project.readiness.find(
  (item) => item.label === "Accepting protected aggregate artifact",
);
const listenerReadiness = project.readiness.find(
  (item) => item.label === "Protected one-shot verifier listener",
);
assert(
  listenerReadiness?.state === "integration" &&
    listenerReadiness.detail.includes(expectedCurrent.protectedVerifierListenerIntegrationCommit.slice(0, 7)) &&
    listenerReadiness.detail.includes("46 service tests with 3 intentional ignores") &&
    listenerReadiness.detail.includes("88 adapter tests") &&
    listenerReadiness.detail.includes("28 source-gate tests") &&
    listenerReadiness.detail.includes("strict Clippy") &&
    listenerReadiness.detail.includes("production R33 backend") &&
    listenerReadiness.detail.includes("source integration only"),
  "protected listener readiness must retain exact gates and deployment nonclaims",
);
assert(
  protectedAcceptance?.state === "open" &&
    protectedAcceptance.detail.includes("remains None") &&
    protectedAcceptance.detail.includes("protected one-shot listener exists in local source") &&
    protectedAcceptance.detail.includes("concrete checker, signer, supervisor") &&
    protectedAcceptance.detail.includes("production R33 backend") &&
    protectedAcceptance.detail.includes("absent") &&
    protectedAcceptance.detail.includes("No independent verifier") &&
    protectedAcceptance.detail.includes("runtime authority exists"),
  "protected aggregate acceptance must remain fail-closed and open",
);
const producerReadiness = project.readiness.find(
  (item) => item.label === "fe2o3 engineering aggregate producer",
);
assert(
    producerReadiness?.state === "integration" &&
    producerReadiness.detail.includes(expectedCurrent.fe2o3CompilerCandidate) &&
    producerReadiness.detail.includes(expectedCurrent.fe2o3CompilerCandidateTree) &&
    producerReadiness.detail.includes(expectedCurrent.localIntegrationCandidate.slice(0, 7)) &&
    producerReadiness.detail.includes("pins that exact revision") &&
    producerReadiness.detail.includes("unpublished, non-final") &&
    producerReadiness.detail.includes("without release qualification") &&
    producerReadiness.detail.includes("generic verification transport only") &&
    producerReadiness.detail.includes("Ferric owns model kernels, inference, and Ferric policy") &&
    producerReadiness.detail.includes(
      "No Ferric artifact, runtime result, inference result, or correctness authority",
    ),
  "fe2o3 producer must retain the current ownership, proof status, and downstream nonclaims",
);
const transportReadiness = project.readiness.find(
  (item) => item.label === "Ferric connected-path and bounded queue adoption",
);
assert(
  transportReadiness?.state === "integration" &&
    transportReadiness.detail.includes(
      expectedCurrent.fe2o3ConnectedPathTransportCommit.slice(0, 7),
    ) &&
    transportReadiness.detail.includes(expectedCurrent.fe2o3BoundedQueueWaitCommit.slice(0, 7)) &&
    transportReadiness.detail.includes(expectedCurrent.localIntegrationCandidate.slice(0, 7)) &&
    transportReadiness.detail.includes(
      expectedCurrent.protectedVerifierListenerIntegrationCommit.slice(0, 7),
    ) &&
    transportReadiness.detail.includes(
      expectedCurrent.boundedProductionQueueWaitIntegrationCommit.slice(0, 7),
    ) &&
    transportReadiness.detail.includes("Deployment remains in progress") &&
    transportReadiness.detail.includes("caller-pinned admission") &&
    transportReadiness.detail.includes("no supervisor, concrete checker, signer") &&
    transportReadiness.detail.includes("production R33 backend") &&
    transportReadiness.detail.includes("Qwen execution"),
  "Ferric transport adoption must remain in progress without Qwen authority",
);
const boundedWaitReadiness = project.readiness.find(
  (item) => item.label === "Bounded production queue waits",
);
assert(
  boundedWaitReadiness?.state === "integration" &&
    boundedWaitReadiness.detail.includes(
      expectedCurrent.boundedProductionQueueWaitIntegrationCommit.slice(0, 7),
    ) &&
    boundedWaitReadiness.detail.includes("569 engine library tests") &&
    boundedWaitReadiness.detail.includes("11 harness tests") &&
    boundedWaitReadiness.detail.includes("75 qualification tests with 2 intentional ignores") &&
    boundedWaitReadiness.detail.includes("148 doctests") &&
    boundedWaitReadiness.detail.includes("strict Clippy") &&
    boundedWaitReadiness.detail.includes("not a GPU, Qwen, serving, or performance result"),
  "bounded production waits must retain exact gates and execution nonclaims",
);
const plannerReadiness = project.readiness.find(
  (item) => item.label === "Variable-cardinality planner",
);
assert(
  plannerReadiness?.state === "integration" &&
    plannerReadiness.detail.includes(expectedCurrent.localVariableCardinalityPlannerCandidate) &&
    plannerReadiness.detail.includes("Local unpublished, non-final") &&
    plannerReadiness.detail.includes("5/5 Rust arithmetic tests") &&
    plannerReadiness.detail.includes("1 function / 33 details / 0 errors") &&
    plannerReadiness.detail.includes("three rejected hostile formula mutations") &&
    plannerReadiness.detail.includes("162 modules / 7,526 bodies") &&
    plannerReadiness.detail.includes(expectedCurrent.localDirectDataIndexCandidate) &&
    plannerReadiness.detail.includes("also local, unpublished, and non-final") &&
    plannerReadiness.detail.includes("Neither implements runtime variable-cardinality wiring") &&
    plannerReadiness.detail.includes("proves transition atomicity"),
  "planner readiness must retain focused evidence and runtime/atomicity nonclaims",
);
const executorReadiness = project.readiness.find(
  (item) => item.label === "Production speculative executor",
);
assert(
  executorReadiness?.state === "integration" &&
    executorReadiness.detail.includes(expectedCurrent.productionSpeculativeExecutorCandidate) &&
    executorReadiness.detail.includes(expectedCurrent.productionSpeculativeExecutorTree) &&
    executorReadiness.detail.includes(expectedCurrent.productionSpeculativeExecutorIntegrationCommit) &&
    executorReadiness.detail.includes("received independent review GO") &&
    executorReadiness.detail.includes("not an aggregate artifact, GPU execution, Qwen token"),
  "speculative executor must retain exact integration, independent GO, and evidence limits",
);
const engineeringLoaderReadiness = project.readiness.find(
  (item) => item.label === "Non-authoritative engineering aggregate loader",
);
assert(
  engineeringLoaderReadiness?.state === "integration" &&
    engineeringLoaderReadiness.detail.includes(expectedCurrent.engineeringAggregateLoaderCandidate) &&
    engineeringLoaderReadiness.detail.includes(expectedCurrent.engineeringAggregateLoaderTree) &&
    engineeringLoaderReadiness.detail.includes(expectedCurrent.engineeringAggregateLoaderIntegrationCommit) &&
    engineeringLoaderReadiness.detail.includes("independent review GO") &&
    engineeringLoaderReadiness.detail.includes("internal move-only program-source capability") &&
    engineeringLoaderReadiness.detail.includes("cannot create Worker V3") &&
    engineeringLoaderReadiness.detail.includes("no production aggregate HSACO"),
  "engineering loader must retain exact integration, nonauthority, and unused-output status",
);
const targetSmokeReadiness = project.readiness.find(
  (item) => item.label === "Non-authoritative target-only engineering smoke",
);
assert(
  targetSmokeReadiness?.state === "integration" &&
    targetSmokeReadiness.detail.includes(
      expectedCurrent.targetEngineeringSmokeIntegrationCommit.slice(0, 7),
    ) &&
    targetSmokeReadiness.detail.includes("monotonic-raw controller timing") &&
    targetSmokeReadiness.detail.includes("truthful request event") &&
    targetSmokeReadiness.detail.includes("excludes artifact, model-memory, and tokenizer setup") &&
    targetSmokeReadiness.detail.includes("has not run with a current aggregate artifact") &&
    targetSmokeReadiness.detail.includes("not continuous serving") &&
    targetSmokeReadiness.detail.includes("an HTTP endpoint") &&
    targetSmokeReadiness.detail.includes("not") &&
    targetSmokeReadiness.detail.includes("M1 evidence"),
  "target smoke must retain event timing and explicit no-serving limits",
);
const binderReadiness = project.readiness.find(
  (item) => item.label === "Protected verifier binder",
);
assert(
  binderReadiness?.state === "qualified" &&
    binderReadiness.detail.includes(expectedCurrent.verifierBinderCandidate) &&
    binderReadiness.detail.includes(expectedCurrent.verifierBinderCandidateTree) &&
    binderReadiness.detail.includes(expectedCurrent.verifierBinderIntegrationCommit) &&
    binderReadiness.detail.includes("ahead of reservation and one-shot FD consumption") &&
    binderReadiness.detail.includes("single absolute-deadline API") &&
    binderReadiness.detail.includes("exact-archive mi300x matrix passed") &&
    binderReadiness.detail.includes("GO with no P0, P1, or P2 findings") &&
    binderReadiness.detail.includes("not public main or deployed authority"),
  "binder candidate must retain exact qualification, independent GO, and deployment limits",
);
const qwenReadiness = project.readiness.find(
  (item) => item.label === "End-to-end Qwen through Ferric",
);
assert(
    qwenReadiness?.state === "open" &&
    qwenReadiness.detail.includes("CURRENT=None") &&
    qwenReadiness.detail.includes("Real target and draft prepack inputs are verified") &&
    qwenReadiness.detail.includes(expectedCurrent.localIntegrationCandidate.slice(0, 7)) &&
    qwenReadiness.detail.includes("public fe2o3 5f46dd2") &&
    qwenReadiness.detail.includes("Focused kernel v20 is fully green") &&
    qwenReadiness.detail.includes("exact v20 produced no HSACO") &&
    qwenReadiness.detail.includes("v21's single-SSA-local fix is active") &&
    qwenReadiness.detail.includes("production R33 backend") &&
    qwenReadiness.detail.includes("authenticated new-window join integrated") &&
    qwenReadiness.detail.includes("narrow pure theorem work") &&
    qwenReadiness.detail.includes("release qualification remain in progress") &&
    qwenReadiness.detail.includes("No aggregate HSACO") &&
    qwenReadiness.detail.includes("Qwen token") &&
    qwenReadiness.detail.includes("serving or HTTP endpoint") &&
    qwenReadiness.detail.includes("measured TTFT/TPOT"),
  "Qwen, numerical, and performance authority must remain open",
);
const baselineReadiness = project.readiness.find(
  (item) => item.label === "vLLM and SGLang baseline comparison",
);
assert(
  baselineReadiness?.state === "open" &&
    baselineReadiness.detail.includes(expectedCurrent.servingComparisonR33V3IntegrationCommit.slice(0, 7)) &&
    baselineReadiness.detail.includes("R33 V3 event-backed comparison support") &&
    baselineReadiness.detail.includes("paired per-request") &&
    baselineReadiness.detail.includes("p50/p90/p99") &&
    baselineReadiness.detail.includes("exact nanosecond units") &&
    baselineReadiness.detail.includes("No Qwen measurement") &&
    baselineReadiness.detail.includes("baseline server run") &&
    baselineReadiness.detail.includes("vLLM/SGLang timing") &&
    baselineReadiness.detail.includes("Ferric comparison result exists"),
  "baseline comparison must remain open with exact environment limits",
);
const prepackProbe = project.readiness.find(
  (item) => item.label === "Canonical Qwen prepack probe",
);
assert(
  prepackProbe?.state === "observed" &&
    prepackProbe.detail.includes("A real mi300x run") &&
    prepackProbe.detail.includes(
      "6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b",
    ) &&
    prepackProbe.detail.includes(
      "6a396e95e715d1be16bbc27b8c762a9308e40e5355c5bd89b9fc28fb06a1dd16",
    ) &&
    prepackProbe.detail.includes("verifies input packaging only") &&
    prepackProbe.detail.includes("not a protected aggregate artifact") &&
    prepackProbe.detail.includes("GPU inference run") &&
    prepackProbe.detail.includes("generated token") &&
    prepackProbe.detail.includes("serving result"),
  "canonical Qwen prepack must remain explicitly packaging-only",
);

for (const group of ["runnable", "experimental", "roadmap"]) {
  assert(
    Array.isArray(project.capabilities[group]) && project.capabilities[group].length > 0,
    `capabilities.${group} is empty`,
  );
}
const r33CaptureCapability = project.capabilities.experimental.find(
  (item) => item.name === "R33 V3 serving comparison capture",
);
assert(
  r33CaptureCapability?.detail.includes("Public feature-branch checkpoint a2bb2dc") &&
    r33CaptureCapability.detail.includes("not on main or final") &&
    r33CaptureCapability.detail.includes("exact per-request arrival, first-token, terminal") &&
    r33CaptureCapability.detail.includes("paired work across Ferric, vLLM, and SGLang") &&
    r33CaptureCapability.detail.includes("E2E, TTFT, and TPOT percentiles") &&
    r33CaptureCapability.detail.includes("has not run against Qwen or any baseline") &&
    r33CaptureCapability.detail.includes("no performance or M1 result"),
  "R33 V3 capability must retain event-backed metrics and no-run claims",
);
const proofLedgerCapability = project.capabilities.experimental.find(
  (item) => item.name === "Qualified proof ledger",
);
assert(
  proofLedgerCapability?.detail.includes("checkpoint 9431cb4") &&
    proofLedgerCapability.detail.includes("160 modules and 7,473 executable bodies") &&
    proofLedgerCapability.detail.includes("666 directly verified") &&
    proofLedgerCapability.detail.includes("6,807 admitted unverified") &&
    proofLedgerCapability.detail.includes("first published at 756e813") &&
    proofLedgerCapability.detail.includes("canonical weight-role proof") &&
    proofLedgerCapability.detail.includes("negative mutation evidence") &&
    proofLedgerCapability.detail.includes(expectedCurrent.daemonLifecycleProofCommit.slice(0, 7)) &&
    proofLedgerCapability.detail.includes("independent, authority-free") &&
    proofLedgerCapability.detail.includes("Four exact positive rows") &&
    proofLedgerCapability.detail.includes("out-of-order measure") &&
    proofLedgerCapability.detail.includes("abandoned response incorrectly advancing") &&
    proofLedgerCapability.detail.includes("fresh full release receipt"),
  "proof ledger must bind the current inventory and integrated lifecycle evidence",
);

assertCommit(project.latestObservation.commit, "latestObservation.commit");
assertState(project.latestObservation.state, "latestObservation");
assert(
  project.latestObservation.generatedTokenIds.every(Number.isInteger),
  "latestObservation.generatedTokenIds must contain integers",
);

for (const key of ["host", "proof", "hardware"]) {
  const validation = project.validation[key];
  assert(validation && typeof validation === "object", `validation.${key} is missing`);
  assertState(validation.state, `validation.${key}`);
  if (validation.source !== null) {
    assertCommit(validation.source, `validation.${key}.source`);
  }
  for (const digestKey of ["closureSha256", "receiptSha256", "logSha256"]) {
    if (validation[digestKey] !== undefined) {
      assert(
        /^[0-9a-f]{64}$/.test(validation[digestKey]),
        `validation.${key}.${digestKey} must be a lowercase SHA-256 digest`,
      );
    }
  }
}
assert(
  project.validation.host.source === expectedCurrent.authenticatedR32Commit,
  "host validation must bind the authenticated R32 implementation commit",
);
assert(
  project.validation.proof.source === expectedCurrent.implementationCommit,
  "proof validation must bind the exact qualified integration commit",
);
for (const [key, expected] of Object.entries(expectedProof)) {
  assert(
    project.validation.proof[key] === expected,
    `validation.proof.${key} must match the exact retained qualification`,
  );
}
assert(
  project.validation.proof.detail.includes("33615415798") &&
    project.validation.proof.detail.includes("33615415693") &&
    project.validation.proof.detail.includes("both completed successfully"),
  "proof validation must expose both successful exact-head workflow runs",
);
assert(
  project.validation.host.detail.includes("No successful R32 hardware trace") &&
    project.validation.host.detail.includes("m1.r32") &&
    project.validation.host.detail.includes("M1"),
  "host validation must deny hardware, m1.r32, and M1 closure",
);
assert(
  project.validation.hardware.state === "observed" &&
    project.validation.hardware.source !== expectedCurrent.implementationCommit,
  "historical hardware observation must not be presented as current integration evidence",
);
assert(
  project.validation.proof.state !== "qualified" ||
    typeof project.validation.proof.closureSha256 === "string",
  "qualified proof validation must bind a source closure digest",
);

assert(
  Array.isArray(project.validation.transitions) &&
    project.validation.transitions.length > 0,
  "validation.transitions is empty",
);
const transitionKeys = new Set();
project.validation.transitions.forEach(([prior, next, state], index) => {
  assert(prior && next, `validation.transitions[${index}] has an empty plan`);
  assertState(state, `validation.transitions[${index}]`);
  const key = `${prior}\u0000${next}`;
  assert(!transitionKeys.has(key), `duplicate transition ${prior} -> ${next}`);
  transitionKeys.add(key);
});

assert(Array.isArray(project.teams) && project.teams.length === 4, "teams must contain four rows");
const expectedTeams = new Set(["Integration", "Kernel", "Engine", "Verification"]);
project.teams.forEach((team, index) => {
  assert(expectedTeams.delete(team.name), `teams[${index}] has an unexpected or duplicate name`);
  assertState(team.state, `teams[${index}]`);
  for (const key of ["scope", "status", "completed", "current", "blockedBy", "next", "validation"]) {
    assert(
      typeof team[key] === "string" && team[key].length > 0,
      `teams[${index}].${key} must be a non-empty string`,
    );
  }
  assert(
    new Set(["Making progress", "Blocked"]).has(team.status),
    `teams[${index}] must report progress or an explicit blocker`,
  );
});
assert(expectedTeams.size === 0, "teams must cover Integration, Kernel, Engine, and Verification");
const integrationTeam = project.teams.find((team) => team.name === "Integration");
const kernelTeam = project.teams.find((team) => team.name === "Kernel");
const engineTeam = project.teams.find((team) => team.name === "Engine");
const verificationTeam = project.teams.find((team) => team.name === "Verification");
assert(
  integrationTeam.completed.includes(expectedCurrent.fe2o3LatestMain.slice(0, 7)) &&
    integrationTeam.completed.includes(
      expectedCurrent.catalogBindingIntegrationCommit.slice(0, 7),
    ) &&
    integrationTeam.completed.includes(
      expectedCurrent.descriptorOrderIntegrationCommit.slice(0, 7),
    ) &&
    integrationTeam.completed.includes(
      expectedCurrent.protectedVerifierListenerIntegrationCommit.slice(0, 7),
    ) &&
    integrationTeam.completed.includes(
      expectedCurrent.boundedProductionQueueWaitIntegrationCommit.slice(0, 7),
    ) &&
    integrationTeam.current.includes(expectedCurrent.localIntegrationCandidate.slice(0, 7)) &&
    integrationTeam.current.includes("local, unpublished/non-final") &&
    integrationTeam.current.includes("46 service tests with 3 ignores") &&
    integrationTeam.current.includes("88 adapter tests") &&
    integrationTeam.current.includes("576 library tests with 5 ignores") &&
    integrationTeam.current.includes("158 doctests") &&
    integrationTeam.current.includes("targeted 25/25") &&
    integrationTeam.current.includes("hostile 28/28") &&
    integrationTeam.current.includes("No release qualification") &&
    integrationTeam.validation.includes("30af5c2 local, unpublished, non-final") &&
    integrationTeam.blockedBy.includes("No active team is idle or blocked") &&
    integrationTeam.blockedBy.includes("production R33 backend"),
  "integration team must expose the exact latest-fe2 and bounded-wait candidates",
);
assert(
  kernelTeam.status === "Making progress" &&
    kernelTeam.completed.includes("12 K1-K7 entrypoints") &&
    kernelTeam.completed.includes("440 exact profiles") &&
    kernelTeam.completed.includes(expectedCurrent.vectorGemmReductionCommit.slice(0, 7)) &&
    kernelTeam.completed.includes(expectedCurrent.swigluCheckedBlockCommit.slice(0, 7)) &&
    kernelTeam.completed.includes(expectedCurrent.fe2o3CheckedComponentProjectionCommit.slice(0, 7)) &&
    kernelTeam.current.includes(expectedCurrent.localIntegrationCandidate.slice(0, 7)) &&
    kernelTeam.current.includes("focused v20 is fully green") &&
    kernelTeam.current.includes("prefill 4/4 profiles") &&
    kernelTeam.current.includes("paged-decode 4/4 profiles") &&
    kernelTeam.current.includes(expectedCurrent.kernelFocusedV20LogSha256) &&
    kernelTeam.current.includes(expectedCurrent.kernelFocusedV20StatusSha256) &&
    kernelTeam.current.includes("All four frozen SHAs remain intact") &&
    kernelTeam.current.includes("Exact v20 produced no HSACO") &&
    kernelTeam.current.includes("distinct semantic values") &&
    kernelTeam.current.includes(expectedCurrent.kernelExactV20FailureLogSha256) &&
    kernelTeam.current.includes("exit status 1") &&
    kernelTeam.current.includes("V21 binds immutable cache_len once") &&
    kernelTeam.current.includes("external proof-shape edits were preserved") &&
    kernelTeam.current.includes("without implying correctness") &&
    kernelTeam.current.includes("Kernel implementation remains in progress") &&
    kernelTeam.current.includes("no aggregate HSACO") &&
    kernelTeam.blockedBy.includes("kernel team is making progress") &&
    kernelTeam.next.includes("without weakening authorization"),
  "kernel team must expose roster, artifact dependency, and ownership",
);
assert(
  engineTeam.completed.includes(expectedCurrent.authenticatedTargetRolloverCommit.slice(0, 7)) &&
    engineTeam.completed.includes(expectedCurrent.healthyAllTerminalShutdownCommit.slice(0, 7)) &&
    engineTeam.completed.includes(
      expectedCurrent.protectedVerifierListenerIntegrationCommit.slice(0, 7),
    ) &&
    engineTeam.completed.includes(
      expectedCurrent.boundedProductionQueueWaitIntegrationCommit.slice(0, 7),
    ) &&
    engineTeam.current.includes(expectedCurrent.localIntegrationCandidate.slice(0, 7)) &&
    engineTeam.current.includes("Local unpublished/non-final") &&
    engineTeam.completed.includes(expectedCurrent.authenticatedNewWindowJoinCandidate.slice(0, 7)) &&
    engineTeam.current.includes("Local unpublished/non-final") &&
    engineTeam.current.includes("all-terminal speculative->paired-prefill->fresh speculative window") &&
    engineTeam.current.includes("private move-only whole-page-set token") &&
    engineTeam.current.includes("inert predecessor physical history and reset active history") &&
    engineTeam.current.includes("Published->Observed->Released") &&
    engineTeam.current.includes("Protected deployment remains in progress") &&
    engineTeam.current.includes("supervisor, concrete checker, signer") &&
    engineTeam.current.includes("production R33 backend") &&
    engineTeam.blockedBy.includes("engine team is making progress") &&
    engineTeam.blockedBy.includes("current aggregate HSACO") &&
    engineTeam.next.includes("Implement and qualify the production R33 backend") &&
    engineTeam.validation.includes("no release qualification") &&
    engineTeam.validation.includes("576 passed / 5 ignored") &&
    engineTeam.validation.includes("token/history/API source-review GOs") &&
    engineTeam.validation.includes("no release qualification, HSACO, hardware, serving, or comparison result"),
  "engine team must expose implemented runtime work and missing execution inputs",
);
assert(
    verificationTeam.completed.includes("756e813") &&
    verificationTeam.completed.includes("Draft06B=>1 same-source mutation") &&
    verificationTeam.completed.includes(expectedCurrent.daemonLifecycleProofCommit.slice(0, 7)) &&
    verificationTeam.current.includes(expectedCurrent.localIntegrationCandidate.slice(0, 7)) &&
    verificationTeam.current.includes("initially failed closed") &&
    verificationTeam.current.includes("#[allow(clippy::too_many_lines)]") &&
    verificationTeam.current.includes("source gate passes 28/28") &&
    verificationTeam.current.includes("6,951 discovered versus 6,854 admitted") &&
    verificationTeam.current.includes("97 unadmitted and 0 stale") &&
    verificationTeam.current.includes("Runtime, Instant, and KFD bodies remain explicitly unverified") &&
    verificationTeam.current.includes("token, history, and API source reviews returned GO") &&
    verificationTeam.current.includes("No release receipt") &&
    verificationTeam.validation.includes("30af5c2 source gate 28/28") &&
    verificationTeam.validation.includes("6,951 discovered / 6,854 admitted / 97 unadmitted / 0 stale") &&
    verificationTeam.validation.includes("release qualification open"),
  "verification team must expose integrated proof work and the open final receipt",
);

const progressCommits = new Set();
project.recentProgress.forEach((item, index) => {
  assertCommit(item.commit, `recentProgress[${index}].commit`);
  assertState(item.state, `recentProgress[${index}]`);
  if (item.repository !== undefined) {
    assert(
      [project.repository, project.fe2o3Repository].includes(item.repository),
      `recentProgress[${index}].repository is not an approved source repository`,
    );
  }
  assert(!progressCommits.has(item.commit), `duplicate progress commit ${item.commit}`);
  progressCommits.add(item.commit);
});
const historicalFe2o3Pin = "57d2d9ced5c113d40546ea1dee603e8ba499cf40";
const historicalPinEntries = project.recentProgress.filter((item) =>
  `${item.commit} ${item.title} ${item.detail}`.includes(historicalFe2o3Pin.slice(0, 8)),
);
assert(historicalPinEntries.length > 0, "historical fe2o3 progress must remain present");
historicalPinEntries.forEach((item) => {
  assert(
    /\b(?:historical|earlier)\b/i.test(`${item.title} ${item.detail}`) &&
      /\b(?:later historical repin checkpoints|later repin checkpoint)\b/i.test(item.detail),
    `historical fe2o3 progress ${item.commit} must be explicitly historical and point forward`,
  );
});
assert(
  progressCommits.has(expectedCurrent.ferricCheckpointCommit),
  "recent progress must include the durable Ferric checkpoint",
);
assert(
  progressCommits.has(expectedCurrent.localIntegrationCandidate) &&
    progressCommits.has(expectedCurrent.protectedVerifierListenerIntegrationCommit) &&
    progressCommits.has(expectedCurrent.boundedProductionQueueWaitIntegrationCommit) &&
    progressCommits.has(expectedCurrent.localBoundedQueueWaitCandidate) &&
    progressCommits.has(expectedCurrent.catalogBindingIntegrationCommit) &&
    progressCommits.has(expectedCurrent.descriptorOrderIntegrationCommit) &&
    progressCommits.has(expectedCurrent.boundedQueueWaitIntegrationCommit) &&
    progressCommits.has(expectedCurrent.combinedPolicyIntegrationCommit),
  "recent progress must include the latest-fe2 integration and its bounded component commits",
);
assert(
  progressCommits.has(expectedCurrent.implementationCommit),
  "recent progress must include the current implementation commit",
);
assert(
  progressCommits.has(expectedCurrent.authenticatedR32Commit),
  "recent progress must include the authenticated R32 implementation commit",
);
assert(
  progressCommits.has(expectedCurrent.integrationCommit),
  "recent progress must include the current integration",
);
assert(
  progressCommits.has(expectedCurrent.productionSpeculativeExecutorCandidate),
  "recent progress must include the independently approved executor",
);
assert(
  progressCommits.has(expectedCurrent.engineeringAggregateLoaderCandidate),
  "recent progress must include the independently approved engineering loader",
);
assert(
  progressCommits.has(expectedCurrent.fe2o3EngineeringSchemaCommit),
  "recent progress must include the frozen fe2o3 engineering schema",
);
assert(
  progressCommits.has(expectedCurrent.fe2o3CompilerCandidate),
  "recent progress must include the pushed compiler candidate",
);
assert(
  progressCommits.has(expectedCurrent.fe2o3CheckedComponentProjectionCommit) &&
    progressCommits.has(expectedCurrent.fe2o3Llvm22WorkerSwitchCommit) &&
    progressCommits.has(expectedCurrent.fe2o3ConnectedPathTransportCommit) &&
    progressCommits.has(expectedCurrent.fe2o3BoundedQueueWaitCommit),
  "recent progress must include the current public fe2o3 stack",
);
assert(
  progressCommits.has(expectedCurrent.localIntegrationCandidate) &&
    progressCommits.has(expectedCurrent.localPolicyCandidate) &&
    progressCommits.has(expectedCurrent.localSelectorCandidate) &&
    progressCommits.has(expectedCurrent.localPhysicalNewWindowProofCandidate) &&
    progressCommits.has(expectedCurrent.localVariableCardinalityPlannerCandidate) &&
    progressCommits.has(expectedCurrent.localDirectDataIndexCandidate) &&
    progressCommits.has(expectedCurrent.vectorGemmReductionCommit),
  "recent progress must include every named local unpublished Ferric candidate",
);
assert(
  progressCommits.has(expectedCurrent.targetEngineeringSmokeCandidate),
  "recent progress must include the independently approved target smoke",
);
assert(
  progressCommits.has(expectedCurrent.verifierBinderCandidate),
  "recent progress must include the qualified verifier binder candidate",
);
assert(
  progressCommits.has(expectedCurrent.swigluCheckedBlockCommit),
  "recent progress must include the public SwiGLU checked-index fix",
);
assert(
  progressCommits.has(expectedCurrent.healthyAllTerminalShutdownCommit),
  "recent progress must include healthy all-terminal queue shutdown",
);
assert(
  progressCommits.has(expectedCurrent.daemonLifecycleProofCommit),
  "recent progress must include the integrated daemon lifecycle proof",
);
assert(
  progressCommits.has(expectedCurrent.supervisedR33WireCommit),
  "recent progress must include the integrated supervised wire foundation",
);

project.evidence.gates.forEach(([label, count, state], index) => {
  assert(label && /^\d+$/.test(count), `evidence.gates[${index}] is malformed`);
  assertState(state, `evidence.gates[${index}]`);
});
const roadmapGate = project.evidence.gates.find(([label]) => label === "Roadmap requirements");
assert(
  roadmapGate?.[1] === String(expectedCurrent.openM1Gates) && roadmapGate?.[2] === "open",
  "the exact M1 roadmap gate count must remain open",
);
const assuranceGate = project.evidence.gates.find(([label]) => label === "Assurance properties");
assert(
  assuranceGate?.[1] === String(expectedCurrent.openAssuranceProperties) &&
    assuranceGate?.[2] === "open",
  "the exact assurance property count must remain open",
);
assert(
  project.evidence.gates.every(([, , state]) => state === "open"),
  "every M1 closure roster remains open without its required evidence",
);
project.evidence.legend.forEach(([state], index) =>
  assertState(state, `evidence.legend[${index}]`),
);

const html = await readFile(join(siteRoot, "index.html"), "utf8");
const normalizedHtml = html.replace(/\s+/g, " ");
const currentProjectData = JSON.stringify(
  Object.fromEntries(
    Object.entries(project).filter(([key]) => key !== "recentProgress"),
  ),
);
const forbiddenCurrentDependencyClaims = [
  /\bselected fe2o3 pin\b/i,
  /\bcurrent (?:fe2o3 )?(?:pin|dependency)\b/i,
];
assert(
  !currentProjectData.includes(historicalFe2o3Pin) &&
    !currentProjectData.includes(historicalFe2o3Pin.slice(0, 8)),
  "current Pages data must not contain the historical fe2o3 pin",
);
assert(
  !currentProjectData.includes("ff21f24") &&
    !currentProjectData.includes("f300ab8") &&
    !currentProjectData.includes("e70ab68") &&
    !currentProjectData.includes("466f88c") &&
    !currentProjectData.includes("a240f98") &&
    !currentProjectData.includes("83bbf0f") &&
    !currentProjectData.includes("Run 32") &&
    !currentProjectData.includes("Aggregate run 7") &&
    !currentProjectData.includes("run 10") &&
    !currentProjectData.includes("Run 10") &&
    !currentProjectData.includes("Run 11") &&
    !currentProjectData.includes("NO-GO") &&
    !currentProjectData.toLowerCase().includes("run 5 has not launched"),
  "current Pages data must not present superseded compiler or executor checkpoints",
);
for (const claim of forbiddenCurrentDependencyClaims) {
  assert(
    !claim.test(currentProjectData),
    `current Pages data contains a forbidden selected-dependency claim: ${claim}`,
  );
}
for (const claim of [
  /fe2o3-production-build-config-v1/i,
  /exact aggregate qualification (?:is )?(?:complete|qualified|green)/i,
  /(?:current|qualified|available) aggregate HSACO (?:is )?(?:ready|accepted|published|available)/i,
  /Qwen serving (?:is )?(?:ready|complete|running)/i,
  /vLLM\/SGLang comparison (?:is )?(?:complete|passed|green)/i,
]) {
  assert(!claim.test(currentProjectData), `current Pages data overclaims open work: ${claim}`);
}
for (const claim of [
  "Public fe2o3 main 5f46dd23cfa3cf58118a93ee2bf5e1252903366c",
  "Compiler, runtime, KFD, and generic transport work stays in fe2o3",
  "Ferric owns all Qwen kernels, inference, and Ferric-specific policy",
  "integration 30af5c2 pins every fe2o3 dependency to exact public 5f46dd2",
  "protected one-shot verifier listener at 64ab4fc",
  "production-wide bounded queue waits",
  "Listener remote checks pass 46 service tests with 3 intentional ignores, 88 adapter tests, 28 source-gate tests, and strict Clippy",
  "Engine checks pass a warning-free library check, 576 library tests with 5 intentional ignores",
  "targeted 25/25, capture 13/13, hostile 28/28, source policy 6/6",
  "Combined formal inventory regeneration first failed closed on unsupported #[allow(clippy::too_many_lines)]",
  "source gate passes 28/28 and canonical regeneration reports exactly 6,951 discovered, 6,854 admitted, 97 unadmitted, and 0 stale",
  "runtime, Instant, and KFD bodies remain explicitly unverified",
  "focused integration evidence, not release qualification",
  "Focused kernel v20 is fully green",
  "prefill 4/4 profiles, 8/8 reference, 19/19 source contracts",
  "paged-decode 4/4 profiles, 9/9 reference, 12/12 source contracts",
  "All four frozen SHAs remain intact. Exact v20 produced no HSACO",
  "two k.len() evaluations lowered to distinct semantic values",
  "active v21 change binds immutable cache_len = k.len() once",
  "Unexpected external proof-shape edits were preserved and the current stable snapshot was frozen",
  "no aggregate HSACO was produced",
  "private move-only whole-page-set token",
  "inert predecessor physical history and reset active history",
  "opaque frozen-timeout Published-&gt;Observed-&gt;Released surface",
  "independent token/history/API source reviews GO",
  "no supervisor, concrete checker, signer",
  "production R33 backend exists",
  "integration is unpublished, non-final, and not release-qualified",
  "No Qwen run, generated token, measured TTFT or TPOT, serving or HTTP endpoint",
  "All 33 M1 gates remain open",
]) {
  assert(normalizedHtml.includes(claim), `index.html is missing current claim: ${claim}`);
}
assert(
  dataSource.includes("7f516e073b8759eb012c998bc9df2eb101d0c7ab") &&
    dataSource.includes("749324c9e287aaec688c8733c88becddc539b12e") &&
    dataSource.includes("eb3b1937ec509cb6ecea080a25965dd3e8bc5457") &&
    dataSource.includes("e187ca52dfdaee79fdc17921c9acffebeed6ca96") &&
    dataSource.includes("24748e11358db7ad3ab5fe35992cff354896e607") &&
    dataSource.includes(expectedCurrent.integrationCommit) &&
    dataSource.includes(expectedCurrent.integrationTree) &&
    dataSource.includes(expectedCurrent.ferricCheckpointCommit) &&
    dataSource.includes(expectedCurrent.ferricCheckpointTree) &&
    dataSource.includes(expectedCurrent.aggregateBuildConfigFormat) &&
    dataSource.includes(expectedCurrent.aggregateBuildObservation) &&
    dataSource.includes(expectedCurrent.fe2o3EngineeringSchemaCommit) &&
    dataSource.includes(expectedCurrent.fe2o3EngineeringSchemaTree) &&
    dataSource.includes(expectedCurrent.fe2o3CompilerCandidate) &&
    dataSource.includes(expectedCurrent.fe2o3CompilerCandidateTree) &&
    dataSource.includes(expectedCurrent.fe2o3CompilerQualificationBase) &&
    dataSource.includes(expectedCurrent.fe2o3LatestMain) &&
    dataSource.includes(expectedCurrent.fe2o3GuardedSubtractionCommit) &&
    dataSource.includes(expectedCurrent.servingComparisonR33V3IntegrationCommit) &&
    dataSource.includes(expectedCurrent.authenticatedTargetRolloverCommit) &&
    dataSource.includes(expectedCurrent.authenticatedTargetServingBridgeCommit) &&
    dataSource.includes(expectedCurrent.swigluCheckedBlockCommit) &&
    dataSource.includes(expectedCurrent.healthyAllTerminalShutdownCommit) &&
    dataSource.includes(expectedCurrent.daemonLifecycleProofCommit) &&
    dataSource.includes(expectedCurrent.daemonLifecycleProofTree) &&
    dataSource.includes(expectedCurrent.supervisedR33WireCommit) &&
    dataSource.includes(expectedCurrent.supervisedR33WireTree) &&
    dataSource.includes(expectedCurrent.canonicalPrepackBundleIdentity) &&
    dataSource.includes(expectedCurrent.canonicalPrepackAdmissionIdentity) &&
    dataSource.includes(expectedCurrent.productionSpeculativeExecutorCandidate) &&
    dataSource.includes(expectedCurrent.productionSpeculativeExecutorTree) &&
    dataSource.includes(expectedCurrent.productionSpeculativeExecutorIntegrationCommit) &&
    dataSource.includes(expectedCurrent.engineeringAggregateLoaderCandidate) &&
    dataSource.includes(expectedCurrent.engineeringAggregateLoaderTree) &&
    dataSource.includes(expectedCurrent.targetEngineeringSmokeCandidate) &&
    dataSource.includes(expectedCurrent.targetEngineeringSmokeTree) &&
    dataSource.includes(expectedCurrent.targetEngineeringSmokeIntegrationCommit) &&
    dataSource.includes(expectedCurrent.protectedVerifierServiceLocalCandidate) &&
    dataSource.includes(expectedCurrent.verifierBinderCandidate) &&
    dataSource.includes(expectedCurrent.verifierBinderCandidateTree) &&
    dataSource.includes(expectedCurrent.verifierBinderIntegrationCommit),
  "Pages data must bind the exact current candidate and retained implementation lineage",
);
assert(
  !dataSource.includes("40cb4337c1b495e43eed66276d81cd4cae36d3bf") &&
    !dataSource.includes("701449c39029de040cd285a2d527dcc185a8750b") &&
    !dataSource.includes("ac00e7ae89d7c73737612d6d0565a632db898890") &&
    !normalizedHtml.includes("57d2d9c"),
  "Pages must not present superseded feature candidates or the historical pin as current",
);
for (const staleBinderClaim of [
  "verifier binder deadline/source-order repair is still in progress",
  "binder deadline/source-order fix is unqualified work in progress",
  "binder's absolute-deadline and source-policy-order fix is still in progress",
  "binder repair and executor custody remediation remain unfinished",
  "companion binder deadline/source-order repair remains unqualified work in progress",
  "verifier binder absolute-deadline and source-policy-order fix remains in progress",
]) {
  assert(
    !dataSource.toLowerCase().includes(staleBinderClaim) &&
      !normalizedHtml.toLowerCase().includes(staleBinderClaim),
    `Pages must not retain stale binder claim: ${staleBinderClaim}`,
  );
}
assert(
  dataSource.includes("private current aggregate publication selection remains None") &&
    dataSource.includes("not independent verifier authority") &&
    dataSource.includes("A real mi300x run canonically prepacked") &&
    dataSource.includes("received independent review GO") &&
    dataSource.includes("observation-only and non-authoritative") &&
    dataSource.includes("R33 V3 event-backed comparison support") &&
    dataSource.includes("No Qwen measurement, baseline server run") &&
    dataSource.includes("12 Ferric-local Rust M1 kernel roots") &&
    dataSource.includes("seven canonical source modules") &&
    dataSource.includes("no prebuilt or vendor kernel dependency") &&
    dataSource.includes("fe2o3-production-build-config-v2") &&
    dataSource.includes("source-isa-summary-v1") &&
    dataSource.includes("reviewed guarded-subtraction support") &&
    dataSource.includes(expectedCurrent.fe2o3CompilerCandidate) &&
    dataSource.includes(expectedCurrent.fe2o3CompilerCandidateTree) &&
    dataSource.includes(expectedCurrent.fe2o3LatestMain) &&
    dataSource.includes("internal move-only non-authoritative program-source capability") &&
    dataSource.includes("The physical runner binds gfx942") &&
    dataSource.includes("160 modules and 7,473 executable bodies") &&
    dataSource.includes("666 directly verified and 6,807 admitted unverified") &&
    dataSource.includes("first published at 756e813") &&
    dataSource.includes("canonical Target8B=1 and Draft06B=2 weight-role byte") &&
    dataSource.includes("rejected same-source mutation") &&
    dataSource.includes("checked DisjointBlockComponentIndex production projection") &&
    dataSource.includes(expectedCurrent.fe2o3CheckedComponentProjectionCommit) &&
    dataSource.includes(expectedCurrent.fe2o3Llvm22WorkerSwitchCommit) &&
    dataSource.includes(expectedCurrent.fe2o3ConnectedPathTransportCommit) &&
    dataSource.includes(expectedCurrent.fe2o3BoundedQueueWaitCommit) &&
    dataSource.includes(expectedCurrent.localIntegrationCandidate) &&
    dataSource.includes(expectedCurrent.localBoundedQueueWaitCandidate) &&
    dataSource.includes(expectedCurrent.localPolicyCandidate) &&
    dataSource.includes(expectedCurrent.localSelectorCandidate) &&
    dataSource.includes(expectedCurrent.localPhysicalNewWindowProofCandidate) &&
    dataSource.includes(expectedCurrent.localVariableCardinalityPlannerCandidate) &&
    dataSource.includes(expectedCurrent.localDirectDataIndexCandidate) &&
    dataSource.includes(expectedCurrent.vectorGemmReductionCommit) &&
    dataSource.includes("local, unpublished, non-final") &&
    dataSource.includes("policy v5 for unpublished 829eb98 on ce44dc0") &&
    dataSource.includes("all quality gates passed over 656 files") &&
    dataSource.includes(expectedCurrent.localPolicyClosureSha256) &&
    dataSource.includes(expectedCurrent.localPolicyReceiptSha256) &&
    dataSource.includes("does not qualify a future combined or 5f46dd2 tree") &&
    dataSource.includes("closes no M1 gate") &&
    dataSource.includes("5/5 Rust arithmetic tests") &&
    dataSource.includes("1 function / 33 details / 0 errors") &&
    dataSource.includes("three rejected hostile formula mutations") &&
    dataSource.includes("162 modules / 7,526 bodies") &&
    dataSource.includes("runtime variable-cardinality wiring") &&
    dataSource.includes("transition atomicity") &&
    dataSource.includes("Focused kernel v20 is fully green") &&
    dataSource.includes("exact v20 produced no HSACO") &&
    dataSource.includes(expectedCurrent.kernelFocusedV19LogSha256) &&
    dataSource.includes(expectedCurrent.kernelExactV19FailureLogSha256) &&
    dataSource.includes(expectedCurrent.kernelExactV19StatusSha256) &&
    dataSource.includes(expectedCurrent.kernelFocusedV20LogSha256) &&
    dataSource.includes(expectedCurrent.kernelFocusedV20StatusSha256) &&
    dataSource.includes(expectedCurrent.kernelExactV20FailureLogSha256) &&
    dataSource.includes("external proof-shape edits were preserved") &&
    dataSource.includes("without implying correctness") &&
    dataSource.includes("Kernel implementation remains in progress") &&
    dataSource.includes(expectedCurrent.protectedVerifierListenerIntegrationCommit) &&
    dataSource.includes(expectedCurrent.boundedProductionQueueWaitIntegrationCommit) &&
    dataSource.includes("failed closed on unsupported #[allow(clippy::too_many_lines)]") &&
    dataSource.includes("6,951 discovered") &&
    dataSource.includes("6,854 admitted") &&
    dataSource.includes("97 unadmitted") &&
    dataSource.includes("0 stale") &&
    dataSource.includes("Runtime, Instant, and KFD bodies remain explicitly unverified") &&
    dataSource.includes("private move-only whole-page-set token") &&
    dataSource.includes("predecessor physical history inertly") &&
    dataSource.includes("reset active history") &&
    dataSource.includes("Published->Observed->Released") &&
    dataSource.includes("576 passed") &&
    dataSource.includes("5 ignored") &&
    dataSource.includes("token, history, and API source reviews returned GO") &&
    dataSource.includes(expectedCurrent.authenticatedNewWindowJoinCandidate) &&
    dataSource.includes("no aggregate Worker V3 HSACO") &&
    dataSource.includes("integrates the independent, authority-free R33 daemon lifecycle proof at 3f5b498") &&
    dataSource.includes("authority-free supervised R33 wire foundation at 66b8547") &&
    dataSource.includes("Four exact positive rows verify") &&
    dataSource.includes("out-of-order measure") &&
    dataSource.includes("abandoned response incorrectly advancing") &&
    dataSource.includes("service-conformance claim") &&
    dataSource.includes("reviewed lower physical model/KV rebind") &&
    dataSource.includes("No aggregate HSACO, Qwen token, serving or HTTP endpoint") &&
    dataSource.includes("received independent GO") &&
    dataSource.includes("production R33 backend") &&
    dataSource.includes("independent review returned GO with no P0, P1, or P2 findings") &&
    dataSource.includes("not public main or deployed authority") &&
    dataSource.includes("not deployed authority") &&
    dataSource.includes("no current aggregate Worker V3 HSACO, Qwen token") &&
    dataSource.includes("All 33 M1 roadmap gates and all 17 assurance properties remain Open"),
  "Pages data must retain service, executor, loader, compiler, baseline, Qwen, selection, and all-open claims",
);
for (const target of [
  "data-readiness",
  "data-capabilities",
  "data-validation",
  "data-transitions",
  "data-teams",
  "data-boundaries",
  "data-observation",
  "data-progress",
  "data-gates",
]) {
  assert(html.includes(target), `index.html is missing ${target}`);
}

const localReferences = [
  ...html.matchAll(/(?:href|src)="([^"]+)"/g),
]
  .map((match) => match[1])
  .filter((reference) => !/^(?:https?:|#)/.test(reference));

for (const reference of localReferences) {
  const cleanReference = reference.split(/[?#]/, 1)[0];
  const target = normalize(join(siteRoot, cleanReference));
  assert(
    !relative(siteRoot, target).startsWith(".."),
    `local reference escapes site root: ${reference}`,
  );
  assert((await stat(target)).isFile(), `missing local file: ${reference}`);
}

console.log(
  `Validated Ferric Pages data: ${project.recentProgress.length} progress entries, ` +
    `${project.validation.transitions.length} active transitions.`,
);
