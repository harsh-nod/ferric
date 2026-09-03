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
  siteRefreshBase: "31b6f4989d961667900bff39935c75024316a2dc",
  integrationCommit: "31b6f4989d961667900bff39935c75024316a2dc",
  integrationTree: "a52398ce57ec765a5d49a97904bc160e586a911b",
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
  fe2o3CompilerCandidate: "83bbf0ffb1ef7c7d66f9251d608f469cf51173b5",
  fe2o3CompilerCandidateTree: "a2675071fb066f6366227a56eae5f4cc62723ddd",
  fe2o3CompilerQualificationBase: "9176b9c27696ac3c86814dea60ef9ecc12f10539",
  fe2o3LatestMain: "2df6130c5f897b5120cdf6ade44d53030690fa8b",
  fe2o3CompilerCandidateStatus: "pushed-independent-go-qualified-on-9176b9c",
  productionSpeculativeExecutorCandidate: "0c2b73bfb8d4e62c100c42a125171c271c8850d8",
  productionSpeculativeExecutorTree: "00c4b8a04aab2f52af0f43de8a26a7e9564c5568",
  productionSpeculativeExecutorIntegrationCommit: "867f863e223d00e3b304d324e89146e27d2c5c28",
  productionSpeculativeExecutorStatus: "independent-go-integrated",
  engineeringAggregateLoaderCandidate: "c9072b0de61a27be917020baf5eecb4b743734f0",
  engineeringAggregateLoaderTree: "c725eb6e3e6f470fa327f94289509fe910eb83ef",
  engineeringAggregateLoaderIntegrationCommit: "99cf0d514feb7fccb916f066c645c3a1cf831a0c",
  engineeringAggregateLoaderStatus: "independent-go-integrated",
  engineeringAggregateHsacoStatus: "not-produced",
  engineeringAggregateRun32OuterExitCode: 1,
  engineeringAggregateRun32NestedCargoExitCode: 101,
  engineeringAggregateRun32Boundary: "logits.rs:409-choice-base-plus-accepted-overflow-proof",
  engineeringAggregateRun32ConnectCount: 0,
  engineeringAggregateRun32OutputCount: 0,
  engineeringAggregateRun32Status: "cleared-raw-u32-optional-entry-failed-next-overflow-proof",
  rawMatchLogitsSha256:
    "0b67a85afb620278efa79d674d757749fe84c858bddc980bef3b7bb3552eb940",
  acceptedBoundGuardSha256:
    "a172807a5a13473775d13d829a68cb4424c689e6285cebdd468124b50618e4a6",
  acceptedBoundGuardStatus: "uncommitted-independent-go-packages-21-of-21",
  engineeringSchemaIntegrationStatus: "v11-tag64-in-progress",
  fe2o3IsFiniteRemediationStatus: "independent-source-go",
  targetEngineeringSmokeCandidate: "951d48ac119089a62546cb6f96f324feaad013af",
  targetEngineeringSmokeTree: "ffad404f1bce2ee8c55d94b226d9d54dcd8fc62c",
  targetEngineeringSmokeIntegrationCommit: "36fb8e9a078953fa7f7078e2e960ba5ea9fc8b4b",
  targetEngineeringSmokeStatus: "independent-go-integrated-not-executed",
  targetEngineeringSmokeEngineTests: 511,
  targetEngineeringSmokeCaptureTests: 84,
  targetEngineeringSmokeDoctests: 145,
  targetEngineeringSmokeExactFinalPinStatus: "open",
  targetEngineeringSmokeHardwareStatus: "not-run",
  servingComparisonR33V2IntegrationCommit: "31b6f4989d961667900bff39935c75024316a2dc",
  servingComparisonR33V2Status: "reviewed-integrated-not-run",
  gpuAvailabilityStatus: "all-gpus-occupied",
  baselineAuditStatus: "container-access-unresolved",
  comparisonStatus: "not-run",
  protectedVerifierServiceLocalCandidate: "9a435522a4a88d55108f7c6a4cb493aabb01ad93",
  protectedVerifierServiceStatus: "foundation-go-local-undeployed",
  verifierBinderCandidate: "6846d9282f858c80dd2b0b4abfe247dc89e9d8f8",
  verifierBinderCandidateTree: "4690d8c9e502de18a947d6def2f8c09d4f153ea1",
  verifierBinderIntegrationCommit: "ed708de7fc906926091be29ff118af95ee50a42b",
  verifierBinderStatus: "qualified-go-local-integration",
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
  sourceGateModules: 151,
  sourceGateBodies: 6916,
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
assert(
  /^[0-9a-f]{64}$/.test(project.current.rawMatchLogitsSha256),
  "current.rawMatchLogitsSha256 must be an exact SHA-256 digest",
);
assert(
  /^[0-9a-f]{64}$/.test(project.current.acceptedBoundGuardSha256),
  "current.acceptedBoundGuardSha256 must be an exact SHA-256 digest",
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
  project.current.servingComparisonR33V2IntegrationCommit,
  "current.servingComparisonR33V2IntegrationCommit",
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
  "M1 integration",
  "fe2o3 engineering producer",
  "Speculative executor",
  "Engineering aggregate loader",
  "Engineering aggregate output",
  "Target-only engineering smoke",
  "R33 V2 serving comparison",
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
  envelope.get("fe2o3 engineering producer")?.includes(
    expectedCurrent.fe2o3EngineeringSchemaCommit,
  ) &&
    envelope.get("fe2o3 engineering producer")?.includes(
      expectedCurrent.fe2o3EngineeringSchemaTree,
    ) &&
    envelope.get("fe2o3 engineering producer")?.includes(expectedCurrent.fe2o3CompilerCandidate) &&
    envelope.get("fe2o3 engineering producer")?.includes(expectedCurrent.fe2o3CompilerCandidateTree) &&
    envelope.get("fe2o3 engineering producer")?.includes(
      expectedCurrent.fe2o3CompilerQualificationBase,
    ) &&
    envelope.get("fe2o3 engineering producer")?.includes(expectedCurrent.fe2o3LatestMain) &&
    envelope.get("fe2o3 engineering producer")?.includes("independently reviewed") &&
    envelope.get("fe2o3 engineering producer")?.includes("V11/tag 64 integration remains in progress") &&
    envelope.get("fe2o3 engineering producer")?.includes("ownership remains in fe2o3"),
  "envelope must expose the reviewed raw-u32 candidate and current schema integration",
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
  envelope.get("Engineering aggregate output")?.includes("Run 32") &&
    envelope.get("Engineering aggregate output")?.includes(
      expectedCurrent.rawMatchLogitsSha256,
    ) &&
    envelope.get("Engineering aggregate output")?.includes("raw-u32 optional-entry proof") &&
    envelope.get("Engineering aggregate output")?.includes("outer invocation exited 1") &&
    envelope.get("Engineering aggregate output")?.includes("nested Cargo exited 101") &&
    envelope.get("Engineering aggregate output")?.includes("logits.rs:409") &&
    envelope.get("Engineering aggregate output")?.includes("choice_base + accepted") &&
    envelope.get("Engineering aggregate output")?.includes("zero connects and zero output or HSACO") &&
    envelope.get("Engineering aggregate output")?.includes(
      expectedCurrent.acceptedBoundGuardSha256,
    ) &&
    envelope.get("Engineering aggregate output")?.includes("two 21/21 package passes") &&
    envelope.get("Engineering aggregate output")?.includes("uncommitted candidate") &&
    envelope.get("Engineering aggregate output")?.includes("V11/tag 64 integration remains in progress") &&
    envelope.get("Engineering aggregate output")?.includes("no artifact or execution authority"),
  "envelope must retain the exact run 32 boundary and uncommitted repair limits",
);
assert(
  envelope.get("Target-only engineering smoke")?.includes(
    expectedCurrent.targetEngineeringSmokeCandidate,
  ) &&
    envelope.get("Target-only engineering smoke")?.includes(
      expectedCurrent.targetEngineeringSmokeTree,
    ) &&
    envelope.get("Target-only engineering smoke")?.includes(
      expectedCurrent.targetEngineeringSmokeIntegrationCommit,
    ) &&
    envelope.get("Target-only engineering smoke")?.includes("independent source-integration GO") &&
    envelope.get("Target-only engineering smoke")?.includes("documentation-only correction") &&
    envelope.get("Target-only engineering smoke")?.includes("511 engine library tests") &&
    envelope.get("Target-only engineering smoke")?.includes("84 capture tests") &&
    envelope.get("Target-only engineering smoke")?.includes("145 doctests") &&
    envelope.get("Target-only engineering smoke")?.includes("all-target strict clippy") &&
    envelope.get("Target-only engineering smoke")?.includes("exact locked final pin") &&
    envelope.get("Target-only engineering smoke")?.includes("live hardware remain open") &&
    envelope.get("Target-only engineering smoke")?.includes("not executed") &&
    envelope.get("Target-only engineering smoke")?.includes("no Qwen token"),
  "envelope must retain exact target-smoke integration, matrix, and execution limits",
);
assert(
  envelope.get("R33 V2 serving comparison")?.includes(
    expectedCurrent.servingComparisonR33V2IntegrationCommit,
  ) &&
    envelope.get("R33 V2 serving comparison")?.includes("Reviewed checker and full collector") &&
    envelope.get("R33 V2 serving comparison")?.includes("collector has not run") &&
    envelope.get("R33 V2 serving comparison")?.includes("no baseline or performance result"),
  "envelope must retain the reviewed, integrated, unrun R33 V2 comparison surface",
);
assert(
  envelope.get("Baseline comparison")?.includes("All GPUs are currently occupied") &&
    envelope.get("Baseline comparison")?.includes("container access remains unresolved") &&
    envelope.get("Baseline comparison")?.includes("comparison not run"),
  "envelope must retain current baseline blockers and comparison nonclaim",
);
assert(
  envelope.get("Protected verifier status")?.includes(expectedCurrent.verifierBinderCandidate) &&
    envelope.get("Protected verifier status")?.includes(expectedCurrent.verifierBinderCandidateTree) &&
    envelope.get("Protected verifier status")?.includes(
      expectedCurrent.verifierBinderIntegrationCommit,
    ) &&
    envelope.get("Protected verifier status")?.includes("no P0/P1/P2") &&
    envelope.get("Protected verifier status")?.includes("not public main or deployed authority"),
  "envelope must expose the qualified binder candidate and its local-only authority",
);
assert(
  envelope.get("M1 integration")?.includes(expectedCurrent.integrationCommit) &&
    envelope.get("M1 integration")?.includes(expectedCurrent.integrationTree),
  "envelope must expose the exact current integration commit and tree",
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
    protectedAcceptance.detail.includes("passed 28 tests and 6 doctests") &&
    protectedAcceptance.detail.includes("foundation GO") &&
    protectedAcceptance.detail.includes("not publicly linked") &&
    protectedAcceptance.detail.includes("not deployed") &&
    protectedAcceptance.detail.includes("protected current, checker, signer, head-store") &&
    protectedAcceptance.detail.includes("Binder candidate 6846d92") &&
    protectedAcceptance.detail.includes("independently reviewed GO with no P0/P1/P2") &&
    protectedAcceptance.detail.includes("integrated locally at ed708de") &&
    protectedAcceptance.detail.includes("not public main or deployed authority"),
  "protected aggregate acceptance must remain fail-closed and open",
);
const producerReadiness = project.readiness.find(
  (item) => item.label === "fe2o3 engineering aggregate producer",
);
assert(
  producerReadiness?.state === "integration" &&
    producerReadiness.detail.includes(expectedCurrent.fe2o3EngineeringSchemaCommit) &&
    producerReadiness.detail.includes(expectedCurrent.fe2o3EngineeringSchemaTree) &&
    producerReadiness.detail.includes(expectedCurrent.fe2o3CompilerCandidate) &&
    producerReadiness.detail.includes(expectedCurrent.fe2o3CompilerCandidateTree) &&
    producerReadiness.detail.includes(expectedCurrent.fe2o3CompilerQualificationBase) &&
    producerReadiness.detail.includes(expectedCurrent.fe2o3LatestMain) &&
    producerReadiness.detail.includes("pushed and independently reviewed") &&
    producerReadiness.detail.includes("focused and full 483-test library validation") &&
    producerReadiness.detail.includes("backend/worker checks") &&
    producerReadiness.detail.includes("ROCm compile passed") &&
    producerReadiness.detail.includes("Run 32") &&
    producerReadiness.detail.includes("raw-u32 optional-entry proof") &&
    producerReadiness.detail.includes("logits.rs:409") &&
    producerReadiness.detail.includes("choice_base + accepted") &&
    producerReadiness.detail.includes("independent GO and two 21/21 package passes") &&
    producerReadiness.detail.includes("not committed") &&
    producerReadiness.detail.includes("V10 tag 63") &&
    producerReadiness.detail.includes("V11/tag 64 integration remains in progress") &&
    producerReadiness.detail.includes("No artifact, runtime, or inference authority"),
  "fe2o3 producer must retain the exact current proof status and downstream nonclaims",
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
    engineeringLoaderReadiness.detail.includes("cannot create Worker V3") &&
    engineeringLoaderReadiness.detail.includes("no real aggregate HSACO"),
  "engineering loader must retain exact integration, nonauthority, and unused-output status",
);
const targetSmokeReadiness = project.readiness.find(
  (item) => item.label === "Non-authoritative target-only engineering smoke",
);
assert(
  targetSmokeReadiness?.state === "integration" &&
    targetSmokeReadiness.detail.includes(expectedCurrent.targetEngineeringSmokeCandidate) &&
    targetSmokeReadiness.detail.includes(expectedCurrent.targetEngineeringSmokeTree) &&
    targetSmokeReadiness.detail.includes(expectedCurrent.targetEngineeringSmokeIntegrationCommit) &&
    targetSmokeReadiness.detail.includes("independent source-integration GO") &&
    targetSmokeReadiness.detail.includes("documentation-only correction") &&
    targetSmokeReadiness.detail.includes("511 engine library tests") &&
    targetSmokeReadiness.detail.includes("84 capture tests") &&
    targetSmokeReadiness.detail.includes("145 doctests") &&
    targetSmokeReadiness.detail.includes("all-target strict clippy") &&
    targetSmokeReadiness.detail.includes("Exact locked final pinning") &&
    targetSmokeReadiness.detail.includes("live hardware execution remain open") &&
    targetSmokeReadiness.detail.includes("has not executed") &&
    targetSmokeReadiness.detail.includes("no Qwen token"),
  "target smoke must retain exact integration, ephemeral matrix, and no-execution limits",
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
    qwenReadiness.detail.includes("canonical prepack result remains a non-final probe") &&
    qwenReadiness.detail.includes("R33 V2 checker and full collector") &&
    qwenReadiness.detail.includes("neither smoke nor collector has executed") &&
    qwenReadiness.detail.includes("Run 32") &&
    qwenReadiness.detail.includes("raw-u32 optional-entry proof") &&
    qwenReadiness.detail.includes("choice_base + accepted") &&
    qwenReadiness.detail.includes("logits.rs:409") &&
    qwenReadiness.detail.includes("status 1, nested Cargo 101") &&
    qwenReadiness.detail.includes("zero connects, and zero output or HSACO") &&
    qwenReadiness.detail.includes(
      "accepted-bound guard with compact-loop structure is reviewed but uncommitted",
    ) &&
    qwenReadiness.detail.includes("V11/tag 64 integration is still underway") &&
    qwenReadiness.detail.includes(
      "no current aggregate or engineering HSACO, Qwen token, hardware, numerical, performance, or baseline result exists",
    ),
  "Qwen, numerical, and performance authority must remain open",
);
const baselineReadiness = project.readiness.find(
  (item) => item.label === "vLLM and SGLang baseline comparison",
);
assert(
  baselineReadiness?.state === "open" &&
    baselineReadiness.detail.includes(expectedCurrent.servingComparisonR33V2IntegrationCommit) &&
    baselineReadiness.detail.includes("reviewed R33 V2 checker and full collector") &&
    baselineReadiness.detail.includes("collector has not run") &&
    baselineReadiness.detail.includes("All GPUs are currently occupied") &&
    baselineReadiness.detail.includes("container access remains unresolved") &&
    baselineReadiness.detail.includes("No baseline server was launched") &&
    baselineReadiness.detail.includes("no Ferric comparison result exists"),
  "baseline comparison must remain open with exact environment limits",
);
const prepackProbe = project.readiness.find(
  (item) => item.label === "Canonical Qwen prepack probe",
);
assert(
  prepackProbe?.state === "observed" &&
    prepackProbe.detail.includes("non-final mi300x probe") &&
    prepackProbe.detail.includes(
      "6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b",
    ) &&
    prepackProbe.detail.includes(
      "6a396e95e715d1be16bbc27b8c762a9308e40e5355c5bd89b9fc28fb06a1dd16",
    ) &&
    prepackProbe.detail.includes("not final-integration evidence") &&
    prepackProbe.detail.includes("a protected artifact") &&
    prepackProbe.detail.includes("a hardware run") &&
    prepackProbe.detail.includes("Qwen execution authority"),
  "canonical Qwen prepack must remain explicitly non-final and non-authoritative",
);

for (const group of ["runnable", "experimental", "roadmap"]) {
  assert(
    Array.isArray(project.capabilities[group]) && project.capabilities[group].length > 0,
    `capabilities.${group} is empty`,
  );
}
const r33CaptureCapability = project.capabilities.experimental.find(
  (item) => item.name === "R33 V2 serving comparison capture",
);
assert(
  r33CaptureCapability?.detail.includes("construct and structurally validate its V2 observations") &&
    r33CaptureCapability.detail.includes(
      "validation command separately constructs the V2 comparison record and applies the gate",
    ) &&
    r33CaptureCapability.detail.includes("Neither command has run against the three engines") &&
    r33CaptureCapability.detail.includes(
      "no baseline, hardware, numerical, performance, Qwen, or M1 result",
    ),
  "R33 V2 capability must distinguish collector observations from validation and retain nonclaims",
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
      /\b(?:current repin|current dependency repin)\b/i.test(item.detail),
    `historical fe2o3 progress ${item.commit} must be explicitly historical and point forward`,
  );
});
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
  "Ferric integration 31b6f49 contains the reviewed R33 V2 checker and full collector",
  "neither the collector nor target-only smoke has run",
  "Pushed fe2o3 candidate 83bbf0f, tree a267507, received independent GO",
  "focused and full 483-test library validation",
  "backend/worker checks, and ROCm compile against main 9176b9c",
  "Run 32 used Ferric raw-match logits SHA-256 0b67a85a",
  "cleared the raw-u32 optional-entry proof",
  "choice_base + accepted at logits.rs:409",
  "outer status was 1, nested Cargo was 101",
  "zero connects and zero output or HSACO",
  "accepted-bound guard with compact-loop structure at source SHA-256 a172807a",
  "independent GO and two 21/21 package passes",
  "remains uncommitted",
  "Current fe2o3 main is 2df6130",
  "sound schema V11/tag 64 integration is in progress",
  "workgroup-scan V10 tag 63 collides with the branch's volatile tag 63",
  "Ferric-specific inference and kernel ownership remain in Ferric",
  "reusable compiler and runtime work remains in fe2o3",
  "publication selection remains None",
  "CURRENT=None",
  "No aggregate HSACO, current Qwen token, hardware, numerical, performance, or vLLM/SGLang comparison result exists",
  "All 33 M1 gates and all 17 assurance properties remain Open",
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
    dataSource.includes(expectedCurrent.fe2o3EngineeringSchemaCommit) &&
    dataSource.includes(expectedCurrent.fe2o3EngineeringSchemaTree) &&
    dataSource.includes(expectedCurrent.fe2o3CompilerCandidate) &&
    dataSource.includes(expectedCurrent.fe2o3CompilerCandidateTree) &&
    dataSource.includes(expectedCurrent.fe2o3CompilerQualificationBase) &&
    dataSource.includes(expectedCurrent.fe2o3LatestMain) &&
    dataSource.includes(expectedCurrent.rawMatchLogitsSha256) &&
    dataSource.includes(expectedCurrent.acceptedBoundGuardSha256) &&
    dataSource.includes(expectedCurrent.servingComparisonR33V2IntegrationCommit) &&
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
    dataSource.includes("non-final mi300x probe") &&
    dataSource.includes("received independent review GO") &&
    dataSource.includes("observation-only and non-authoritative") &&
    dataSource.includes("reviewed R33 V2 checker and full collector") &&
    dataSource.includes("collector has not run") &&
    dataSource.includes("focused and full 483-test library validation") &&
    dataSource.includes("backend/worker checks") &&
    dataSource.includes("ROCm compile passed") &&
    dataSource.includes("Run 32") &&
    dataSource.includes("raw-u32 optional-entry proof") &&
    dataSource.includes("logits.rs:409") &&
    dataSource.includes("choice_base + accepted") &&
    dataSource.includes("status 1, nested Cargo 101") &&
    dataSource.includes("zero connects, and zero output or HSACO") &&
    dataSource.includes(expectedCurrent.rawMatchLogitsSha256) &&
    dataSource.includes(expectedCurrent.acceptedBoundGuardSha256) &&
    dataSource.includes("two 21/21 package passes") &&
    dataSource.includes("uncommitted") &&
    dataSource.includes("V10 tag 63") &&
    dataSource.includes("V11/tag 64 integration") &&
    dataSource.includes("received independent GO") &&
    dataSource.includes("independent source-integration GO") &&
    dataSource.includes("documentation-only correction") &&
    dataSource.includes("511 engine library tests") &&
    dataSource.includes("84 capture tests") &&
    dataSource.includes("145 doctests") &&
    dataSource.includes("all-target strict clippy") &&
    dataSource.includes("exact locked final pinning and live hardware execution remain open") &&
    dataSource.includes("smoke has not executed") &&
    dataSource.includes("All GPUs are currently occupied") &&
    dataSource.includes("baseline container access remains unresolved") &&
    dataSource.includes("No baseline server was launched") &&
    dataSource.includes("passed 28 tests and 6 doctests") &&
    dataSource.includes("independent review returned GO with no P0, P1, or P2 findings") &&
    dataSource.includes("not public main or deployed authority") &&
    dataSource.includes("not deployed") &&
    dataSource.includes("no current aggregate or engineering HSACO, Qwen token") &&
    dataSource.includes("All 33 M1 roadmap gates and all 17 assurance properties remain Open"),
  "Pages data must retain service, executor, loader, compiler, baseline, Qwen, selection, and all-open claims",
);
for (const target of [
  "data-readiness",
  "data-capabilities",
  "data-validation",
  "data-transitions",
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
