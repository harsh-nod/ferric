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
  siteRefreshBase: "709212109a5d177e581002f0cc8502afa703e3ed",
  implementationCommit: "4369786fde888e1ec64fe6b05fbced39bc33090d",
  integrationBranchHead: "c021f707df4bf1cbfd17a7a9eaf384a30c783ef0",
  aggregateCheckpoint: "5514afe176a090aa3f1da9e5354799bb4ca5a8b3",
  bindingCheckerHardening: "1138506d2ac3ca5fc5d736c420e6b458c2fecc1d",
  historicalImplementationBaseline: "5f40e404ba4bc76c16eed15868c63a72e60e716c",
  selectedFe2o3Pin: "5978cefb2c4b4ce2600ea7af8294b1bab5685ea5",
  qualifiedFe2o3Pin: "9f97985ee0a4a8ef0bc8f0fa0fd33771c8180592",
  historicalFe2o3Baseline: "b5374c6e6a4c1215ad481cefcd294334dcb1cbeb",
  repinState: "integration",
  githubCiRun: "33490985105",
  githubCiState: "qualified",
  authenticatedReleaseRun: "33490985170",
  authenticatedReleaseState: "qualified",
  remoteRootAdapterState: "qualified",
  genericCoreState: "integration",
  fallbackBindingParityState: "open",
  freshFe2o3QualificationState: "integration",
  aggregateRuntimeMigrationState: "integration",
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
  sourceGateExecutableBodies: 6850,
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
assertCommit(project.current.integrationBranchHead, "current.integrationBranchHead");
assertCommit(project.current.aggregateCheckpoint, "current.aggregateCheckpoint");
assertCommit(project.current.bindingCheckerHardening, "current.bindingCheckerHardening");
assertCommit(
  project.current.historicalImplementationBaseline,
  "current.historicalImplementationBaseline",
);
assertCommit(project.current.selectedFe2o3Pin, "current.selectedFe2o3Pin");
assertCommit(project.current.qualifiedFe2o3Pin, "current.qualifiedFe2o3Pin");
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
assertState(
  project.current.aggregateRuntimeMigrationState,
  "current.aggregateRuntimeMigrationState",
);
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

assert(Array.isArray(project.envelope) && project.envelope.length > 0, "envelope is empty");
const envelope = new Map(project.envelope);
assert(
  envelope.get("Active fe2o3 transition")?.includes(expectedCurrent.selectedFe2o3Pin),
  "envelope must expose the exact active fe2o3 transition",
);
assert(
  envelope.get("Historical fe2o3 baseline")?.includes(expectedCurrent.historicalFe2o3Baseline),
  "envelope must preserve the exact historical fe2o3 baseline",
);
assert(
  envelope.get("Current implementation")?.includes(expectedCurrent.integrationBranchHead) &&
    envelope.get("Current implementation")?.includes("uncommitted integration work"),
  "envelope must expose the current integration base without presenting dirty work as a commit",
);
assert(
  envelope.get("Historical implementation baseline")?.includes(
    expectedCurrent.historicalImplementationBaseline,
  ),
  "envelope must preserve the exact historical implementation baseline",
);
assert(
  envelope.get("Active fe2o3 transition")?.includes(
    expectedCurrent.selectedFe2o3Pin,
  ) && envelope.get("Active fe2o3 transition")?.includes("remain pending"),
  "active fe2o3 transition must identify the selected pin and pending qualification",
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
  envelope.get("Aggregate runtime migration")?.startsWith("INTEGRATION:") &&
    envelope.get("Aggregate runtime migration")?.includes("M1AllKernelsWorkerV3RosterV1") &&
    envelope.get("Aggregate runtime migration")?.includes(
      "[2,0,10,1,5,3,7,6,4,11,8,9]",
    ) &&
    envelope.get("Aggregate runtime migration")?.includes(
      "No terminal exact-state qualification",
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
    envelope.get("Current qualification")?.includes("4369786f/9f97985e") &&
    envelope.get("Current qualification")?.includes("5514afe/9f97985e") &&
    envelope.get("Current qualification")?.includes("neither establishes current"),
  "envelope must distinguish historical qualified scopes from current authority",
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
  project.validation.host.state === "integration" &&
    project.validation.host.source === expectedCurrent.aggregateCheckpoint &&
    project.validation.host.result.includes("historical 436/9f") &&
    project.validation.host.result.includes("one roster and fe2o3 5978") &&
    project.validation.host.result.includes(
      "OPEN: exact-state qualification and protected receipt",
    ),
  "current host validation must distinguish historical passes from open one-roster qualification",
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
  "recent progress must preserve the last terminal implementation commit",
);
assert(
  progressCommits.has(expectedCurrent.integrationBranchHead),
  "recent progress must include the current integration branch base",
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
    integrationBranchHead.state === "integration" &&
    integrationBranchHead.detail.includes("Uncommitted work above it") &&
    integrationBranchHead.detail.includes("12-program roster") &&
    integrationBranchHead.detail.includes("no terminal exact-state qualification"),
  "integration progress must separate the pushed base from current dirty one-roster work",
);
const selectedFe2o3 = progressByCommit.get(expectedCurrent.selectedFe2o3Pin);
assert(
  selectedFe2o3?.repository === project.fe2o3Repository &&
    selectedFe2o3.state === "integration" &&
    selectedFe2o3.detail.includes("selected by the active Ferric integration source") &&
    selectedFe2o3.detail.includes("qualification at this pin remains pending"),
  "selected fe2o3 progress must retain its pending-qualification boundary",
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
    upstreamRosterHandoff.detail.includes("5978cefb"),
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
