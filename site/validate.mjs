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
  siteRefreshBase: "3ee14ea6a38b84b74166ca4e3254050c12c77a56",
  implementationCommit: "7f516e073b8759eb012c998bc9df2eb101d0c7ab",
  authenticatedR32Commit: "d67fae3b063b1997aaa92b0cbc6f4c960c3b010b",
  aggregateSelectionCommit: "eceffdf00c1ec0f7241be95d6b636fa1ea69a46d",
  aggregateSelectionStatus: "noncurrent-candidate",
  pendingVerifierProjectionCommit: "75c5f724fbc7928bf1b231a86aec0f1d5fdcc3f9",
  commonCustodyPreflightCommit: "e187ca52dfdaee79fdc17921c9acffebeed6ca96",
  associationPreflightCommit: "eb3b1937ec509cb6ecea080a25965dd3e8bc5457",
  finalizedHsacoReinspectionCommit: "749324c9e287aaec688c8733c88becddc539b12e",
  fe2o3DeadlineCandidate: "ff21f24f5349d78583a2a832ba3aa37bf3e0846c",
  fe2o3DeadlineCandidateTree: "861ad57c9725d06a5bed14739269ddd20b70e86a",
  fe2o3DeadlineCandidateBase: "308d8fa00fa41e098b2a1a47bbfea1bc29735464",
  productionSpeculativeExecutorCandidate: "f300ab8b174ff4e71d5d5fdaf038741db159907e",
  productionSpeculativeExecutorStatus: "no-go-remediation",
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
assertCommit(
  project.current.fe2o3DeadlineCandidate,
  "current.fe2o3DeadlineCandidate",
);
assertCommit(
  project.current.fe2o3DeadlineCandidateTree,
  "current.fe2o3DeadlineCandidateTree",
);
assertCommit(
  project.current.fe2o3DeadlineCandidateBase,
  "current.fe2o3DeadlineCandidateBase",
);
assertCommit(
  project.current.productionSpeculativeExecutorCandidate,
  "current.productionSpeculativeExecutorCandidate",
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

assert(Array.isArray(project.envelope) && project.envelope.length > 0, "envelope is empty");
const envelope = new Map(project.envelope);
assert(
  envelope.get("Qualified fe2o3 candidate")?.includes(expectedCurrent.fe2o3DeadlineCandidate) &&
    envelope.get("Qualified fe2o3 candidate")?.includes(expectedCurrent.fe2o3DeadlineCandidateTree) &&
    envelope.get("Qualified fe2o3 candidate")?.includes(expectedCurrent.fe2o3DeadlineCandidateBase),
  "envelope must expose the exact qualified fe2o3 candidate, tree, and main base",
);
assert(
  envelope.get("Speculative executor candidate")?.includes(
    expectedCurrent.productionSpeculativeExecutorCandidate,
  ) && envelope.get("Speculative executor candidate")?.includes("NO-GO"),
  "envelope must expose the exact speculative executor candidate as NO-GO",
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
  envelope.get("M1 implementation")?.includes(expectedCurrent.implementationCommit),
  "envelope must expose the exact current implementation commit",
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
const deadlineReadiness = project.readiness.find(
  (item) => item.label === "fe2o3 absolute-deadline Worker V3 candidate",
);
assert(
  deadlineReadiness?.state === "qualified" &&
    deadlineReadiness.detail.includes(expectedCurrent.fe2o3DeadlineCandidate) &&
    deadlineReadiness.detail.includes(expectedCurrent.fe2o3DeadlineCandidateTree) &&
    deadlineReadiness.detail.includes(expectedCurrent.fe2o3DeadlineCandidateBase) &&
    deadlineReadiness.detail.includes("exact-archive matrix on mi300x") &&
    deadlineReadiness.detail.includes("not yet Ferric's integrated dependency"),
  "fe2o3 deadline candidate must retain exact qualification and integration limits",
);
const executorReadiness = project.readiness.find(
  (item) => item.label === "Production speculative executor",
);
assert(
  executorReadiness?.state === "integration" &&
    executorReadiness.detail.includes(expectedCurrent.productionSpeculativeExecutorCandidate) &&
    executorReadiness.detail.includes("492 library tests and 136 doctests") &&
    executorReadiness.detail.includes("four P1 custody and lifecycle escapes") &&
    executorReadiness.detail.includes("explicitly NO-GO") &&
    executorReadiness.detail.includes("not integrated production code"),
  "speculative executor candidate must remain an explicit independently reviewed NO-GO",
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
    qwenReadiness.detail.includes("only a local foundation") &&
    qwenReadiness.detail.includes("canonical prepack result is a non-final probe") &&
    qwenReadiness.detail.includes(
      "no authenticated current-source Qwen execution, hardware run, numerical result, or performance result",
    ),
  "Qwen, numerical, and performance authority must remain open",
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
assert(
  progressCommits.has(expectedCurrent.implementationCommit),
  "recent progress must include the current implementation commit",
);
assert(
  progressCommits.has(expectedCurrent.authenticatedR32Commit),
  "recent progress must include the authenticated R32 implementation commit",
);
assert(
  progressCommits.has(expectedCurrent.fe2o3DeadlineCandidate),
  "recent progress must include the qualified fe2o3 absolute-deadline candidate",
);
assert(
  progressCommits.has(expectedCurrent.productionSpeculativeExecutorCandidate),
  "recent progress must include the independently rejected executor candidate",
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
for (const claim of [
  "Public fe2o3 candidate ff21f24, tree 861ad57c",
  "based on latest main 308d8fa",
  "passed exact-archive qualification on mi300x",
  "one caller-supplied absolute deadline through Worker V3 V2",
  "safely destroys or releases unpublished prepared queues",
  "Ferric has not completed its dependency repin",
  "Speculative executor candidate f300ab8 passed 492 library tests and 136 doctests",
  "independent re-review found four P1 custody and lifecycle escapes",
  "explicitly NO-GO and remediation is in progress",
  "Local protected-verifier service candidate 9a435522 is not publicly linked",
  "independent foundation GO",
  "it is not deployed",
  "real protected current, checker, signer, head-store, supervisor, and IPC facilities",
  "Binder candidate 6846d92, tree 4690d8c, passed its exact-archive mi300x matrix",
  "independent review returned GO with no P0, P1, or P2 findings",
  "integrated locally into the M1 branch at ed708de",
  "not public main or deployed authority",
  "Ferric-specific inference and kernel ownership remain in Ferric",
  "selection remains None",
  "CURRENT=None",
  "No authenticated current-source Qwen, hardware, numerical, or performance run exists",
  "All 33 M1 roadmap gates and all 17 assurance properties remain Open",
]) {
  assert(normalizedHtml.includes(claim), `index.html is missing current claim: ${claim}`);
}
assert(
  dataSource.includes("7f516e073b8759eb012c998bc9df2eb101d0c7ab") &&
    dataSource.includes("749324c9e287aaec688c8733c88becddc539b12e") &&
    dataSource.includes("eb3b1937ec509cb6ecea080a25965dd3e8bc5457") &&
    dataSource.includes("e187ca52dfdaee79fdc17921c9acffebeed6ca96") &&
    dataSource.includes("24748e11358db7ad3ab5fe35992cff354896e607") &&
    dataSource.includes(expectedCurrent.fe2o3DeadlineCandidate) &&
    dataSource.includes(expectedCurrent.fe2o3DeadlineCandidateTree) &&
    dataSource.includes(expectedCurrent.fe2o3DeadlineCandidateBase) &&
    dataSource.includes(expectedCurrent.productionSpeculativeExecutorCandidate) &&
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
    dataSource.includes("passed 492 library tests and 136 doctests") &&
    dataSource.includes("four P1 custody and lifecycle escapes") &&
    dataSource.includes("explicitly NO-GO") &&
    dataSource.includes("passed 28 tests and 6 doctests") &&
    dataSource.includes("independent review returned GO with no P0, P1, or P2 findings") &&
    dataSource.includes("not public main or deployed authority") &&
    dataSource.includes("not deployed") &&
    dataSource.includes("no authenticated current-source Qwen") &&
    dataSource.includes("All 33 M1 roadmap gates and all 17 assurance properties remain Open"),
  "Pages data must retain service, executor, Qwen, selection, and all-open nonclaims",
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
