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
  siteRefreshBase: "e419160a3d21db5e8b25f414fd696982a959a171",
  implementationCommit: "5f40e404ba4bc76c16eed15868c63a72e60e716c",
  selectedFe2o3Pin: "b5374c6e6a4c1215ad481cefcd294334dcb1cbeb",
  devicePackages: [
    "gemm",
    "logits",
    "paged-decode",
    "prefill",
    "rmsnorm",
    "rope-kv",
    "swiglu",
  ],
  generatedExpectations: 12,
  sourceGateModules: 151,
  sourceGateExecutableBodies: 6850,
  plannerSlots: 354,
  openM1Gates: 33,
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
assertCommit(project.current.selectedFe2o3Pin, "current.selectedFe2o3Pin");
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
assert(Array.isArray(project.readiness) && project.readiness.length > 0, "readiness is empty");
project.readiness.forEach((item, index) =>
  assertState(item.state, `readiness[${index}]`),
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
