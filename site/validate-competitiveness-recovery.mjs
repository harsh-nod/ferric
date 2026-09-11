import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile, stat } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const expected = {
  source: "4f215bc92588fc521a1b7425b64e3cc246822e21",
  core: "21682228486f7186cc3c37ddf165fffc438d8b6a",
  currentReadinessCount: 14,
  fixedRequests: 4, fixedOutputTokens: 8, repetitionsPerMode: 2,
  ordered: {
    initialRegression: { rateChangePercent: -35.40, reuseTtftChangePercent: 52.38, reuseTpotChangePercent: 58.56 },
    oldOrderedRecovery: { rateChangePercent: 85.60, reuseTtftChangePercent: -48.21, reuseTpotChangePercent: -51.56 },
    freshSerialPair: { rateChangePercent: 22.19, reuseTtftChangePercent: -21.81, reuseTpotChangePercent: -24.62 },
    newDefault: false, confidenceIntervalClaimed: false,
    nativePacketsPerMode: 184, nativeChainsPerMode: 26,
    legacySerialSequenceUnchanged: true,
  },
  draft: {
    model: "Qwen3-0.6B", referencePasses: 2, forwardsPerPass: 6,
    generatedTokens: [12095, 13], generatedText: " Paris.",
    packets: 2544, standaloneReferencePassed: true,
    controllerSource: "346d588b8c189e7365f2c82d683efdd497e206e3",
    rawReferenceSha256: "8d09033c639e7f562d8ca772b0b72baebeb156ea34ddbd536297f83af11f3c08",
    rawFerricSha256: "bf9da61e5c2a4fbec0a3779aac1f40a3de8f13833b245ba0e9bb27860e56e770",
    wrapperSha256: "f1f68dc66b4198ab4b5bf00c97d23a0113640b8dd8fc60797635e444f313232f",
    fastKernelRoots: 14, fastKernelNativeCases: 36, fastKernelNativePassed: true,
    fastImageSha256: "3a308c9cc8f1509d45e14620674200e94684ee7930263c4fd7d808ed082559e1",
    fastNativeReportSha256: "573256efddc1686a861c50dc9eedc23894a4b6f44571a57a5f5d9307c3a5c7d0",
    fastNativeWrapperSha256: "d4faeaf840c2e83d56641510026307ccad21a8bc6325b14c088d6a5a54c2b49f",
    pagedModelQualified: false, speculativeServingImplemented: false,
    consumedInputDriverSource: "58ed5c56227068130c7a4c84743bde56e125b8ba",
    consumedInputHostSha256: "12590c09eb3d359b99b1fded99ccb700a766b20c2f7d924777822cd1d3cf6536",
    consumedInputHostInvocations: 526, consumedInputHostIgnores: 19,
    consumedInputDoctests: 6, consumedInputImageTests: 1,
    consumedInputCompletionSealImplemented: true, automaticProposalsImplemented: false,
  },
  aggregate: {
    source: "656edb2bd043b191541b0ad48b74dc827e3255ee",
    sha256: "c0819d39d6a9a882bac2f6da3b6050fac8231a60ab915fd0032a31915da46f4b",
    steps: 28, adapterTestInvocations: 516, explicitIgnores: 18,
    doctests: 5, sourceGateTests: 38, verifierPolicyTests: 31,
    metadataConfigurations: 29, unchangedInventories: 5,
    gpuExecution: false, newVerusProof: false,
  },
  continuousV3: {
    collectorTests: 90, pairedSeriesTests: 109, newPairedMethods: 19,
    collectorTestsSha256: "bf37d890b65359f64f0107db4428831f2906e1482c2c84055b357b0c98991186",
    pairedTestsSha256: "c5ae878807a0c54797818f40cf44372c872dd5b1f139635e2d173824310a82fc",
    drainBetweenWindows: false, failedWindowsRetained: true,
    minimumFreshPairsForDescriptiveIntervals: 3, gpuMeasurementQualified: false,
  },
  sequentialBaselineLaunchesApproved: true,
  target128Reference: {
    independent: true, promptTokens: 128, outputTokens: 128, repetitions: 2,
    decoder: "BF16 eager", head: "explicit FP32 operands and output",
    rawSha256: "fa725662e8ec9e7ab4f30c981c048167a5b63a4fda972d4fe570527b80fcf314",
    adaptedSha256: "cd7f512537e44d677244c606cba919e57747e8c26caf3c0df4a0aca7acc0eb8b",
    wrapperSha256: "b4f7f97fe6ed3141dbfc66d529633b42cdde67e0e997bfcf64b97716f11cf365",
    ferricMatched: false, baselineServingRun: false, performanceQualified: false,
  },
  matchedExternalComparisonAvailable: false, sustainedServingQualified: false,
  competitiveWinClaimed: false, m1Complete: false, authority: "none",
};
const plain = (value) => JSON.parse(JSON.stringify(value));
export function validateRecovery(value) {
  assert.deepEqual(plain(value), expected, "recovery checkpoint drifted");
}
export function testRecoveryRejections(value) {
  const paths = [
    ["source"], ["core"], ["fixedRequests"], ["fixedOutputTokens"], ["repetitionsPerMode"],
    ["ordered", "initialRegression", "rateChangePercent"],
    ["ordered", "oldOrderedRecovery", "rateChangePercent"],
    ["ordered", "freshSerialPair", "rateChangePercent"],
    ["ordered", "freshSerialPair", "reuseTtftChangePercent"],
    ["ordered", "freshSerialPair", "reuseTpotChangePercent"],
    ["ordered", "newDefault"], ["ordered", "confidenceIntervalClaimed"],
    ["draft", "controllerSource"], ["draft", "rawFerricSha256"],
    ["draft", "fastNativeReportSha256"], ["draft", "fastKernelNativeCases"],
    ["draft", "pagedModelQualified"], ["draft", "speculativeServingImplemented"],
    ["draft", "consumedInputHostInvocations"], ["draft", "consumedInputCompletionSealImplemented"],
    ["draft", "automaticProposalsImplemented"], ["target128Reference", "rawSha256"],
    ["target128Reference", "head"], ["target128Reference", "ferricMatched"],
    ["target128Reference", "baselineServingRun"], ["target128Reference", "performanceQualified"],
    ["aggregate", "source"], ["aggregate", "adapterTestInvocations"],
    ["aggregate", "explicitIgnores"], ["aggregate", "gpuExecution"], ["aggregate", "newVerusProof"],
    ["continuousV3", "drainBetweenWindows"], ["continuousV3", "failedWindowsRetained"],
    ["continuousV3", "minimumFreshPairsForDescriptiveIntervals"],
    ["continuousV3", "gpuMeasurementQualified"], ["sequentialBaselineLaunchesApproved"],
    ["matchedExternalComparisonAvailable"], ["sustainedServingQualified"],
    ["competitiveWinClaimed"], ["m1Complete"], ["authority"],
  ];
  for (const path of paths) {
    const altered = plain(value);
    const parent = path.slice(0, -1).reduce((object, key) => object[key], altered);
    const key = path.at(-1);
    parent[key] = typeof parent[key] === "boolean" ? !parent[key] : `${parent[key]}-changed`;
    assert.throws(() => validateRecovery(altered));
  }
  console.log(`Recovery checkpoint: ${paths.length} scope mutations rejected.`);
}

const cohorts = [
  ["ordered-model", "initialRegression", "control", "candidate", [
    "6dde963c95353645a7bb26d7e6e3a95eeeb7ee980d81194a36db2c7251b1fee5",
    "1764597215684728f3631095fca40e987d36068dff32ca371e5ac8f7b983497e",
    "b72e31e91355ea61049c0d4a07129e98acb3a6e4e7caa0b3e138012870f4f5d7",
    "218a922d521ebbc0f0e753496a96188541b099a94efb3c47b863eec81cd963d4",
  ]],
  ["ordered-boundary-model", "oldOrderedRecovery", "control", "candidate", [
    "0cfa828dd8129a67740742ab9286dfc0a6a7dfbf5008ceea48b5748b8895b0a0",
    "fe6cae3878a2697e43800f47e5aa99213d6faadd947906dc7e348811b9c82ad3",
    "70ed858a4d99acfadd2cf1f0a7ec437ef600514776b21cbf4cd1651209d39755",
    "b34ecc6128671e1200c075b8bcd039af569c928d040ef4531461333a3e417af8",
  ]],
  ["ordered-final", "freshSerialPair", "serial", "ordered", [
    "e223718339d1f6d7d13c3d2b93705ae9823c1796cbf93c56e2260929255c3bfa",
    "7498aadaf850d2853ebac6478047c79e260a7c10529ce5af31794e56916e12f2",
    "fcfa908244c61c48afa14120570e16cca2c4d91feb39f8f605a46651b339fc4c",
    "76b5bebc626f94e4f02030a72a3826aa31f090788ecc0404097b1337bba4464e",
  ]],
];
const newWorker = "b91ddef78135829f607d1b83f5cf898d745b36013327f0d2d12771aba1b5150b";
const oldWorker = "761027c596b896a58da822cf919adf4d480e9b4b71d8c79e93265e3d4002c5b6";
async function pinned(path, hash) {
  assert((await stat(path)).size <= 8 * 1024 * 1024);
  const raw = await readFile(path);
  assert.equal(createHash("sha256").update(raw).digest("hex"), hash, `evidence hash drift: ${path}`);
  return raw;
}
const json = async (path, hash) => JSON.parse(await pinned(path, hash));
const lines = (raw) => raw.toString("utf8").trimEnd().split("\n").map((line) => JSON.parse(line));
function checkWrapper(receipt) {
  for (const key of ["all_eight_idle_before", "all_eight_idle_after", "passed"]) assert.equal(receipt[key], true);
  assert.equal(receipt.performance_qualified, false);
  assert.deepEqual(receipt.errors, []);
}

// This binds captions to root-reviewed immutable evidence. It is not a new
// token/native qualifier and never promotes a diagnostic timing to performance.
async function validateEvidence(root, value) {
  for (const [prefix, metricKey, control, candidate, hashes] of cohorts) {
    const runs = [];
    for (const [index, name] of [`${control}-r1`, `${candidate}-r1`, `${candidate}-r2`, `${control}-r2`].entries()) {
      const folder = join(root, `${prefix}-${name}`);
      const report = await json(join(folder, "comparison.json"), hashes[index]);
      for (const key of ["passed", "all_reference_tokens_and_bytes_match", "clean_teardown_recorded", "gpu_idle_before_and_after"]) assert.equal(report[key], true);
      assert.equal(report.qualification, false);
      assert.equal(report.generated_tokens, 8);
      assert.equal(report.tensor_parallel, 1);
      assert.equal(report.physical_token_rows, 34);
      assert.equal(report.batch_count, 5);
      assert.deepEqual(report.rank_dispatch_counts, [3077]);
      const events = lines(await pinned(join(folder, "results.jsonl"), report.input_sha256["results.jsonl"]));
      for (const file of ["gpu-before.json", "gpu-after.json", "status"]) await pinned(join(folder, file), report.input_sha256[file]);
      assert.equal((await readFile(join(folder, "status"))).toString(), "0\n");
      const setup = events[0];
      const selected = index === 1 || index === 2;
      const ordered = metricKey === "oldOrderedRecovery" || selected;
      if (ordered) assert.equal(setup.runtime_ordered_batches, true);
      else assert.equal(Object.hasOwn(setup, "runtime_ordered_batches"), false);
      assert.equal(setup.worker_sha256, metricKey === "initialRegression" || (metricKey === "oldOrderedRecovery" && !selected) ? oldWorker : newWorker);
      assert.deepEqual(setup.running_worker_sha256, [setup.worker_sha256]);
      assert.equal(setup.controller_sha256, "108feeb846d452f3e0b5a3e3c2c8b9c5bde7bb597d7a3b693bc51a06debfd1a3");
      assert.equal(setup.artifact_hsaco_id, "98b5fdb14ac7242e324e1801e1885c98e77473d622b2e9b3f9f12f14c7d75502");
      assert.equal(setup.fp32_head_artifact.artifact_hsaco_id, "5f19b3ba59035a5f0ebc90cdf3a40466f9910908a45e082d146cb674028da6cb");
      assert.equal(setup.head_precision, "fp32-v8");
      assert.equal(setup.fp32_head_workspace_bytes, 19447808);
      assert.equal(setup.kernel_profile, "v5-mfma32");
      assert.equal(setup.batch_tokens, 16);
      assert.equal(setup.prefill_chunk, 16);
      assert.equal(setup.collective, "device-tp1-v3");
      assert.equal(setup.output_head_pruning, true);
      assert.equal(setup.prefix_cache, true);
      assert.deepEqual(setup.performance_profile, { attention: "baseline", dispatch_sequences: false,
        projection: "mfma", queue_rollover: false, runtime_cache_admission: true,
        runtime_operational: true, runtime_profiling: false });
      const requests = events.filter((event) => event.schema === "FerricQwen3TpBatchRequestV2");
      assert.equal(requests.length, 4);
      for (const request of requests) {
        assert.deepEqual(request.generated_tokens, report.requests[request.name].generated_tokens);
        assert.equal(request.generated_text, report.requests[request.name].generated_text);
        assert.deepEqual(request.generated_utf8_bytes, [...Buffer.from(request.generated_text)]);
      }
      const starts = requests.map((request) => request.arrival_ns);
      const ends = requests.flatMap((request) => [...request.output_timestamps_ns,
        ...(request.cancelled_ns === null ? [] : [request.cancelled_ns])]);
      assert([...starts, ...ends].every(Number.isSafeInteger));
      const rate = 8e9 / (Math.max(...ends) - Math.min(...starts));
      assert(Math.abs(rate - report.metrics.output_tokens_per_second) < 1e-12);
      assert.equal(events.at(-1).all_workers_exited, true);
      assert.deepEqual(events.at(-1).rank_dispatch_counts, [3077]);
      const reuse = requests.find((request) => request.name === "reuse-prefix");
      runs.push({ rate, ttft: reuse.ttft_ns, tpot: reuse.tpot_ns, setup });
    }
    const mean = (indices, key) => indices.reduce((sum, index) => sum + runs[index][key], 0) / 2;
    const change = (key) => Number(((mean([1, 2], key) / mean([0, 3], key) - 1) * 100).toFixed(2));
    const metric = value.ordered[metricKey];
    assert.equal(change("rate"), metric.rateChangePercent);
    assert.equal(change("ttft"), metric.reuseTtftChangePercent);
    assert.equal(change("tpot"), metric.reuseTpotChangePercent);
    for (const key of ["controller_sha256", "artifact_hsaco_id", "artifact_manifest_id", "artifact_handoff_id", "fp32_head_artifact", "performance_profile"]) {
      for (const run of runs) assert.deepEqual(run.setup[key], runs[0].setup[key]);
    }
    console.log(`${metricKey}: rate ${mean([0, 3], "rate").toFixed(6)} -> ${mean([1, 2], "rate").toFixed(6)}, ${change("rate")}% / TTFT ${change("ttft")}% / TPOT ${change("tpot")}%`);
  }
  const draft = value.draft;
  const reference = await json(join(root, "draft-torch-raw.json"), draft.rawReferenceSha256);
  const trace = lines(await pinned(join(root, "draft-results.jsonl"), draft.rawFerricSha256));
  assert.equal(reference.authority, "independent-draft-reference-only");
  assert.equal(reference.passes.length, 2);
  const choices = [15846, 315, 279, 374, 12095, 13];
  for (const pass of reference.passes) {
    assert.deepEqual(pass.generated_tokens, draft.generatedTokens);
    assert.deepEqual(pass.steps.map((step) => step.choice), choices);
    assert(pass.steps.every((step) => step.finite_count === 151936 && step.logits_dtype === "torch.bfloat16"));
  }
  assert.equal(trace.length, 3);
  assert.equal(trace[0].controller_sha256, "e84b104e18054e4f808a0dab962658ff4ab4737a3f0e1bc725633839a6950da0");
  assert.equal(trace[0].worker_sha256, newWorker);
  assert.equal(trace[0].artifact_hsaco_id, "7c0b1934a27569a97cf535c96a8a56dd57babb63a6d1becad1c7be3a3d26edec");
  assert.equal(trace[0].model_role, "Draft06B");
  assert.equal(trace[0].speculative_execution, false);
  assert.deepEqual(trace[1].steps.map((step) => step.next_token), choices);
  assert.deepEqual(trace[1].generated_tokens, draft.generatedTokens);
  assert.equal(trace[1].generated_text, draft.generatedText);
  assert.deepEqual(trace[1].generated_utf8_bytes, [...Buffer.from(draft.generatedText)]);
  assert.equal(trace[2].all_workers_exited, true);
  assert.equal(trace[2].reference_passed, true);
  assert.deepEqual(trace[2].rank_dispatch_counts, [draft.packets]);
  checkWrapper(await json(join(root, "draft-wrapper.json"), draft.wrapperSha256));
  const native = await json(join(root, "native-v10.json"), draft.fastNativeReportSha256);
  assert.equal(native.artifact_sha256, draft.fastImageSha256);
  assert.equal(native.checks_pass, true);
  assert.equal(native.clean_teardown, true);
  for (const field of ["benchmark", "model_inference", "model_parity_qualified"]) assert.equal(native[field], false);
  assert.equal(native.results.length, 36);
  assert.equal(new Set(native.results.map((result) => result.symbol)).size, 14);
  for (const result of native.results) {
    assert.equal(result.full_output_exact, true);
    assert(result.checks.every((check) => check.guards_unchanged === true));
  }
  assert(native.results.some((result) => result.name === "attention_logical8191_physical511"));
  checkWrapper(await json(join(root, "native-v10-wrapper.json"), draft.fastNativeWrapperSha256));
  const paged = await json(join(root, "draft-paged-host.json"), draft.consumedInputHostSha256);
  assert.equal(paged.ferric_commit, draft.consumedInputDriverSource);
  assert.equal(paged.core_commit, value.core);
  assert.equal(paged.adapter_tests.passed, draft.consumedInputHostInvocations);
  assert.equal(paged.adapter_tests.ignored, draft.consumedInputHostIgnores);
  assert.equal(paged.doctests.passed, draft.consumedInputDoctests);
  assert.equal(paged.image_admission.passed, draft.consumedInputImageTests);
  assert.equal(paged.image_hsaco_sha256, draft.fastImageSha256);
  for (const field of ["native_model_qualified", "performance_qualified", "new_verus_proof"]) assert.equal(paged[field], false);
  assert.deepEqual(paged.gate_steps, ["fmt", "metadata", "tests", "doctests", "image-admission", "clippy", "release", "source-unchanged"]);
  const target = value.target128Reference;
  const targetRaw = await json(join(root, "target128-torch-raw.json"), target.rawSha256);
  const targetReference = await json(join(root, "target128-reference.json"), target.adaptedSha256);
  const targetWrapper = await json(join(root, "target128-wrapper.json"), target.wrapperSha256);
  checkWrapper(targetWrapper);
  assert.equal(targetWrapper.container.absent, true);
  assert.equal(targetWrapper.container.forced_stop, false);
  assert.equal(targetWrapper.raw_sha256, target.rawSha256);
  assert.equal(targetRaw.authority, "independent-target-reference-only");
  assert.equal(targetRaw.model, "Qwen/Qwen3-8B");
  assert.equal(targetRaw.performance_qualified, false);
  assert.equal(targetRaw.policy.prompt_tokens, 128);
  assert.equal(targetRaw.policy.output_tokens, 128);
  assert.equal(targetRaw.policy.decoder_dtype, "bfloat16");
  assert.equal(targetRaw.policy.attention, "eager");
  assert.equal(targetRaw.policy.head_compute, "torch.nn.functional.linear-explicit-float32-operands-and-output");
  assert.equal(targetRaw.passes.length, 2);
  assert.equal(targetReference.generated_token_ids.length, 128);
  assert.equal(targetReference.prompt_token_ids.length, 128);
  assert.equal(targetReference.producer_evidence_sha256, target.rawSha256);
  for (const pass of targetRaw.passes) {
    assert.deepEqual(pass.generated_token_ids, targetReference.generated_token_ids);
    assert.equal(pass.steps.length, 128);
  }
  assert.equal(targetRaw.repeated_output_exact, true);
  const aggregate = await json(join(root, "aggregate.json"), value.aggregate.sha256);
  assert.equal(aggregate.source_commit, value.aggregate.source);
  assert.equal(aggregate.fe2o3.commit, value.core);
  assert.equal(aggregate.adapter_tests.passed, 516);
  assert.equal(aggregate.adapter_tests.ignored, 18);
  assert.equal(aggregate.adapter_tests.failed, 0);
  assert.equal(aggregate.adapter_doctests.passed, 5);
  assert.equal(aggregate.source_gate_tests.passed, 38);
  assert.equal(aggregate.verifier_source_policy_tests.passed, 31);
  assert.equal(aggregate.metadata_configurations, 29);
  assert.equal(aggregate.inventory_comparisons, 5);
  assert.equal(aggregate.checks.length, 28);
  assert(aggregate.checks.every((check) => check.status === 0));
  assert.equal(aggregate.gpu_execution, false);
  assert.equal(aggregate.new_verus_proof, false);
  assert.equal(aggregate.performance_qualified, false);
  for (const [file, count, hash] of [["collector-tests.log", 90, value.continuousV3.collectorTestsSha256],
    ["paired-tests.log", 109, value.continuousV3.pairedTestsSha256]]) {
    const log = (await pinned(join(root, file), hash)).toString();
    assert(log.includes(`Ran ${count} tests in `));
    assert(log.trimEnd().endsWith("OK"));
  }
  const approval = await json(join(root, "baseline-approval.json"), "ad503bec4a26871af284f4d2bd177e023c7076f8c57194a5a4d71a9cc7a288e2");
  assert.equal(approval.launch_approved, true);
  assert.equal(approval.performance_claim_approved_or_established, false);
  console.log("PASS: ordered cohorts, independent draft/target references, native36, consumed-input host gate, exact656 and V3 logs; no promotion.");
}
if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const site = dirname(fileURLToPath(import.meta.url));
  const context = { window: {} };
  vm.runInNewContext(await readFile(join(site, "data/project.js"), "utf8"), context);
  const value = plain(context.window.FERRIC_PROJECT.competitivenessRecovery);
  validateRecovery(value);
  testRecoveryRejections(value);
  if (process.argv[2]) await validateEvidence(process.argv[2], value);
}
