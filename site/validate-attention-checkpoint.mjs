import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const expected = {
  schema: "FerricPagesAttentionCheckpointV1",
  attribution: {
    source: "809af24513f71cde7439bd8f7cfbff4c11351ade",
    tree: "29f9a335f2317b570c67f1f8958aebb339f00a3e",
    controllerSha256: "950b42c13f5f2fa548cce4a0c0c029a6b2bb87a1addb9f8cc8119a3467517a82",
    reporterSha256: "43282d1f74fea4730beb26912e4fa90b34b548bf205a0a74e10e15053c55d715",
    reportSha256: "0ebd4dd53827536f8dbca6f4ed3c5be964dfd6a5e1f36d29eb86bfe488f6b633",
    traceSha256: "9cfa0d042b41833acd3b7f27627478a766d731345146a065af75151125a8ed91",
    timingSha256: "f3f57f7ba82f639d4ff690de0c0df7b8088d99af4d4d4126498776ee888e7aa1",
    wrapperReceiptSha256: "9b5c000eb0e2462fd40c25b2dd8f7d1d0fcca8d7726a70b391b055476ee3fa52",
    inputTokens: 128, outputTokens: 128, tp: 1, attention: "baseline", argmax: "wave-v11",
    ordered: false, prefixCaching: false, repetitions: 1, batches: 135, packets: 83139,
    layersPerBatch: 36, groupRecords: 945, parentNs: 56579512157,
    groups: [
      ["attention_gqa_math", "GQA computation", 41068390375],
      ["attention_input_norm", "Input normalization", 4165605679],
      ["attention_qkv_projection", "QKV projection", 4119716878],
      ["attention_kv_append", "KV append", 3637546242],
      ["attention_qk_norm", "QK normalization", 1530490487],
      ["attention_output_projection", "Output projection", 1275791124],
      ["attention_rope", "RoPE", 778113253],
    ],
    clock: "controller-std-instant", referencePassed: true, normalWorkerClose: true,
    allEightIdleAfter: true, crossRunComparison: false, gpuDuration: false, causalSpeedup: false,
  },
  fixtures: {
    source: "c26c1f4dc267b53f840b2fb01cafd19234596ed2",
    integratedSource: "8da127ba203f192ef673a4f4b5c41bba1b7311ad",
    probeSha256: "6a7c7baf533a9fa7e0773126519992bb62dfdaefa3ed5f239729319fd6a7f789",
    reportSha256: "26963599a868ee033919d9d369ef8ac18376eb57629e76b4250aa315211952c0",
    replaySha256: "ff826ed5b092c613cd9547a4709e2fcd9672205bb28c9fb7b82ec2d410c65ea8",
    wrapperReceiptSha256: "f0cd79931550212bca45de085525a613e501b6bcaa7a52f0957bc18f5b6136e6",
    prelaunchSha256: "34699b79c7c33c444cd86e21096aa589012c5d8806037b48f91a82b4afe1efae",
    imageSha256: "98b5fdb14ac7242e324e1801e1885c98e77473d622b2e9b3f9f12f14c7d75502",
    compilerSource: "3e74a9324a5acd7107e96a4a9b5319d3dd5ecde8",
    cases: 8, rows: [1, 17, 31, 32], tp: 1, capacity: 32,
    outputSlices: 8, immutableInputSlices: 40, guardSides: 96, outputBytesPerCase: 262144,
    exactBf16Bytes: true, normalWorkerClose: true, allEightIdleAfter: true,
    modelParityQualified: false, broadNumericalGuarantee: false,
  },
  composition: {
    source: "ed112e0b8298d1d2070b93dea638e7572ec65ba9",
    tree: "e3fd5c5757298312394522bbd80c89eb05e35f73",
    hostGateNoteSha256: "139e1696797b43049dfa42c8a140c56375714242f1aa62caaf2c5487f306ea9d",
    hostGateArchiveSha256: "f3860c912f5c44b611a2209c14e7eb733c092c9ea7481ddad20237ae7d785dfd",
    requiredCommandsPassed: 42, actualCommandAttempts: 43,
    testInvocationsPassed: 685, testInvocationsIgnored: 34, doctests: 8,
    sourceGateTests: 38, protectedSourcePolicies: 31, unchangedInventories: 5,
    sourceUnchanged: true, warmTarget: true,
    nestedTmpFailureArchiveSha256: "589c753d66bd6783295620a6452236328c4301cd959dbf5be09e824756ebee20",
    earlierClippyFailureArchiveSha256: "68a8617b7f96c83478c68916cd37b16e3fdbd31bc8c4080e4af577cff9ca48de",
    state: "host-and-native-passed-abba-pending", fullHostGatePassed: true, nativeModelGatePassed: true,
    qualification: {
      noteSha256: "764fdd72e56011bad94794adb1329c3dce775fee8e37f31133a055f4d876449a",
      archiveSha256: "d00d832c6a4e0e8732b7902c9e6470414324c615589ccf23ab9eb6ae2b4ba45f",
      localRosterSha256: "2d08a46276c19e23ad789b5b795fa1f127d7fd0b8843c38c85c567faceb6f270",
      remoteRosterSha256: "b1b59a374be2a4f5ec604c653eec9219e19118010735387ccb4265cf002e6087",
      controllerSha256: "d0ec403ea4713f4d795ee55221da5282a5668b7ebff283ba56c531f236cc0468",
      checkerSha256: "5e8428f236c7056d355b7a81ee9e5e45e086d8c6b1fbae50edd8b9cee4b4a5c5",
      wrapperSha256: "58a857e3aa420c2ccb429061ff2e15a1560bba1cf836c98de92594b693cf1662",
      referenceSha256: "cd7f512537e44d677244c606cba919e57747e8c26caf3c0df4a0aca7acc0eb8b",
      prefixReferenceSha256: "e5588cefc924a8be5b77345876b6893d53436213de03108a7fc4702cc86356a0",
      cases: [
        ["attention-argmax-ed112e0-baseline-8-r1", "baseline", 8, 9219, 15, 135],
        ["attention-argmax-ed112e0-wave-8-r1", "wave", 8, 9219, 15, 135],
        ["attention-argmax-ed112e0-baseline-128-r1", "baseline", 128, 83139, 135, 255],
        ["attention-argmax-ed112e0-wave-128-r1", "wave", 128, 83139, 135, 255],
      ],
      rawFiles: 40, inputTokens: 128, tp: 1, capacity: 32, context: 256, pages: 16,
      prefillChunk: 16, argmax: "wave-v11", projection: "mfma", collective: "device-tp1-v3",
      prefixCaching: false, speculation: false, ordered: false,
      exactIdsAndUtf8: true, retirementPassed: true, normalWorkerClose: true,
      allEightIdleBeforeAndAfter: true, comparisonSamples: false,
    },
    full128AbbaAdmitted: false, metrics: null,
  },
  authority: "none", performanceGainClaimed: false, httpMeasurement: false,
  newVerusProof: false, defaultPromotion: false, m1Complete: false,
};
const readinessContract = [
  ["Attention attribution: one native diagnostic", "observed", "41.068 s", "72.59%",
    "Sibling groups are disjoint, but parent and IPC spans overlap", "not GPU durations", "a measured speedup"],
  ["TP1 attention: eight exact native fixtures", "observed", "1/17/31/32", "40 unchanged inputs and 96 guard sides",
    "Baseline/wave bytes match exactly", "no softmax tolerance", "not broad numerical qualification",
    "a new Verus proof or a performance result", "original fe2o3 3e74 provenance"],
  ["Combined attention / argmax: host and native gates passed", "observed", "42 required commands across 43 actual attempts",
    "685 passed test invocations, 34 ignored", "failed nested-TMP invocation and earlier Clippy failure",
    "warm, not a clean rebuild", "All four baseline/wave attention model checks pass exact IDs and UTF-8",
    "9,219 packets, 15 batches and cursor 135", "83,139 packets, 135 batches and cursor 255",
    "correctness diagnostics, not comparison samples", "full128 ABBA cohort remains pending at this checkpoint",
    "No combined-route metrics, default promotion or change to the matched HTTP results is admitted"],
];
const plain = (value) => JSON.parse(JSON.stringify(value));

export function validateAttentionCheckpoint(value, readiness) {
  assert.deepEqual(plain(value), expected, "attention checkpoint source or scope drifted");
  assert.equal(readiness.length, readinessContract.length);
  readinessContract.forEach(([label, state, ...phrases], index) => {
    assert.deepEqual(Object.keys(readiness[index]).sort(), ["detail", "label", "state"]);
    assert.equal(readiness[index].label, label);
    assert.equal(readiness[index].state, state);
    for (const phrase of phrases) assert(readiness[index].detail.includes(phrase), phrase);
  });
}

export function testAttentionCheckpointRejections(value, readiness) {
  let mutations = 0;
  function visit(object, path = []) {
    for (const [key, child] of Object.entries(object)) {
      const next = [...path, key];
      if (child !== null && typeof child === "object") visit(child, next);
      else {
        const changed = plain(value);
        next.slice(0, -1).reduce((node, field) => node[field], changed)[key] =
          typeof child === "boolean" ? !child : String(child) + "-changed";
        assert.throws(() => validateAttentionCheckpoint(changed, readiness));
        mutations += 1;
      }
    }
  }
  visit(expected);
  for (const key of Object.keys(expected)) {
    const changed = plain(value);
    delete changed[key];
    assert.throws(() => validateAttentionCheckpoint(changed, readiness));
    mutations += 1;
  }
  readinessContract.forEach(([, , ...phrases], index) => {
    for (const phrase of phrases) {
      const changed = plain(readiness);
      changed[index].detail = changed[index].detail.replace(phrase, "changed");
      assert.throws(() => validateAttentionCheckpoint(value, changed));
      mutations += 1;
    }
  });
  assert.throws(() => validateAttentionCheckpoint({ ...value, qualified: true }, readiness));
  console.log(`Attention checkpoint: ${mutations + 1} mutations rejected.`);
}

export async function validateAttentionCheckpointEvidence(root, value) {
  const { attribution: a, fixtures: f, composition: c } = plain(value);
  async function pinned(file, digest, parse = true) {
    const raw = await readFile(join(root, file));
    assert(raw.length > 0 && raw.length < 4 * 1024 * 1024, file);
    assert.equal(createHash("sha256").update(raw).digest("hex"), digest, file);
    return parse ? JSON.parse(raw) : raw;
  }
  const hostNote = (await pinned("composition-HOST-GATE.md", c.hostGateNoteSha256, false)).toString("utf8");
  for (const identity of [c.source, c.tree, c.hostGateArchiveSha256,
    c.nestedTmpFailureArchiveSha256, c.earlierClippyFailureArchiveSha256]) assert(hostNote.includes(identity));
  for (const fact of ["All 42 required command steps passed", "43 actual ed112 command",
    "685 passed, 34 ignored", "Doctests 8", "source-gate tests 38", "31 passed",
    "All five inventories", "No source changed between them", "not a clean rebuild",
    "original ed112 all-targets invocation remains a distinct failed attempt",
    "earlier ba82 strict-Clippy failure also remains separately archived"]) assert(hostNote.includes(fact), fact);
  const q = c.qualification;
  const qualificationNote = (await pinned("qualification/QUALIFICATION.md", q.noteSha256, false)).toString("utf8");
  for (const identity of [c.source, c.tree, q.controllerSha256, q.archiveSha256,
    q.localRosterSha256, q.remoteRosterSha256]) assert(qualificationNote.includes(identity));
  for (const fact of ["All four prospective correctness cases pass", "not comparison samples",
    "No reference was replaced", "all forty", "second independent remote tar stream"])
    assert(qualificationNote.includes(fact), fact);
  function roster(raw, prefix = "") {
    return raw.toString("utf8").trim().split("\n").map((line) => {
      const match = /^([a-f0-9]{64})  (\S+)$/.exec(line);
      assert(match, "malformed qualification roster");
      assert(match[2].startsWith(prefix));
      return [match[1], match[2].slice(prefix.length)];
    });
  }
  const localRoster = roster(await pinned("qualification/qualification-local.sha256", q.localRosterSha256, false));
  const remoteRoster = roster(await pinned("qualification/qualification-remote.sha256", q.remoteRosterSha256, false),
    "/tmp/ferric-compete-gpu.VabkOGCx/");
  const rawNames = ["results.jsonl", "host-timing.json", "wrapper-result.json", "prelaunch.json",
    "owned-process.json", "group-cleanup.json", "wall-clock.json", "resources-before.json",
    "gpu-before.json", "gpu-after.json"];
  assert.deepEqual(remoteRoster, localRoster);
  assert.equal(localRoster.length, q.rawFiles);
  assert.deepEqual(localRoster.map(([, file]) => file), q.cases.flatMap(([id]) => rawNames.map((name) => `${id}/${name}`)));
  const qualificationRaw = new Map();
  for (const [digest, file] of localRoster) qualificationRaw.set(file, await pinned(`qualification/${file}`, digest, false));
  const targetReference = await pinned("qualification-reference.json", q.referenceSha256);
  const prefixReference = await pinned("qualification-prefix-reference.json", q.prefixReferenceSha256);
  assert.equal(targetReference.schema, "FerricMatched128ReferenceV1");
  assert.equal(prefixReference.source_reference_sha256, q.referenceSha256);
  assert.deepEqual(prefixReference.prompt_token_ids, targetReference.prompt_token_ids);
  for (const [id, mode, outputs, packets, batches, cursor] of q.cases) {
    const json = (name) => JSON.parse(qualificationRaw.get(`${id}/${name}`));
    const records = qualificationRaw.get(`${id}/results.jsonl`).toString("utf8").trim().split("\n").map((line) => JSON.parse(line));
    const setup = records[0];
    const observation = records.at(-2);
    const closed = records.at(-1);
    const receipt = json("wrapper-result.json");
    const prelaunch = json("prelaunch.json");
    const checked = receipt.checked_trace;
    assert.equal(setup.schema, "FerricAttentionArgmaxCanarySetupV1");
    assert.equal(observation.schema, "FerricAttentionArgmaxCanaryObservationV1");
    assert.equal(closed.schema, "FerricAttentionArgmaxCanaryClosedV1");
    assert.equal(receipt.schema, "FerricAttentionArgmaxModelWrapperV1");
    assert.equal(checked.schema, "FerricAttentionArgmaxCanaryCheckedV1");
    assert.equal(prelaunch.controller_source, c.source);
    assert.equal(prelaunch.pins[prelaunch.argv[0]], q.controllerSha256);
    for (const [name, digest] of [["check_attention_argmax.py", q.checkerSha256],
      ["run_attention_argmax.py", q.wrapperSha256]]) {
      assert.equal(prelaunch.pins[`/tmp/ferric-compete-gpu.VabkOGCx/attention-argmax-wrapper-v1/${name}`], digest);
    }
    assert.deepEqual(setup.prompt_token_ids, targetReference.prompt_token_ids);
    assert.equal(setup.prompt_token_ids.length, q.inputTokens);
    for (const [field, value] of Object.entries({ controller_sha256: q.controllerSha256,
      context: q.context, pages: q.pages, row_capacity: q.capacity, prefill_chunk: q.prefillChunk,
      argmax_mode: q.argmax, projection: q.projection, collective: q.collective,
      attention: mode, max_new_tokens: outputs, expected_packets: packets, expected_batches: batches,
      prefix_cache: q.prefixCaching, runtime_ordered_batches: q.ordered,
      source_reference_sha256: q.referenceSha256, prefix_reference_sha256: q.prefixReferenceSha256 }))
      assert.equal(setup[field], value, `${id}: ${field}`);
    assert.equal(setup.target_artifact.hsaco, f.imageSha256);
    assert.equal(setup.target_head_artifact.hsaco, "5f19b3ba59035a5f0ebc90cdf3a40466f9910908a45e082d146cb674028da6cb");
    assert.equal(setup.argmax_artifact.hsaco, "de9db78c0ef7ad5d84fc903d41ee9db59026113d79e23f12d3909761363b9390");
    const expectedIds = targetReference.generated_token_ids.slice(0, outputs);
    const expectedUtf8 = outputs === 128 ? targetReference.generated_utf8_hex : prefixReference.prefix_utf8_hex[7];
    for (const result of [observation, checked]) {
      assert.equal(result.reference_passed, true);
      assert.deepEqual(result.generated_token_ids, expectedIds);
      assert.equal(result.generated_utf8_hex, expectedUtf8);
      assert.equal(result.committed_inputs_before_retirement, cursor);
      assert.equal(result.completed_batches, batches);
    }
    assert.equal(observation.pool_retired, true);
    assert.equal(observation.completed_packets, packets);
    assert.equal(checked.completed_packets, packets);
    assert.equal(checked.controller_sha256, q.controllerSha256);
    assert.equal(checked.attention, mode);
    assert.equal(checked.argmax_mode, q.argmax);
    assert.equal(checked.speculative_serving, false);
    for (const flag of ["http_measurement", "gpu_clock_measurement"]) assert.equal(checked[flag], false);
    for (const value of [setup, observation, closed, receipt, checked, prelaunch]) {
      assert.equal(value.authority, "none");
      assert.equal(value.performance_qualified, false);
    }
    assert.equal(receipt.passed, true);
    assert.equal(receipt.serving_qualified, false);
    assert.equal(receipt.attention, mode);
    assert.equal(receipt.max_new_tokens, outputs);
    assert.deepEqual(receipt.errors, []);
    assert.equal(receipt.all_eight_idle_before, true);
    assert.equal(receipt.all_eight_idle_after, true);
    assert.equal(closed.execution_completed, true);
    assert.equal(closed.reference_passed, true);
    assert.equal(closed.worker_exited, true);
    assert.equal(closed.completed_batches, batches);
    assert.deepEqual(closed.completed_packets, [packets]);
    assert(Number.isInteger(setup.worker_pid) && setup.worker_pid > 0);
    assert.equal(closed.worker_pid, setup.worker_pid);
    assert.equal(checked.worker_pid, setup.worker_pid);
    assert.deepEqual(json("group-cleanup.json"), { absent: true, cleanup_error: null,
      controller_returncode: 0, forced: false, reason: null });
    for (const name of ["gpu-before.json", "gpu-after.json"]) {
      const gpu = json(name);
      assert.deepEqual(Object.keys(gpu).sort(), Array.from({ length: 8 }, (_, index) => `card${index}`));
      for (const card of Object.values(gpu)) {
        assert.equal(card["GPU use (%)"], "0");
        assert.equal(card["GPU Memory Allocated (VRAM%)"], "0");
      }
    }
  }
  await pinned("attention-reporter.py", a.reporterSha256, false);
  const diagnostic = await pinned("attention-diagnostic.json", a.reportSha256);
  const timing = await pinned("attention-host-timing.json", a.timingSha256);
  const trace = (await pinned("attention-results.jsonl", a.traceSha256, false))
    .toString("utf8").trim().split("\n").map((line) => JSON.parse(line));
  const attentionWrapper = await pinned("attention-wrapper.json", a.wrapperReceiptSha256);
  assert.equal(diagnostic.schema, "FerricAttentionGroupDiagnosticV1");
  assert.equal(diagnostic.source, a.source);
  assert.equal(diagnostic.tree, a.tree);
  assert.equal(diagnostic.controller_sha256, a.controllerSha256);
  assert.equal(diagnostic.clock, a.clock);
  assert.equal(diagnostic.reference_passed, true);
  assert.equal(diagnostic.authority, "none");
  for (const flag of ["cross_run_comparison", "gpu_clock_measurement", "causal_speedup_claimed",
    "performance_qualified", "http_measurement"]) assert.equal(diagnostic[flag], false);
  assert.equal(diagnostic.attention_parent_ns, a.parentNs);
  assert.equal(diagnostic.observed_group_records, a.groupRecords);
  assert.equal(diagnostic.batches, a.batches);
  assert.equal(diagnostic.layers_per_batch, a.layersPerBatch);
  assert.equal(diagnostic.packets, a.packets);
  assert.equal(diagnostic.evidence_sha256["host-timing.json"], a.timingSha256);
  assert.equal(diagnostic.evidence_sha256["results.jsonl"], a.traceSha256);
  assert.equal(diagnostic.evidence_sha256["wrapper-result.json"], a.wrapperReceiptSha256);
  assert.deepEqual(Object.keys(diagnostic.groups).sort(), a.groups.map(([key]) => key).sort());
  for (const [label, , elapsed] of [...a.groups, ["attention", "parent", a.parentNs]]) {
    const records = timing.records.filter((row) => row.phase === "attention" && row.category === "span" && row.label === label);
    assert.equal(records.length, a.batches);
    assert.deepEqual(records.map((row) => row.batch).sort((x, y) => x - y), Array.from({ length: 135 }, (_, index) => index + 1));
    assert.equal(records.reduce((sum, row) => sum + row.elapsed_ns, 0), elapsed);
    assert(records.every((row) => row.count === 36 && row.failed === 0 && row.rank === null && row.dispatches === 0));
    if (label !== "attention") {
      assert.equal(diagnostic.groups[label].elapsed_ns, elapsed);
      assert.equal(diagnostic.groups[label].count, a.batches * a.layersPerBatch);
      assert.equal(diagnostic.groups[label].percent_of_parent, 100 * elapsed / a.parentNs);
    }
  }
  assert.equal(trace[0].controller_sha256, a.controllerSha256);
  assert.equal(trace[0].prompt_token_ids.length, a.inputTokens);
  assert.equal(trace[0].max_new_tokens, a.outputTokens);
  assert.equal(trace[0].attention, a.attention);
  assert.equal(trace[0].argmax_mode, a.argmax);
  assert.equal(trace[0].runtime_ordered_batches, a.ordered);
  assert.equal(trace[0].prefix_cache, a.prefixCaching);
  assert.equal(trace.at(-1).reference_passed, true);
  assert.equal(trace.at(-1).worker_exited, true);
  assert.deepEqual(trace.at(-1).completed_packets, [a.packets]);
  assert.equal(trace.at(-1).completed_batches, a.batches);
  assert.equal(attentionWrapper.passed, true);
  assert.equal(attentionWrapper.all_eight_idle_after, true);

  await pinned("fixture-probe.py", f.probeSha256, false);
  const fixture = await pinned("fixture-result.json", f.reportSha256);
  const replay = await pinned("fixture-replay.json", f.replaySha256);
  const wrapper = await pinned("fixture-wrapper.json", f.wrapperReceiptSha256);
  const prelaunch = await pinned("fixture-prelaunch.json", f.prelaunchSha256);
  assert.equal(fixture.schema, "FerricTp1AttentionProbeV1");
  assert.equal(fixture.source_sha256, f.probeSha256);
  assert.equal(fixture.artifact_sha256, f.imageSha256);
  assert.equal(fixture.clean_teardown, f.normalWorkerClose);
  assert.equal(fixture.checks_pass, true);
  assert.equal(fixture.benchmark, false);
  assert.equal(fixture.model_parity_qualified, false);
  assert.equal(fixture.runtime_operational, true);
  assert.equal(replay.schema, "FerricTp1AttentionNativeReplayV1");
  assert.equal(replay.passed, true);
  assert.equal(replay.source, f.source);
  assert.equal(replay.source_sha256, f.probeSha256);
  assert.equal(replay.artifact_sha256, f.imageSha256);
  assert.equal(replay.compiler_source, f.compilerSource);
  assert.equal(replay.worker_pid, fixture.worker_pid);
  assert.equal(replay.worker_start_ticks, fixture.worker_start_ticks);
  assert.equal(replay.source_unchanged, true);
  assert.equal(replay.performance_qualified, false);
  assert.equal(replay.model_parity_qualified, false);
  assert.equal(replay.native_pins["native/result.json"], f.reportSha256);
  assert.equal(replay.native_pins["native/wrapper-result.json"], f.wrapperReceiptSha256);
  assert.equal(replay.native_pins["native/prelaunch.json"], f.prelaunchSha256);
  assert.deepEqual(replay.counts, { cases: 8, guard_sides: 96, input_slices: 40, output_slices: 8 });
  assert.deepEqual(wrapper, replay.root_receipt);
  assert.equal(wrapper.passed, true);
  assert.equal(wrapper.all_eight_idle_before, true);
  assert.equal(wrapper.all_eight_idle_after, true);
  assert.deepEqual(wrapper.errors, []);
  assert.equal(prelaunch.probe_source, f.source);
  assert.equal(prelaunch.compiler_source, f.compilerSource);
  assert.equal(fixture.results.length, f.cases);
  const names = ["query", "keys", "values", "positions", "table", "output"];
  const sizes = [262144, 2228224, 2228224, 128, 256, 262144];
  fixture.results.forEach((row, index) => {
    const rows = f.rows[Math.floor(index / 2)];
    const family = rows === 32 ? "selector" : "uniform";
    const mode = index % 2 ? "wave" : "baseline";
    assert.equal(row.name, `${mode}_${family}_rows${rows}_tp1_causal_pages`);
    assert.equal(row.symbol, fixture.closed_roots[index % 2]);
    assert.equal(row.checks.length, 6);
    row.checks.forEach((check, buffer) => {
      assert.equal(check.name, names[buffer]);
      assert.equal(check.bytes, sizes[buffer]);
      assert.equal(check.access, buffer === 5 ? "write" : "read");
      assert.equal(check.guards_unchanged, true);
      assert.equal(check.guard_bytes_each_side, 64);
    });
    assert.deepEqual(replay.cases[index], { name: row.name, symbol: row.symbol, checks: row.checks });
    if (index % 2) assert.deepEqual(row.checks, fixture.results[index - 1].checks);
  });
  console.log("PASS: source-bound attention attribution, eight native fixtures, composite host gate with retained failures and four exact model qualifications; no comparison claim.");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const context = { window: {} };
  vm.runInNewContext(await readFile(join(dirname(fileURLToPath(import.meta.url)), "data/project.js"), "utf8"), context);
  const project = context.window.FERRIC_PROJECT;
  validateAttentionCheckpoint(project.attentionCheckpoint, project.attentionReadiness);
  testAttentionCheckpointRejections(project.attentionCheckpoint, project.attentionReadiness);
  if (process.argv[2]) await validateAttentionCheckpointEvidence(process.argv[2], project.attentionCheckpoint);
}
