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
  siteRefreshBase: "e187ca52dfdaee79fdc17921c9acffebeed6ca96",
  implementationCommit: "e187ca52dfdaee79fdc17921c9acffebeed6ca96",
  authenticatedR32Commit: "d67fae3b063b1997aaa92b0cbc6f4c960c3b010b",
  aggregateSelectionCommit: "eceffdf00c1ec0f7241be95d6b636fa1ea69a46d",
  aggregateSelectionStatus: "noncurrent-candidate",
  pendingVerifierProjectionCommit: "75c5f724fbc7928bf1b231a86aec0f1d5fdcc3f9",
  commonCustodyPreflightCommit: "e187ca52dfdaee79fdc17921c9acffebeed6ca96",
  selectedFe2o3Pin: "52815c9ed52a3075e26322cf506144cb22da12d2",
  aggregateSourceCommit: "5514afe176a090aa3f1da9e5354799bb4ca5a8b3",
  aggregateProducerCommit: "e57c42523050922ad76538150df691cc5ab975a7",
  aggregateKernelCount: 12,
  diagnosticBridgeCommit: "24748e11358db7ad3ab5fe35992cff354896e607",
  diagnosticStatus: "partial-non-evidence",
  diagnosticDispatchGeneration: 1,
  diagnosticCopyCount: 5,
  proofQueries: 1493,
  directVerifiedBodies: 645,
  sourceGateModules: 151,
  sourceGateBodies: 6916,
  sourceClosureFiles: 603,
  openM1Gates: 33,
});
const expectedProof = Object.freeze({
  source: "e187ca52dfdaee79fdc17921c9acffebeed6ca96",
  closureSha256:
    "4920f55d8c98681e6ee154b8d5bba64f80d17241e89e07505626f9f365a8a2e2",
  receiptSha256:
    "fbdb2ad3f3acdf9f46480be16e993cee2ededf548be22e6e35b787749ed65d21",
  logSha256:
    "1a5bb9049f496d1f74f4233147b4a72b588333d4dfbf30ae1658fcb0d67c47fa",
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
assertCommit(project.current.selectedFe2o3Pin, "current.selectedFe2o3Pin");
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
  envelope.get("Selected fe2o3 pin")?.includes(expectedCurrent.selectedFe2o3Pin),
  "envelope must expose the exact selected fe2o3 pin",
);
assert(
  envelope.get("M1 implementation")?.includes(expectedCurrent.implementationCommit),
  "envelope must expose the exact current implementation commit",
);
assert(
  envelope.get("Common-custody preflight")?.includes(
    "sole terminal result MissingProtectedVerificationReceipt",
  ),
  "envelope must expose the reject-only common-custody preflight",
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
const commonCustodyReadiness = project.readiness.find(
  (item) => item.label === "Reject-only common-custody verifier preflight",
);
assert(
  commonCustodyReadiness?.state === "integration" &&
    commonCustodyReadiness.detail.includes(
      "independently revalidate the exact-pinned finalizer derivation from the borrowed replay",
    ) &&
    commonCustodyReadiness.detail.includes(
      "validate common multi-root compiler proof inputs",
    ) &&
    commonCustodyReadiness.detail.includes(
      "validate common multi-root target lineage by borrowing those proof inputs",
    ) &&
    commonCustodyReadiness.detail.includes("MissingProtectedVerificationReceipt") &&
    commonCustodyReadiness.detail.includes("grants no protected, load, launch, hardware, Qwen"),
  "common-custody preflight must preserve its exact order, terminal rejection, and nonclaims",
);
const protectedAcceptance = project.readiness.find(
  (item) => item.label === "Accepting protected aggregate artifact",
);
assert(
  protectedAcceptance?.state === "open" &&
    protectedAcceptance.detail.includes("remains None") &&
    protectedAcceptance.detail.includes(
      "sole terminal result MissingProtectedVerificationReceipt",
    ) &&
    protectedAcceptance.detail.includes("earlier preflight failures return their distinct"),
  "protected aggregate acceptance must remain fail-closed and open",
);
const qwenReadiness = project.readiness.find(
  (item) => item.label === "End-to-end Qwen through Ferric",
);
assert(
  qwenReadiness?.state === "open" &&
    qwenReadiness.detail.includes(
      "no authenticated full-Qwen execution, numerical result, or performance result",
    ),
  "Qwen, numerical, and performance authority must remain open",
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
  project.validation.proof.detail.includes("33599537169") &&
    project.validation.proof.detail.includes("33599537184") &&
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
  progressCommits.has(expectedCurrent.selectedFe2o3Pin),
  "recent progress must include the selected fe2o3 pin",
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
assert(
  project.evidence.gates.every(([, , state]) => state === "open"),
  "every M1 closure roster remains open without its required evidence",
);
project.evidence.legend.forEach(([state], index) =>
  assertState(state, `evidence.legend[${index}]`),
);

const html = await readFile(join(siteRoot, "index.html"), "utf8");
for (const claim of [
  "Exact head e187ca5 passed strict proof and release qualification on mi300x",
  "33599537169",
  "33599537184 both completed successfully",
  "independently revalidates the exact-pinned finalizer derivation",
  "multi-root proof inputs",
  "target lineage by borrowing those inputs",
  "passing preflight has the sole terminal result MissingProtectedVerificationReceipt",
  "private current aggregate publication selection",
  "successful current-source R32 trace",
  "all 33 M1 exit gates remain open",
]) {
  assert(html.includes(claim), `index.html is missing current claim: ${claim}`);
}
assert(
  dataSource.includes("e187ca52dfdaee79fdc17921c9acffebeed6ca96"),
  "Pages data must bind the exact qualified common-custody preflight commit",
);
assert(
  dataSource.includes("sole terminal result MissingProtectedVerificationReceipt") &&
    dataSource.includes("private current aggregate publication selection remains None") &&
    dataSource.includes("all 33 M1 exit gates remain open"),
  "Pages data must retain terminal rejection, None selection, and all-open gate nonclaims",
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
