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
  siteRefreshBase: "6fc3d2f4fa5aa0859a363ac607d1145247af6b14",
  integrationCommit: "756e81369efc9fd0fcfed50c4605c0377a4a119b",
  integrationTree: "09510ffd61284dd03e8574ddc6a0310cd3bca832",
  ferricCheckpointCommit: "756e81369efc9fd0fcfed50c4605c0377a4a119b",
  ferricCheckpointTree: "09510ffd61284dd03e8574ddc6a0310cd3bca832",
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
  fe2o3CompilerCandidate: "d8fa0835c64d6574c8589ac3e69e3c34b0350758",
  fe2o3CompilerCandidateTree: "462305af35405cd4d29031a98e2a4f6a7400da37",
  fe2o3CompilerQualificationBase: "d8fa0835c64d6574c8589ac3e69e3c34b0350758",
  fe2o3LatestMain: "d8fa0835c64d6574c8589ac3e69e3c34b0350758",
  fe2o3GuardedSubtractionCommit: "e745bc75c",
  fe2o3CompilerCandidateStatus: "public-main-consumed-by-ferric-feature-branch",
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
  engineeringAggregateAttemptBoundary:
    "semantic-to-ranked-repeated-slice-projection-before-worker-or-link",
  engineeringAggregateAttemptConnectCount: 0,
  engineeringAggregateAttemptOutputCount: 0,
  engineeringAggregateAttemptStatus: "latest-d8fa-aggregate-stopped-before-worker-no-hsaco",
  followupProofFixStatus: "repeated-slice-projection-fix-in-progress-not-public",
  fe2o3CurrentnessStatus: "public-main-d8fa0835-consumed-by-ferric-feature-branch",
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
  protectedVerifierServiceStatus: "blocked-protected-profile-and-socket-absent",
  verifierBinderCandidate: "6846d9282f858c80dd2b0b4abfe247dc89e9d8f8",
  verifierBinderCandidateTree: "4690d8c9e502de18a947d6def2f8c09d4f153ea1",
  verifierBinderIntegrationCommit: "ed708de7fc906926091be29ff118af95ee50a42b",
  verifierBinderStatus: "qualified-go-local-integration",
  authenticatedTargetRolloverCommit: "047ee32f6d0bb1861adb211c9ced1f403a22514c",
  authenticatedTargetRolloverStatus: "implemented-integrated-not-run",
  authenticatedTargetServingBridgeCommit: "f8f8ce60e23a1331a83606029fca2d0958bd3157",
  authenticatedTargetServingBridgeStatus:
    "new-window-control-transaction-public-lower-physical-rebind-unavailable",
  canonicalPrepackBundleIdentity:
    "6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b",
  canonicalPrepackAdmissionIdentity:
    "6a396e95e715d1be16bbc27b8c762a9308e40e5355c5bd89b9fc28fb06a1dd16",
  canonicalPrepackStatus: "real-qwen3-8b-and-06b-verified-non-execution",
  vectorGemmSsaWorkObserved: 94192887,
  vectorGemmSsaWorkLimit: 67108864,
  priorAggregateStorageObserved: 3188438,
  priorAggregateStorageLimit: 2097152,
  aggregateRerunStatus: "not-rerun-after-current-gemm-reductions",
  aggregateSourceCommit: "5514afe176a090aa3f1da9e5354799bb4ca5a8b3",
  aggregateProducerCommit: "e57c42523050922ad76538150df691cc5ab975a7",
  aggregateKernelCount: 12,
  diagnosticBridgeCommit: "24748e11358db7ad3ab5fe35992cff354896e607",
  diagnosticStatus: "partial-non-evidence",
  diagnosticDispatchGeneration: 1,
  diagnosticCopyCount: 5,
  proofQueries: 1493,
  directVerifiedBodies: 645,
  proofErrors: 0,
  proofPackages: 8,
  actualBodyHostileMutations: 37,
  sourceQualityPassMarkers: 13,
  sourceGateModules: 159,
  sourceGateBodies: 7429,
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
  "Speculative executor",
  "Engineering aggregate loader",
  "Engineering aggregate output",
  "Canonical Qwen prepack",
  "Authenticated target rollover",
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
      expectedCurrent.servingComparisonR33V3IntegrationCommit,
    ) &&
    envelope.get("Ferric implementation checkpoint")?.includes(
      expectedCurrent.ferricFeatureBranch,
    ) &&
    envelope.get("Ferric implementation checkpoint")?.includes(
      "neither checkpoint is on Ferric main or final",
    ) &&
    envelope.get("Ferric implementation checkpoint")?.includes(
      "all 33 M1 gates remain open",
    ) &&
    envelope.get("fe2o3 public main")?.includes(expectedCurrent.fe2o3LatestMain) &&
    envelope.get("fe2o3 public main")?.includes(expectedCurrent.fe2o3CompilerCandidateTree) &&
    envelope.get("fe2o3 public main")?.includes(
      "compiler, runtime, and KFD ownership remains in fe2o3",
    ) &&
    envelope.get("fe2o3 public main")?.includes(
      "model kernels and inference remain in Ferric",
    ),
  "envelope must expose the durable Ferric and fe2o3 checkpoints without current authority",
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
  envelope.get("Engineering aggregate output")?.includes(
    "one aggregate Worker V3 publication",
  ) &&
    envelope.get("Engineering aggregate output")?.includes("12 K1-K7 entrypoints") &&
    envelope.get("Engineering aggregate output")?.includes("440 exact profiles") &&
    envelope.get("Engineering aggregate output")?.includes(
      "latest clean aggregate attempt against public fe2o3 d8fa0835",
    ) &&
    envelope.get("Engineering aggregate output")?.includes(
      "repeated-slice semantic-to-ranked projection",
    ) &&
    envelope.get("Engineering aggregate output")?.includes(
      "reusable fe2o3 correction is in progress and is not public",
    ) &&
    envelope.get("Engineering aggregate output")?.includes("No aggregate HSACO") &&
    envelope.get("Engineering aggregate output")?.includes("Qwen serving") &&
    envelope.get("Engineering aggregate output")?.includes(
      "artifact authority, or execution authority",
    ),
  "envelope must retain the current aggregate qualification and execution limits",
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
      expectedCurrent.authenticatedTargetServingBridgeCommit,
    ) &&
    envelope.get("Authenticated target rollover")?.includes("scheduler-authority check") &&
    envelope.get("Authenticated target rollover")?.includes("predecessor restore") &&
    envelope.get("Authenticated target rollover")?.includes("lower physical model/KV rebind") &&
    envelope.get("Authenticated target rollover")?.includes("retryable RolloverUnavailable"),
  "envelope must distinguish new-window control publication from the absent physical rebind",
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
  envelope.get("Protected verifier status")?.includes("protected execution profile") &&
    envelope.get("Protected verifier status")?.includes("protected service socket") &&
    envelope.get("Protected verifier status")?.includes("absent") &&
    envelope.get("Protected verifier status")?.includes("do not create a deployed verifier"),
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
assert(
  protectedAcceptance?.state === "open" &&
    protectedAcceptance.detail.includes("remains None") &&
    protectedAcceptance.detail.includes("protected execution profile") &&
    protectedAcceptance.detail.includes("protected service socket") &&
    protectedAcceptance.detail.includes("absent") &&
    protectedAcceptance.detail.includes("cannot provide a deployed verifier") &&
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
    producerReadiness.detail.includes("public Ferric feature-branch checkpoint 756e813") &&
    producerReadiness.detail.includes("consumes that revision") &&
    producerReadiness.detail.includes("not on main or final") &&
    producerReadiness.detail.includes("compiler, runtime, and KFD work only") &&
    producerReadiness.detail.includes("Ferric owns model kernels and inference") &&
    producerReadiness.detail.includes(
      "No Ferric artifact, runtime result, inference result, or correctness authority",
    ),
  "fe2o3 producer must retain the current ownership, proof status, and downstream nonclaims",
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
    qwenReadiness.detail.includes("new-window control transaction is implemented") &&
    qwenReadiness.detail.includes("lower physical model/KV rebind is unavailable") &&
    qwenReadiness.detail.includes("retryable RolloverUnavailable") &&
    qwenReadiness.detail.includes("latest aggregate attempt stopped at repeated-slice projection") &&
    qwenReadiness.detail.includes("protected profile/socket are absent") &&
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
  proofLedgerCapability?.detail.includes("checkpoint 756e813") &&
    proofLedgerCapability.detail.includes("159 modules and 7,429 executable bodies") &&
    proofLedgerCapability.detail.includes("canonical weight-role proof") &&
    proofLedgerCapability.detail.includes("negative mutation evidence") &&
    !proofLedgerCapability.detail.includes("checkpoint a2bb2dc has an exact source-gate inventory"),
  "proof ledger must bind the 159-module / 7,429-body inventory to checkpoint 756e813",
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
  integrationTeam.completed.includes(expectedCurrent.fe2o3LatestMain.slice(0, 8)) &&
    integrationTeam.completed.includes(expectedCurrent.authenticatedTargetServingBridgeCommit.slice(0, 7)) &&
    integrationTeam.completed.includes(expectedCurrent.ferricCheckpointCommit.slice(0, 7)) &&
    integrationTeam.completed.includes(expectedCurrent.ferricFeatureBranch) &&
    integrationTeam.current.includes("unpublished repeated-slice compiler fix") &&
    integrationTeam.current.includes("R33 daemon/wire") &&
    integrationTeam.current.includes("lower physical new-window rebind") &&
    integrationTeam.validation.includes("public on the feature branch, not on main or final"),
  "integration team must expose public upstream and public feature-branch Ferric checkpoints",
);
assert(
  kernelTeam.status === "Making progress" &&
    kernelTeam.completed.includes("12 K1-K7 entrypoints") &&
    kernelTeam.completed.includes("440 exact profiles") &&
    kernelTeam.current.includes("repeated-slice bounds projection defect") &&
    kernelTeam.current.includes("reusable fe2o3 correction is in progress") &&
    kernelTeam.current.includes("Ferric kernel source remains unchanged") &&
    kernelTeam.blockedBy.includes("team is making progress") &&
    kernelTeam.next.includes("without weakening slice authorization"),
  "kernel team must expose roster, artifact dependency, and ownership",
);
assert(
  engineTeam.completed.includes(expectedCurrent.authenticatedTargetRolloverCommit.slice(0, 7)) &&
    engineTeam.completed.includes(expectedCurrent.servingComparisonR33V3IntegrationCommit.slice(0, 7)) &&
    engineTeam.completed.includes(expectedCurrent.authenticatedTargetServingBridgeCommit.slice(0, 7)) &&
    engineTeam.current.includes("R33 daemon/wire adapter") &&
    engineTeam.current.includes("retryable RolloverUnavailable") &&
    engineTeam.blockedBy.includes("engine team is making progress") &&
    engineTeam.blockedBy.includes("lower physical all-terminal model/KV rebind") &&
    engineTeam.next.includes("lower model/KV rebind") &&
    engineTeam.next.includes("HTTP serving remains later work"),
  "engine team must expose implemented runtime work and missing execution inputs",
);
assert(
    verificationTeam.completed.includes("756e813") &&
    verificationTeam.completed.includes("Draft06B=>1 same-source mutation") &&
    verificationTeam.current.includes("159-module / 7,429-body source gate") &&
    verificationTeam.current.includes("constructor-to-physical-batch refinement") &&
    verificationTeam.validation.includes("source gate 159 modules / 7,429 bodies") &&
    verificationTeam.validation.includes("final receipt open"),
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
  progressCommits.has(expectedCurrent.targetEngineeringSmokeCandidate),
  "recent progress must include the independently approved target smoke",
);
assert(
  progressCommits.has(expectedCurrent.verifierBinderCandidate),
  "recent progress must include the qualified verifier binder candidate",
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
  "Public fe2o3 main d8fa0835c64d6574c8589ac3e69e3c34b0350758",
  "Compiler, runtime, and KFD work stays in fe2o3",
  "Ferric owns all Qwen kernels and inference",
  "Ferric checkpoint 756e813 is public on feature branch origin/codex/fe2o3-current-main-repin-v1, but is not on main or final",
  "12 K1-K7 entrypoints across 440 profiles",
  "545-packet target runner",
  "same-shape decode rearm",
  "R33 V3 event-backed TTFT/TPOT records",
  "new-window control-plane publication, restore, and scheduler-authority transaction is public",
  "lower physical model/KV rebind remains unavailable and returns retryable RolloverUnavailable",
  "Target8B=1 and Draft06B=2 weight-role byte has a positive Verus proof and rejected negative mutation",
  "Real Qwen3-8B and Qwen3-0.6B inputs are canonically prepacked",
  "latest aggregate attempt stopped before Worker or link at repeated-slice semantic-to-ranked projection",
  "reusable fe2o3 fix and the R33 daemon/wire foundation are in progress and not public",
  "aggregate Worker V3 HSACO is still absent",
  "protected execution profile and service socket",
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
    dataSource.includes("159 modules and 7,429 executable bodies") &&
    dataSource.includes("Public feature-branch checkpoint 756e813 has an exact source-gate inventory") &&
    dataSource.includes("canonical Target8B=1 and Draft06B=2 weight-role byte") &&
    dataSource.includes("rejected same-source mutation") &&
    dataSource.includes("repeated-slice semantic-to-ranked projection") &&
    dataSource.includes("reusable upstream correction is in progress and not public") &&
    dataSource.includes("lower physical model/KV rebind remains unavailable") &&
    dataSource.includes("retryable RolloverUnavailable") &&
    dataSource.includes("daemon/wire foundation is in progress and not public") &&
    dataSource.includes("No aggregate HSACO, Qwen token, serving or HTTP endpoint") &&
    dataSource.includes("received independent GO") &&
    dataSource.includes("protected execution profile and protected service socket are absent") &&
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
