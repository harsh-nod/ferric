import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile, stat } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const expected = {
  currentReadinessCount: 10,
  core: "f85bb375e7d6f4b8193e697293d29a8888439f0b",
  historicalCheckpointPreserved: true,
  fixedRequests: 4, fixedOutputTokens: 8, repetitionsPerMode: 2,
  wave: {
    core: "3e3a77284a61654134211f8145dd0ddeebb2ff91",
    exactReferencePassed: true, rateChangePercent: 8.70,
    reuseTtftChangePercent: -9.08, reuseTpotChangePercent: -10.51,
    headPrecision: "fp32-v8", batchTokens: 16, prefillChunk: 16,
    largeKvSupported: false, newDefault: false,
    reportSha256: [
      "a45e36d09a3ceba55c44925ee9612b35ec11c18a3b79e00e37459362c9a6d28c",
      "45839772c5641b78dc38cd7269706fc4846de9269c57790756919ec244c7a239",
      "1369d31bf3c42ea8ee6154cf06f5d1b7b7a07dd00711acc6df550da73e007a22",
      "1723b73813c5a03908cd185718031f088a90c841732074582f993f2483ab5045",
    ],
  },
  polling: {
    publicDefaultChanged: false, candidatesPublished: false,
    adaptive: {
      maximumBackoffMicroseconds: 1000, exactReferencePassed: true,
      performanceRejected: true, rateChangePercent: -18.35,
      reportSha256: [
        "f5cc20a8ec0642c3769838e667c537a3c3a9b4f884659da70efce0ee80a94525",
        "bc455411a2d41c54c66679da68f67d0a8bf42e77d01b5491c68fb8efdfeef962",
        "503950606239232b2e2ad54ee282d74fa9a525e25abbc3db8ba80172cdd7606f",
        "7286420a7f3e078736483e80d9ee72fb68c2bbcaff89063e6881dc9089fe374a",
      ],
    },
    capped: {
      maximumBackoffMicroseconds: 50, exactReferencePassed: true,
      rateChangePercent: 16.23, adopted: false,
      reportSha256: [
        "c8f8ac9a01f5801ed23e1c1148dca9722abf530d7b5b0ca72f3065425357c8d5",
        "58ff35325490b60f7ea7f8c87752716eff42520e9fbce70866787f3ac5a3491c",
        "8e03ca8a93fb3e7245dc19c3ab38a2c84f649e10eae7958c374bfa080e6369cb",
        "7999203e03ebe6e7114a5ff8e105f50c3fa65d79d3e23eb59911eb2310517ce6",
      ],
    },
  },
  largeKv: {
    hostImplemented: true, source: "77c9aa4f28102e43e7c089db3850d430947cd54f",
    profile: "large-kv-v9", world: 1, rowCapacity: 32,
    maximumPhysicalPages: 16384, maximumKvPayloadBytes: 38654705664,
    logicalContextTokens: 8192, legacyPhysicalPages: 512,
    nativeCases: 9, nativePassed: true, allEightGpusIdle: true,
    maximumPhysicalPageExercised: 16383, maximumLogicalPositionExercised: 8191,
    modelQualified: false, concurrency32Qualified: false,
    hostAggregateSha256: "726864fdc14b1d4ad44e74f0abc93e8c540db68d0d8b518ba42cda125cc6e618",
    nativeReportSha256: "4b9791bf859236a031c56f2de197068ebfd3ec86c5591ebedba348a9c9ef7ceb",
    nativeReviewSha256: "d69a9d9ab627661ee7dfddd662783b5911bb0eeff4dd05aff88f762f9a7428f2",
    modelCanary: {
      core: "3e3a77284a61654134211f8145dd0ddeebb2ff91",
      repetitionsPerMode: 1, physicalPages: [8, 16384],
      outputTokensPerRun: 8, physicalRowsPerRun: 34, batchesPerRun: 5,
      maximumRowsObserved: 16, dispatchesPerRun: 3077,
      exactReferencePassed: true, fullMaximumKvAllocationObserved: true,
      allEightGpusIdle: true, speedupClaimed: false,
      reportSha256: [
        "f7392fd154fcd77ae186387d4da3018e7315826d29be9da9ed2ec71cc2d5dd4d",
        "bf372d42f51dffdf8144cee96b9a3f68c5105ee60a96406272c541cead250fb4",
      ],
    },
  },
  orderedBatches: {
    published: true, maximumPackets: 16, nativeModes: ["full", "operational"],
    packetsPerMode: 184, chainsPerMode: 26, actualRolloversPerMode: 1,
    nativePassed: true, allEightGpusIdle: true, serialSequenceUnchanged: true,
    modelSpeedQualified: false,
    nativeReportSha256: [
      "376d2fb2adc88af1481fe3c89c8e768dd01f68fa50065cd95a6615230a770a4e",
      "0cf0c74446e9d1945076aec74d33bb9dad0dfee849ee1a5ce5c5d3e58fc272fc",
    ],
    nativeReviewSha256: "5754053d21a8de9e5f79c1ce2cfbdd70859c1c0db9179bff0232587046c2cd42",
  },
  draftIntake: {
    source: "8d6418f", optIn: true, authenticatedPayloadBytes: 1503264768,
    defaultRetainsDraftPayload: false, hostTests: 305,
    testsSha256: "9045a562cf3f4fbc58e7be1f97e868a6c76ad2247dd6f949d4232723cf6514e1",
    modelBackedSuccessTest: false, speculativeFastPathImplemented: false,
  },
  matchedExternalComparisonAvailable: false, sustainedServingQualified: false,
  competitiveWinClaimed: false, newVerusClaimed: false, authority: "none",
};

const plain = (value) => JSON.parse(JSON.stringify(value));
export function validateFollowup(value) {
  assert.deepEqual(plain(value), expected, "competitiveness follow-up scope drifted");
}

export function testFollowupRejections(value) {
  const mutations = [
    (x) => { x.core = x.wave.core; },
    (x) => { x.fixedRequests = 32; },
    (x) => { x.fixedOutputTokens = 64; },
    (x) => { x.repetitionsPerMode = 4; },
    (x) => { x.wave.headPrecision = "bf16"; },
    (x) => { x.wave.largeKvSupported = true; },
    (x) => { x.wave.newDefault = true; },
    (x) => { x.wave.rateChangePercent = 10; },
    (x) => { x.polling.adaptive.performanceRejected = false; },
    (x) => { x.polling.adaptive.rateChangePercent = 18.35; },
    (x) => { x.polling.capped.adopted = true; },
    (x) => { x.polling.candidatesPublished = true; },
    (x) => { x.largeKv.maximumPhysicalPages = 8192; },
    (x) => { x.largeKv.logicalContextTokens = 262144; },
    (x) => { x.largeKv.maximumKvPayloadBytes = 36_000_000_000; },
    (x) => { x.largeKv.modelQualified = true; },
    (x) => { x.largeKv.concurrency32Qualified = true; },
    (x) => { x.largeKv.modelCanary.maximumRowsObserved = 32; },
    (x) => { x.largeKv.modelCanary.core = x.core; },
    (x) => { x.largeKv.modelCanary.speedupClaimed = true; },
    (x) => { x.orderedBatches.modelSpeedQualified = true; },
    (x) => { x.orderedBatches.actualRolloversPerMode = 0; },
    (x) => { x.orderedBatches.chainsPerMode = 24; },
    (x) => { x.draftIntake.modelBackedSuccessTest = true; },
    (x) => { x.draftIntake.speculativeFastPathImplemented = true; },
    (x) => { x.draftIntake.defaultRetainsDraftPayload = true; },
    (x) => { x.matchedExternalComparisonAvailable = true; },
    (x) => { x.sustainedServingQualified = true; },
    (x) => { x.competitiveWinClaimed = true; },
    (x) => { x.newVerusClaimed = true; },
  ];
  for (const mutate of mutations) {
    const changed = plain(value);
    mutate(changed);
    assert.throws(() => validateFollowup(changed));
  }
  console.log(`Competitiveness follow-up: ${mutations.length} scope mutations rejected.`);
}

const digest = (raw) => createHash("sha256").update(raw).digest("hex");
async function readPinned(path, hash) {
  assert((await stat(path)).size <= 8 * 1024 * 1024, `oversized evidence: ${path}`);
  const raw = await readFile(path);
  assert.equal(digest(raw), hash, `evidence hash drift: ${path}`);
  return raw;
}
async function jsonPinned(path, hash) {
  return JSON.parse(await readPinned(path, hash));
}

// Caption checks bind frozen root-validated records; they do not replace their
// native/token validators or manufacture qualification from a JSON receipt.
async function validateEvidence(root, value) {
  for (const [prefix, candidate, metric] of [
    ["wave", "candidate", value.wave],
    ["wait", "adaptive", value.polling.adaptive],
    ["wait-capped", "candidate", value.polling.capped],
  ]) {
    const names = ["control-r1", `${candidate}-r1`, `${candidate}-r2`, "control-r2"];
    const runs = [];
    for (const [index, name] of names.entries()) {
      const dir = join(root, `${prefix}-${name}`);
      const report = await jsonPinned(join(dir, "comparison.json"), metric.reportSha256[index]);
      for (const key of ["passed", "all_reference_tokens_and_bytes_match", "clean_teardown_recorded", "gpu_idle_before_and_after"]) assert.equal(report[key], true);
      assert.equal(report.qualification, false);
      assert.equal(report.generated_tokens, value.fixedOutputTokens);
      assert.equal(report.tensor_parallel, 1);
      const raw = await readPinned(join(dir, "results.jsonl"), report.input_sha256["results.jsonl"]);
      const events = raw.toString("utf8").trimEnd().split("\n").map((line) => JSON.parse(line));
      const setup = events[0];
      const external = report.expected_candidate;
      for (const key of ["controller_sha256", "worker_sha256", "artifact_hsaco_id", "artifact_manifest_id", "artifact_handoff_id", "fp32_head_artifact"]) assert.deepEqual(setup[key], external[key]);
      assert.deepEqual(setup.running_worker_sha256, [setup.worker_sha256]);
      assert.equal(setup.head_precision, "fp32-v8");
      assert.equal(setup.fp32_head_workspace_bytes, 19447808);
      assert.equal(setup.kernel_profile, "v5-mfma32");
      assert.equal(setup.batch_tokens, 16);
      assert.equal(setup.prefill_chunk, 16);
      assert.equal(setup.collective, "device-tp1-v3");
      assert.equal(setup.output_head_pruning, true);
      assert.equal(setup.prefix_cache, true);
      assert.deepEqual(setup.performance_profile, {
        attention: prefix === "wave" && (index === 1 || index === 2) ? "wave" : "baseline",
        dispatch_sequences: false, projection: "mfma", queue_rollover: false,
        runtime_cache_admission: true, runtime_operational: true, runtime_profiling: false,
      });
      const requests = events.filter((event) => event.schema === "FerricQwen3TpBatchRequestV2");
      assert.equal(requests.length, value.fixedRequests);
      for (const request of requests) {
        assert.deepEqual(request.generated_tokens, report.requests[request.name].generated_tokens);
        assert.equal(request.generated_text, report.requests[request.name].generated_text);
      }
      const starts = requests.map((request) => request.arrival_ns);
      const ends = requests.flatMap((request) => [...request.output_timestamps_ns,
        ...(request.cancelled_ns === null ? [] : [request.cancelled_ns])]);
      assert([...starts, ...ends].every(Number.isSafeInteger));
      const rate = value.fixedOutputTokens * 1e9 / (Math.max(...ends) - Math.min(...starts));
      assert(Math.abs(rate - report.metrics.output_tokens_per_second) < 1e-12);
      for (const file of ["gpu-before.json", "gpu-after.json", "status"]) await readPinned(join(dir, file), report.input_sha256[file]);
      assert.equal((await readFile(join(dir, "status"))).toString("utf8"), "0\n");
      assert.equal(events.at(-1).all_workers_exited, true);
      assert.deepEqual(events.at(-1).rank_dispatch_counts, [3077]);
      runs.push({ rate, ttft: report.requests["reuse-prefix"].ttft_ns,
        tpot: report.requests["reuse-prefix"].tpot_ns, setup });
    }
    const mean = (indices, key) => indices.reduce((sum, index) => sum + runs[index][key], 0) / indices.length;
    const change = (key) => Number(((mean([1, 2], key) / mean([0, 3], key) - 1) * 100).toFixed(2));
    assert.equal(change("rate"), metric.rateChangePercent);
    if (prefix === "wave") {
      assert.equal(change("ttft"), metric.reuseTtftChangePercent);
      assert.equal(change("tpot"), metric.reuseTpotChangePercent);
    }
    for (const key of ["controller_sha256", "artifact_hsaco_id", "artifact_manifest_id", "artifact_handoff_id", "fp32_head_artifact"]) {
      for (const run of runs) assert.deepEqual(run.setup[key], runs[0].setup[key]);
    }
    assert.equal(runs[0].setup.worker_sha256, runs[3].setup.worker_sha256);
    assert.equal(runs[1].setup.worker_sha256, runs[2].setup.worker_sha256);
    if (prefix === "wave") assert.equal(runs[0].setup.worker_sha256, runs[1].setup.worker_sha256);
    else assert.notEqual(runs[0].setup.worker_sha256, runs[1].setup.worker_sha256);
  }

  const large = value.largeKv;
  const host = await jsonPinned(join(root, "large-kv-host-aggregate.json"), large.hostAggregateSha256);
  assert.equal(host.source_commit, large.source);
  assert.equal(host.gpu_execution, false);
  assert.equal(host.required_batch_gate.status, 0);
  assert.equal(["library_passed", "batch_cli_passed", "tp_cli_passed", "replica_control_passed", "source_policy_passed"].reduce((sum, key) => sum + host.required_batch_gate[key], 0), 315);
  assert.equal(host.explicit_image_tests.v9_passed + host.explicit_image_tests.reused_public511_v7_passed, 4);
  const native = await jsonPinned(join(root, "large-kv-native-v9/result.json"), large.nativeReportSha256);
  assert.equal(native.schema, "FerricTpLargeKvProbeV9");
  assert.equal(native.artifact_sha256, host.explicit_image_tests.v9_image_hsaco);
  for (const key of ["checks_pass", "clean_teardown"]) assert.equal(native[key], true);
  for (const key of ["benchmark", "model_inference", "model_parity_qualified"]) assert.equal(native[key], false);
  assert.equal(native.results.length, large.nativeCases);
  assert(native.results.some((result) => result.name === "attention_rows1_pages16384_context8192"));
  for (const result of native.results) {
    assert.equal(result.full_output_exact, true);
    for (const check of result.checks) assert.equal(check.guards_unchanged, true);
  }
  const review = await jsonPinned(join(root, "large-kv-native-v9/root-review.json"), large.nativeReviewSha256);
  for (const key of ["checks_pass", "worker_absent", "all_eight_gpus_idle"]) assert.equal(review[key], true);
  assert.equal(review.result_sha256, large.nativeReportSha256);
  assert.equal(review.qualification, false);

  const model = large.modelCanary;
  for (const [index, mode] of ["legacy", "v9"].entries()) {
    const dir = join(root, `tp1-large-kv-${mode}-r1`);
    const report = await jsonPinned(join(dir, "comparison.json"), model.reportSha256[index]);
    for (const key of ["passed", "all_reference_tokens_and_bytes_match", "clean_teardown_recorded", "gpu_idle_before_and_after"]) assert.equal(report[key], true);
    assert.equal(report.qualification, false);
    assert.equal(report.generated_tokens, model.outputTokensPerRun);
    assert.equal(report.batch_count, model.batchesPerRun);
    assert.equal(report.physical_token_rows, model.physicalRowsPerRun);
    assert.equal(report.maximum_batch_rows_observed, model.maximumRowsObserved);
    assert.deepEqual(report.rank_dispatch_counts, [model.dispatchesPerRun]);
    const raw = await readPinned(join(dir, "results.jsonl"), report.input_sha256["results.jsonl"]);
    const events = raw.toString("utf8").trimEnd().split("\n").map((line) => JSON.parse(line));
    const setup = events[0];
    assert.equal(setup.controller_sha256, "489e1751376db55c17a4df2a2d2b21abb850cee4904c1690f194a375538cf8ff");
    assert.equal(setup.worker_sha256, "933d73d25b0db7001cf9522215436b0c97157dd0d2e3ad4e034e7f1986b4b322");
    assert.deepEqual(setup.running_worker_sha256, [setup.worker_sha256]);
    assert.equal(setup.physical_pages, model.physicalPages[index]);
    assert.equal(setup.tensor_parallel, 1);
    assert.equal(setup.batch_tokens, 16);
    assert.equal(setup.prefill_chunk, 16);
    assert.equal(setup.head_precision, "fp32-v8");
    assert.equal(setup.collective, "device-tp1-v3");
    assert.equal(setup.output_head_pruning, true);
    assert.equal(setup.performance_profile.attention, "baseline");
    assert.equal(setup.performance_profile.projection, "mfma");
    if (mode === "v9") {
      assert.equal(setup.kv_pool_profile, large.profile);
      assert.equal(setup.kv_pool_max_physical_pages, large.maximumPhysicalPages);
      assert.equal(setup.kv_pool_payload_bytes, large.maximumKvPayloadBytes);
      assert.equal(setup.kv_pool_artifact.artifact_hsaco_id, native.artifact_sha256);
    } else assert.equal(Object.hasOwn(setup, "kv_pool_profile"), false);
    for (const file of ["gpu-before.json", "gpu-after.json", "status"]) await readPinned(join(dir, file), report.input_sha256[file]);
    const requests = events.filter((event) => event.schema === "FerricQwen3TpBatchRequestV2");
    assert.equal(requests.length, 4);
    for (const request of requests) {
      assert.deepEqual(request.generated_tokens, report.requests[request.name].generated_tokens);
      assert.equal(request.generated_text, report.requests[request.name].generated_text);
    }
    assert.equal(events.at(-1).all_workers_exited, true);
    assert.deepEqual(events.at(-1).rank_dispatch_counts, [model.dispatchesPerRun]);
  }

  const ordered = value.orderedBatches;
  const orderedReview = await jsonPinned(join(root, "ordered-batch-native-root-review.json"), ordered.nativeReviewSha256);
  assert.equal(orderedReview.checks_pass, true);
  assert.equal(orderedReview.model_qualified, false);
  assert.equal(orderedReview.performance_qualified, false);
  for (const [index, mode] of ordered.nativeModes.entries()) {
    const report = await jsonPinned(join(root, `ordered-batch-native-${mode}/result.json`), ordered.nativeReportSha256[index]);
    assert.equal(report.schema, "FerricOrderedBatchNativeProbeV1");
    assert.equal(report.runtime_operational, mode === "operational");
    for (const key of ["checks_pass", "clean_teardown"]) assert.equal(report[key], true);
    for (const key of ["benchmark", "model_qualified"]) assert.equal(report[key], false);
    assert.equal(report.total_packets, ordered.packetsPerMode);
    assert.equal(report.queue_rollovers, ordered.actualRolloversPerMode);
    const chains = report.results.flatMap((result) => result.runs.filter((run) => run.mode !== "rollover"));
    assert.equal(chains.length, ordered.chainsPerMode);
    for (const chain of chains) {
      assert.equal(chain.exact_dependency_chain, true);
      for (const check of chain.checks) assert.equal(check.guards_unchanged, true);
    }
    const receipt = orderedReview.results[index];
    assert.equal(receipt.mode, mode);
    assert.equal(receipt.result_sha256, ordered.nativeReportSha256[index]);
    assert.equal(receipt.worker_absent, true);
    assert.equal(receipt.all_eight_gpus_idle, true);
  }
  const draft = (await readPinned(join(root, "draft-intake-tests.log"), value.draftIntake.testsSha256)).toString("utf8");
  for (const count of [212, 55, 16, 22]) assert(draft.includes(`${count} passed; 0 failed`));
  console.log("PASS: twelve paired canary reports, nine large-KV fixtures plus two bounded allocation model reports, two ordered native modes and scoped host receipts; no serving promotion.");
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  assert.equal(process.argv.length, 3, "usage: node validate-competitiveness-followup.mjs COMPETE_EVIDENCE");
  const context = { window: {} };
  vm.runInNewContext(await readFile(join(dirname(fileURLToPath(import.meta.url)), "data/project.js"), "utf8"), context);
  const value = context.window.FERRIC_PROJECT.competitivenessFollowup;
  validateFollowup(value);
  await validateEvidence(process.argv[2], plain(value));
}
