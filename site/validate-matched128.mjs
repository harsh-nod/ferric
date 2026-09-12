import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const expected = {
  schema: "FerricPagesMatched128V1", date: "2026-09-12",
  sourceSummarySha256: "f6fb4811915afce5d004320f3d6f1d455b7a3a9093b22858ac9c7fba925a254f",
  clientSha256: "979136caea4f134f33f19c62b82a8ac9537205eaa11d11a43b7a3af466af0a2d",
  referenceSha256: "cd7f512537e44d677244c606cba919e57747e8c26caf3c0df4a0aca7acc0eb8b",
  model: "Qwen3-8B", hardware: "1 x MI350X, physical GPU 0", tensorParallel: 1,
  contextTokens: 8192, promptTokens: 128, outputTokens: 128,
  concurrency: 1, freshStartsPerEngine: 1, warmups: 10, measuredRequests: 30,
  weightsAndDecoder: "BF16", head: "explicit FP32 output head",
  speculation: false, prefixCache: false, greedyFixedLength: true,
  exactDiagnosticIdsAndBytes: true, exactTimedBytesAndUsage: true,
  unforcedCleanup: true, allEightIdleAfter: true,
  steadyStateQualified: false, stockDefaultsComparison: false, confidenceIntervalClaimed: false,
  tokenItlAvailable: false, frameworkWinClaimed: false, m1Complete: false,
  metrics: [
    { engine: "Ferric", source: "656edb2", ttftMeanMs: 3705.9668576333333,
      ttftP50Ms: 3684.9233145, ttftP99Ms: 3963.11035453,
      tpotMeanMs: 506.96937418110235, tpotP50Ms: 505.0726207244095, tpotP99Ms: 516.9290390844882,
      outputTokensPerSecond: 1.8798052098123519, windowSeconds: 2042.764846036,
      receiptSha256: "6192619afddae282d66813c9c9f50aee98c0cac3bde14f0507c8e59759781a2e" },
    { engine: "vLLM 0.28.0", source: "cached ROCm image", ttftMeanMs: 19.242703333333335,
      ttftP50Ms: 19.485638, ttftP99Ms: 20.16712958,
      tpotMeanMs: 4.413837543044619, tpotP50Ms: 4.413801976377952, tpotP99Ms: 4.420472721811024,
      outputTokensPerSecond: 220.55811110798334, windowSeconds: 17.410377613,
      receiptSha256: "f88d8a4d49aee99cbadf532fd0aa9c899a8beb227492e4043b52f53828f636b0" },
  ],
  sglang: { state: "startup-failed", metrics: null, numericalResult: false },
};
const draftExpected = {
  independentPassed: true, schedules: ["full", "tokenwise"], repetitionsPerSchedule: 2,
  fullLogitVectors: 16, logitsPerVector: 151936, logitPayloadBytes: 9723904,
  rawSha256: "79e63ac536c48a0773d45c6db6d9ad47289ef301b3e0a3d70e7f97ce4f5f7981",
  wrapperReceiptSha256: "faa149d55c45f33a743f98e7ceef1816a79c0879ef1b02aeecd2aaface18e921",
  fullReferenceSha256: "cd24d0b24c74095af08c7d99621b97496f6913219fa22bd6ddcbcfced078a1c4",
  tokenwiseReferenceSha256: "b026f5382ca26bca3f0e47a15490e17ebba9542b557273ce0f66fee5ea082d44",
  producerSha256: "a264124d132833a25d8e4012544d0e793c4a86d08c854776b88fbb2d172d3fd6",
  legacyHelperSha256: "1d429cc87a8bc46018da9f8d84a7a4328dad05c43de8867ed791226c913f2959",
  nativeMatrixQualified: false, speculativeServingQualified: false,
  baselineFullNativePassed: true, completedNativeCases: 1, plannedNativeCases: 8,
  baselineFullNativeReceiptSha256: "cc6ad61c0199c7c261703066ae23df6f6c589c4bdf909f83c01d6ccb05a785b0",
  baselineFullNativeTraceSha256: "b6cd219d61d36f448c00260af797ff99327b5510da68b4597b8d6494e7d3d8be",
  privateProposalHostGatePassed: true, privateProposalSource: "4cd0b7e03f894c48ccb810028ed1e475c59b0e9a",
  privateProposalReceiptSha256: "6182379c96d7d442f9c023cea4501551c2a20999b6a3b3458b6cc1f35d917bd3",
  privateProposalFocusedTests: 14, privateProposalHostInvocations: 566,
  privateProposalImageIgnores: 22, privateProposalDoctests: 7,
  combinedIntegrationGatePassed: true,
  combinedIntegrationSource: "82703c6a788e99e23ef8d4204ea5c976413ae6e1",
  combinedIntegrationReceiptSha256: "81a847daa89ab1b7829b3229a197e6ace567ea84585f54be2d9dc2248a9d0863",
  combinedIntegrationHostInvocations: 568, combinedIntegrationImageSkips: 22,
  combinedIntegrationDoctests: 7, combinedIntegrationPythonTests: 31,
  combinedIntegrationSourceFiles: 1040, newVerusProof: false,
};
const plain = (value) => JSON.parse(JSON.stringify(value));
const textFields = ["scope", "interpretation", "measurement", "correctness", "sglangNote"];

export function validateMatched128(value, draft) {
  assert.deepEqual(Object.keys(value).sort(), [...Object.keys(expected), ...textFields].sort());
  const numeric = plain(value);
  for (const key of textFields) {
    assert.equal(typeof numeric[key], "string");
    assert(numeric[key].length > 40);
    delete numeric[key];
  }
  assert.deepEqual(numeric, expected, "matched cell data or qualification scope drifted");
  assert.deepEqual(plain(draft), draftExpected, "independent draft reference scope drifted");
  for (const text of ["concurrency 1", "128 input and 128 output", "10 warmups", "30 measured", "FP32"])
    assert(value.scope.includes(text), `matched scope missing ${text}`);
  assert(value.interpretation.includes("substantially slower"));
  assert(value.interpretation.includes("single-start"));
  assert(value.interpretation.includes("stock-default"));
  assert(value.measurement.includes("127 post-first tokens"));
  assert(value.measurement.includes("including inter-request gaps and final drain"));
  assert(value.correctness.includes("Ferric's MFMA head and vLLM use BF16 operands with FP32 accumulation/output"));
  assert(value.correctness.includes("independent reference explicitly converts head operands to FP32"));
  assert(value.correctness.includes("frozen before engine outputs"));
  assert(value.sglangNote.includes("no measured result"));
  assert(value.sglangNote.includes("not assigned zero throughput"));
}

export function testMatched128Rejections(value, draft) {
  let mutations = 0;
  const visit = (object, path = []) => {
    for (const [key, child] of Object.entries(object)) {
      const next = [...path, key];
      if (child !== null && typeof child === "object") visit(child, next);
      else {
        const altered = plain(value);
        const parent = next.slice(0, -1).reduce((node, field) => node[field], altered);
        parent[key] = typeof child === "boolean" ? !child : `${child}-changed`;
        assert.throws(() => validateMatched128(altered, draft));
        mutations += 1;
      }
    }
  };
  visit(expected);
  for (const key of Object.keys(draftExpected)) {
    const changed = { ...plain(draft), [key]: null };
    assert.throws(() => validateMatched128(value, changed));
    mutations += 1;
  }
  for (const phrase of ["Ferric's MFMA head and vLLM use BF16 operands with FP32 accumulation/output",
    "independent reference explicitly converts head operands to FP32", "frozen before engine outputs"]) {
    const changed = { ...plain(value), correctness: value.correctness.replace(phrase, "changed") };
    assert.throws(() => validateMatched128(changed, draft));
    mutations += 1;
  }
  assert.throws(() => validateMatched128({ ...value, qualification: true }, draft));
  console.log(`Matched128 and draft checkpoint: ${mutations + 1} mutations rejected.`);
}

const sha = (raw) => createHash("sha256").update(raw).digest("hex");
async function pinnedBytes(root, file, hash) {
  const raw = await readFile(join(root, file));
  assert(raw.length < 2 * 1024 * 1024);
  assert.equal(sha(raw), hash, `${file} source evidence changed`);
  return raw;
}
async function pinned(root, file, hash) {
  return JSON.parse(await pinnedBytes(root, file, hash));
}

export async function validateMatchedEvidence(root, value, draft) {
  const report = await pinned(root, "matched-summary.json", value.sourceSummarySha256);
  assert.equal(report.compatible_timed_scope, true);
  assert.equal(report.qualification, false);
  assert.equal(report.framework_win_claim, false);
  assert.equal(report.ratios_or_rankings, null);
  assert.equal(report.runs.length, 2);
  assert.equal(report.timed_scope.client_sha256, value.clientSha256);
  assert.equal(report.timed_scope.reference_sha256, value.referenceSha256);
  const settings = report.timed_scope.settings;
  for (const [key, expectedValue] of Object.entries({ context: 8192, prompt_tokens: 128,
    completion_tokens: 128, concurrency: 1, warmups: 10, samples: 30, speculation: false,
    prefix_cache: false, dtype: "bfloat16", head_dtype: "float32", temperature: 0, ignore_eos: true }))
    assert.equal(settings[key], expectedValue);
  for (const [index, run] of report.runs.entries()) {
    assert.equal(run.admitted, true);
    assert.equal(run.qualification, false);
    assert.deepEqual(run.validation_errors, []);
    assert.equal(run.metrics.requests, 30);
    assert.equal(run.metrics.failed_requests, 0);
    assert(run.replayed_numerical_diagnostics.every((item) => item.admitted && item.classification === "exact_match"));
    assert.equal(run.replayed_timing_admission.classification, "exact_utf8_and_usage");
    const row = value.metrics[index];
    assert.equal(row.receiptSha256, run.receipt_sha256);
    for (const [field, metric, key] of [["ttftMeanMs", "ttft_ms", "mean"], ["ttftP50Ms", "ttft_ms", "p50"],
      ["ttftP99Ms", "ttft_ms", "p99"], ["tpotMeanMs", "tpot_ms", "mean"],
      ["tpotP50Ms", "tpot_ms", "p50"], ["tpotP99Ms", "tpot_ms", "p99"]])
      assert.equal(row[field], run.metrics[metric][key]);
    assert.equal(row.outputTokensPerSecond, run.metrics.output_tokens_per_second);
    assert.equal(row.windowSeconds, run.metrics.window_seconds);
  }
  const raw = await pinned(root, "draft-raw.json", draft.rawSha256);
  const wrapper = await pinned(root, "draft-wrapper.json", draft.wrapperReceiptSha256);
  assert.equal(wrapper.passed, true);
  assert.deepEqual(wrapper.errors, []);
  assert.equal(wrapper.container.absent, true);
  assert.equal(wrapper.container.forced_stop, false);
  assert.equal(wrapper.all_eight_idle_after, true);
  assert.equal(raw.producer_sha256, draft.producerSha256);
  assert.equal(raw.legacy_helper_sha256, draft.legacyHelperSha256);
  assert.equal(raw.repeated_exact, true);
  assert.equal(raw.logit_payload_bytes, draft.logitPayloadBytes);
  let vectors = 0;
  for (const mode of draft.schedules) {
    const reference = await pinned(root, `draft-${mode}.json`, draft[`${mode}ReferenceSha256`]);
    assert.equal(reference.prefill, mode);
    assert.equal(raw.schedules[mode].length, 2);
    for (const pass of raw.schedules[mode]) {
      assert.deepEqual(pass.generated_tokens, reference.generated_tokens);
      assert.deepEqual(pass.generated_utf8_bytes, reference.generated_utf8_bytes);
      for (const step of pass.steps) {
        assert.equal(step.finite_count, 151936);
        assert.equal(step.logits_dtype, "torch.float32");
        assert.equal(step.logits.bytes, 607744);
        vectors += 1;
      }
    }
  }
  assert.equal(vectors, 16);
  const native = await pinned(root, "draft-native-baseline-full.json", draft.baselineFullNativeReceiptSha256);
  assert.equal(native.schema, "FerricDraftPagedCanaryWrapperV10");
  assert.equal(native.projection, "baseline");
  assert.equal(native.prefill, "full");
  assert.equal(native.repetition, 1);
  assert.equal(native.passed, true);
  assert.deepEqual(native.errors, []);
  assert.equal(native.authority, "none");
  assert.equal(native.performance_qualified, false);
  assert.equal(native.all_eight_idle_before, true);
  assert.equal(native.all_eight_idle_after, true);
  const trace = (await pinnedBytes(root, "draft-native-baseline-full.jsonl", draft.baselineFullNativeTraceSha256))
    .toString("utf8").trim().split("\n").map((line) => JSON.parse(line));
  assert.equal(trace.length, 3);
  const [setup, observed, closed] = trace;
  assert.equal(setup.schema, "FerricDraftPagedCanarySetupV10");
  assert.equal(setup.reference_sha256, draft.fullReferenceSha256);
  assert.equal(setup.expected_dispatches, 960);
  assert.equal(observed.schema, "FerricDraftPagedCanaryObservationV10");
  assert.equal(observed.reference_passed, true);
  assert.deepEqual(observed.rank_dispatch_counts, [960]);
  assert.equal(closed.schema, "FerricDraftPagedCanaryClosedV10");
  assert.equal(closed.all_workers_exited, true);
  assert.equal(closed.execution_completed, true);
  assert.equal(closed.reference_passed, true);
  assert.deepEqual(closed.rank_dispatch_counts, [960]);
  assert(trace.every((record) => record.authority === "none" && record.performance_qualified === false));
  const proposal = await pinned(root, "draft-private-proposal-host.json", draft.privateProposalReceiptSha256);
  assert.equal(proposal.schema, "FerricPrivateDraftProposalHostGateV1");
  assert.equal(proposal.source_commit, draft.privateProposalSource);
  assert.equal(proposal.authority, "none");
  assert.equal(proposal.performance_qualified, false);
  assert.equal(proposal.gpu_executed, false);
  assert.equal(proposal.focused_proposal_tests_passed, draft.privateProposalFocusedTests);
  assert.equal(proposal.all_target_test_invocations_passed, draft.privateProposalHostInvocations);
  assert.equal(proposal.all_target_ignored, draft.privateProposalImageIgnores);
  assert.equal(proposal.doctests_passed, draft.privateProposalDoctests);
  for (const field of ["format_check", "strict_all_target_release_clippy", "release_build", "source_unchanged"])
    assert.equal(proposal[field], true);
  const combined = await pinned(root, "draft-integrated-host.json", draft.combinedIntegrationReceiptSha256);
  assert.equal(combined.schema, "FerricIntegratedPrivateProposalsHostGateV1");
  assert.equal(combined.authority, "none");
  assert.equal(combined.gpu_executed, false);
  assert.equal(combined.performance_qualified, false);
  assert.equal(combined.source_commit, draft.combinedIntegrationSource);
  assert.equal(combined.core_commit, "21682228486f7186cc3c37ddf165fffc438d8b6a");
  assert.equal(combined.gate_steps_passed, 8);
  assert.equal(combined.all_target_test_invocations_passed, draft.combinedIntegrationHostInvocations);
  assert.equal(combined.all_target_ignored, draft.combinedIntegrationImageSkips);
  assert.equal(combined.doctests_passed, draft.combinedIntegrationDoctests);
  assert.equal(combined.python_reference_tests_passed, draft.combinedIntegrationPythonTests);
  assert.equal(combined.source_files, draft.combinedIntegrationSourceFiles);
  for (const field of ["format_check", "strict_all_target_release_clippy", "release_build", "source_unchanged"])
    assert.equal(combined[field], true);
  console.log("PASS: matched table exactly matches pinned admitted summary; draft captions bind raw/adapter/lifecycle, one native case and private host receipts. This is not a new native or performance qualifier.");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const context = { window: {} };
  vm.runInNewContext(await readFile(join(dirname(fileURLToPath(import.meta.url)), "data/project.js"), "utf8"), context);
  const project = context.window.FERRIC_PROJECT;
  validateMatched128(project.matched128, project.pagedDraftReference);
  testMatched128Rejections(project.matched128, project.pagedDraftReference);
  if (process.argv[2]) await validateMatchedEvidence(process.argv[2], project.matched128, project.pagedDraftReference);
}
