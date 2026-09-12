import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";
import { validateC1Checkpoint } from "./validate-c1-checkpoint.mjs";

const siteRoot = dirname(fileURLToPath(import.meta.url));
const pins = {
  summarySha256: "cb0a76b9b50dae39d28e6f03c0f2d758692c6bdd0123d9fde749b4003978496d",
  manifestSha256: "efeb4b3c6bcbcecef23f2a26ee7e9c35740ec2ab1fa177984707f9cd11ec883b",
  replaySha256: "0cfe47be5d520b104e54feaaeb820060beefbedbefea72fd1f29fe79dd284828",
  replayNoteSha256: "a789df6256ef84ca316b3b032e29560cbfd894a7ba8aa339af019372f4599eba",
  acquisitionNoteSha256: "71995f7ac394ff64bf44c2a3dfb905b0b355cac5629f69fa7298f42fe441c94a",
  nativeArchiveSha256: "b9c658552b29bfe11910449f61f22f1a886cc50ce072fb23df2129d208c34f01",
  replayArchiveSha256: "3d3d123fcd3d0f99c8df82bdd2b74813f586faa13b3e631c3d15f667356b4e26",
};
const order = ["old-a1", "new-b1", "new-b2", "old-a2"];
const metrics = {
  ttftMeanSeconds: "diagnostic_ttft_seconds",
  tpotMeanSeconds: "diagnostic_tpot_seconds",
  workloadMeanSeconds: "diagnostic_workload_seconds",
  meanPerRunOutputTokensPerSecond: "diagnostic_output_tokens_per_second",
};
const expectedRows = [
  { profile: "old", label: "Singleton residuals (7cb6522)",
    source: "7cb6522430b6990e1d3f12bc15958865358459f4",
    controllerSha256: "0da9c59a9172d809dd5109ec30681a847c5f83e7da2c396a943f51d99235c075",
    ttftMeanSeconds: 2.859059196, tpotMeanSeconds: 0.19452115546062992,
    workloadMeanSeconds: 27.5632460095, meanPerRunOutputTokensPerSecond: 4.648212283634937,
    tpotRangeSeconds: [0.18820138162992125, 0.20084092929133857] },
  { profile: "new", label: "Group-tail residuals (2396a82)",
    source: "2396a82ff42c94654261a2ee8313f881a77aee1d",
    controllerSha256: "40fcfff3bb5ce567532b96c88ae11c11ca8d744a08489380b14984f36ba2430b",
    ttftMeanSeconds: 2.7487362765, tpotMeanSeconds: 0.21833266387795275,
    workloadMeanSeconds: 30.476984739000002, meanPerRunOutputTokensPerSecond: 4.200225076073074,
    tpotRangeSeconds: [0.21617611607086612, 0.22048921168503935] },
];
const fixed = {
  schema: "FerricResidualTailPagesDiagnosticV1", authority: "none",
  descriptiveComparisonAdmitted: true, result: "decode-regression",
  performanceQualified: false, stablePerformanceGainClaimed: false,
  httpMeasurement: false, gpuDurationMeasurement: false, servingQualified: false,
  defaultPromotion: false, competitiveRanking: false, m1Completion: false,
  qualificationTimingsExcluded: true, historicalCohortsMixed: false,
  context: 256, inputTokens: 128, outputTokens: 128, repetitionsPerSource: 2,
};
const clone = (value) => JSON.parse(JSON.stringify(value));
const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");

function close(actual, expected, label) {
  assert.equal(typeof actual, "number", label);
  assert(Number.isFinite(actual) && Number.isFinite(expected), label);
  assert(Math.abs(actual - expected) <= 1e-12 * Math.max(1, Math.abs(expected)), label);
}

function projectFrom(bytes) {
  const context = { window: {} };
  vm.runInNewContext(bytes.toString("utf8"), context);
  return clone(context.window.FERRIC_PROJECT);
}

export function validateResidualDiagnostic(input) {
  const value = clone(input);
  assert.deepEqual(Object.keys(value).sort(), [...Object.keys(fixed), ...Object.keys(pins),
    "acquisitionOrder", "rows", "scope", "interpretation", "limitations", "correctness"].sort());
  for (const [name, expected] of Object.entries({ ...fixed, ...pins }))
    assert.deepEqual(value[name], expected, name);
  assert.deepEqual(value.acquisitionOrder, order);
  assert.deepEqual(value.rows, expectedRows, "exact admitted descriptive means/ranges and binary identities");
  for (const [field, phrases] of Object.entries({
    scope: ["native TP1", "context 256", "n=2 per exact binary", "not HTTP latency or GPU durations"],
    interpretation: ["Decode regressed", "increased 12.24%", "fell 9.64%", "improved 3.86%",
      "Both new TPOT samples are slower than both old samples", "did not produce an overall gain"],
    limitations: ["warm build", "source-fresh build", "not proved identical", "descriptive n=2",
      "not stable-performance", "correctness-run timings", "earlier HTTP/native cohorts are excluded",
      "not 128 divided by mean duration", "not additive", "not ratioed"],
    correctness: ["128-token ID/UTF8 oracle", "83,139 packets", "135 batches", "cursor 255",
      "normal close", "all eight GPUs idle before and after", "independently replayed",
      "Published serving measurements remain unchanged"],
  })) {
    assert.equal(typeof value[field], "string");
    for (const phrase of phrases) assert(value[field].includes(phrase), `${field}: ${phrase}`);
  }
}

export function testResidualDiagnosticRejections(value) {
  const mutations = [
    (x) => { x.result = "speedup"; },
    (x) => { x.httpMeasurement = true; },
    (x) => { x.stablePerformanceGainClaimed = true; },
    (x) => { x.qualificationTimingsExcluded = false; },
    (x) => { x.summarySha256 = "0".repeat(64); },
    (x) => { x.rows[1].tpotMeanSeconds = x.rows[0].tpotMeanSeconds; },
    (x) => { x.rows[1].meanPerRunOutputTokensPerSecond = 128 / x.rows[1].workloadMeanSeconds; },
    (x) => { x.acquisitionOrder.reverse(); },
    (x) => { x.limitations = "Identical builds, stable gain"; },
    (x) => { x.extra = true; },
  ];
  for (const mutate of mutations) {
    const changed = clone(value);
    mutate(changed);
    assert.throws(() => validateResidualDiagnostic(changed));
  }
}

export async function validateResidualDiagnosticEvidence(root, project) {
  const value = clone(project.residualDiagnostic);
  validateResidualDiagnostic(value);
  async function pinned(name, expected) {
    const bytes = await readFile(join(root, name));
    assert(bytes.length > 0 && bytes.length <= 2 * 1024 * 1024, `${name}: bounded input`);
    assert.equal(digest(bytes), expected, name);
    return bytes;
  }
  const baseline = projectFrom(await pinned("residual-baseline-project.js",
    "dc728b4b7199e33fce99a03e141559ad1140123918404d5b9d97136d08035e34"));
  const historical = clone(project);
  validateC1Checkpoint(historical.c1Checkpoint);
  delete historical.c1Checkpoint;
  delete historical.residualDiagnostic;
  assert.deepEqual(historical, baseline, "every preexisting public object is unchanged");
  assert.equal(digest(await readFile(join(siteRoot, "data/performance.js"))),
    "05ad1f50575547c0b0c8244b2912e1d7518258772527f7ff236fdb8d2e42102f");
  const summary = JSON.parse(await pinned("residual-summary.json", pins.summarySha256));
  const manifest = JSON.parse(await pinned("residual-manifest.json", pins.manifestSha256));
  const replay = JSON.parse(await pinned("residual-replay.json", pins.replaySha256));
  const note = (await pinned("residual-replay-note.md", pins.replayNoteSha256)).toString("utf8");
  const acquisition = (await pinned("residual-acquisition-note.md", pins.acquisitionNoteSha256)).toString("utf8");
  assert.equal(summary.schema, "FerricResidualTailAbbaSummaryV1");
  assert.equal(summary.authority, "none");
  for (const flag of ["performance_qualified", "serving_qualified", "http_measurement", "gpu_clock_measurement",
    "confidence_qualified", "stable_performance_gain_claimed", "competitive_ranking", "default_promotion",
    "m1_completion", "new_verus_proof", "old_cohort_mixed", "additive_gain_claim", "flush_frontier_ratio_claimed"])
    assert.equal(summary[flag], false, flag);
  assert.equal(summary.evidence_manifest_sha256, pins.manifestSha256);
  assert.deepEqual(summary.profiles, manifest.profiles);
  for (const name of ["experiment_plan_sha256", "reducer_sha256", "base_checker_sha256", "legacy_helper_sha256"])
    assert.equal(summary[name], manifest[name], name);
  assert.equal(summary.cohort.repetitions_per_source, 2);
  assert.deepEqual(summary.runs.map((run) => run.id), order);
  assert.deepEqual(manifest.runs.map((run) => run.id), order);
  assert.equal(summary.scope, "exact old/new/new/old ordered128 binaries; all qualifications excluded; build environments not proved identical");
  assert.equal(summary.timing_boundary,
    "host-prefill-start-through-generated-token-commit; excludes setup; not HTTP or GPU duration");
  let rawBytes = 0;
  for (const [index, run] of summary.runs.entries()) {
    assert.equal(run.profile, index === 0 || index === 3 ? "old" : "new");
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
    close(run.metrics.diagnostic_output_tokens_per_second,
      128 / run.metrics.diagnostic_workload_seconds, `${run.id}: per-run output rate`);
  }
  assert.equal(rawBytes, 6698665);
  for (const row of value.rows) {
    assert.equal(summary.profiles[row.profile].source, row.source);
    assert.equal(summary.profiles[row.profile].sha256, row.controllerSha256);
    const runs = summary.runs.filter((run) => run.profile === row.profile);
    assert.equal(runs.length, 2);
    for (const [field, metric] of Object.entries(metrics)) {
      const values = runs.map((run) => run.metrics[metric]);
      for (const number of values) assert(Number.isFinite(number) && number > 0);
      const mean = (values[0] + values[1]) / 2;
      close(row[field], mean, field);
      close(summary.cohort.arithmetic_metric_means[row.profile][metric], mean, metric);
      const variability = summary.cohort.within_source_variability[row.profile][metric];
      assert.equal(variability.n, 2);
      assert.deepEqual(variability.values, values);
      close(variability.arithmetic_mean, mean, "variability mean");
      close(variability.minimum, Math.min(...values), "minimum");
      close(variability.maximum, Math.max(...values), "maximum");
      close(variability.range, Math.max(...values) - Math.min(...values), "range");
      if (field === "tpotMeanSeconds")
        assert.deepEqual(row.tpotRangeSeconds, [variability.minimum, variability.maximum]);
    }
  }
  for (const [field, metric] of Object.entries(metrics)) {
    const oldMean = value.rows[0][field], newMean = value.rows[1][field];
    const outputRate = metric === metrics.meanPerRunOutputTokensPerSecond;
    const ratio = outputRate ? newMean / oldMean : oldMean / newMean;
    const change = outputRate ? 100 * (newMean / oldMean - 1) : 100 * (1 - newMean / oldMean);
    close(summary.cohort.ratio_of_arithmetic_means[metric].speed_ratio, ratio, metric);
    close(summary.cohort.ratio_of_arithmetic_means[metric].improvement_percent, change, metric);
    assert.equal(summary.cohort.repetition_pairs.length, 2);
    for (const [index, pair] of summary.cohort.repetition_pairs.entries()) {
      assert.equal(pair.repetition, index + 1);
      assert.equal(pair.old_id, `old-a${index + 1}`);
      assert.equal(pair.new_id, `new-b${index + 1}`);
      const oldValue = summary.runs.find((run) => run.id === pair.old_id).metrics[metric];
      const newValue = summary.runs.find((run) => run.id === pair.new_id).metrics[metric];
      close(pair.relative_changes[metric].speed_ratio,
        outputRate ? newValue / oldValue : oldValue / newValue, "paired ratio");
      close(pair.relative_changes[metric].improvement_percent,
        outputRate ? 100 * (newValue / oldValue - 1) : 100 * (1 - newValue / oldValue), "paired change");
    }
  }
  assert(value.rows[1].tpotRangeSeconds[0] > value.rows[0].tpotRangeSeconds[1]);
  assert.equal(replay.schema, "FerricResidualTailReplayGateV1");
  assert.equal(replay.authority, "none");
  assert.equal(replay.performance_qualified, false);
  assert.equal(replay.passed, true);
  assert.deepEqual(replay.errors, []);
  assert.deepEqual(replay.commands.map((command) => command.exit_code), [0, 1]);
  assert.equal(replay.commands[0].argv.at(-1), pins.manifestSha256);
  assert.equal(replay.commands[1].argv.at(-1), "0".repeat(64));
  for (const phrase of [pins.summarySha256, pins.replaySha256, pins.nativeArchiveSha256,
    pins.replayArchiveSha256, "Decode Regression", "Old warm-build versus new source-fresh",
    "No qualification timings were pooled."])
    assert(note.includes(phrase), `replay note: ${phrase}`);
  for (const phrase of [pins.summarySha256, pins.nativeArchiveSha256, pins.manifestSha256,
    "All four controllers exited zero", "unforced cleanup", "observed decode regression is retained"])
    assert(acquisition.includes(phrase), `acquisition note: ${phrase}`);
  // The pinned Python replay preserves raw u64 identities. This JS check rechecks only descriptive arithmetic.
  console.log("PASS: residual decode regression matches the admitted byte-pinned replay; means/ranges/pairs rechecked; historical objects unchanged.");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  assert.equal(process.argv.length, 3, "usage: node validate-residual-diagnostic.mjs EVIDENCE_ROOT");
  await validateResidualDiagnosticEvidence(process.argv[2],
    projectFrom(await readFile(join(siteRoot, "data/project.js"))));
}
