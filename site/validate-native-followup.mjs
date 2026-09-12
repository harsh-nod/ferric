import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const expected = {
  "schema": "FerricPagesNativeFollowupV1",
  "pairedHostCore": "21682228486f7186cc3c37ddf165fffc438d8b6a",
  "waveOrdered": {
    "source": "f65c4603f936b3cf5d009991bc19a9645ecdb604",
    "controllerSha256": "d1c006b89f918541820a3335ba0f931a7394d1f7fa1a4eca2de5e4653e824237",
    "order": [
      "baseline",
      "wave",
      "wave",
      "baseline"
    ],
    "requestsPerRun": 4,
    "outputsPerRun": 8,
    "repetitionsPerMode": 2,
    "hostTimingEnabled": true,
    "contextTokens": 64,
    "physicalPages": 8,
    "prefillChunk": 16,
    "prefixCache": true,
    "queueRollover": false,
    "meanRateGainPercent": 12.27,
    "newDefault": false,
    "servingQualified": false,
    "runs": [
      {
        "run": "control-r1",
        "outputTokensPerSecond": 5.495400995763088,
        "comparisonSha256": "f6a43299f8061e4e007ba7022a98dabf7647ab87860dc85bafbb100f4b791ea1"
      },
      {
        "run": "candidate-r1",
        "outputTokensPerSecond": 6.178239764442994,
        "comparisonSha256": "9da2ff2198b5b9f8937de4a3a1fee01514fadef5105e1ee55f400533575dd949"
      },
      {
        "run": "candidate-r2",
        "outputTokensPerSecond": 6.174699472398327,
        "comparisonSha256": "80a0d25350e87d55c2f8cde40671bf499d7ed0bef70a3d6837a1e9482a5f2b63"
      },
      {
        "run": "control-r2",
        "outputTokensPerSecond": 5.5076840812458965,
        "comparisonSha256": "a6a674e7a0075bb6cf20ab3a19590106a0307903d4f7ca70634236c6312e36be"
      }
    ]
  },
  "draftMatrix": {
    "completedCases": 8,
    "plannedCases": 8,
    "projections": [
      "baseline",
      "mfma"
    ],
    "prefill": [
      "full",
      "tokenwise"
    ],
    "repetitionsPerProfile": 2,
    "generatedTokens": [
      12095,
      13
    ],
    "generatedText": " Paris.",
    "archiveSha256": "7ce5999a57f97285a6519a4376453992271567518ddafeac7fd648229ded6c3b",
    "allReferenceOutputsExact": true,
    "unforcedCleanup": true,
    "allEightIdleAfter": true,
    "servingQualified": false
  },
  "parallelArgmax": {
    "source": "67b729046d78b8f48205eeab2152fb90499b374f",
    "artifactSha256": "122ee2a146b1f0735a315764c5eb1e2ff9aa83c26f5800b42ce6d69b0602236b",
    "wrapperSha256": "4c4fda5650912f8b26198ee91f1d27c27ca50e18898ee8cc9a295d2d45c7e3ac",
    "reportSha256": "824c6404e07fdef5887ad87cd475f9cae65ed3797c690abd14ade118ff7ced7a",
    "nativeCases": 14,
    "activeInputsFinite": true,
    "fullByteAndGuardChecks": true,
    "adapterIntegrated": false,
    "modelSpeedupClaimed": false,
    "defaultRoute": "v8"
  },
  "pairedK4": {
    "source": "ae355e52e7674f027ca63f561dc6ec2b31ce5b3c",
    "controllerSha256": "bd0cf2480d94c509f00323c39ed469a04d604ce3db21c801cab85e1876580766",
    "hostAggregateSha256": "f67227b267089f52ec7618dc14bc2bcc2b0051f6f21645c13acbca2ac8a827e1",
    "hostTests": 594,
    "imageSkips": 25,
    "doctests": 8,
    "sourceFiles": 1053,
    "referenceSha256": "e5588cefc924a8be5b77345876b6893d53436213de03108a7fc4702cc86356a0",
    "freshRuns": 2,
    "roundsPerRun": 2,
    "draftWidth": 4,
    "acceptedPerRun": [
      [
        4,
        4
      ],
      [
        4,
        4
      ]
    ],
    "catchUpsPerRun": [
      2,
      2
    ],
    "generatedTokens": [
      13934,
      9489,
      9079,
      13,
      5443,
      30339,
      4128,
      323,
      14175,
      10295
    ],
    "targetPacketsPerRun": 6136,
    "draftPacketsPerRun": 8610,
    "exactTargetPrefix": true,
    "unforcedCleanup": true,
    "allEightIdleAfter": true,
    "nativeRejectionBranchesCovered": false,
    "speculativeHttpServing": false,
    "performanceQualified": false,
    "wrapperSha256": [
      "70d6ff75e7f8e69d4dfe7b026ed6d854d02e0813744e114c22aaa83f06d17865",
      "bb662c2dfe6be7a0661e66bf9d2f65f201136165362d001105f7d83ed0373926"
    ],
    "traceSha256": [
      "1ef9662d533222252067a85c61e5c1960e7987cec9dbb1f0c7be4ac2e05f81a5",
      "f682807721a55a5733bb528b419a5abe4eb5da058c33fa97f3674683473d75a9"
    ]
  },
  "sglangR7": {
    "state": "numerical-rejected",
    "receiptSha256": "69bba54516149f01ca0bf52e415f7577b5b9d5e8addcd7ea8afe510196118a6a",
    "auditSha256": "e228350363d13b0e3639c3c21f5fa93c1da1fc060d1ec426c0c3f8ae924df412",
    "warmups": 10,
    "measuredRequests": 30,
    "measuredTextMismatches": 10,
    "warmupTextMismatches": 4,
    "allLengths128": true,
    "extraReasoningTokensZero": true,
    "nativeBeforeExact": true,
    "nativeAfterExact": false,
    "afterFirstMismatchIndex": 4,
    "unforcedCleanup": true,
    "allEightIdleAfter": true,
    "metrics": null
  },
  "authority": "none",
  "competitiveWinClaimed": false,
  "newVerusClaimed": false,
  "m1Complete": false
};
const evidencePins = {
  "paired-reference.json": "e5588cefc924a8be5b77345876b6893d53436213de03108a7fc4702cc86356a0",
  "paired-host-aggregate.json": "f67227b267089f52ec7618dc14bc2bcc2b0051f6f21645c13acbca2ac8a827e1",
  "sglang-r7-audit.json": "e228350363d13b0e3639c3c21f5fa93c1da1fc060d1ec426c0c3f8ae924df412",
  "argmax-v11-native-216-r1/wrapper-result.json": "4c4fda5650912f8b26198ee91f1d27c27ca50e18898ee8cc9a295d2d45c7e3ac",
  "argmax-v11-native-216-r1/probe/result.json": "824c6404e07fdef5887ad87cd475f9cae65ed3797c690abd14ade118ff7ced7a",
  "ordered-wave-f65-control-r1/comparison.json": "f6a43299f8061e4e007ba7022a98dabf7647ab87860dc85bafbb100f4b791ea1",
  "ordered-wave-f65-control-r1/runner-result.json": "19f4116986a1494e74d605ced931e903415f9a96adf6f4ba33f96366e26f0d8c",
  "ordered-wave-f65-control-r1/results.jsonl": "c461fa5676bf935ca1fc89427d03ad1f74618fbe7727dec27f0174c8066619bf",
  "ordered-wave-f65-candidate-r1/comparison.json": "9da2ff2198b5b9f8937de4a3a1fee01514fadef5105e1ee55f400533575dd949",
  "ordered-wave-f65-candidate-r1/runner-result.json": "1a95cfbd4a95fbc50d5dc714f57d166804c3287dcadcb5e4686b89c98e13a9eb",
  "ordered-wave-f65-candidate-r1/results.jsonl": "3f1e739ac6db2dd2c1fe0af6eaa21e4df9c4487d97b13f39ad5127d8c495dc9e",
  "ordered-wave-f65-candidate-r2/comparison.json": "80a0d25350e87d55c2f8cde40671bf499d7ed0bef70a3d6837a1e9482a5f2b63",
  "ordered-wave-f65-candidate-r2/runner-result.json": "528af1ff5abf47499654992eed2f8932d9332ebb302a56b48a281af4cbbef543",
  "ordered-wave-f65-candidate-r2/results.jsonl": "285b11146a051b38c7db5939318fb1dedd26cfd12e9684b5d1808b2a09617680",
  "ordered-wave-f65-control-r2/comparison.json": "a6a674e7a0075bb6cf20ab3a19590106a0307903d4f7ca70634236c6312e36be",
  "ordered-wave-f65-control-r2/runner-result.json": "2e5a9eb9e7f2f7d5ad250ed3d9784bdd16687ce1a26c205c40f27aabe93ce6cd",
  "ordered-wave-f65-control-r2/results.jsonl": "e178e94ae98ac9b4e2707d1d459016bc8a0dc30c7f366f97ca2f864cebf3baa2",
  "draft-paged-v10-baseline-full-r1/wrapper-result.json": "cc6ad61c0199c7c261703066ae23df6f6c589c4bdf909f83c01d6ccb05a785b0",
  "draft-paged-v10-baseline-full-r1/results.jsonl": "b6cd219d61d36f448c00260af797ff99327b5510da68b4597b8d6494e7d3d8be",
  "draft-paged-v10-baseline-full-r2/wrapper-result.json": "8a956a5c1f35615a29119caa0fe841f9ee44a6fff5b38c5210caf9911964c688",
  "draft-paged-v10-baseline-full-r2/results.jsonl": "03b437d0022450789973b2d78d5408a2877f585b70c5654a64341554b4599e44",
  "draft-paged-v10-baseline-tokenwise-r1/wrapper-result.json": "520f8647fbb0e3ab1c90b91ab8f297c0c203292e98ff5253a513d6ca27e51897",
  "draft-paged-v10-baseline-tokenwise-r1/results.jsonl": "c29db9a70514c26b4fd8fb67a31378e56d184be7dee3f4288204ffde68d5b100",
  "draft-paged-v10-baseline-tokenwise-r2/wrapper-result.json": "ffee78ae440e0a505ec41c701ac06725cc2e046552ca90593a447dd3b7ea2d99",
  "draft-paged-v10-baseline-tokenwise-r2/results.jsonl": "25c327e658547017506a3598f48a54a2671ef1ea9a40e586ead694cd025d1698",
  "draft-paged-v10-mfma-full-r1/wrapper-result.json": "e6970c35672e3b3cf2d6a2892c3c4d6d80ff4252cc269718649c75385237cbf3",
  "draft-paged-v10-mfma-full-r1/results.jsonl": "876ce6d9bc1291a3ad0760483cc8ce5d156eba711dfdfd4cff44efb010ac6070",
  "draft-paged-v10-mfma-full-r2/wrapper-result.json": "e406d9e11ac2e1d2e37b614e265a2603b645798b1f63eb287ac084ea8cf2d81e",
  "draft-paged-v10-mfma-full-r2/results.jsonl": "c718d3ab3e367b6b289f86ce7690f74b6b7578c523d322de786e2e8b3650edd8",
  "draft-paged-v10-mfma-tokenwise-r1/wrapper-result.json": "3881edbe577d29ce4405b68e384aec917912c1da59781365f153f9bbdca44f11",
  "draft-paged-v10-mfma-tokenwise-r1/results.jsonl": "ae3a3f1b7111a6ef8aee19f4f39db40aa3d535a4a25f1513cd5d8fbf85005ee2",
  "draft-paged-v10-mfma-tokenwise-r2/wrapper-result.json": "dbf962f9ac10c2b7e37e37220a4b5af689c299199191425cca1ae7817e8bac7e",
  "draft-paged-v10-mfma-tokenwise-r2/results.jsonl": "d40eb80130d2f08abb3a015620eb7e51e6c61d4368e86a9d40dd9c4ec92fc532",
  "paired-paged-k4-216-r1/wrapper-result.json": "70d6ff75e7f8e69d4dfe7b026ed6d854d02e0813744e114c22aaa83f06d17865",
  "paired-paged-k4-216-r1/results.jsonl": "1ef9662d533222252067a85c61e5c1960e7987cec9dbb1f0c7be4ac2e05f81a5",
  "paired-paged-k4-216-r1/prelaunch.json": "68238896e9481ba874965a1a3d9deaf7c6da997c3015a24651a904f0e2676677",
  "paired-paged-k4-216-r2/wrapper-result.json": "bb662c2dfe6be7a0661e66bf9d2f65f201136165362d001105f7d83ed0373926",
  "paired-paged-k4-216-r2/results.jsonl": "f682807721a55a5733bb528b419a5abe4eb5da058c33fa97f3674683473d75a9",
  "paired-paged-k4-216-r2/prelaunch.json": "68238896e9481ba874965a1a3d9deaf7c6da997c3015a24651a904f0e2676677"
};
const plain = (value) => JSON.parse(JSON.stringify(value));
const requiredReadiness = [
  [1, "Paged draft: all eight native cases pass", "eight of eight", "not speculative serving"],
  [2, "Paired K4: two fresh native passes", "[4,4]", "two genuine last-proposal catch-ups",
    "Native rejection/rollback branches still need other workloads", "not HTTP speculative serving"],
  [3, "Parallel FP32 argmax: 14 native fixtures", "all 14 finite-active native fixtures",
    "The adapter still uses v8", "not an integrated model speedup"],
  [4, "Wave plus ordered: short-canary ABBA", "+12.27%, n=2 per mode", "host-timing instrumentation",
    "Do not substitute this short canary into the unprofiled matched 128/128 table"],
  [5, "SGLang r7: numerical rejection retained", "10 of 30 measured texts",
    "final native diagnostic also fails", "No SGLang timing"],
];

export function validateNativeFollowup(value, readiness) {
  assert.deepEqual(plain(value), expected, "native follow-up data or qualification scope drifted");
  for (const [index, label, ...phrases] of requiredReadiness) {
    assert.equal(readiness[index].label, label);
    assert.equal(readiness[index].state, index === 5 ? "open" : "observed");
    for (const phrase of phrases) assert(readiness[index].detail.includes(phrase), label + ": " + phrase);
  }
}

export function testNativeFollowupRejections(value, readiness) {
  let mutations = 0;
  const visit = (object, path = []) => {
    for (const [key, child] of Object.entries(object)) {
      const next = [...path, key];
      if (child !== null && typeof child === "object") visit(child, next);
      else {
        const altered = plain(value);
        const parent = next.slice(0, -1).reduce((node, field) => node[field], altered);
        parent[key] = typeof child === "boolean" ? !child : String(child) + "-changed";
        assert.throws(() => validateNativeFollowup(altered, readiness));
        mutations += 1;
      }
    }
  };
  visit(expected);
  for (const key of Object.keys(expected)) {
    const altered = plain(value);
    delete altered[key];
    assert.throws(() => validateNativeFollowup(altered, readiness));
    mutations += 1;
  }
  for (const [index, , ...phrases] of requiredReadiness) {
    for (const phrase of phrases) {
      const altered = plain(readiness);
      altered[index].detail = altered[index].detail.replace(phrase, "changed");
      assert.throws(() => validateNativeFollowup(value, altered));
      mutations += 1;
    }
  }
  assert.throws(() => validateNativeFollowup({ ...value, qualification: true }, readiness));
  console.log("Native follow-up: " + (mutations + 1) + " mutations rejected.");
}

export async function validateNativeEvidence(root, value) {
  value = plain(value);
  const records = {};
  for (const [file, hash] of Object.entries(evidencePins)) {
    const raw = await readFile(join(root, file));
    assert(raw.length > 0 && raw.length < 2 * 1024 * 1024, file + " extent");
    assert.equal(createHash("sha256").update(raw).digest("hex"), hash, file + " evidence pin");
    records[file] = file.endsWith(".jsonl")
      ? raw.toString("utf8").trim().split("\n").map((line) => JSON.parse(line))
      : JSON.parse(raw);
  }
  const wrapper = (row, schema) => {
    assert.equal(row.schema, schema);
    assert.equal(row.passed, true);
    assert.deepEqual(row.errors, []);
    assert.equal(row.authority, "none");
    assert.equal(row.performance_qualified, false);
    assert.equal(row.all_eight_idle_before, true);
    assert.equal(row.all_eight_idle_after, true);
  };
  const rates = [];
  for (const [index, run] of value.waveOrdered.runs.entries()) {
    const base = "ordered-wave-f65-" + run.run + "/";
    const comparison = records[base + "comparison.json"];
    const receipt = records[base + "runner-result.json"];
    assert.equal(receipt.passed, true);
    assert.equal(receipt.qualification, false);
    assert.equal(receipt.diagnostic_host_timing, true);
    assert.equal(receipt.controller_sha256, value.waveOrdered.controllerSha256);
    assert.equal(receipt.controller_source, value.waveOrdered.source);
    assert.equal(receipt.comparison_sha256, run.comparisonSha256);
    assert.equal(comparison.attention, value.waveOrdered.order[index]);
    for (const field of ["passed", "all_reference_tokens_and_bytes_match", "clean_teardown_recorded",
      "gpu_idle_before_and_after"])
      assert.equal(comparison[field], true, base + field);
    assert.deepEqual(comparison.externally_pinned_identities, ["artifact_handoff_id", "artifact_hsaco_id",
      "artifact_manifest_id", "controller_sha256", "worker_sha256"]);
    assert.equal(comparison.qualification, false);
    assert.equal(comparison.authority, "none");
    assert.equal(comparison.prefix_cache, true);
    assert.equal(comparison.generated_tokens, 8);
    assert.equal(comparison.batch_count, 5);
    assert.equal(comparison.physical_token_rows, 34);
    assert.deepEqual(comparison.rank_dispatch_counts, [3077]);
    assert.equal(comparison.metrics.output_tokens_per_second, run.outputTokensPerSecond);
    assert.equal(Object.keys(comparison.metrics.requests).length, 4);
    rates.push(comparison.metrics.output_tokens_per_second);
  }
  const controlMean = (rates[0] + rates[3]) / 2;
  const waveMean = (rates[1] + rates[2]) / 2;
  assert.equal(Number(((waveMean / controlMean - 1) * 100).toFixed(2)), value.waveOrdered.meanRateGainPercent);
  assert.equal(controlMean.toFixed(6), "5.501543");
  assert.equal(waveMean.toFixed(6), "6.176470");

  let draftCases = 0;
  for (const projection of value.draftMatrix.projections)
    for (const prefill of value.draftMatrix.prefill)
      for (const repetition of [1, 2]) {
        const base = "draft-paged-v10-" + projection + "-" + prefill + "-r" + repetition + "/";
        const receipt = records[base + "wrapper-result.json"];
        wrapper(receipt, "FerricDraftPagedCanaryWrapperV10");
        assert.equal(receipt.projection, projection);
        assert.equal(receipt.prefill, prefill);
        assert.equal(receipt.repetition, repetition);
        const trace = records[base + "results.jsonl"];
        assert.equal(trace.length, 3);
        const [setup, observed, closed] = trace;
        const packets = prefill === "full" ? 960 : 2880;
        assert.equal(setup.expected_dispatches, packets);
        assert.equal(observed.reference_passed, true);
        assert.deepEqual(observed.generated_tokens, value.draftMatrix.generatedTokens);
        assert.equal(Buffer.from(observed.generated_utf8_bytes).toString("utf8"), value.draftMatrix.generatedText);
        assert.deepEqual(observed.rank_dispatch_counts, [packets]);
        assert.equal(closed.all_workers_exited, true);
        assert.equal(closed.execution_completed, true);
        assert.equal(closed.reference_passed, true);
        assert.deepEqual(closed.rank_dispatch_counts, [packets]);
        assert(trace.every((row) => row.authority === "none" && row.performance_qualified === false));
        draftCases += 1;
      }
  assert.equal(draftCases, value.draftMatrix.completedCases);

  wrapper(records["argmax-v11-native-216-r1/wrapper-result.json"], "FerricArgmaxNativeWrapperV11");
  const argmax = records["argmax-v11-native-216-r1/probe/result.json"];
  assert.equal(argmax.schema, "FerricTpFp32ArgmaxProbeV11");
  assert.equal(argmax.artifact_sha256, value.parallelArgmax.artifactSha256);
  for (const field of ["checks_pass", "active_inputs_finite", "clean_teardown"]) assert.equal(argmax[field], true);
  for (const field of ["benchmark", "model_inference", "model_parity_qualified"]) assert.equal(argmax[field], false);
  assert.equal(argmax.results.length, 14);
  assert.equal(new Set(argmax.results.map((row) => row.name)).size, 14);
  assert(argmax.results.every((row) => row.checks.length === 2
    && row.checks.every((check) => check.guards_unchanged === true)));

  const host = records["paired-host-aggregate.json"];
  assert.equal(host.schema, "FerricPairedPagedK4CpuGateV1");
  assert.equal(host.source_commit, value.pairedK4.source);
  assert.equal(host.fe2o3_revision, value.pairedHostCore);
  assert.equal(host.binary.sha256, value.pairedK4.controllerSha256);
  assert.equal(host.tests.passed_invocations, value.pairedK4.hostTests);
  assert.equal(host.tests.ignored_image_gated_invocations, value.pairedK4.imageSkips);
  assert.equal(host.tests.doctests_passed, value.pairedK4.doctests);
  assert.equal(host.source_files, value.pairedK4.sourceFiles);
  assert.equal(host.source_unchanged, true);
  assert.equal(host.gpu_execution, false);
  const reference = records["paired-reference.json"];
  assert.equal(reference.source_reference_sha256, "cd7f512537e44d677244c606cba919e57747e8c26caf3c0df4a0aca7acc0eb8b");
  assert.deepEqual(reference.generated_token_ids.slice(0, 10), value.pairedK4.generatedTokens);
  for (const run of [1, 2]) {
    const base = "paired-paged-k4-216-r" + run + "/";
    const receipt = records[base + "wrapper-result.json"];
    wrapper(receipt, "FerricPairedPagedK4WrapperV1");
    assert.equal(receipt.serving_qualified, false);
    assert.deepEqual(receipt.checked_trace.accepted_draft_tokens, [4, 4]);
    assert.equal(receipt.checked_trace.catch_up_count, 2);
    assert.deepEqual(receipt.checked_trace.generated_tokens, value.pairedK4.generatedTokens);
    assert.equal(records[base + "prelaunch.json"].controller_source, value.pairedK4.source);
    const trace = records[base + "results.jsonl"];
    assert.equal(trace.length, 6);
    const [setup, , first, second, observed, closed] = trace;
    const emitted = [];
    let cursor = 127;
    let anchor = setup.initial_anchor;
    for (const [index, round] of [first, second].entries()) {
      assert.equal(round.schema, "FerricPairedPagedK4RoundV1");
      assert.equal(round.cursor_before, cursor);
      assert.equal(round.anchor, anchor);
      const draft = round.proposals.map((item) => item.choice);
      const target = round.target_verification.choices;
      let accepted = 0;
      while (accepted < 4 && draft[accepted] === target[accepted]) accepted += 1;
      assert.equal(accepted, 4);
      assert.equal(round.accepted_draft_tokens, accepted);
      assert.equal(round.epoch, index + 1);
      assert.equal(round.next_epoch, index + 2);
      assert.deepEqual(round.emitted_tokens, [...draft, target[4]]);
      assert.deepEqual(round.catch_up.inputs, [{ position: cursor + 4, token: draft[3] }]);
      assert.equal(round.catch_up.epoch, round.next_epoch);
      assert.deepEqual(round.catch_up.selected_rows, []);
      emitted.push(...round.emitted_tokens);
      cursor += 5;
      anchor = target[4];
      assert.equal(round.next_anchor, anchor);
    }
    assert.deepEqual(emitted, value.pairedK4.generatedTokens);
    assert.deepEqual(observed.generated_tokens, emitted);
    assert.equal(observed.generated_utf8_hex, reference.prefix_utf8_hex[9]);
    assert.equal(observed.reference_passed, true);
    assert.equal(observed.native_catch_up_exercised, true);
    assert.equal(observed.catch_up_count, 2);
    assert.equal(observed.target_cursor, cursor);
    assert.equal(observed.draft_cursor, cursor);
    assert.equal(observed.target_packets, value.pairedK4.targetPacketsPerRun);
    assert.equal(observed.draft_packets, value.pairedK4.draftPacketsPerRun);
    assert.deepEqual(closed.target_packets, [value.pairedK4.targetPacketsPerRun]);
    assert.deepEqual(closed.draft_packets, [value.pairedK4.draftPacketsPerRun]);
    for (const field of ["execution_completed", "reference_passed", "target_worker_exited", "draft_worker_exited"])
      assert.equal(closed[field], true);
    assert(trace.every((row) => row.authority === "none" && row.performance_qualified === false));
  }

  const sg = records["sglang-r7-audit.json"];
  assert.equal(sg.schema, "FerricSglangR7NumericalFailureAuditV1");
  for (const field of ["timing_admitted", "latency_metrics_computed", "qualification"]) assert.equal(sg[field], false);
  assert.equal(sg.reference_policy_unchanged, true);
  assert.equal(sg.native_diagnostic_request_bytes_identical, true);
  assert.equal(sg.phases.samples.requests, 30);
  assert.equal(sg.phases.samples.text_mismatch_indices.length, 10);
  assert.equal(sg.phases.warmups.requests, 10);
  assert.equal(sg.phases.warmups.text_mismatch_indices.length, 4);
  for (const phase of Object.values(sg.phases))
    for (const field of ["all_core_counts_exact", "all_only_extra_reasoning_zero", "all_successful", "all_finish_length"])
      assert.equal(phase[field], true);
  assert.equal(sg.diagnostics.before.output_ids_exact, true);
  assert.equal(sg.diagnostics.before.output_utf8_exact, true);
  assert.equal(sg.diagnostics.after.output_ids_exact, false);
  assert.equal(sg.diagnostics.after.output_utf8_exact, false);
  assert.equal(sg.diagnostics.after.first_token_mismatch, 4);
  for (const field of ["completed", "container_absence_recorded", "owned_cache_absent",
    "stopped_before_removal", "unforced_process_exit", "all_eight_idle_from_existing_record"])
    assert.equal(sg.cleanup[field], true);
  assert(Object.values(sg.evidence_sha256).includes(value.sglangR7.receiptSha256));
  console.log("PASS: pinned wave ABBA, eight draft cases, 14 argmax fixtures, both paired K4 traces and rejected SG audit. No new native launch or qualification.");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const context = { window: {} };
  vm.runInNewContext(await readFile(join(dirname(fileURLToPath(import.meta.url)), "data/project.js"), "utf8"), context);
  const project = context.window.FERRIC_PROJECT;
  validateNativeFollowup(project.nativeFollowup, project.latestReadiness);
  testNativeFollowupRejections(project.nativeFollowup, project.latestReadiness);
  if (process.argv[2]) await validateNativeEvidence(process.argv[2], project.nativeFollowup);
}
