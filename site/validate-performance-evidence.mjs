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
  // The legacy key holds the generator-script digest, not a canonical report ID.
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
async function repeatedPair(repeated, first, file, hash, world) {
  const repeatedLedger = await pinned(join(measurementRoot, file), hash);
  assert.equal(repeated.ledgerFileSha256, hash);
  assert.equal(repeatedLedger.variants.length, 2);
  assert.equal(repeatedLedger.comparator_sha256, first.pins.comparatorSha256);
  for (const [index, second] of repeated.secondProfiles.entries()) {
    const variant = repeatedLedger.variants[index];
    assert.equal(variant.repetitions, repeated.repetitions);
    assert.equal(variant.runs.length, 2);
    common(variant.expected, world, second.projection ?? "baseline");
    assert.equal(variant.expected.collective, world === 1 && index === 1 ? "device-tp1-v3" : null);
    for (const [target, source] of basePins) assert.equal(first.pins[target], variant.expected[source]);
    const runs = [first.profiles[index], second];
    runs.forEach((profile, runIndex) => requests(profile, variant.runs[runIndex]));
    for (const [target, source] of metricNames) {
      const values = runs.map((run) => run[target]);
      assert.equal(variant.metrics[source].mean, (values[0] + values[1]) / 2);
      assert.equal(variant.metrics[source].p50, Math.min(...values));
      assert.equal(variant.metrics[source].p95, Math.max(...values));
    }
    data.requests.forEach((request, requestIndex) => {
      for (const [valueIndex, source] of ["ttft_seconds", "tpot_seconds"].entries()) {
        const metrics = variant.requests[request.name].metrics[source];
        const values = runs.map((run) => run.requestLatencies[requestIndex][valueIndex]);
        if (values[0] === null) assert.equal(metrics, null);
        else {
          assert.equal(metrics.mean, (values[0] + values[1]) / 2);
          assert.equal(metrics.p50, Math.min(...values));
          assert.equal(metrics.p95, Math.max(...values));
        }
      }
    });
    checkedReport(await pinned(join(measurementRoot, `${variant.runs[1].id}-comparison.json`), second.comparisonSha256), world);
    console.log("EXACT REPEATED MODEL PAIR", variant.name);
  }
  const [control, candidate] = repeatedLedger.variants;
  const percentage = (base, value, lower) => ((lower ? 1 - value / base : value / base - 1) * 100).toFixed(2);
  for (const [base, value, lower] of [
    [control.metrics.output_tokens_per_second.mean, candidate.metrics.output_tokens_per_second.mean, false],
    ...["ttft_seconds", "tpot_seconds"].map((key) =>
      [control.requests["reuse-prefix"].metrics[key].mean, candidate.requests["reuse-prefix"].metrics[key].mean, true]),
  ]) assert(repeated.interpretation.includes(`${percentage(base, value, lower)}%`));
  return repeatedLedger;
}
const repeatedLedger = await repeatedPair(data.mfmaRepeated, data.mfmaPair, "mfma-paired-2reps.json",
  "6b14222502f670b4cd8a0d607777885c590b88874ad40190fbf56799528bcef2", 8);
await repeatedPair(data.deviceTp1Repeated, data.deviceTp1Pair, "device-tp1-paired-2reps.json",
  "41b064209d601df9d733210825fd39bfee7665950e67c7fd77ff647ef64f6f32", 1);
const cumulative = data.mfmaPruning;
const cumulativeLedger = await pinned(join(measurementRoot, "mfma-pruning-cumulative-r1.json"),
  "01a030080f363f83b40d9a382bd2b82b1c16ebff51810924efb5091a122994ba");
assert.equal(cumulative.ledgerFileSha256, "01a030080f363f83b40d9a382bd2b82b1c16ebff51810924efb5091a122994ba");
assert.equal(cumulativeLedger.variants.length, 3);
assert.deepEqual(cumulativeLedger.variants.slice(0, 2).map((variant) => variant.runs), repeatedLedger.variants.map((variant) => variant.runs));
const cumulativeVariant = cumulativeLedger.variants[2];
assert.equal(cumulativeVariant.repetitions, 1);
assert.equal(cumulativeVariant.runs.length, 1);
assert.equal(cumulativeVariant.expected.output_head_pruning, true);
common({ ...cumulativeVariant.expected, output_head_pruning: false }, 8, "mfma");
for (const [target, source] of basePins) assert.equal(data.mfmaPair.pins[target], cumulativeVariant.expected[source]);
requests(cumulative.profiles[0], cumulativeVariant.runs[0]);
checkedReport(await pinned(join(measurementRoot, "mfma-pruning-tp8-cache-r1-comparison.json"), cumulative.profiles[0].comparisonSha256), 8);
console.log("EXACT CUMULATIVE MFMA AND PRUNING");
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
const current = data.currentCompatibility;
const currentAcceptedHashes = [
  ["f7f25c01e5b7d99350fbb15fee0c35b73d20ab8188cdfb89e350b04ffe961ce7", "37122de41dabe0287976f572627bce042e9a3be4e9820b143124371a8aa88400"],
  ["67ad69a2ec6d29f2cc03f6eabe65cfcde7837cf606a37363837d65ddadb37c59", "8f395b5dbfa30b276efaeacec155ce6b033c1c612e1c64e4fa843752154c8c93"],
  ["66649461712577c7df2cef26cad9449beb288f7b34eb5156154925acb27eca06", "89a5521453330d8726d335febcd2ea3367e71093f69a54717bb0ad93f0ea69d9"],
];
for (const [index, profile] of current.accepted.entries()) {
  const [comparisonHash, metricsHash] = currentAcceptedHashes[index];
  const report = await pinned(join(measurementRoot, `${profile.id}-comparison.json`), comparisonHash);
  let record;
  if (index < 2) {
    record = await pinned(join(measurementRoot, `${profile.id}-metrics.json`), metricsHash);
  } else {
    const ledger = await pinned(join(measurementRoot, "latest-tp1-residual-pruning-pair-r1.json"), metricsHash);
    assert.equal(ledger.variants.length, 2);
    requests(current.accepted[0], ledger.variants[0].runs[0]);
    const variant = ledger.variants[1];
    assert.equal(variant.runs.length, 1);
    common({ ...variant.expected, output_head_pruning: false }, 1);
    const bytes = await pinned(join(measurementRoot, `${profile.id}-results.jsonl`), report.input_sha256["results.jsonl"], false);
    const rows = bytes.toString("utf8").trimEnd().split("\n").map((line) => JSON.parse(line))
      .filter((item) => item.schema === "FerricQwen3TpBatchCompletedV2").map((item) => item.rows.length);
    record = {
      passed: report.passed, case: variant.runs[0].id, repetitions: variant.repetitions,
      output_head_pruning: variant.expected.output_head_pruning, collective: variant.expected.collective,
      performance_profile: variant.expected.performance_profile, actual_completed_rows: rows,
      max_observed_rows: Math.max(...rows), metrics: variant.runs[0], comparison_sha256: variant.runs[0].comparison_sha256,
    };
  }
  assert.equal(profile.comparisonSha256, comparisonHash);
  assert.equal(profile.metricsFileSha256, metricsHash);
  checkedReport(report, profile.world);
  assert.equal(record.passed, true);
  assert.equal(record.case, profile.id);
  assert.equal(record.repetitions, 1);
  assert.equal(record.output_head_pruning, profile.outputHeadPruning);
  assert.equal(record.collective, profile.collective);
  assert.deepEqual(record.performance_profile, { ...data.runtimeProfile, projection: profile.projection });
  assert.deepEqual(report.expected_execution_profile.performance_profile, record.performance_profile);
  assert.equal(report.expected_execution_profile.output_head_pruning, profile.outputHeadPruning);
  assert.equal(report.expected_execution_profile.collective, profile.collective);
  assert.deepEqual(profile.actualBatchRows, record.actual_completed_rows);
  assert.equal(Math.max(...profile.actualBatchRows), record.max_observed_rows);
  if (profile.imageProfile === "v5-mfma32") {
    assert.equal(report.kernel_row_capacity, 32);
    assert.equal(report.maximum_batch_rows_observed, 17);
  }
  assert.equal(report.physical_token_rows, 34);
  const imagePins = profile.imageProfile === "v5-mfma32" ? data.wideRowPair.pins : data.mfmaPair.pins;
  for (const [target, source] of basePins) {
    assert.equal(report.identities[source], target.startsWith("controller") || target.startsWith("worker")
      ? current.pins[target] : imagePins[target]);
  }
  requests(profile, { ...record.metrics, comparison_sha256: record.comparison_sha256 });
  console.log("EXACT CURRENT-CONTROLLER ACCEPTED", profile.id);
}
for (const [index, profile] of current.rejected.entries()) {
  const hash = ["be835327607d5a88c4b421327f87378c57c8513947881e1747044fe9ccf7a5b9",
    "2402b01e3d4896d97b0546ff2da09ab23fe655d41e406e731c366d995473bd26"][index];
  const rejection = await pinned(join(measurementRoot, `${profile.id}-rejection.json`), hash);
  assert.equal(profile.rejectionSha256, hash);
  assert.equal(rejection.passed, false);
  assert.equal(rejection.case, profile.id);
  assert.equal(rejection.repetitions, 1);
  assert.equal(Object.hasOwn(rejection, "metrics"), false);
  assert.equal(rejection.output_head_pruning, profile.outputHeadPruning);
  assert.equal(rejection.collective, profile.collective);
  assert.deepEqual(rejection.performance_profile, { ...data.runtimeProfile, projection: profile.projection });
  for (const [target, source] of basePins) {
    assert.equal(rejection.expected_identities[source], target.startsWith("controller") || target.startsWith("worker")
      ? current.pins[target] : data.mfmaPair.pins[target]);
  }
  assert.equal(rejection.mismatches.length, 1);
  const mismatch = rejection.mismatches[0];
  assert.equal(mismatch.name, profile.requestName);
  assert.deepEqual(mismatch.expected_tokens, profile.expectedTokens);
  assert.deepEqual(mismatch.observed_tokens, profile.observedTokens);
  console.log("EXACT CURRENT-CONTROLLER REJECTION", profile.id);
}
const currentControl = current.accepted[0];
const currentCumulative = current.accepted[2];
for (const percentage of [
  ((currentCumulative.outputTokensPerSecond / currentControl.outputTokensPerSecond - 1) * 100).toFixed(2),
  ...[0, 1].map((index) => ((1 - currentCumulative.requestLatencies[3][index] / currentControl.requestLatencies[3][index]) * 100).toFixed(2)),
]) assert(current.interpretation.includes(`${percentage}%`));
const host = data.hostTranspose;
const sourcePeer = data.peerSourceControls;
const transposeModel = data.transposeModelPair;
const transposeLedger = await pinned(join(measurementRoot, "transpose-source-paired-r1.json"),
  "389c7b8ee88fb443e2f3396cb0978e92509142e58c3087321de43bc097d0b191");
assert.equal(transposeModel.pins.ledgerFileSha256, "389c7b8ee88fb443e2f3396cb0978e92509142e58c3087321de43bc097d0b191");
assert.equal(transposeLedger.variants.length, 2);
for (const [index, profile] of transposeModel.profiles.entries()) {
  const variant = transposeLedger.variants[index];
  assert.equal(variant.name, profile.id);
  assert.equal(variant.repetitions, 1);
  common(variant.expected, 8, "mfma");
  assert.equal(variant.expected.controller_sha256, profile.controllerSha256);
  for (const [target, source] of basePins.slice(1)) assert.equal(transposeModel.pins[target], variant.expected[source]);
  requests(profile, variant.runs[0]);
  checkedReport(await pinned(join(measurementRoot, `transpose-${index === 0 ? "untiled" : "tiled"}-mfma-tp8-r1-comparison.json`), profile.comparisonSha256), 8);
  console.log("EXACT TRANSPOSE MODEL OBSERVATION", profile.id);
}
for (const [key, percentage] of [["setupSeconds", "15.33"], ["wholeSeconds", "13.78"]]) {
  const [control, candidate] = transposeModel.profiles;
  assert.equal(((1 - candidate[key] / control[key]) * 100).toFixed(2), percentage);
  assert(transposeModel.interpretation.includes(`${percentage}%`));
}
for (const [index, control] of sourcePeer.controls.entries()) {
  const hash = ["d59696cb6616994677e42db34aab16cd33396a99a90a29f29965392b135f5a85",
    "886a3d29d7555cda2de47ab3ea305f0567bab235a7d1384ee2700b0259ab1ccc"][index];
  const ledger = await pinned(join(measurementRoot, `peer-source-matched-tp${control.world}-r1.json`), hash);
  assert.equal(control.ledgerFileSha256, hash);
  assert.equal(ledger.variants.length, 2);
  for (const [variantIndex, profile] of [control, data.peerObservations.profiles[index]].entries()) {
    const variant = ledger.variants[variantIndex];
    assert.equal(variant.repetitions, 1);
    common(variant.expected, control.world);
    assert.equal(variant.expected.controller_sha256, sourcePeer.controllerSha256);
    assert.equal(variant.expected.worker_sha256, variantIndex === 0 ? sourcePeer.hostWorkerSha256 : sourcePeer.peerWorkerSha256);
    assert.equal(variant.expected.collective, variantIndex === 0 ? null : "device-peer-serial-v4");
    for (const [target, source] of basePins.slice(2)) assert.equal(variant.expected[source], data.identities[target]);
    requests(profile, variant.runs[0]);
  }
  checkedReport(await pinned(join(measurementRoot, `host-source-control-tp${control.world}-r1-comparison.json`), control.comparisonSha256), control.world);
  const reduction = ((1 - data.peerObservations.profiles[index].outputTokensPerSecond / control.outputTokensPerSecond) * 100).toFixed(2);
  assert(sourcePeer.interpretation.includes(`${reduction}%`));
  console.log("EXACT SOURCE-MATCHED PEER CONTROL", control.world);
}
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
const wide = data.wideRowPair;
const widePair = await pinned(join(replicaRoot, "replica-wide-policy-r1-pair.json"),
  "0bd49a1c66cb297e9eb20fd502ca5c99faefbaf722bd963de3acb96b5cc6fb6e");
assert.equal(widePair.passed, true);
assert.equal(widePair.authority, "none");
assert.equal(widePair.all_cohort_intervals_serialized, true);
assert.equal(wide.pins.pairComparisonSha256, "0bd49a1c66cb297e9eb20fd502ca5c99faefbaf722bd963de3acb96b5cc6fb6e");
for (const [target, source] of basePins) assert.equal(wide.pins[target], widePair.common_policy[source]);
assert.deepEqual(widePair.common_policy.performance_profile, data.runtimeProfile);
assert.equal(widePair.common_policy.kernel_profile, "v5-mfma32");
assert.equal(widePair.common_policy.layout, "1xTP8");
const wideHashes = ["01a6e3579689a7bf9c219f5dcc00480bfd3540f237127065d495ee76c26ab5a7",
  "0f67a449f4d17a5ec7012d551e899c4b9303d880c7ce06f6f0a55ecec9e026b7"];
for (const [index, profile] of wide.profiles.entries()) {
  const report = await pinned(join(replicaRoot, `replica-wide${profile.rows}-operational-r1-comparison.json`), wideHashes[index]);
  assert.equal(report.passed, true);
  assert.equal(report.authority, "none");
  assert.equal(report.global_before_after_idle, true);
  assert.equal(report.all_controllers_reaped, true);
  assert.equal(report.global_output_tokens, 64);
  assert.equal(report.replica_count, 1);
  assert.equal(report.physical_token_rows, 96);
  assert.equal(report.repetition_count, profile.repetitions);
  assert.equal(report.expectation.batch_tokens, profile.rows);
  assert.equal(report.expectation.prefill_chunk, profile.prefillChunk);
  assert.equal(report.expectation_sha256, profile.expectationSha256);
  assert.equal(report.workload_sha256, cohorts.pins.workloadSha256);
  assert.deepEqual(report.weight_payload_bytes, { device_base: 16385728512, device_transposed: 0, host_target: 16381470720 });
  for (const [target, source] of basePins) assert.equal(wide.pins[target], report.expectation[source]);
  assert.deepEqual(profile.actualBatchRows, widePair.policies[index].actual_batch_rows);
  assert.equal(profile.comparisonSha256, wideHashes[index]);
  assert.equal(widePair.policies[index].report_sha256, profile.comparisonSha256);
  for (const [target, source] of [["outputTokensPerSecond", "global_output_tokens_per_second"],
    ["releaseToLastOutputNs", "release_to_last_output_ns"], ["barrierSetupNs", "barrier_setup_ns"],
    ["spawnToReapNs", "whole_cohort_spawn_to_reap_ns"], ["releaseEpochNs", "global_release_epoch_ns"]]) {
    assert.equal(profile[target], report[source]);
  }
  assert.equal(profile.maximumLatenessNs, report.replicas[0].lateness_ns);
  for (const [requestIndex, values] of profile.requestLatencies.entries()) {
    const actual = report.requests[`replica-request-${String(requestIndex).padStart(2, "0")}`];
    assert.equal(actual.output_tokens, 8);
    assert.equal(actual.decode_interval_count, 7);
    assert.deepEqual(values, [actual.admission_ttft_ns, actual.release_to_first_token_ns, actual.tpot_seconds]);
  }
  console.log("EXACT WIDE ROW POLICY", profile.rows);
}
assert.equal(((wide.profiles[1].outputTokensPerSecond / wide.profiles[0].outputTokensPerSecond - 1) * 100).toFixed(2), "12.56");
assert(wide.interpretation.includes("12.56%"));
console.log("PASS: model observations, allocation and wide cohorts, rejected wave pins, CPU helper aggregates and historical measurements match frozen evidence");
