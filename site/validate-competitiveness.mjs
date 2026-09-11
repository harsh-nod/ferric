import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile, stat } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const expected = {
  source: "6148e61",
  currentReadinessCount: 6,
  core: "3e3a77284a61654134211f8145dd0ddeebb2ff91",
  implementationPrivate: true,
  jsonlImplemented: true,
  loopbackHttpImplemented: true,
  deployedServingQualified: false,
  hostTests: { library: 206, batchCli: 55, replicaControl: 6, sourcePolicy: 22, http: 16, ignored: 3 },
  hostTestsSha256: "6c8a0d7cf0793bce6a798279b012161caed0aeebdac1e6c758488e6cce513e5c",
  httpTestsSha256: "ed9cde7e582e47528838202c0ff89c6c2e986dc33dd7dd264d34bd57d3b36c73",
  strictClippyPassed: true,
  sourceGate: {
    source: "121609f5fa06ae8ee410024d859fdc470ead406d",
    passed: true, sourceTests: 38, verifierPolicyTests: 31,
    metadataConfigurations: 28, metadataManifests: 24,
    exactGeneratedInventories: 5, coverageModules: 173, coverageBodies: 8235,
    logSha256: "2338d1915d2ee00a335f5310ea7813bcbaa1271db4f2dec3e821fda12255914c",
    metadataSha256: "a0a5534dfb31f772325f605091400de53f35d3c7220eba231c2553deb049e0b4",
    newVerusProof: false, gpuQualification: false,
  },
  aggregateQualificationClaimed: false,
  head32: {
    world: 1, capacity: 32, nativeRows: [1, 16, 17, 31, 32], nativeCases: 15,
    nativePassed: true, allEightGpusIdle: true, modelEvidenceIncluded: false,
    fp32WorkspaceBytes: 19447808,
    imageSha256: "5f19b3ba59035a5f0ebc90cdf3a40466f9910908a45e082d146cb674028da6cb",
    nativeReportSha256: "5e27c9f5175321c570954b601d7d1cd0d5d14fd330ab347a4f9376ec1cf0a85a",
    nativeReviewSha256: "2dc850dd23d8f08fb88cdc7691ecea4ca99f1670ae664da8d98b1824b6414bab",
  },
  sharedCurrentness: { defaultEnabled: false, worlds: [2, 8], nativeCases: 4, nativePassed: true, modelSpeedClaimed: false },
  admissionCacheCanary: {
    core: "5110577a6d8c45390dfb353386cde748efd5d76c",
    repetitionsPerMode: 2, requests: 4, outputTokens: 8, exactReferencePassed: true,
    rateChangePercent: 27.72, reuseTtftChangePercent: -26.09, reuseTpotChangePercent: -37.16,
    wholeProcessChangePercent: 1.14, servingComparison: false,
    reportSha256: [
      "dd0f502b87cee2cdc42e9846775dfb7139a771989a5df97eaec3c9a45afd5abd",
      "54444deeeb0ef9b703e7b971cb5d05b6e32c114896ba0e85b80be96ce84ce673",
      "8cc8b0a8073ef9cb77e0c7f5fa5be1c9f0986b0875ae0f8fce65b3bb60283a52",
      "0f7b6db78afc006b5a1b3de11dc414c0b0d96f605f6c62e2b9b42834c697a0c5",
    ],
  },
  baselineLaunchApprovalPending: true,
  matchedExternalComparisonAvailable: false,
  speculativeFastPathImplemented: false,
  largeKvPoolImplemented: false,
  currentPhysicalPages: 512,
  aggregateKvSlots: 8192,
  primaryLongContext: { concurrentRequests: 32, inputTokens: 4096, outputTokens: 256, requiredPages: 8704, plannedMaximumPages: 16384, plannedKvGiB: 36 },
  competitiveWinClaimed: false,
  newVerusClaimed: false,
  authority: "none",
};

const plain = (value) => JSON.parse(JSON.stringify(value));

export function validateCompetitiveness(value) {
  assert.deepEqual(plain(value), expected, "competitiveness checkpoint or scope drifted");
}

export function testCompetitivenessRejections(value) {
  const mutations = [
    (x) => { x.core = x.admissionCacheCanary.core; },
    (x) => { x.head32.modelEvidenceIncluded = true; },
    (x) => { x.head32.nativeCases = 14; },
    (x) => { x.head32.capacity = 16; },
    (x) => { x.head32.fp32WorkspaceBytes = 9723904; },
    (x) => { x.sharedCurrentness.defaultEnabled = true; },
    (x) => { x.sharedCurrentness.modelSpeedClaimed = true; },
    (x) => { x.admissionCacheCanary.repetitionsPerMode = 4; },
    (x) => { x.admissionCacheCanary.servingComparison = true; },
    (x) => { x.admissionCacheCanary.wholeProcessChangePercent = -1.14; },
    (x) => { x.deployedServingQualified = true; },
    (x) => { x.largeKvPoolImplemented = true; },
    (x) => { x.speculativeFastPathImplemented = true; },
    (x) => { x.primaryLongContext.plannedMaximumPages = 8192; },
    (x) => { x.competitiveWinClaimed = true; },
    (x) => { x.newVerusClaimed = true; },
    (x) => { x.aggregateQualificationClaimed = true; },
    (x) => { x.sourceGate.newVerusProof = true; },
    (x) => { x.sourceGate.gpuQualification = true; },
    (x) => { x.extra = true; },
  ];
  for (const change of mutations) {
    const changed = plain(value);
    change(changed);
    assert.throws(() => validateCompetitiveness(changed));
  }
  console.log(`Competitiveness scope: ${mutations.length} negative mutations rejected.`);
}

const digest = (raw) => createHash("sha256").update(raw).digest("hex");
async function readPinned(path, hash) {
  assert((await stat(path)).size <= 8 * 1024 * 1024, `evidence exceeds bound: ${path}`);
  const raw = await readFile(path);
  assert.equal(digest(raw), hash, `evidence hash drift: ${path}`);
  return raw;
}

// This checks the new caption facts against frozen root-reviewed receipts. It
// does not replace the original native/token validators or qualify a model.
async function validateEvidence(root, serving, value) {
  const native = JSON.parse(await readPinned(join(root, "head32-native-v8/result.json"), value.head32.nativeReportSha256));
  assert.equal(native.schema, "FerricTpFp32Head32ProbeV8");
  assert.equal(native.artifact_sha256, value.head32.imageSha256);
  for (const key of ["checks_pass", "clean_teardown"]) assert.equal(native[key], true);
  for (const key of ["benchmark", "model_inference", "model_parity_qualified"]) assert.equal(native[key], false);
  assert.equal(native.results.length, 15);
  assert.deepEqual(native.results.map((r) => r.name), value.head32.nativeRows.flatMap((rows) =>
    ["scalar_head", "mfma_head", "fp32_argmax"].map((name) => `${name}_rows${rows}`)));
  for (const result of native.results) {
    if (!result.name.startsWith("fp32_argmax")) assert.equal(result.full_output_exact, true);
    for (const check of result.checks) assert.equal(check.guards_unchanged, true);
  }
  const review = JSON.parse(await readPinned(join(root, "head32-native-review.json"), value.head32.nativeReviewSha256));
  assert.equal(review.checks_pass, true);
  assert.equal(review.all_eight_gpu_idle_before_after, true);
  assert.equal(review.model_qualified, false);
  assert.equal(review.performance_qualified, false);
  for (const [path, hash] of Object.entries(review.inputs)) await readPinned(join(root, path), hash);
  await readPinned(join(serving, "current/final-tests.log"), value.hostTestsSha256);
  await readPinned(join(serving, "current/final-http-tests.log"), value.httpTestsSha256);
  const sourceLog = (await readPinned(join(root, "source-gate/check.log"), value.sourceGate.logSha256)).toString("utf8");
  assert(sourceLog.includes("38 passed; 0 failed"));
  assert(sourceLog.includes("31 passed; 0 failed"));
  assert(sourceLog.includes("173 modules, 8235 executable bodies"));
  assert(sourceLog.trimEnd().endsWith("PASS"));
  const metadata = JSON.parse(await readPinned(join(root, "source-gate/metadata-summary.json"), value.sourceGate.metadataSha256));
  assert.equal(metadata.revision, value.core);
  assert.equal(metadata.records.length, value.sourceGate.metadataConfigurations);
  assert.equal(new Set(metadata.records.map((r) => r.manifest)).size, value.sourceGate.metadataManifests);

  const names = ["control-r1", "cache-r1", "cache-r2", "control-r2"];
  const runs = [];
  for (const [index, name] of names.entries()) {
    const dir = join(root, `tp1-mfma-${name}`);
    const report = JSON.parse(await readPinned(join(dir, "comparison.json"), value.admissionCacheCanary.reportSha256[index]));
    for (const key of ["passed", "all_reference_tokens_and_bytes_match", "clean_teardown_recorded", "gpu_idle_before_and_after"]) assert.equal(report[key], true);
    assert.equal(report.generated_tokens, 8);
    assert.equal(report.tensor_parallel, 1);
    assert.equal(report.head_precision, "fp32-v7");
    const raw = await readPinned(join(dir, "results.jsonl"), report.input_sha256["results.jsonl"]);
    const events = raw.toString("utf8").trimEnd().split("\n").map((line) => JSON.parse(line));
    const setup = events[0];
    const policy = plain(setup.performance_profile);
    assert.equal(policy.runtime_cache_admission, name.startsWith("cache"));
    delete policy.runtime_cache_admission;
    assert.deepEqual(policy, { attention: "baseline", dispatch_sequences: false, projection: "mfma", queue_rollover: false, runtime_operational: true, runtime_profiling: false });
    assert.equal(setup.controller_sha256, "3d039c49bf4a4610f3b704d52c17cc46398f0c0f010c060af66c369b8ea48192");
    assert.equal(setup.worker_sha256, "aaa0216a77de0d5f12c2d668b31ca8c340d8975407c2b446bb5e20b5d820bd6e");
    assert.equal(setup.artifact_hsaco_id, "8c81d3fe869d3346d95486354f366988ce99210111cd0fc712dfb2194450b125");
    assert.equal(setup.fp32_head_artifact.artifact_hsaco_id, "d6086650521f72a27559cc049e51d167ea27d1990f0bbbb342b32e925057ff12");
    const requests = events.filter((e) => e.schema === "FerricQwen3TpBatchRequestV2");
    assert.equal(requests.length, 4);
    const times = requests.flatMap((r) => [r.arrival_ns, ...r.output_timestamps_ns, ...(r.cancelled_ns === null ? [] : [r.cancelled_ns])]);
    assert(times.every(Number.isSafeInteger));
    const start = Math.min(...requests.map((r) => r.arrival_ns));
    const end = Math.max(...requests.flatMap((r) => [...r.output_timestamps_ns, ...(r.cancelled_ns === null ? [] : [r.cancelled_ns])]));
    const reuse = report.requests["reuse-prefix"];
    runs.push({ rate: 8e9 / (end - start), ttft: reuse.ttft_ns, tpot: reuse.tpot_ns, whole: report.whole_seconds });
  }
  const mean = (indices, field) => indices.reduce((sum, i) => sum + runs[i][field], 0) / indices.length;
  const change = (field) => Number(((mean([1, 2], field) / mean([0, 3], field) - 1) * 100).toFixed(2));
  assert.equal(change("rate"), value.admissionCacheCanary.rateChangePercent);
  assert.equal(change("ttft"), value.admissionCacheCanary.reuseTtftChangePercent);
  assert.equal(change("tpot"), value.admissionCacheCanary.reuseTpotChangePercent);
  assert.equal(change("whole"), value.admissionCacheCanary.wholeProcessChangePercent);
  console.log("PASS: exact native/host receipts and four frozen cache-canary reports; caption metrics recomputed without serving qualification.");
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  assert.equal(process.argv.length, 4, "usage: node validate-competitiveness.mjs COMPETE_EVIDENCE SERVING_EVIDENCE");
  const context = { window: {} };
  vm.runInNewContext(await readFile(join(dirname(fileURLToPath(import.meta.url)), "data/project.js"), "utf8"), context);
  const value = context.window.FERRIC_PROJECT.competitivenessSprint;
  validateCompetitiveness(value);
  await validateEvidence(process.argv[2], process.argv[3], plain(value));
}
