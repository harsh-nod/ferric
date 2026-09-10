import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const [measurementRoot, transposeRoot, previousPerformance, replicaRoot] = process.argv.slice(2);
assert(measurementRoot && transposeRoot && previousPerformance && replicaRoot,
  "usage: node validate-performance-evidence.mjs MEASUREMENT_ARCHIVE TRANSPOSE_ARCHIVE PREVIOUS_PERFORMANCE_JS REPLICA_ARCHIVE");
const root = dirname(fileURLToPath(import.meta.url));
async function siteData(path) {
  const context = { window: {} };
  vm.runInNewContext(await readFile(path, "utf8"), context);
  return JSON.parse(JSON.stringify(context.window.FERRIC_PERFORMANCE));
}
async function pinned(path, expected, json = true) {
  const bytes = await readFile(path);
  assert.equal(createHash("sha256").update(bytes).digest("hex"), expected, path);
  return json ? JSON.parse(bytes) : bytes;
}
const data = await siteData(join(root, "data/performance.js"));
const previous = await siteData(previousPerformance);
for (const field of ["variants", "requests", "pruningReuse", "runtimeProfile", "ablations"]) {
  assert.deepEqual(data[field], previous[field], `historical ${field} changed`);
}
for (const [key, value] of Object.entries(previous.identities)) assert.equal(data.identities[key], value);
const metricNames = [["outputTokensPerSecond", "output_tokens_per_second"],
  ["workloadSeconds", "workload_window_seconds"], ["setupSeconds", "setup_seconds"], ["wholeSeconds", "whole_seconds"]];
const basePins = [["controllerSha256", "controller_sha256"], ["workerSha256", "worker_sha256"],
  ["hsacoSha256", "artifact_hsaco_id"], ["manifestSha256", "artifact_manifest_id"], ["handoffSha256", "artifact_handoff_id"]];
function requests(profile, metrics) {
  for (const [index, request] of data.requests.entries()) {
    const source = metrics.requests[request.name];
    assert.equal(source.decode_interval_count, request.gapsPerRepetition);
    assert.deepEqual(profile.requestLatencies[index], [source.ttft_seconds, source.tpot_seconds]);
  }
  assert.equal(metrics.output_tokens, 8);
  for (const [target, source] of metricNames) assert.equal(profile[target], metrics[source]);
  assert.equal(profile.comparisonSha256, metrics.comparison_sha256);
}
function common(expected, world, projection = "baseline") {
  assert.equal(expected.world, world);
  assert.equal(expected.output_head_pruning, false);
  assert.equal(expected.prefix_cache, true);
  assert.equal(expected.workload_sha256, data.identities.workloadSha256);
  assert.equal(expected.reference_sha256, data.identities.referenceSha256);
  assert.deepEqual(expected.performance_profile, { ...data.runtimeProfile, projection });
}
function checkedReport(report, world) {
  assert.equal(report.passed, true);
  assert.equal(report.authority, "none");
  assert.equal(report.tensor_parallel, world);
  assert.equal(report.all_reference_tokens_and_bytes_match, true);
  assert.equal(report.clean_teardown_recorded, true);
  assert.equal(report.gpu_idle_before_and_after, true);
}
for (const [section, file, hash, reports, world] of [
  [data.mfmaPair, "mfma-paired-r1-ledger.json", "8660be59f4d452e3944131069c5a7824c0c72f6c3e87bfcd83f4a7d475ad3884",
    ["mfma-image-control-r1-comparison.json", "mfma-projection-r1-comparison.json"], 8],
  [data.deviceTp1Pair, "device-tp1-paired-r1-ledger.json", "07cae3b84184cf8fa806a3ce452a2a6959b7ad3f16bf27587abf06e0e4d1d385",
    ["device-tp1-control-r1-comparison.json", "device-tp1-residual-r1-comparison.json"], 1],
]) {
  const ledger = await pinned(join(measurementRoot, file), hash);
  assert.equal(section.pins.ledgerFileSha256, hash);
  assert.equal(section.pins.comparatorSha256, ledger.comparator_sha256);
  if (section.pins.ledgerCanonicalId) assert.equal(section.pins.ledgerCanonicalId, ledger.ledger_sha256);
  assert.equal(ledger.variants.length, 2);
  for (const [index, profile] of section.profiles.entries()) {
    const variant = ledger.variants[index];
    assert.equal(variant.name, profile.id);
    assert.equal(variant.repetitions, profile.repetitions);
    assert.equal(variant.runs.length, 1);
    common(variant.expected, world, profile.projection ?? "baseline");
    for (const [target, source] of basePins) assert.equal(section.pins[target], variant.expected[source]);
    requests(profile, variant.runs[0]);
    assert.equal(variant.expected.collective, world === 1 && index === 1 ? "device-tp1-v3" : null);
    for (const [target, source] of metricNames) assert.equal(profile[target], variant.metrics[source].mean);
    checkedReport(await pinned(join(measurementRoot, reports[index]), profile.comparisonSha256), world);
    console.log("EXACT MODEL PAIR", profile.id);
  }
}
for (const [index, world] of [2, 8].entries()) {
  const profile = data.peerObservations.profiles[index];
  const hashes = ["b7b5af6dea6b681b054a7bc664395c2992705df5cf39ad44c5a027d11dea4004",
    "f27a0729075dbc812735fffe96f5a7c41dd44be8b50c40aeb90c40df09fb3bb4"];
  const record = await pinned(join(measurementRoot, `peer-tp${world}-operational-r1-metrics.json`), hashes[index]);
  assert.equal(record.passed, true);
  assert.equal(record.authority, "none");
  assert.equal(record.repetition_count, 1);
  assert.equal(record.matched_control, null);
  assert.equal(profile.metricsFileSha256, hashes[index]);
  assert.equal(profile.world, world);
  common(record.expect, world);
  requests(profile, record.metrics);
  assert.equal(record.expect.collective, data.peerObservations.collective);
  for (const [target, source] of basePins) {
    const key = target.startsWith("controller") || target.startsWith("worker")
      ? target : `base${target[0].toUpperCase()}${target.slice(1)}`;
    assert.equal(data.peerObservations.pins[key], record.expect[source]);
  }
  for (const [target, source] of basePins.slice(2)) {
    assert.equal(data.peerObservations.pins[`peer${target[0].toUpperCase()}${target.slice(1)}`], record.expect.peer_artifact[source]);
  }
  checkedReport(await pinned(join(measurementRoot, `peer-tp${world}-operational-r1-comparison.json`), profile.comparisonSha256), world);
  console.log("EXACT UNMATCHED PEER", profile.id);
}
for (const [mode, hash] of [["projection", "ad9ecdbc5e35a7013a1e3dd68fcaba5344ed3b5b3b3dc07981b5250d930002b1"],
  ["attention", "494a2a258ed6df34a5f86c4f216b1032eed5f383757dc3ce6b83f029e171756c"]]) {
  await pinned(join(measurementRoot, `wave-${mode}-operational-r1-rejection.json`), hash);
  assert.equal(data.identities[`wave${mode[0].toUpperCase()}${mode.slice(1)}OperationalRejectionSha256`], hash);
}
const host = data.hostTranspose;
for (const [section, rateGain, ttftGain, tpotGain] of [[data.mfmaPair, "27.91", "30.85", "32.21"],
  [data.deviceTp1Pair, "14.19", "6.76", "0.97"]]) {
  const [control, candidate] = section.profiles;
  assert.equal(((candidate.outputTokensPerSecond / control.outputTokensPerSecond - 1) * 100).toFixed(2), rateGain);
  assert.equal(((1 - candidate.requestLatencies[3][0] / control.requestLatencies[3][0]) * 100).toFixed(2), ttftGain);
  assert.equal(((1 - candidate.requestLatencies[3][1] / control.requestLatencies[3][1]) * 100).toFixed(2), tpotGain);
  for (const number of [rateGain, ttftGain, tpotGain]) assert(section.interpretation.includes(`${number}%`));
}
for (const profile of data.mfmaPair.profiles) {
  assert(data.mfmaPair.interpretation.includes(profile.setupSeconds.toFixed(3)));
  assert(data.mfmaPair.interpretation.includes(profile.wholeSeconds.toFixed(3)));
}
const summary = await pinned(join(transposeRoot, "remote/tiled-v3/summary.json"),
  "bec975d68a204c7073204f6e5851475d249047564d7352269ebcc2fa5e47e40e");
await pinned(join(transposeRoot, "remote/tiled-v3/benchmark.log"),
  "33ef5e1277f3733814dbd83da4ffbce4af6a1673cd739b64329860324b27f6e6", false);
assert.equal(host.summarySha256, "bec975d68a204c7073204f6e5851475d249047564d7352269ebcc2fa5e47e40e");
assert.equal(host.rawLogSha256, "33ef5e1277f3733814dbd83da4ffbce4af6a1673cd739b64329860324b27f6e6");
assert.equal(host.modelTiming, summary.model_timing);
assert.equal(host.allocationHashUploadIncluded, summary.allocation_hash_upload_included);
assert.equal(host.fullArrayComparisons, summary.full_array_equal_comparisons);
assert.equal(summary.cases.length * host.repetitions, host.fullArrayComparisons);
assert.deepEqual(host.groups, summary.groups.map((group) => ({
  world: group.tp, cases: group.cases, baselineSeconds: group.suite_baseline_seconds,
  tiledSeconds: group.suite_tiled_seconds, helperSpeedup: group.helper_speedup,
})));
const cohorts = data.replicaCohorts;
const cohortHashes = ["8891e3d384f97030446d232e8ba24dac3128e192829c755f8747fd9d4ef42951",
  "67ef374b3d6419b780a9f7281a9b1696fd9967de4d3ba1efeb09b8d7547ecb7d",
  "729d282edf92ac0418f9f501afee60143382dab64dbf04537240ba8983257856"];
const allocation = await pinned(join(replicaRoot, "replica-allocation-r1-3-cases.json"),
  "47d98300668c31db0dd8f91846a4d6a7b361e353c419d7d17a911c7625ec7b32");
assert.equal(cohorts.pins.allocationComparisonSha256, "47d98300668c31db0dd8f91846a4d6a7b361e353c419d7d17a911c7625ec7b32");
assert.equal(allocation.all_cohort_intervals_serialized, true);
assert.equal(allocation.authority, "none");
assert.equal(allocation.layouts.length, 3);
for (const [index, profile] of cohorts.profiles.entries()) {
  const report = await pinned(join(replicaRoot, `replica-${profile.layout.toLowerCase()}-operational-r1-comparison.json`), cohortHashes[index]);
  assert.equal(profile.comparisonSha256, cohortHashes[index]);
  assert.equal(allocation.layouts[index].comparison_sha256, profile.comparisonSha256);
  assert.equal(allocation.layouts[index].global_output_tokens_per_second, profile.outputTokensPerSecond);
  assert.equal(report.passed, true);
  assert.equal(report.authority, "none");
  assert.equal(report.global_before_after_idle, true);
  assert.equal(report.all_controllers_reaped, true);
  assert.equal(report.global_output_tokens, 64);
  assert.equal(report.repetition_count, profile.repetitions);
  assert.equal(report.expectation.layout, profile.layout);
  assert.equal(report.replica_count, profile.replicas);
  assert.equal(report.expectation_sha256, profile.expectationSha256);
  assert.deepEqual(report.row_policy, { row_budget: 16, scope: "per-instance" });
  assert.equal(profile.perInstanceRows, report.per_instance_row_budget);
  assert.equal(profile.totalRowBudget, report.total_row_budget);
  assert.equal(report.expectation.clock_domain.clock, "CLOCK_MONOTONIC_RAW");
  assert.equal(report.expectation.output_head_pruning, false);
  assert.deepEqual(report.expectation.performance_profile, data.runtimeProfile);
  for (const [target, source] of basePins) assert.equal(cohorts.pins[target], report.expectation[source]);
  assert.equal(cohorts.pins.workloadSha256, report.workload_sha256);
  assert.equal(cohorts.pins.referenceSha256, report.reference_sha256);
  assert.equal(cohorts.pins.cohortComparatorSha256, report.checker_sha256["compare_replica_cohort.py"]);
  assert.equal(cohorts.pins.traceComparatorSha256, report.checker_sha256["replica_trace.py"]);
  assert.equal(cohorts.pins.batchComparatorSha256, report.checker_sha256["compare_tp_batch.py"]);
  for (const [target, source] of [["outputTokensPerSecond", "global_output_tokens_per_second"],
    ["releaseToLastOutputNs", "release_to_last_output_ns"], ["barrierSetupNs", "barrier_setup_ns"],
    ["spawnToReapNs", "whole_cohort_spawn_to_reap_ns"], ["releaseEpochNs", "global_release_epoch_ns"],
    ["physicalTokenRows", "physical_token_rows"]]) assert.equal(profile[target], report[source]);
  assert.equal(profile.maximumLatenessNs, Math.max(...report.replicas.map((replica) => replica.lateness_ns)));
  assert.equal(profile.hostWeightBytes, report.weight_payload_bytes.host_target);
  assert.equal(profile.gpuBaseWeightBytes, report.weight_payload_bytes.device_base);
  assert.equal(profile.gpuTransposedWeightBytes, report.weight_payload_bytes.device_transposed);
  for (const [requestIndex, values] of profile.requestLatencies.entries()) {
    const name = `replica-request-${String(requestIndex).padStart(2, "0")}`;
    const actual = report.requests[name];
    assert.equal(actual.output_tokens, 8);
    assert.equal(actual.decode_interval_count, 7);
    const instance = report.replicas.findIndex((replica) => Object.hasOwn(replica.requests, name));
    assert.deepEqual(values, [instance, actual.admission_ttft_ns, actual.release_to_first_token_ns, actual.tpot_seconds]);
  }
  console.log("EXACT COMMON-CLOCK COHORT", profile.layout);
}
for (const profile of cohorts.profiles.slice(1)) {
  const ratio = (profile.outputTokensPerSecond / cohorts.profiles[0].outputTokensPerSecond).toFixed(3);
  assert(cohorts.interpretation.includes(`${ratio}x`));
}
console.log("PASS: six model observations, cohort timing/custody, rejected wave pins, CPU helper aggregates and historical measurements match frozen evidence");
