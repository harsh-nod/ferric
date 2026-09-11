import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";
import { headProfiles, headRequests, validateHeadPerformance } from "./validate-head-performance.mjs";

const [archive, previousPath] = process.argv.slice(2);
assert(archive && previousPath, "usage: node validate-head-performance-evidence.mjs ARCHIVE PREVIOUS_PERFORMANCE_JS");
async function data(path) {
  const context = { window: {} };
  vm.runInNewContext(await readFile(path, "utf8"), context);
  return JSON.parse(JSON.stringify(context.window.FERRIC_PERFORMANCE));
}
async function pinned(path, digest, json = true) {
  const raw = await readFile(path);
  assert.equal(createHash("sha256").update(raw).digest("hex"), digest, path);
  return json ? JSON.parse(raw) : raw;
}
const current = await data(join(dirname(fileURLToPath(import.meta.url)), "data/performance.js"));
const previous = await data(previousPath);
for (const [key, value] of Object.entries(previous)) {
  if (!["updated", "scope", "interpretation"].includes(key)) assert.deepEqual(current[key], value, `historical ${key} changed`);
}
const section = current.fp32HeadV7;
validateHeadPerformance(section);
const pins = section.pins;
const base = join(archive, "fp32-v7");
const approved = await pinned(join(base, "preflight/prelaunch-approved-v1.json"), pins.approvedPrelaunchSha256);
assert.equal(approved.schema, "FerricHeadPrecisionApprovedPrelaunchV1");
assert.equal(approved.launch_approved, true);
assert.equal(approved.raw_runs_consumed, false);
assert.equal(approved.controller_source, pins.controllerSourceRevision.slice(0, 7));
assert.equal(approved.dependency_git_revision, pins.fe2o3Revision);
assert.equal(approved.controller.sha256, pins.controllerSha256);
assert.equal(approved.worker.sha256, pins.workerSha256);
const summary = await pinned(join(base, "reports/three-case-r1.timing.json"), pins.summaryFileSha256);
const manifest = await pinned(join(base, "reports/three-case-r1.timing.manifest.json"), pins.summaryManifestSha256);
const sources = {
  "compare_tp_head_precision_v7.py": pins.comparatorSourceSha256,
  "compare_tp_batch.py": pins.legacyComparatorSourceSha256,
  "host_timing_summary.py": pins.legacyTimingSourceSha256,
  "performance_ledger.py": pins.legacyLedgerSourceSha256,
};
assert.deepEqual(summary.generator_dependencies, sources);
assert.equal(summary.generator_sha256, pins.generatorSourceSha256);
assert.equal(summary.manifest_sha256, pins.summaryManifestSha256);
for (const [name, digest] of Object.entries(sources)) {
  await pinned(join(name === "compare_tp_head_precision_v7.py" ? base : join(archive, "tools"), name), digest, false);
}
await pinned(join(base, "summarize_tp_head_precision_v7.py"), pins.generatorSourceSha256, false);
await pinned(join(archive, "workload.json"), pins.workloadSha256);
await pinned(join(archive, "reference.json"), pins.referenceSha256);
assert.equal(summary.schema, "FerricTpHeadPrecisionTimingSummaryV1");
assert.equal(summary.authority, "none");
assert.equal(summary.measurement, "host-wall-latency-not-gpu-duration");
assert.equal(summary.sidecar_normalization, "none; original v7 Setup/Closed bind directly");
assert.equal(summary.warmup_policy, "fresh-worker-no-warmup");
assert.equal(summary.equivalence.tensor_parallel, 1);
assert.equal(summary.equivalence.dtype, "BF16");
assert.equal(summary.equivalence.batch_tokens, 16);
assert.equal(summary.equivalence.prefill_chunk, 16);
assert.equal(summary.equivalence.prefix_cache, true);
assert.deepEqual(summary.variants.map((item) => item.name), headProfiles);
assert.deepEqual(manifest.pairs, approved.pairs);
const fp32Artifact = { artifact_hsaco_id: pins.fp32HsacoSha256, artifact_manifest_id: pins.fp32ManifestSha256,
  artifact_handoff_id: pins.fp32HandoffSha256 };
for (const [key, value] of Object.entries(fp32Artifact)) assert.equal(approved.fp32_head_artifact[key], value);
let commonExpected;
const metrics = [["outputTokensPerSecond", "output_tokens_per_second"], ["workloadSeconds", "workload_window_seconds"],
  ["setupSeconds", "setup_seconds"], ["wholeSeconds", "whole_seconds"]];
for (const [index, profile] of section.profiles.entries()) {
  const variant = summary.variants[index];
  const id = `tp1-v7-${profile.name}-r1`;
  const preflight = approved.cases[index];
  assert.equal(preflight.name, id);
  assert.equal(preflight.variant, profile.name);
  assert.equal(preflight.expectation_sha256, profile.expectationFileSha256);
  const expected = await pinned(join(base, "preflight", `${id}.expect.json`), profile.expectationFileSha256);
  assert.deepEqual(variant.expected, expected);
  assert.equal(expected.head_precision, profile.headPrecision);
  assert.equal(expected.collective, null);
  assert.equal(expected.output_head_pruning, false);
  assert.equal(expected.prefix_cache, true);
  assert.deepEqual(expected.fp32_head_artifact, fp32Artifact);
  assert.deepEqual(expected.performance_profile, { attention: "baseline", dispatch_sequences: false,
    projection: profile.projection, queue_rollover: false, runtime_cache_admission: false,
    runtime_operational: true, runtime_profiling: false });
  for (const [field, key] of [["controller_sha256", "controllerSha256"], ["worker_sha256", "workerSha256"],
    ["artifact_hsaco_id", "hsacoSha256"], ["artifact_manifest_id", "manifestSha256"], ["artifact_handoff_id", "handoffSha256"],
    ["workload_sha256", "workloadSha256"], ["reference_sha256", "referenceSha256"]]) assert.equal(expected[field], pins[key]);
  const matched = JSON.parse(JSON.stringify(expected));
  delete matched.head_precision;
  delete matched.performance_profile.projection;
  if (commonExpected === undefined) commonExpected = matched;
  else assert.deepEqual(matched, commonExpected);
  assert.equal(variant.repetitions, 1);
  assert.equal(variant.runs.length, 1);
  const run = variant.runs[0];
  assert.equal(run.id, id);
  assert.equal(run.comparison_sha256, profile.comparisonFileSha256);
  assert.equal(run.timing_sha256, profile.rawTimingSha256);
  assert.equal(run.fp32_head_workspace_bytes, profile.workspaceBytes);
  assert.equal(run.output_tokens, 8);
  assert.equal(run.physical_token_rows, 34);
  assert.equal(run.batch_count, 5);
  assert.deepEqual(run.rank_dispatch_counts, [2720]);
  assert.equal(manifest.variants[index].expectation_sha256, profile.expectationFileSha256);
  assert.equal(manifest.variants[index].runs[0].comparison_sha256, profile.comparisonFileSha256);
  assert.equal(manifest.variants[index].runs[0].timing_sha256, profile.rawTimingSha256);
  const comparison = await pinned(join(base, "reports", `${id}.comparison.json`), profile.comparisonFileSha256);
  assert.equal(comparison.schema, "FerricQwen3TpHeadPrecisionComparisonV1");
  for (const key of ["passed", "all_reference_tokens_and_bytes_match", "clean_teardown_recorded", "gpu_idle_before_and_after"]) assert.equal(comparison[key], true);
  assert.equal(comparison.comparator_sha256, pins.comparatorSourceSha256);
  assert.equal(comparison.semantic_checker_sha256, pins.legacyComparatorSourceSha256);
  assert.equal(comparison.expectation_sha256, profile.expectationFileSha256);
  assert.equal(comparison.head_precision, profile.headPrecision);
  assert.equal(comparison.fp32_head_workspace_bytes, profile.workspaceBytes);
  assert.deepEqual(comparison.identities, run.identities);
  assert.deepEqual(comparison.requests["seed-prefix"].generated_tokens, [17689, 374]);
  const directory = join(archive, "runs", id);
  for (const file of ["gpu-before.json", "gpu-after.json", "status"]) await pinned(join(directory, file), run.input_sha256[file], false);
  await pinned(join(directory, "host-timing.json"), profile.rawTimingSha256);
  const raw = await pinned(join(directory, "results.jsonl"), run.input_sha256["results.jsonl"], false);
  const records = raw.toString("utf8").trim().split("\n").map((line) => JSON.parse(line));
  const setup = records[0], closed = records.at(-1);
  assert.equal(setup.tensor_parallel, 1);
  assert.equal(setup.head_precision, profile.headPrecision);
  assert.equal(setup.fp32_head_workspace_bytes, profile.workspaceBytes);
  assert.deepEqual(setup.fp32_head_artifact, fp32Artifact);
  assert.equal(setup.setup_seconds, profile.setupSeconds);
  assert.equal(closed.whole_seconds, profile.wholeSeconds);
  assert.equal(closed.all_workers_exited, true);
  assert.deepEqual(closed.rank_dispatch_counts, [2720]);
  const requests = records.filter((record) => record.schema === "FerricQwen3TpBatchRequestV2");
  assert.equal(requests.length, 4);
  const terminals = [], arrivals = [];
  let count = 0;
  headRequests.forEach((name, requestIndex) => {
    const record = requests.find((item) => item.name === name);
    assert(record);
    const timestamps = record.output_timestamps_ns;
    const gaps = timestamps.slice(1).map((value, i) => value - timestamps[i]);
    const ttft = (timestamps[0] - record.arrival_ns) / 1e9;
    const tpot = gaps.length ? gaps.reduce((a, b) => a + b, 0) / gaps.length / 1e9 : null;
    assert.equal(gaps.length, [1, 2, 0, 1][requestIndex]);
    assert.deepEqual(profile.requestLatencies[requestIndex], [ttft, tpot]);
    assert.equal(run.requests[name].ttft_seconds, ttft);
    assert.equal(run.requests[name].tpot_seconds, tpot);
    assert.deepEqual(record.generated_tokens, comparison.requests[name].generated_tokens);
    arrivals.push(record.arrival_ns);
    terminals.push(record.state === "Cancelled" ? record.cancelled_ns : timestamps.at(-1));
    count += record.generated_tokens.length;
  });
  const window = Math.max(...terminals) - Math.min(...arrivals);
  assert.equal(count, 8);
  assert.equal(profile.workloadSeconds, window / 1e9);
  assert.equal(profile.outputTokensPerSecond, count * 1e9 / window);
  for (const [site, key] of metrics) {
    assert.equal(profile[site], run[key]);
    for (const statistic of ["min", "max", "mean", "p50", "p95"]) assert.equal(variant.metrics[key][statistic], profile[site]);
    assert.equal(variant.metrics[key].n, 1);
  }
  console.log("EXACT FP32-HEAD CASE", id);
}
for (const [index, pair] of section.pairs.entries()) {
  const stem = index === 0 ? "precision" : "mfma";
  const report = await pinned(join(base, "reports", `${stem}-pair-r1.timing.json`), pins[`${stem}PairFileSha256`]);
  const pairManifest = await pinned(join(base, "reports", `${stem}-pair-r1.timing.manifest.json`), pins[`${stem}PairManifestSha256`]);
  assert.equal(report.manifest_sha256, pins[`${stem}PairManifestSha256`]);
  assert.equal(report.generator_sha256, pins.generatorSourceSha256);
  assert.deepEqual(report.generator_dependencies, sources);
  assert.deepEqual(report.variants, summary.variants.slice(index, index + 2));
  assert.deepEqual(pairManifest.pairs, [{ baseline: pair.baseline, candidate: pair.candidate }]);
  assert.deepEqual(report.pairs, [summary.pairs[index]]);
  const measured = report.pairs[0];
  for (const [site, raw] of [["rateRatio", "output_tokens_per_second"], ["setupRatio", "setup_seconds"], ["wholeRatio", "whole_seconds"]]) {
    assert.equal(pair[site], measured.metrics[raw].mean.variant_over_baseline);
  }
  assert.equal(pair.reuseTtftRatio, measured.requests["reuse-prefix"].ttft_seconds.mean.variant_over_baseline);
  assert.equal(pair.reuseTpotRatio, measured.requests["reuse-prefix"].tpot_seconds.mean.variant_over_baseline);
}
console.log("PASS: three source-bound FP32-head cases, raw per-request clocks and two explicit pairings; historical BF16/transport data unchanged.");
