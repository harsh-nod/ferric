import { access, readFile } from "node:fs/promises";
import { constants } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const siteRoot = dirname(fileURLToPath(import.meta.url));
const dataSource = await readFile(join(siteRoot, "data/project.js"), "utf8");
const context = { window: {} };
vm.runInNewContext(dataSource, context, { filename: "site/data/project.js" });
const project = context.window.FERRIC_PROJECT;

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function assertExactKeys(value, keys, location) {
  assert(value && typeof value === "object", `${location} must be an object`);
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  assert(
    JSON.stringify(actual) === JSON.stringify(expected),
    `${location} keys drifted: ${actual.join(", ")}`,
  );
}

function assertCommit(value, location) {
  assert(
    typeof value === "string" && /^[0-9a-f]{40}$/.test(value),
    `${location} must be an exact lowercase 40-character commit`,
  );
}

const states = new Set([
  "implemented",
  "integration",
  "observed",
  "verified",
  "qualified",
  "open",
]);

function assertState(value, location) {
  assert(states.has(value), `${location} has unknown state ${value}`);
}

assertExactKeys(
  project,
  [
    "updated",
    "repository",
    "fe2o3Repository",
    "current",
    "milestone",
    "readiness",
    "envelope",
    "capabilities",
    "validation",
    "teams",
    "boundaries",
    "latestObservation",
    "recentProgress",
    "evidence",
  ],
  "project",
);
assert(/^\d{4}-\d{2}-\d{2}$/.test(project.updated), "updated must use YYYY-MM-DD");
assert(project.repository === "https://github.com/harsh-nod/ferric", "Ferric repository drifted");
assert(project.fe2o3Repository === "https://github.com/harsh-nod/fe2o3", "fe2o3 repository drifted");

const expectedCurrent = {
  siteRefreshBase: "0e683a68037138d0d92cd88e10be03b405d3402d",
  integrationCommit: "78c720551c4b9809ae7e75ba639959ae0d968b1d",
  integrationTree: "685fb93ef9c7ddcb27aed761be0b757cef130c6f",
  r33LifecycleCommit: "eebdb38dab764143b023b33311363a452a1238ee",
  prefillProofSourceCommit: "240bb3d1ce394436cc62244f51888d7737ea6b9c",
  prefillProofIntegratedCommit: "f709a5da2f0130087bcfc0605a4c84ebd966ea0c",
  directReadbackSourceCommit: "18eed253d30b40a23f3984d7249d24b4db7318d2",
  fe2o3LatestMain: "dd802ce4fc5f759a49cb655ed530af664fe4bc61",
  formalVerified: 81,
  formalErrors: 0,
  proofTestsPassed: 26,
  sourceGateTestsPassed: 28,
  scopedUnadmittedRuntimeBodies: 23,
  combinedInventoryCurrent: false,
  r33AdapterTestsPassed: 55,
  r33AdapterHardwareIgnored: 2,
  r33AdapterDoctestsPassed: 3,
  openM1Gates: 33,
  currentAggregateHsaco: false,
  qwenTokenObserved: false,
  servingEndpointAvailable: false,
};
assertExactKeys(project.current, Object.keys(expectedCurrent), "current");
for (const [key, value] of Object.entries(expectedCurrent)) {
  assert(project.current[key] === value, `current.${key} drifted`);
}
for (const key of [
  "siteRefreshBase",
  "integrationCommit",
  "integrationTree",
  "r33LifecycleCommit",
  "prefillProofSourceCommit",
  "prefillProofIntegratedCommit",
  "directReadbackSourceCommit",
  "fe2o3LatestMain",
]) {
  assertCommit(project.current[key], `current.${key}`);
}
assert(
  project.current.scopedUnadmittedRuntimeBodies === 23,
  "scoped bootstrap runtime admission drift must remain explicit",
);
assert(project.current.combinedInventoryCurrent === false, "site must not claim current combined inventory closure");
assert(project.current.formalErrors === 0, "strict Verus errors must remain zero");
assert(project.current.openM1Gates === 33, "all 33 M1 exit gates remain open");
assert(project.current.currentAggregateHsaco === false, "site must not claim a current HSACO");
assert(project.current.qwenTokenObserved === false, "site must not claim a Qwen token");
assert(project.current.servingEndpointAvailable === false, "site must not claim serving");

assertExactKeys(project.milestone, ["name", "label", "state", "summary"], "milestone");
assert(project.milestone.name === "M1", "milestone must remain M1");
assertState(project.milestone.state, "milestone.state");

assert(Array.isArray(project.readiness) && project.readiness.length >= 5, "readiness roster is incomplete");
project.readiness.forEach((item, index) => {
  assertExactKeys(item, ["label", "state", "detail"], `readiness[${index}]`);
  assertState(item.state, `readiness[${index}].state`);
});

assert(Array.isArray(project.envelope) && project.envelope.length >= 8, "M1 envelope is incomplete");
const envelopeNames = new Set();
project.envelope.forEach((entry, index) => {
  assert(Array.isArray(entry) && entry.length === 2, `envelope[${index}] must be a pair`);
  assert(!envelopeNames.has(entry[0]), `duplicate envelope label ${entry[0]}`);
  envelopeNames.add(entry[0]);
});

assertExactKeys(project.capabilities, ["runnable", "experimental", "roadmap"], "capabilities");
for (const group of ["runnable", "experimental", "roadmap"]) {
  assert(Array.isArray(project.capabilities[group]) && project.capabilities[group].length > 0, `${group} is empty`);
  project.capabilities[group].forEach((item, index) => {
    assertExactKeys(item, ["name", "detail"], `capabilities.${group}[${index}]`);
  });
}

assertExactKeys(project.validation, ["host", "proof", "hardware", "transitions", "limitation"], "validation");
for (const kind of ["host", "proof", "hardware"]) {
  const item = project.validation[kind];
  const sourceKey = kind === "hardware" ? "sourceStatus" : "source";
  assertExactKeys(item, ["title", "state", sourceKey, "result", "detail"], `validation.${kind}`);
  assertState(item.state, `validation.${kind}.state`);
  if (item.source) assertCommit(item.source, `validation.${kind}.source`);
}
const transitionKeys = new Set();
project.validation.transitions.forEach((entry, index) => {
  assert(Array.isArray(entry) && entry.length === 3, `transition[${index}] must have three fields`);
  assertState(entry[2], `transition[${index}].state`);
  const key = `${entry[0]} -> ${entry[1]}`;
  assert(!transitionKeys.has(key), `duplicate transition ${key}`);
  transitionKeys.add(key);
});

assert(Array.isArray(project.teams) && project.teams.length === 4, "team ledger must contain four teams");
const teamNames = new Set();
project.teams.forEach((team, index) => {
  assertExactKeys(
    team,
    ["name", "scope", "state", "status", "completed", "current", "blockedBy", "next", "validation"],
    `teams[${index}]`,
  );
  assertState(team.state, `teams[${index}].state`);
  assert(!teamNames.has(team.name), `duplicate team ${team.name}`);
  assert(team.status.includes("no team blocker"), `${team.name} must report current blocker state`);
  assert(team.blockedBy.startsWith("No team-local blocker."), `${team.name} dependencies must remain explicit`);
  teamNames.add(team.name);
});

assertExactKeys(project.boundaries, ["ferric", "fe2o3"], "boundaries");
for (const key of ["ferric", "fe2o3"]) {
  assert(Array.isArray(project.boundaries[key]) && project.boundaries[key].length >= 5, `${key} boundary is incomplete`);
}

assertExactKeys(
  project.latestObservation,
  ["title", "state", "sourceStatus", "environment", "result", "buildId", "generatedTokenIds", "authority"],
  "latestObservation",
);
assertState(project.latestObservation.state, "latestObservation.state");
assert(project.latestObservation.state === "open", "a failed exact compile is not a hardware observation");
assert(!("commit" in project.latestObservation), "unintegrated kernel v27 must not claim a source commit");
assert(project.latestObservation.generatedTokenIds.length === 0, "no Qwen token has been observed");

assert(Array.isArray(project.recentProgress) && project.recentProgress.length >= 4, "progress ledger is incomplete");
project.recentProgress.forEach((item, index) => {
  const expected = item.repository
    ? ["commit", "repository", "title", "state", "detail"]
    : ["commit", "title", "state", "detail"];
  assertExactKeys(item, expected, `recentProgress[${index}]`);
  assertCommit(item.commit, `recentProgress[${index}].commit`);
  assertState(item.state, `recentProgress[${index}].state`);
});

assertExactKeys(project.evidence, ["summary", "legend", "gates"], "evidence");
assert(project.evidence.gates.length >= 6, "evidence gate roster is incomplete");
project.evidence.gates.forEach((entry, index) => {
  assert(Array.isArray(entry) && entry.length === 3, `evidence.gates[${index}] must be a triple`);
  assertState(entry[2], `evidence.gates[${index}].state`);
  assert(entry[2] === "open", `evidence gate ${entry[0]} must remain open`);
});
project.evidence.legend.forEach((entry, index) => {
  assert(Array.isArray(entry) && entry.length === 2, `evidence.legend[${index}] must be a pair`);
  assertState(entry[0], `evidence.legend[${index}].state`);
});

const snapshot = JSON.stringify(project);
for (const claim of [
  "78c720551c4b9809ae7e75ba639959ae0d968b1d",
  "eebdb38dab764143b023b33311363a452a1238ee",
  "240bb3d1ce394436cc62244f51888d7737ea6b9c",
  "f709a5da2f0130087bcfc0605a4c84ebd966ea0c",
  "18eed253d30b40a23f3984d7249d24b4db7318d2",
  "dd802ce4fc5f759a49cb655ed530af664fe4bc61",
  "23 unadmitted bootstrap runtime bodies",
  "no current whole-tree",
  "strict Verus 81 verified / 0 errors",
  "proof tests 26/26",
  "source gate passes 28/28",
  "55 passed, 2 hardware-dependent ignored, 3 compile-fail doctests",
  "outside the root-workspace formal inventory",
  "Focused v27",
  "deterministic scalar-reachability work bound",
  "Diagnostic-only measurement is running",
  "no HSACO",
  "production paired-prefill executor is in progress",
  "All 33 M1 exit gates remain open",
  "No Qwen token",
  "TTFT",
  "TPOT",
  "vLLM/SGLang baseline",
]) {
  assert(snapshot.includes(claim), `current snapshot is missing claim: ${claim}`);
}
for (const staleOrForbidden of [
  "6,854 admitted",
  "97 unadmitted",
  "7,632 executable bodies",
  "681 directly verified",
  "6,951 explicitly unverified",
  "0 unadmitted",
  "0 stale",
  "focused v20",
  "Focused v26",
  "v21's single-SSA-local fix",
  "production R33 backend remain absent",
  "Qwen serving is ready",
  "bootstrap remains under review",
  "bootstrap is not integrated",
  "current aggregate HSACO is ready",
]) {
  assert(!snapshot.includes(staleOrForbidden), `stale or forbidden claim remains: ${staleOrForbidden}`);
}

const indexSource = await readFile(join(siteRoot, "index.html"), "utf8");
const appSource = await readFile(join(siteRoot, "app.js"), "utf8");
for (const target of [
  "data-readiness",
  "data-envelope",
  "data-capabilities",
  "data-validation",
  "data-transitions",
  "data-teams",
  "data-boundaries",
  "data-observation",
  "data-progress",
  "data-gates",
]) {
  assert(indexSource.includes(target), `index is missing ${target}`);
}
for (const asset of ["styles.css", "app.js", "data/project.js", "assets/mark.svg", "assets/architecture.svg"]) {
  await access(join(siteRoot, asset), constants.R_OK);
}
assert(!appSource.includes("innerHTML"), "renderer must not inject status through innerHTML");
assert(!appSource.includes("eval("), "renderer must not evaluate status strings");

console.log("Ferric site data is structurally valid and current claims remain fail-closed.");
