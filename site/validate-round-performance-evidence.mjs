import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";
import { requestNames, validateRoundPerformance } from "./validate-round-performance.mjs";

const [archive, previousPath] = process.argv.slice(2);
assert(archive && previousPath,
  "usage: node validate-round-performance-evidence.mjs TIMING_ARCHIVE PREVIOUS_PERFORMANCE_JS");
const siteRoot = dirname(fileURLToPath(import.meta.url));
async function dataset(path) {
  const context = { window: {} };
  vm.runInNewContext(await readFile(path, "utf8"), context);
  return JSON.parse(JSON.stringify(context.window.FERRIC_PERFORMANCE));
}
async function pinned(path, digest, json = true) {
  const bytes = await readFile(path);
  assert.equal(createHash("sha256").update(bytes).digest("hex"), digest, path);
  return json ? JSON.parse(bytes) : bytes;
}
const data = await dataset(join(siteRoot, "data/performance.js"));
const previous = await dataset(previousPath);
for (const [key, value] of Object.entries(previous)) {
  if (!["updated", "scope", "interpretation"].includes(key)) {
    assert.deepEqual(data[key], value, `historical ${key} changed`);
  }
}
const section = data.concurrentRounds;
validateRoundPerformance(section);
const pins = section.pins;
const prelaunch = await pinned(join(archive, "prelaunch-v1.json"), pins.prelaunchSha256);
assert.equal(prelaunch.schema, "FerricHostTimingPrelaunchV1");
assert.equal(prelaunch.controller_source, pins.controllerSourceRevision);
assert.equal(prelaunch.controller_dependency_cutoff, pins.controllerFe2o3Revision);
assert.equal(prelaunch.worker_core_source, pins.workerFe2o3Revision);
assert.equal(prelaunch.independent_worker_sha256, pins.hostWorkerSha256);
assert.equal(prelaunch.peer_worker_sha256, pins.peerWorkerSha256);
for (const [file, key] of [["compare_tp_batch.py", "comparatorSourceSha256"],
  ["performance_ledger.py", "ledgerGeneratorSourceSha256"], ["host_timing_summary.py", "hostTimingGeneratorSourceSha256"]]) {
  assert.equal(prelaunch.tool_sha256[file], pins[key]);
  await pinned(join(archive, "tools", file), pins[key], false);
}
await pinned(join(archive, "workload.json"), pins.workloadSha256);
await pinned(join(archive, "reference.json"), pins.referenceSha256);
const basePins = [["controllerSha256", "controller_sha256"], ["hsacoSha256", "artifact_hsaco_id"],
  ["manifestSha256", "artifact_manifest_id"], ["handoffSha256", "artifact_handoff_id"]];
for (const [site, raw] of basePins) assert.equal(prelaunch[raw], pins[site]);
const peerArtifact = {
  artifact_hsaco_id: pins.peerHsacoSha256,
  artifact_manifest_id: pins.peerManifestSha256,
  artifact_handoff_id: pins.peerHandoffSha256,
};
assert.deepEqual(prelaunch.peer_artifact, peerArtifact);
assert.equal(prelaunch.prefix_cache, true);
assert.equal(prelaunch.output_head_pruning, false);
assert.equal(prelaunch.batch_tokens, 16);
assert.equal(prelaunch.prefill_chunk, 16);
assert.equal(prelaunch.warmup_policy, "fresh-worker-no-warmup");
const expectedPerformance = { runtime_cache_admission: false, runtime_operational: true,
  dispatch_sequences: false, queue_rollover: false, projection: "baseline", attention: "baseline", runtime_profiling: false };
assert.deepEqual(prelaunch.performance_profile, expectedPerformance);
const metricKeys = [["outputTokensPerSecond", "output_tokens_per_second"], ["workloadSeconds", "workload_window_seconds"],
  ["setupSeconds", "setup_seconds"], ["wholeSeconds", "whole_seconds"]];
const cases = new Map();
for (const profile of section.profiles) {
  const item = await pinned(join(archive, "reports", `${profile.id}.json`), profile.caseFileSha256);
  const report = await pinned(join(archive, "reports", `${profile.id}.comparison.json`), profile.comparisonFileSha256);
  assert.equal(item.schema, "FerricHostTimingCaseV1");
  assert.equal(item.case, profile.id);
  assert.equal(item.passed, true);
  assert.equal(item.host_timing_passed, true);
  assert.equal(item.measurement, "host-wall-latency-not-gpu-duration");
  assert.equal(item.prelaunch_sha256, pins.prelaunchSha256);
  const collective = profile.mode === "host" ? null
    : profile.mode === "serial" ? "device-peer-serial-v4" : "device-peer-concurrent-round-v1";
  assert.equal(item.expected.collective, collective);
  assert.equal(item.expected.world, profile.world);
  assert.equal(item.expected.output_head_pruning, false);
  assert.equal(item.expected.prefix_cache, true);
  assert.equal(item.expected.workload_sha256, pins.workloadSha256);
  assert.equal(item.expected.reference_sha256, pins.referenceSha256);
  assert.deepEqual(item.expected.performance_profile, expectedPerformance);
  assert.equal(item.expected.worker_sha256, profile.mode === "host" ? pins.hostWorkerSha256 : pins.peerWorkerSha256);
  for (const [site, raw] of basePins) assert.equal(item.expected[raw], pins[site]);
  if (profile.mode === "host") assert.equal(item.expected.peer_artifact, undefined);
  else assert.deepEqual(item.expected.peer_artifact, peerArtifact);
  const metrics = item.metrics;
  assert.equal(metrics.comparison_sha256, profile.comparisonFileSha256);
  assert.equal(metrics.output_tokens, 8);
  assert.equal(metrics.physical_token_rows, 34);
  assert.equal(metrics.batch_count, 5);
  assert.deepEqual(Object.keys(metrics.requests).sort(), [...requestNames].sort());
  for (const [site, raw] of metricKeys) assert.equal(profile[site], metrics[raw]);
  requestNames.forEach((name, index) => {
    assert.equal(metrics.requests[name].decode_interval_count, [1, 2, 0, 1][index]);
    assert.deepEqual(profile.requestLatencies[index], [metrics.requests[name].ttft_seconds, metrics.requests[name].tpot_seconds]);
  });
  assert.equal(report.passed, true);
  assert.equal(report.authority, "none");
  assert.equal(report.tensor_parallel, profile.world);
  assert.equal(report.all_reference_tokens_and_bytes_match, true);
  assert.equal(report.clean_teardown_recorded, true);
  assert.equal(report.gpu_idle_before_and_after, true);
  assert.equal(report.comparator_sha256, pins.comparatorSourceSha256);
  assert.equal(report.expected_execution_profile.collective, collective);
  assert.deepEqual(report.expected_execution_profile.performance_profile, expectedPerformance);
  assert.deepEqual(report.identities, metrics.identities);
  cases.set(profile.id, item);
  console.log("EXACT CONCURRENT MATRIX CASE", profile.id);
}
for (const world of section.worlds) {
  const ledger = await pinned(join(archive, "reports", `tp${world.world}-performance.json`), world.ledgerFileSha256);
  const serial = await pinned(join(archive, "reports", `tp${world.world}-serial-vs-round-performance.json`), world.serialLedgerFileSha256);
  for (const [report, baseline, names] of [[ledger, "host", ["host", "serial", "round"]],
    [serial, "serial", ["serial", "round"]]]) {
    assert.equal(report.schema, "FerricTpPerformanceLedgerV1");
    assert.equal(report.authority, "none");
    assert.equal(report.baseline, baseline);
    assert.equal(report.warmup_policy, prelaunch.warmup_policy);
    assert.equal(report.equivalence.tensor_parallel, world.world);
    assert.equal(report.equivalence.workload_sha256, pins.workloadSha256);
    assert.equal(report.equivalence.reference_sha256, pins.referenceSha256);
    assert.deepEqual(report.variants.map((variant) => variant.name), names);
    for (const variant of report.variants) {
      const item = cases.get(`tp${world.world}-${variant.name}-profile-r1`);
      assert.equal(variant.repetitions, 1);
      assert.equal(variant.runs.length, 1);
      assert.deepEqual(variant.expected, item.expected);
      assert.deepEqual(variant.runs[0], item.metrics);
      for (const [, key] of metricKeys) assert.equal(variant.metrics[key].mean, item.metrics[key]);
    }
  }
  assert.deepEqual(serial.variants.map((variant) => variant.runs), ledger.variants.slice(1).map((variant) => variant.runs));
  assert.equal(ledger.variants[2].baseline_relative.output_tokens_per_second.mean.variant_over_baseline, world.roundOverHost);
  assert.equal(serial.variants[1].baseline_relative.output_tokens_per_second.mean.variant_over_baseline, world.roundOverSerial);
}
console.log("PASS: six matrix cases and both baselines exactly match frozen source-bound reports; all historical data is unchanged.");
