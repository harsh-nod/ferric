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
  siteRefreshBase: "f8718ba98c62489f7e8a6a2613ca0c6ac973faff",
  implementationCommit: "b2da60626a05fa53983002d36b27cddcf9743b13",
  implementationTree: "c1309907047713b486c08316c4e73f1662080774",
  integrationBranchHead: "b2da60626a05fa53983002d36b27cddcf9743b13",
  previousHostQualificationCommit: "4369786fde888e1ec64fe6b05fbced39bc33090d",
  aggregateCheckpoint: "5514afe176a090aa3f1da9e5354799bb4ca5a8b3",
  bindingCheckerHardening: "1138506d2ac3ca5fc5d736c420e6b458c2fecc1d",
  historicalImplementationBaseline: "5f40e404ba4bc76c16eed15868c63a72e60e716c",
  selectedFe2o3Pin: "2d275684d7a22f8f913114b51b1d1dd524d1ed9b",
  qualifiedFe2o3Pin: "2d275684d7a22f8f913114b51b1d1dd524d1ed9b",
  qualifiedFe2o3Tree: "18cb8aa756d43a4425552e4bb2df467f48f54e13",
  previousQualifiedFe2o3Pin: "9f97985ee0a4a8ef0bc8f0fa0fd33771c8180592",
  qualifiedDriverSha256:
    "b34cd38ceb71f7d4fa96eae7d9d42de691a2a1112b2e3d8ced3810db3a507914",
  aggregatePortableMetadataSha256:
    "fd9422ca24e74cfa49ffe25beba04c976ed0d64d06f0388e2d4d466fff81f18a",
  aggregateCompilerBindingSha256:
    "242e6241a2c7f00b0a62aa52ca4008d3abe43416da0feeaf1970d6d1a7446902",
  historicalFe2o3Baseline: "b5374c6e6a4c1215ad481cefcd294334dcb1cbeb",
  repinState: "qualified",
  githubCiRun: "33490985105",
  githubCiState: "qualified",
  authenticatedReleaseRun: "33490985170",
  authenticatedReleaseState: "qualified",
  remoteRootAdapterState: "qualified",
  genericCoreState: "qualified",
  fallbackBindingParityState: "open",
  freshFe2o3QualificationState: "qualified",
  aggregateBindingState: "qualified",
  aggregateRuntimeMigrationState: "qualified",
  sourcePinExtractorState: "implemented",
  aggregateV2PublicationState: "open",
  protectedVerifierState: "open",
  currentQualificationState: "open",
  devicePackages: ["all-kernels"],
  repinCompilationTestValidatedDevicePackages: [
    "gemm",
    "logits",
    "paged-decode",
    "prefill",
    "rmsnorm",
    "rope-kv",
    "swiglu",
  ],
  generatedExpectations: 12,
  aggregateRosterCount: 1,
  aggregateProgramCount: 12,
  sourceGateModules: 151,
  sourceGateExecutableBodies: 6853,
  plannerSlots: 354,
  openM1Gates: 33,
});
const supersededProgress = Object.freeze({
  implementationCommit: "0c04ab7f94072eb6b763ffdcaa878af6e3c5a2f7",
  fe2o3Pin: "61967a3cb3958faddcda3a5e7ed6b19fd6e68ebb",
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
assertCommit(project.current.siteRefreshBase, "current.siteRefreshBase");
assertCommit(project.current.implementationCommit, "current.implementationCommit");
assertCommit(project.current.implementationTree, "current.implementationTree");
assertCommit(project.current.integrationBranchHead, "current.integrationBranchHead");
assertCommit(
  project.current.previousHostQualificationCommit,
  "current.previousHostQualificationCommit",
);
assertCommit(project.current.aggregateCheckpoint, "current.aggregateCheckpoint");
assertCommit(project.current.bindingCheckerHardening, "current.bindingCheckerHardening");
assertCommit(
  project.current.historicalImplementationBaseline,
  "current.historicalImplementationBaseline",
);
assertCommit(project.current.selectedFe2o3Pin, "current.selectedFe2o3Pin");
assertCommit(project.current.qualifiedFe2o3Pin, "current.qualifiedFe2o3Pin");
assertCommit(project.current.qualifiedFe2o3Tree, "current.qualifiedFe2o3Tree");
assertCommit(project.current.previousQualifiedFe2o3Pin, "current.previousQualifiedFe2o3Pin");
for (const key of [
  "qualifiedDriverSha256",
  "aggregatePortableMetadataSha256",
  "aggregateCompilerBindingSha256",
]) {
  assert(
    /^[0-9a-f]{64}$/.test(project.current[key]),
    `current.${key} must be a lowercase SHA-256 digest`,
  );
}
assertCommit(project.current.historicalFe2o3Baseline, "current.historicalFe2o3Baseline");
assertState(project.current.repinState, "current.repinState");
assert(/^\d+$/.test(project.current.githubCiRun), "current.githubCiRun must be numeric");
assertState(project.current.githubCiState, "current.githubCiState");
assert(
  /^\d+$/.test(project.current.authenticatedReleaseRun),
  "current.authenticatedReleaseRun must be numeric",
);
assertState(project.current.authenticatedReleaseState, "current.authenticatedReleaseState");
assertState(project.current.remoteRootAdapterState, "current.remoteRootAdapterState");
assertState(project.current.genericCoreState, "current.genericCoreState");
assertState(project.current.fallbackBindingParityState, "current.fallbackBindingParityState");
assertState(
  project.current.freshFe2o3QualificationState,
  "current.freshFe2o3QualificationState",
);
assertState(project.current.aggregateBindingState, "current.aggregateBindingState");
assertState(
  project.current.aggregateRuntimeMigrationState,
  "current.aggregateRuntimeMigrationState",
);
assertState(project.current.sourcePinExtractorState, "current.sourcePinExtractorState");
assertState(project.current.aggregateV2PublicationState, "current.aggregateV2PublicationState");
assertState(project.current.protectedVerifierState, "current.protectedVerifierState");
assertState(project.current.currentQualificationState, "current.currentQualificationState");
for (const [key, expected] of Object.entries(expectedCurrent)) {
  const actual = project.current[key];
  assert(
    JSON.stringify(actual) === JSON.stringify(expected),
    `current.${key} must match the selected implementation status`,
  );
}
assertState(project.milestone.state, "milestone");
assert(
    project.milestone.state === "integration" &&
    project.milestone.summary.includes("M1 remains incomplete") &&
    project.milestone.summary.includes(expectedCurrent.implementationCommit.slice(0, 8)) &&
    project.milestone.summary.includes(expectedCurrent.implementationTree.slice(0, 8)) &&
    project.milestone.summary.includes("M1AllKernelsWorkerV3RosterV1") &&
    project.milestone.summary.includes("[7,1,9,8,2,4,11,5,6,0,3,10]") &&
    project.milestone.summary.includes("151-module/6,853-body source gate") &&
    project.milestone.summary.includes("aggregate V2 publication is absent") &&
    project.milestone.summary.includes("MissingProtectedVerificationReceipt"),
  "milestone summary must preserve the one-roster architecture and open authority gates",
);

assert(Array.isArray(project.envelope) && project.envelope.length > 0, "envelope is empty");
const envelope = new Map(project.envelope);
assert(
  envelope.get("Qualified fe2o3 scope")?.includes(expectedCurrent.selectedFe2o3Pin) &&
    envelope.get("Qualified fe2o3 scope")?.includes(expectedCurrent.qualifiedFe2o3Tree) &&
    envelope.get("Qualified fe2o3 scope")?.includes("generic-core exited 0"),
  "envelope must expose the exact qualified fe2o3 commit, tree, and generic-core result",
);
assert(
  envelope.get("Historical fe2o3 baseline")?.includes(expectedCurrent.historicalFe2o3Baseline),
  "envelope must preserve the exact historical fe2o3 baseline",
);
assert(
  envelope.get("Current implementation")?.includes(expectedCurrent.integrationBranchHead) &&
    envelope.get("Current implementation")?.includes(expectedCurrent.implementationTree) &&
    envelope.get("Current implementation")?.includes("committed and pushed"),
  "envelope must expose the exact pushed current implementation",
);
assert(
  envelope.get("Current scoped Ferric checkpoint")?.startsWith("PASS") &&
    envelope.get("Current scoped Ferric checkpoint")?.includes(
      expectedCurrent.implementationCommit,
    ) &&
    envelope.get("Current scoped Ferric checkpoint")?.includes(
      expectedCurrent.implementationTree,
    ) &&
    envelope.get("Current scoped Ferric checkpoint")?.includes(
      "codex/m1-lineage-integration-v10",
    ) &&
    envelope.get("Current scoped Ferric checkpoint")?.includes("full tests") &&
    envelope.get("Current scoped Ferric checkpoint")?.includes("release closure"),
  "envelope must bind the pushed checkpoint to its scoped mi300x pass",
);
assert(
  envelope.get("Historical implementation baseline")?.includes(
    expectedCurrent.historicalImplementationBaseline,
  ),
  "envelope must preserve the exact historical implementation baseline",
);
assert(
  envelope.get("Qualified driver identity")?.includes(
    expectedCurrent.qualifiedDriverSha256,
  ),
  "envelope must expose the exact qualified driver identity",
);
assert(
  envelope.get("Two-root aggregate binding")?.includes(
    expectedCurrent.aggregatePortableMetadataSha256,
  ) &&
    envelope.get("Two-root aggregate binding")?.includes(
      expectedCurrent.aggregateCompilerBindingSha256,
    ) &&
    envelope.get("Two-root aggregate binding")?.includes("both roots") &&
    envelope.get("Two-root aggregate binding")?.includes("both checkers") &&
    envelope.get("Two-root aggregate binding")?.includes("10 tests passed"),
  "envelope must expose the exact two-root aggregate-binding qualification",
);
assert(
  envelope.get("GitHub CI")?.includes(expectedCurrent.githubCiRun) &&
    envelope.get("GitHub CI")?.includes("passed"),
  "envelope must expose the terminal GitHub CI pass",
);
assert(
  envelope.get("Authenticated release")?.includes(expectedCurrent.authenticatedReleaseRun) &&
    envelope.get("Authenticated release")?.startsWith("PASS:"),
  "envelope must expose the terminal authenticated release pass",
);
assert(
  envelope.get("Aggregate source checkpoint")?.includes(expectedCurrent.aggregateCheckpoint) &&
    envelope.get("Aggregate source checkpoint")?.includes("qualified source-only baseline") &&
    envelope.get("Aggregate source checkpoint")?.includes("predates runtime migration"),
  "envelope must scope the aggregate checkpoint away from runtime authority",
);
assert(
  envelope.get("Aggregate runtime migration")?.startsWith("QUALIFIED") &&
    envelope.get("Aggregate runtime migration")?.includes("M1AllKernelsWorkerV3RosterV1") &&
    envelope.get("Aggregate runtime migration")?.includes(
      "[7,1,9,8,2,4,11,5,6,0,3,10]",
    ) &&
    envelope.get("Aggregate runtime migration")?.includes(
      "No real artifact custody",
    ),
  "envelope must expose the exact one-roster architecture and its qualification boundary",
);
assert(
  envelope.get("Protected verifier backend")?.startsWith("OPEN:") &&
    envelope.get("Protected verifier backend")?.includes(
      "MissingProtectedVerificationReceipt",
    ) &&
    envelope.get("Protected verifier backend")?.includes("constructs no verification evidence"),
  "envelope must expose the protected verifier's unconditional fail-closed state",
);
assert(
  envelope.get("Current qualification")?.startsWith("OPEN") &&
    envelope.get("Current qualification")?.includes("fe2o3 2d275684") &&
    envelope.get("Current qualification")?.includes("aggregate V2 publication") &&
    envelope.get("Current qualification")?.includes("protected theorem backend") &&
    envelope.get("Current qualification")?.includes("no current protected verification"),
  "envelope must distinguish scoped qualification from the open Ferric authority path",
);
assert(
  envelope.get("Typed aggregate source-pin extraction")?.startsWith("IMPLEMENTED") &&
    envelope.get("Typed aggregate source-pin extraction")?.includes("tested") &&
    envelope.get("Typed aggregate source-pin extraction")?.includes(
      "does not publish the aggregate V2 artifact",
    ),
  "envelope must expose the implemented source-pin extractor without publication authority",
);
assert(
  envelope.get("Aggregate V2 publication")?.startsWith("OPEN:") &&
    envelope.get("Aggregate V2 publication")?.includes("no current"),
  "envelope must keep aggregate V2 publication explicitly open",
);
assert(
  envelope.get("Aggregate mi300x matrix")?.includes("direct tests") &&
    envelope.get("Aggregate mi300x matrix")?.includes("all seven compatibility suites") &&
    envelope.get("Aggregate mi300x matrix")?.includes("preparatory source ownership only"),
  "envelope must expose the scoped aggregate and compatibility validation",
);
assert(
  envelope.get("Corrected device matrix")?.includes("all seven exact") &&
    envelope.get("Corrected device matrix")?.includes("not fallback binding parity"),
  "envelope must scope the all-seven matrix away from fallback parity",
);
assert(
  envelope.get("Fallback binding parity")?.startsWith("OPEN:") &&
    envelope.get("Fallback binding parity")?.includes(
      expectedCurrent.bindingCheckerHardening,
    ) &&
    envelope.get("Fallback binding parity")?.includes("rejects mismatches"),
  "envelope must expose the fail-closed checker and open historical parity",
);
assert(Array.isArray(project.readiness) && project.readiness.length > 0, "readiness is empty");
project.readiness.forEach((item, index) =>
  assertState(item.state, `readiness[${index}]`),
);
const qwenReadiness = project.readiness.find(
  (item) => item.label === "End-to-end Qwen through Ferric",
);
assert(
  qwenReadiness?.state === "open" && qwenReadiness.detail.includes("cannot yet run Qwen"),
  "end-to-end Qwen must remain explicitly unrunnable",
);
const sourcePinReadiness = project.readiness.find(
  (item) => item.label === "Typed aggregate source-pin extraction",
);
assert(
  sourcePinReadiness?.state === "implemented" &&
    sourcePinReadiness.detail.includes("Pushed Ferric checkpoint b2da6062") &&
    sourcePinReadiness.detail.includes("no aggregate V2 publication exists") &&
    sourcePinReadiness.detail.includes("no theorem"),
  "source-pin extraction must remain implemented but authority-free",
);
const latestFe2o3Validation = project.readiness.find(
  (item) => item.label === "Latest fe2o3 device validation",
);
assert(
  latestFe2o3Validation?.state === "qualified" &&
    latestFe2o3Validation.detail.includes(expectedCurrent.qualifiedFe2o3Pin) &&
    latestFe2o3Validation.detail.includes(expectedCurrent.qualifiedFe2o3Tree) &&
    latestFe2o3Validation.detail.includes("10 tests passing") &&
    latestFe2o3Validation.detail.includes("not a protected runtime path"),
  "latest fe2o3 readiness must retain its exact scoped qualification boundary",
);

for (const group of ["runnable", "experimental", "roadmap"]) {
  assert(
    Array.isArray(project.capabilities[group]) && project.capabilities[group].length > 0,
    `capabilities.${group} is empty`,
  );
}

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
  if (validation.repository !== undefined) {
    assert(
      [project.repository, project.fe2o3Repository].includes(validation.repository),
      `validation.${key}.repository is not an approved source repository`,
    );
  }
  if (validation.closureSha256 !== undefined) {
    assert(
      /^[0-9a-f]{64}$/.test(validation.closureSha256),
      `validation.${key}.closureSha256 must be a lowercase SHA-256 digest`,
    );
  }
}
assert(
  project.validation.proof.state !== "qualified" ||
    typeof project.validation.proof.closureSha256 === "string",
  "qualified proof validation must bind a source closure digest",
);
assert(
  project.validation.host.state === "qualified" &&
    project.validation.host.source === expectedCurrent.implementationCommit &&
    project.validation.host.repository === project.repository &&
    project.validation.host.result.includes("b2da6062/c1309907") &&
    project.validation.host.result.includes("151/6853 source gate") &&
    project.validation.host.result.includes(
      "OPEN: aggregate V2 publication and protected theorem backend",
    ) &&
    project.validation.host.detail.includes(expectedCurrent.implementationCommit) &&
    project.validation.host.detail.includes(expectedCurrent.implementationTree) &&
    project.validation.host.detail.includes("Root formatting, check, strict clippy, full tests") &&
    project.validation.host.detail.includes("151-module/6,853-body source gate") &&
    project.validation.host.detail.includes("release closure") &&
    project.validation.host.detail.includes("354-slot planner") &&
    project.validation.host.detail.includes("No artifact custody, GPU execution, Qwen"),
  "current host validation must bind the qualified Ferric scope and retain open authority gates",
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
const progressByCommit = new Map();
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
  progressByCommit.set(item.commit, item);
});
assert(
  progressCommits.has(expectedCurrent.implementationCommit),
  "recent progress must include the current qualified implementation commit",
);
assert(
  progressCommits.has(expectedCurrent.integrationBranchHead),
  "recent progress must include the current pushed integration checkpoint",
);
assert(
  progressCommits.has(expectedCurrent.previousHostQualificationCommit),
  "recent progress must preserve the previous host qualification commit",
);
assert(
  progressCommits.has(expectedCurrent.aggregateCheckpoint),
  "recent progress must include the aggregate checkpoint",
);
assert(
  progressCommits.has(expectedCurrent.bindingCheckerHardening),
  "recent progress must include the binding-checker hardening",
);
assert(
  progressCommits.has(expectedCurrent.historicalImplementationBaseline),
  "recent progress must preserve the historical implementation baseline",
);
assert(
  progressCommits.has(expectedCurrent.selectedFe2o3Pin),
  "recent progress must include the active fe2o3 transition",
);
assert(
  progressCommits.has(expectedCurrent.historicalFe2o3Baseline),
  "recent progress must preserve the historical fe2o3 baseline",
);
const upstreamRosterHandoff = progressByCommit.get(
  "62e527c960b40716290ba8cb82ba5594be4f3706",
);
const aggregateCheckpoint = progressByCommit.get(expectedCurrent.aggregateCheckpoint);
assert(
  aggregateCheckpoint?.detail.includes("all 12 attributed Qwen roots") &&
    aggregateCheckpoint.detail.includes("all seven compatibility suites") &&
    aggregateCheckpoint.detail.includes("last qualified aggregate source-only baseline") &&
    aggregateCheckpoint.detail.includes("predates the current one-roster runtime migration"),
  "aggregate progress must retain its preparatory authority boundary",
);
const integrationBranchHead = progressByCommit.get(expectedCurrent.integrationBranchHead);
assert(
  (integrationBranchHead?.repository === undefined ||
    integrationBranchHead.repository === project.repository) &&
    integrationBranchHead.state === "qualified" &&
    integrationBranchHead.detail.includes(expectedCurrent.implementationTree) &&
    integrationBranchHead.detail.includes("12-program roster") &&
    integrationBranchHead.detail.includes("[7,1,9,8,2,4,11,5,6,0,3,10]") &&
    integrationBranchHead.detail.includes("151-module/6,853-body source gate") &&
    integrationBranchHead.detail.includes("Aggregate V2 publication") &&
    integrationBranchHead.detail.includes("GPU execution, Qwen, and M1 remain open"),
  "integration progress must bind the exact pushed checkpoint and open production gates",
);
const selectedFe2o3 = progressByCommit.get(expectedCurrent.selectedFe2o3Pin);
assert(
  selectedFe2o3?.repository === project.fe2o3Repository &&
    selectedFe2o3.state === "qualified" &&
    selectedFe2o3.detail.includes(expectedCurrent.qualifiedFe2o3Tree) &&
    selectedFe2o3.detail.includes("generic-core with exit 0") &&
    selectedFe2o3.detail.includes("two-root aggregate-binding lane") &&
    selectedFe2o3.detail.includes("aggregate V2 artifact") &&
    selectedFe2o3.detail.includes("protected theorem result"),
  "selected fe2o3 progress must bind the scoped qualification and downstream boundary",
);
const bindingCheckerHardening = progressByCommit.get(
  expectedCurrent.bindingCheckerHardening,
);
assert(
  bindingCheckerHardening?.detail.includes("fails on any mismatch") &&
    bindingCheckerHardening.detail.includes("still require regeneration"),
  "binding hardening must not overclaim historical family parity",
);
assert(
  upstreamRosterHandoff?.repository === project.fe2o3Repository &&
    upstreamRosterHandoff.state === "observed" &&
    upstreamRosterHandoff.detail.includes("superseded in the active Ferric selection") &&
    upstreamRosterHandoff.detail.includes("2d275684"),
  "superseded upstream roster handoff must remain historical progress only",
);
for (const [label, commit] of Object.entries(supersededProgress)) {
  const item = progressByCommit.get(commit);
  assert(
    item?.state === "observed" &&
      item.title.startsWith("Superseded:") &&
      item.detail.toLowerCase().includes("supersed"),
    `superseded ${label} must remain historical progress only`,
  );
}

project.evidence.gates.forEach(([label, count, state], index) => {
  assert(label && /^\d+$/.test(count), `evidence.gates[${index}] is malformed`);
  assertState(state, `evidence.gates[${index}]`);
});
const roadmapGate = project.evidence.gates.find(([label]) => label === "Roadmap requirements");
assert(
  roadmapGate?.[1] === String(expectedCurrent.openM1Gates) && roadmapGate?.[2] === "open",
  "the exact M1 roadmap gate count must remain open",
);
project.evidence.legend.forEach(([state], index) =>
  assertState(state, `evidence.legend[${index}]`),
);

const html = await readFile(join(siteRoot, "index.html"), "utf8");
assert(
  html.includes("Ferric cannot run Qwen through the production path") &&
    html.includes("Pushed Ferric checkpoint b2da6062, tree") &&
    html.includes("151-module/6,853-body source gate") &&
    html.includes("Exact fe2o3 2d275684, tree") &&
    html.includes("aggregate V2 publication is absent") &&
    html.includes("no Qwen or M1"),
  "static checkpoint copy must preserve current qualification facts and open Qwen/M1 status",
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
