import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";
import { validateLiveC1Checkpoint } from "./validate-live-c1-checkpoint.mjs";

const siteRoot = dirname(fileURLToPath(import.meta.url));
const clone = (value) => JSON.parse(JSON.stringify(value));
const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");
const fixed = {
  schema: "FerricLayerC1PagesCheckpointV1", authority: "none", m1OpenGates: 33,
  source: "c2a235e44008bd8e0ed9e3bf6e62000e4f18f51c",
  tree: "4a35eda9a8334418e83ad5539743a992f245ac0c",
  controllerSha256: "ae5fad8d668c8c4336bf5a4e1cbb66116994bca826c7a58d6febe575e0eae97a",
  summarySha256: "2b94e482ce94db6ff6358eac9a5c381156fbab108714ed210f9acbd9be85bda8",
  manifestSha256: "4ffb2d5225cf94366a2ca9236c8cc7f7b7cc98000fa59a6ed7aed686cd6f6c7c",
  replaySha256: "8bbd004d8d175485ce5aacc61c45f6643f1d03415b17b965365dc6dd3544191a",
  replayNoteSha256: "998c1ea5ff7c8fddfe94c86a07d142e1826423373ce784b2a5d6095f05a90ff7",
  nativeArchiveSha256: "fd35466851b5b0b065b3f36bd9b39b6019adb5ac545ef2d4219729702427e8d2",
  replayArchiveSha256: "e2f30d792e67d0b7c82499f913db41192acdf7cca58c910be8378f642fc10af9",
  context: 256, inputTokens: 128, outputTokens: 128, repetitionsPerMode: 2,
  descriptiveComparisonAdmitted: true, performanceQualified: false,
  stablePerformanceGainClaimed: false, httpMeasurement: false, gpuDurationMeasurement: false,
  servingQualified: false, defaultPromotion: false, competitiveRanking: false,
  m1Completion: false, qualificationTimingsExcluded: true, historicalCohortsMixed: false,
};
const order = ["mfma-a1", "c1-wave-b1", "c1-wave-b2", "mfma-a2"];
const rows = [
  { profile: "mfma", label: "MFMA layers", ttftMeanSeconds: 2.7873206985000003,
    tpotMeanSeconds: 0.1806510178070866, workloadMeanSeconds: 25.729999999999997,
    meanPerRunOutputTokensPerSecond: 4.976281347512148,
    tpotRangeSeconds: [0.1774872383464567, 0.18381479726771652] },
  { profile: "c1-wave", label: "C1 wave layers", ttftMeanSeconds: 2.7723724990000003,
    tpotMeanSeconds: 0.16299861548031497, workloadMeanSeconds: 23.47319682,
    meanPerRunOutputTokensPerSecond: 5.5236709436682165,
    tpotRangeSeconds: [0.14225163870078741, 0.18374559225984252] },
];
const proof = {
  source: "ed2ceb502c539d00356213850cc32c023bf824c5",
  sourceSha256: "1111950bab5b6613865be024b62cdda09a01b38833d443329322a84d88acc009",
  rawSha256: "f1ec34e3d1e07973bc1f8ecfd3ff9c9b8462f013c2beafee504e415898ffda22",
  archiveSha256: "ce0feb569029a23933b744ab5b9c9e4316f57cbc691aa24ebd8c3c1355687702",
  verifiedQueries: 18, requiredFunctions: 16, errors: 0,
  verusVersion: "0.2026.08.02.b677dd5", wholeCrate: true,
  actualKernelRefinement: false, fp32Proof: false, runtimeQualification: false, m1Admission: false,
};
const teams = [
  { name: "Sharded argmax v13", source: "256a6e4be4c0197ba87519c9c0dde8e1b7ed58de",
    tree: "f6f86ae6268e7458241e5a1f69008f40081aef06", state: "host-only-passed", label: "Host passed",
    hostNoteSha256: "176265645a4566c941b5fb6c003404cd85a137bb0a5b39873e4414f062f53c3b",
    hostArchiveSha256: "88fb14461cd48795c4fab23f2db9ce9bfbc3862aaaf47350236212d7e2e18c09",
    hostTestsPassed: 23, strictClippyExit: 0,
    emissionAdmitted: false, nativeAdmitted: false, performanceGainClaimed: false },
  { name: "C1 live route", source: "87f38de73cf7604497ac838c30e06efe84d4102d",
    tree: "e2ba291cb4abff7ebde5fd313cf3ad3238c00513", state: "host-only-passed", label: "Host passed",
    hostNoteSha256: "499b74180817cd60988302b0b3e83af5d6e275e314425e0dc33266b6a8e5a8bf",
    hostArchiveSha256: "7aec042f6d41450644d6ecbd2eed94e5ccc7b9115236e96eb00fe9ce990a0fa5",
    controllerSha256: "8ae69215cf93524ae3438f84bad6d4ce146a92d6636d41a8fc854b897233d450",
    commandsPassed: 40, hostTestsPassed: 892, hostIgnored: 48, hostResultRows: 21, strictClippyExit: 0,
    hostAdmitted: true, nativeAdmitted: false, httpRemeasured: false },
];
const metrics = {
  ttftMeanSeconds: "diagnostic_ttft_seconds", tpotMeanSeconds: "diagnostic_tpot_seconds",
  workloadMeanSeconds: "diagnostic_workload_seconds",
  meanPerRunOutputTokensPerSecond: "diagnostic_output_tokens_per_second",
};
function exact(actual, expected, extra = []) {
  assert.deepEqual(Object.keys(actual).sort(), [...Object.keys(expected), ...extra].sort());
  for (const [name, value] of Object.entries(expected)) assert.deepEqual(actual[name], value, name);
}
function phrases(text, required) {
  assert.equal(typeof text, "string");
  for (const phrase of required) assert(text.includes(phrase), `missing boundary: ${phrase}`);
}
function close(actual, expected, label) {
  assert(Number.isFinite(actual) && Number.isFinite(expected), label);
  assert(Math.abs(actual - expected) <= 1e-12 * Math.max(1, Math.abs(expected)), label);
}
function projectFrom(bytes) {
  const context = { window: {} };
  vm.runInNewContext(bytes.toString("utf8"), context);
  return clone(context.window.FERRIC_PROJECT);
}

export function validateC1Checkpoint(input) {
  const value = clone(input);
  exact(value, fixed, ["acquisitionOrder", "rows", "scope", "interpretation", "limitations",
    "correctness", "integerProof", "teams"]);
  assert.deepEqual(value.acquisitionOrder, order);
  assert.deepEqual(value.rows, rows);
  exact(value.integerProof, proof, ["scope"]);
  assert.equal(value.teams.length, 2);
  value.teams.forEach((team, index) => exact(team, teams[index], ["detail"]));
  phrases(value.scope, ["same binary", "TP1/context256", "FP32-v8 head", "wave-v11 argmax",
    "ordered residual tails", "MFMA / C1 / C1 / MFMA", "n=2 per mode", "not HTTP latency or GPU durations"]);
  phrases(value.interpretation, ["9.772% lower", "11.000% higher", "2787.321 to 2772.372 ms",
    "Material pair variation", "0.038% lower and 19.852% lower", "does not establish a stable gain"]);
  phrases(value.limitations, ["Descriptive n=2", "no confidence interval", "Correctness-run timings",
    "earlier HTTP/native cohorts are excluded", "not 128 divided by mean duration", "not additive",
    "All 33 M1 gates remain open", "residual regression and historical public HTTP measurements remain unchanged"]);
  phrases(value.correctness, ["unchanged token-ID/UTF8 oracle", "83,139 packets", "135 batches",
    "cursor 255", "normal unforced close", "All eight GPUs", "Forty original core files",
    "without rewriting their schema or identities"]);
  phrases(value.integerProof.scope, ["Standalone integer model only", "18 queries", "16 required functions",
    "zero errors", "not actual-kernel refinement", "FP32/ABI/GPU", "runtime qualification or M1 admission",
    "first type-check failure"]);
  phrases(value.teams[0].detail, ["15 numerical and 8 source/contract tests", "strict Clippy actually exits zero",
    "not integrated into the C1 measurements", "Typed emission, native parity and performance gain remain unadmitted",
    "not a refinement of this kernel"]);
  phrases(value.teams[1].detail, ["all 40 host commands", "892 test invocations, 48 ignored across 21 result rows",
    "strict Clippy", "93 own compiler artifacts were fresh:false", "four early/release pairs",
    "host-only admission, not a new native or HTTP measurement",
    "do not substitute for context8192", "defaults remain unchanged"]);
}

export function testC1CheckpointRejections(input) {
  const mutations = [
    (x) => { x.rows[1].tpotMeanSeconds = x.rows[0].tpotMeanSeconds; },
    (x) => { x.rows[1].meanPerRunOutputTokensPerSecond = 128 / x.rows[1].workloadMeanSeconds; },
    (x) => { x.rows[1].tpotRangeSeconds.reverse(); },
    (x) => { x.acquisitionOrder.reverse(); },
    (x) => { x.context = 8192; },
    (x) => { x.stablePerformanceGainClaimed = true; },
    (x) => { x.httpMeasurement = true; },
    (x) => { x.gpuDurationMeasurement = true; },
    (x) => { x.defaultPromotion = true; },
    (x) => { x.m1OpenGates = 32; },
    (x) => { x.summarySha256 = "0".repeat(64); },
    (x) => { x.interpretation = "Stable gain without variation"; },
    (x) => { x.integerProof.actualKernelRefinement = true; },
    (x) => { x.integerProof.fp32Proof = true; },
    (x) => { x.integerProof.verifiedQueries = 16; },
    (x) => { x.integerProof.source = x.source; },
    (x) => { x.teams[0].emissionAdmitted = true; },
    (x) => { x.teams[1].hostAdmitted = false; },
    (x) => { x.teams[1].httpRemeasured = true; },
    (x) => { x.extra = true; },
  ];
  for (const mutate of mutations) {
    const changed = clone(input);
    mutate(changed);
    assert.throws(() => validateC1Checkpoint(changed), assert.AssertionError);
  }
}

export async function validateC1CheckpointEvidence(root, project) {
  const value = clone(project.c1Checkpoint);
  validateC1Checkpoint(value);
  async function pinned(name, expected) {
    const bytes = await readFile(join(root, name));
    assert(bytes.length > 0 && bytes.length <= 2 * 1024 * 1024, `${name}: bounded input`);
    assert.equal(digest(bytes), expected, name);
    return bytes;
  }
  const baseline = projectFrom(await pinned("c1-baseline-project.js",
    "b8757d8e3072f7dd78ba0744dc892d59d50ef7fb7861782808b6bbe366741987"));
  const historical = clone(project);
  validateLiveC1Checkpoint(historical.liveC1Checkpoint);
  delete historical.liveC1Checkpoint;
  delete historical.c1Checkpoint;
  assert.deepEqual(historical, baseline, "every preexisting public object, including HTTP and residual regression, is unchanged");
  assert.equal(digest(await readFile(join(siteRoot, "data/performance.js"))),
    "05ad1f50575547c0b0c8244b2912e1d7518258772527f7ff236fdb8d2e42102f");
  const summary = JSON.parse(await pinned("c1-summary.json", fixed.summarySha256));
  const manifest = JSON.parse(await pinned("c1-manifest.json", fixed.manifestSha256));
  const replay = JSON.parse(await pinned("c1-replay.json", fixed.replaySha256));
  const note = (await pinned("c1-replay-note.md", fixed.replayNoteSha256)).toString("utf8");
  assert.equal(summary.schema, "FerricLayerC1WaveAbbaSummaryV1");
  assert.equal(summary.authority, "none");
  for (const flag of ["performance_qualified", "serving_qualified", "http_measurement", "gpu_clock_measurement",
    "confidence_qualified", "stable_performance_gain_claimed", "competitive_ranking", "default_promotion",
    "m1_completion", "new_verus_proof", "old_cohort_mixed", "additive_gain_claim", "flush_frontier_ratio_claimed"])
    assert.equal(summary[flag], false, flag);
  assert.equal(summary.evidence_manifest_sha256, fixed.manifestSha256);
  assert.deepEqual(summary.profiles, manifest.profiles);
  for (const name of ["experiment_plan_sha256", "reducer_sha256", "base_checker_sha256", "legacy_helper_sha256"])
    assert.equal(summary[name], manifest[name]);
  assert.equal(summary.cohort.repetitions_per_mode, 2);
  assert.deepEqual(summary.runs.map((run) => run.id), order);
  assert.deepEqual(manifest.runs.map((run) => run.id), order);
  assert.equal(summary.scope, "same-binary mfma/c1-wave/c1-wave/mfma ordered128; all qualifications and prior cohorts excluded");
  assert.equal(summary.timing_boundary,
    "host-prefill-start-through-generated-token-commit; excludes setup; not HTTP or GPU duration");
  const common = clone(summary.profiles.mfma);
  const candidate = clone(summary.profiles["c1-wave"]);
  assert.equal(common.layer_projection, "mfma");
  assert.equal(candidate.layer_projection, "c1-wave");
  delete common.layer_projection;
  delete candidate.layer_projection;
  assert.deepEqual(common, candidate, "same binary/source/worker profile apart from layer selector");
  assert.equal(common.source, fixed.source);
  assert.equal(common.tree, fixed.tree);
  assert.equal(common.sha256, fixed.controllerSha256);
  let rawBytes = 0;
  for (const [index, run] of summary.runs.entries()) {
    assert.equal(run.profile, index === 0 || index === 3 ? "mfma" : "c1-wave");
    assert.equal(run.repetition, index < 2 ? 1 : 2);
    assert.equal(run.submission, "ordered");
    assert.equal(run.outputs, 128);
    assert.deepEqual(run.controller, summary.profiles[run.profile]);
    assert.deepEqual(run.evidence_files, manifest.runs[index].files);
    assert.equal(Object.keys(run.evidence_files).length, 10);
    for (const file of Object.values(run.evidence_files)) {
      assert(Number.isSafeInteger(file.bytes) && file.bytes > 0);
      assert.match(file.sha256, /^[0-9a-f]{64}$/);
      rawBytes += file.bytes;
    }
    close(run.metrics.diagnostic_output_tokens_per_second, 128 / run.metrics.diagnostic_workload_seconds, "per-run rate");
  }
  assert.equal(rawBytes, 6442016);
  for (const row of value.rows) {
    const runs = summary.runs.filter((run) => run.profile === row.profile);
    assert.equal(runs.length, 2);
    for (const [field, metric] of Object.entries(metrics)) {
      const values = runs.map((run) => run.metrics[metric]);
      for (const number of values) assert(Number.isFinite(number) && number > 0);
      const mean = (values[0] + values[1]) / 2;
      close(row[field], mean, field);
      close(summary.cohort.arithmetic_metric_means[row.profile][metric], mean, metric);
      const variability = summary.cohort.within_mode_variability[row.profile][metric];
      assert.equal(variability.n, 2);
      assert.deepEqual(variability.values, values);
      close(variability.arithmetic_mean, mean, "variability mean");
      close(variability.minimum, Math.min(...values), "minimum");
      close(variability.maximum, Math.max(...values), "maximum");
      close(variability.range, Math.max(...values) - Math.min(...values), "range");
      if (field === "tpotMeanSeconds") assert.deepEqual(row.tpotRangeSeconds, [variability.minimum, variability.maximum]);
    }
  }
  for (const [field, metric] of Object.entries(metrics)) {
    const base = value.rows[0][field], next = value.rows[1][field];
    const rate = metric === metrics.meanPerRunOutputTokensPerSecond;
    close(summary.cohort.ratio_of_arithmetic_means[metric].speed_ratio, rate ? next / base : base / next, metric);
    close(summary.cohort.ratio_of_arithmetic_means[metric].improvement_percent,
      rate ? 100 * (next / base - 1) : 100 * (1 - next / base), metric);
    assert.equal(summary.cohort.repetition_pairs.length, 2);
    for (const [index, pair] of summary.cohort.repetition_pairs.entries()) {
      assert.equal(pair.repetition, index + 1);
      assert.equal(pair.baseline_id, `mfma-a${index + 1}`);
      assert.equal(pair.candidate_id, `c1-wave-b${index + 1}`);
      const before = summary.runs.find((run) => run.id === pair.baseline_id).metrics[metric];
      const after = summary.runs.find((run) => run.id === pair.candidate_id).metrics[metric];
      close(pair.relative_changes[metric].speed_ratio, rate ? after / before : before / after, "pair ratio");
      close(pair.relative_changes[metric].improvement_percent,
        rate ? 100 * (after / before - 1) : 100 * (1 - after / before), "pair change");
    }
  }
  assert.equal(replay.schema, "FerricLayerC1ReplayGateV1");
  assert.equal(replay.authority, "none");
  assert.equal(replay.performance_qualified, false);
  assert.equal(replay.passed, true);
  assert.deepEqual(replay.errors, []);
  assert.deepEqual(replay.commands.map((command) => command.exit_code), [0, 1]);
  assert.equal(replay.commands[0].argv.at(-1), fixed.manifestSha256);
  assert.equal(replay.commands[1].argv.at(-1), "0".repeat(64));
  phrases(note.replace(/\s+/g, " "), [fixed.summarySha256, fixed.replaySha256, fixed.nativeArchiveSha256,
    fixed.replayArchiveSha256, "Variability is material", "does not establish a stable gain"]);

  await pinned("v13-proof.rs", proof.sourceSha256);
  const raw = JSON.parse(await pinned("v13-verus.json", proof.rawSha256));
  const required = (await pinned("v13-required-functions.txt",
    "ddb905b44edc8b19cd0f6df1b6bb7fda4028561d744a70099304e9e933db73e2"))
    .toString("utf8").trim().split("\n");
  assert.equal(required.length, 16);
  assert.equal(new Set(required).size, 16);
  assert.deepEqual(raw["verification-results"], { "encountered-error": false, "encountered-vir-error": false,
    success: true, verified: 18, errors: 0, "is-verifying-entire-crate": true });
  assert.equal(raw.verus.version, proof.verusVersion);
  assert.equal(raw.verus.commit, "b677dd5a766f25f56e9aa1e32621aa4e53304b47");
  assert.equal(raw.verus.toolchain, "1.97.1-x86_64-unknown-linux-gnu");
  for (const name of required) assert.deepEqual(raw["func-details"][`sharded_argmax_v13::${name}`],
    { obligation_proof_notes: [], failed_proof_notes: [] });
  const host = (await pinned("v13-host-note.md", teams[0].hostNoteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(host, [teams[0].source, teams[0].tree, teams[0].hostArchiveSha256, "All ten frozen commands passed",
    "Numerical/model tests: 15 passed", "Source/contract tests: 8 passed",
    "Strict all-target release Clippy: actual 0", "fresh:false", "not typed device emission"]);
  const live = (await pinned("c1-live-host-note.md", teams[1].hostNoteSha256)).toString("utf8").replace(/\s+/g, " ");
  phrases(live, [teams[1].source, teams[1].tree, teams[1].hostArchiveSha256, teams[1].controllerSha256,
    "All 40 frozen commands and their wrappers returned zero", "892 passed invocations, 48 ignored, 21 result rows",
    "all 93 own compiler artifacts", "fresh:false", "strict all-target Clippy with -D warnings",
    "Four early/explicit-release executable pairs", "all five final recorded binaries",
    "not context8192 native or HTTP admission"]);
  // The accepted Python replay preserves raw u64 identities. This page check rechecks descriptive arithmetic only.
  console.log("PASS: same-binary C1 means/ranges/pairs and integer-only Verus result match admitted pins; all historical objects unchanged.");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  assert.equal(process.argv.length, 3, "usage: node validate-c1-checkpoint.mjs EVIDENCE_ROOT");
  await validateC1CheckpointEvidence(process.argv[2], projectFrom(await readFile(join(siteRoot, "data/project.js"))));
}
