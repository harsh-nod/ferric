import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const expected = {
  schema: "FerricPagesSubmissionCheckpointV1",
  source: "7cb6522430b6990e1d3f12bc15958865358459f4",
  tree: "2bfbe7b0ce3a6912e2909f31db11d873f422c14e",
  controllerSha256: "0da9c59a9172d809dd5109ec30681a847c5f83e7da2c396a943f51d99235c075",
  planSha256: "a13346d5587c5a6308c9790511d712a8d687fddbf03720cb5ea166be698a786b",
  host: {
    noteSha256: "66001b37ea6ce8fbf739625cd43744671ae888b85bc3f57fd81bbfca0e5e13d1",
    archiveSha256: "6aadf1c013f96760d658d8130466f7694d2b918ba659e36ab35cbcf09166f92d",
    sourceLedgerSha256: "9ec12a68c85d3fbea892340ff2be4cff743a1cb82d4d8e9b42da18826f5454d8",
    commandsPassed: 51, attempts: 51, testInvocationsPassed: 745, testInvocationsIgnored: 38,
    doctests: 8, adapterPolicies: 27, sourceGateTests: 38, protectedPolicies: 31,
    unchangedInventories: 5, custodyLedgers: 104, sourceFiles: 1073, warmTarget: true,
    sourceUnchanged: true, strictClippyPassed: true,
    checkerMethods: 26, checkerArchiveSha256: "7acf112b8922a0199d37b08a73182eecc3d81421d30893700272884cee6cd5ca",
    reducerMethods: 18, reducerArchiveSha256: "9dacc9c055dc767ef0f99897483fa2fd9a984fe5323296be1899918c4cf99970",
  },
  qualification: {
    noteSha256: "a18da51f81a76e2ff90979a01badc3ef3eef2ccda238c59523d98ff2b1c4feda",
    archiveSha256: "33da7d5e241e5d23d8a4be4ea2cc279b949bb4c34a7ad8b1bf89b868727fc6c1",
    rosterSha256: "bdb463cd800c1f9075603d41c0a712dfebe2dab8508c2ea04cf7231196316cbc",
    sizesSha256: "4ca3014ebd79c39c22de2db628865781945ba733855c4e227756195027a05b67",
    checkerSha256: "a05aaf85a6474dc8adf6a2a86374ad2a56252f0df4f4eb665f754a862b42e468",
    wrapperSha256: "7f9acd31c3b821b8c5bcfa6c567d8e605dd2ea642ede77916ec04f3f2012153b",
    referenceSha256: "cd7f512537e44d677244c606cba919e57747e8c26caf3c0df4a0aca7acc0eb8b",
    prefixReferenceSha256: "e5588cefc924a8be5b77345876b6893d53436213de03108a7fc4702cc86356a0",
    cases: [
      ["wave-submission-7cb6522-synchronous-8-r1", "synchronous", 8, 9219, 15, 135],
      ["wave-submission-7cb6522-ordered-8-r1", "ordered", 8, 9219, 15, 135],
      ["wave-submission-7cb6522-synchronous-128-r1", "synchronous", 128, 83139, 135, 255],
      ["wave-submission-7cb6522-ordered-128-r1", "ordered", 128, 83139, 135, 255],
    ],
    rawFiles: 40, inputTokens: 128, tp: 1, context: 256, pages: 16, capacity: 32, prefillChunk: 16,
    attention: "wave", argmax: "wave-v11", prefixCaching: false, speculation: false,
    exactIdsAndUtf8: true, retirementPassed: true, normalWorkerCloses: true,
    allEightIdleBeforeAndAfter: true, comparisonSamples: false,
  },
  abba: {
    summarySha256: "2de0c6a0fd2520f77f53c570959f653b55caa71e8a87ab94d937f565febe2ace",
    manifestSha256: "be4495ac63eaa8b05e2321a39232f3dcd49b12966c7ba9dce01064bd6b86c505",
    replaySha256: "2dd3d91a35632adddbc50dbe21196a6c6b59f86d3d507914df1d82fbf30bc29e",
    auditSha256: "3c7d4feeb742a73505668c319c2489b47b203b61b90f3367d180d29a91021a03",
    reducerSha256: "f0f08a9f621620fe5006c7b941fbe35641f1d1c99a7dd372544b952ccca9533f",
    rawArchiveSha256: "af4751cbc88b626b7a8a9c200ae055069ea45117d2276d198bf54df4b5fca716",
    replayArchiveSha256: "b2197021e895393c8e6b5b9abe847d29cd640ad420378d70fc7ee5cd5bc8b43d",
    runs: ["synchronous-a1", "ordered-b1", "ordered-b2", "synchronous-a2"],
    repetitionsPerMode: 2, rawFiles: 40, outputs: 128, packets: 83139, batches: 135, cursor: 255,
    synchronous: { ttftSeconds: 3.1265115569999997, tpotSeconds: 0.23055936558267717,
      outputTokensPerSecond: 3.9498842931270257, workloadSeconds: 32.407551031,
      tpotRangeSeconds: [0.22896383051181105, 0.2321549006535433],
      outputRateRange: [3.9226694139363847, 3.9770991723176667] },
    ordered: { ttftSeconds: 2.848069924, tpotSeconds: 0.19492733181496064,
      outputTokensPerSecond: 4.643287102223024, workloadSeconds: 27.603841144500002,
      tpotRangeSeconds: [0.18728891182677168, 0.20256575180314962],
      outputRateRange: [4.472915377847863, 4.813658826598184] },
    ttftReductionPercent: 8.905824524351825, tpotReductionPercent: 15.454602625951065,
    meanOutputRateGainPercent: 17.555015733057044,
    pairedTpotReductionPercent: [18.20152929477199, 12.745433659637062],
    pairedOutputRateGainPercent: [21.034417751091915, 14.027334599152685],
    clock: "controller-std-instant", outputRateAggregation: "arithmetic-mean-of-per-run-rates",
    comparisonAggregation: "ratio-of-arithmetic-means", qualificationTimingsIncluded: false,
    oldCohortMixed: false, additiveGainClaim: false, operationGroupComparison: false,
  },
  state: "host-native-and-descriptive-abba-passed",
  descriptiveComparisonAdmitted: true, stablePerformanceGainClaimed: false,
  operationGroupComparison: false, authority: "none", httpMeasurement: false,
  gpuClockMeasurement: false, confidenceQualified: false, competitiveRanking: false,
  defaultPromotion: false, newVerusProof: false, m1Complete: false,
};

export function validateSubmissionCheckpoint(value) {
  assert.deepEqual(JSON.parse(JSON.stringify(value)), expected);
}

export function testSubmissionCheckpointRejections(value) {
  let mutations = 0;
  function visit(node, path = []) {
    for (const [key, child] of Object.entries(node)) {
      const next = [...path, key];
      if (child !== null && typeof child === "object") visit(child, next);
      else {
        const changed = JSON.parse(JSON.stringify(value));
        let parent = changed;
        for (const part of next.slice(0, -1)) parent = parent[part];
        parent[key] = typeof child === "boolean" ? !child : "mutated";
        assert.throws(() => validateSubmissionCheckpoint(changed));
        mutations++;
      }
    }
  }
  visit(value);
  for (const path of [[], ["host"], ["qualification"]]) {
    const changed = JSON.parse(JSON.stringify(value));
    let node = changed;
    for (const key of path) node = node[key];
    node.unreviewed = true;
    assert.throws(() => validateSubmissionCheckpoint(changed));
  }
  const changed = JSON.parse(JSON.stringify(value));
  changed.qualification.cases.reverse();
  assert.throws(() => validateSubmissionCheckpoint(changed));
  console.log(`PASS: submission checkpoint rejects ${mutations} scalar mutations and closed-roster changes.`);
}

export async function validateSubmissionEvidence(root, value) {
  validateSubmissionCheckpoint(value);
  async function pinned(file, digest, parse = true) {
    const raw = await readFile(join(root, file));
    assert(raw.length > 0 && raw.length <= 8 * 1024 * 1024, "bounded evidence file");
    assert.equal(createHash("sha256").update(raw).digest("hex"), digest, file);
    return parse ? JSON.parse(raw) : raw;
  }
  const host = (await pinned("HOST-GATE.md", value.host.noteSha256, false)).toString("utf8");
  for (const fact of [value.source, value.tree, value.controllerSha256, value.host.sourceLedgerSha256,
    value.host.archiveSha256, "All 51 actual command exits", "745 passed invocations,38 ignored,18 result rows",
    "doctests8, source-gate38", "source policy31", "All104", "contains1,073 files",
    "warm", "All five generated inventories matched", "without inline fixes or retries"])
    assert(host.includes(fact), fact);
  const log = (await pinned("host-gate-run-r1.log",
    "b4b971ac5bf2d2ff3da9674247af0a349d99b0df329fc166e5937a9cd852969a", false)).toString("utf8");
  assert.equal([...log.matchAll(/^[a-z0-9-]+: exit=0; stage=\d+ KiB$/gm)].length, value.host.commandsPassed);
  await pinned("PLAN.md", value.planSha256, false);
  const q = value.qualification;
  await pinned("check_wave_argmax_submission.py", q.checkerSha256, false);
  await pinned("run_wave_argmax_submission.py", q.wrapperSha256, false);
  const note = (await pinned("qualification/QUALIFICATION.md", q.noteSha256, false)).toString("utf8").replace(/\s+/g, " ");
  for (const fact of [value.source, value.tree, value.controllerSha256, value.planSha256,
    q.archiveSha256, q.rosterSha256, q.sizesSha256, "All four predeclared correctness cases passed",
    "normal unforced", "timings are excluded", "No comparison", "all40 raw files"])
    assert(note.includes(fact), fact);
  const rosterBytes = await pinned("qualification/qualification-local.sha256", q.rosterSha256, false);
  assert.deepEqual(await pinned("qualification/qualification-remote.sha256", q.rosterSha256, false), rosterBytes);
  const sizeBytes = await pinned("qualification/qualification-local.sizes", q.sizesSha256, false);
  assert.deepEqual(await pinned("qualification/qualification-remote.sizes", q.sizesSha256, false), sizeBytes);
  const roster = rosterBytes.toString("utf8").trim().split("\n").map((line) => {
    const match = /^([a-f0-9]{64})  (\S+)$/.exec(line);
    assert(match, "closed hash roster line");
    return [match[2], match[1]];
  });
  const sizes = sizeBytes.toString("utf8").trim().split("\n").map((line) => {
    const match = /^([1-9][0-9]*) (\S+)$/.exec(line);
    assert(match, "closed size roster line");
    return [match[2], Number(match[1])];
  });
  const names = ["results.jsonl", "host-timing.json", "wrapper-result.json", "prelaunch.json",
    "owned-process.json", "group-cleanup.json", "wall-clock.json", "resources-before.json", "gpu-before.json", "gpu-after.json"];
  const expectedNames = q.cases.flatMap(([id]) => names.map((name) => `${id}/${name}`));
  assert.equal(expectedNames.length, q.rawFiles);
  assert.deepEqual(roster.map(([name]) => name), expectedNames);
  assert.deepEqual(sizes.map(([name]) => name), expectedNames);
  const raw = new Map();
  for (const [index, [name, digest]] of roster.entries()) {
    const bytes = await pinned(`qualification/${name}`, digest, false);
    assert.equal(bytes.length, sizes[index][1]);
    raw.set(name, bytes);
  }
  const reference = await pinned("target-reference.json", q.referenceSha256);
  const prefix = await pinned("prefix-reference.json", q.prefixReferenceSha256);
  assert.equal(reference.schema, "FerricMatched128ReferenceV1");
  assert.equal(prefix.source_reference_sha256, q.referenceSha256);
  assert.deepEqual(prefix.prompt_token_ids, reference.prompt_token_ids);
  const devices = ["0xe3233d81d822f3eb", "0x966895650c2e8ae1", "0xd4e5658294b0b967", "0xf695011eb05a2497",
    "0x33231df6bb92857", "0x80a9a2ba09978a65", "0x92050148915dd40c", "0x6ad88437269ef781"];
  for (const [id, mode, outputs, packets, batches, cursor] of q.cases) {
    const json = (name) => JSON.parse(raw.get(`${id}/${name}`));
    const text = raw.get(`${id}/results.jsonl`).toString("utf8");
    assert(text.endsWith("\n"));
    const records = text.trim().split("\n").map((line) => JSON.parse(line));
    assert.equal(records.length, 4);
    const [setup, , observation, closed] = records;
    const wrapper = json("wrapper-result.json");
    const checked = wrapper.checked_trace;
    const prelaunch = json("prelaunch.json");
    const timing = json("host-timing.json");
    for (const [record, suffix] of [[setup, "Setup"], [observation, "Observation"], [closed, "Closed"], [checked, "Checked"]])
      assert.equal(record.schema, `FerricWaveArgmaxSubmissionCanary${suffix}V1`);
    assert.equal(wrapper.schema, "FerricWaveArgmaxSubmissionModelWrapperV1");
    assert.equal(prelaunch.schema, "FerricWaveArgmaxSubmissionModelPrelaunchV1");
    assert.equal(prelaunch.controller_source, value.source);
    assert.equal(prelaunch.pins[prelaunch.argv[0]], value.controllerSha256);
    for (const [file, digest] of [["check_wave_argmax_submission.py", q.checkerSha256], ["run_wave_argmax_submission.py", q.wrapperSha256]])
      assert.equal(prelaunch.pins[`/tmp/ferric-compete-gpu.VabkOGCx/wave-argmax-submission-wrapper-v1/${file}`], digest);
    for (const [flag, expectedValue] of [["--submission", mode], ["--attention", q.attention],
      ["--argmax-mode", q.argmax], ["--max-new-tokens", String(outputs)]])
      assert.equal(prelaunch.argv[prelaunch.argv.indexOf(flag) + 1], expectedValue);
    assert.deepEqual(setup.prompt_token_ids, reference.prompt_token_ids);
    assert.equal(setup.prompt_token_ids.length, q.inputTokens);
    for (const [field, expectedValue] of Object.entries({ controller_sha256: value.controllerSha256,
      context: q.context, pages: q.pages, row_capacity: q.capacity, prefill_chunk: q.prefillChunk,
      attention: q.attention, argmax_mode: q.argmax, max_new_tokens: outputs,
      expected_packets: packets, expected_batches: batches, runtime_ordered_batches: mode === "ordered",
      prefix_cache: false, source_reference_sha256: q.referenceSha256, prefix_reference_sha256: q.prefixReferenceSha256 }))
      assert.equal(setup[field], expectedValue, `${id}: ${field}`);
    for (const [role, sha] of [["target_artifact", "98b5fdb14ac7242e324e1801e1885c98e77473d622b2e9b3f9f12f14c7d75502"],
      ["target_head_artifact", "5f19b3ba59035a5f0ebc90cdf3a40466f9910908a45e082d146cb674028da6cb"],
      ["argmax_artifact", "de9db78c0ef7ad5d84fc903d41ee9db59026113d79e23f12d3909761363b9390"]])
      assert.equal(setup[role].hsaco, sha);
    for (const result of [observation, checked]) {
      assert.equal(result.reference_passed, true);
      assert.deepEqual(result.generated_token_ids, reference.generated_token_ids.slice(0, outputs));
      assert.equal(result.generated_utf8_hex, outputs === 128 ? reference.generated_utf8_hex : prefix.prefix_utf8_hex[7]);
      assert.equal(result.completed_packets, packets);
      assert.equal(result.completed_batches, batches);
      assert.equal(result.committed_inputs_before_retirement, cursor);
    }
    assert.equal(observation.pool_retired, true);
    assert.equal(checked.controller_sha256, value.controllerSha256);
    assert.equal(checked.attention, q.attention);
    assert.equal(checked.argmax_mode, q.argmax);
    assert.equal(checked.submission, mode);
    assert.equal(checked.submission_host.attention_groups_measure,
      mode === "ordered" ? "command-preparation-only" : "submit-and-wait");
    for (const record of [setup, observation, closed, checked, wrapper, prelaunch]) {
      assert.equal(record.authority, "none");
      assert.equal(record.performance_qualified, false);
    }
    for (const flag of ["speculative_serving", "http_measurement", "gpu_clock_measurement"]) assert.equal(checked[flag], false);
    assert.equal(wrapper.passed, true);
    assert.equal(wrapper.serving_qualified, false);
    assert.equal(wrapper.submission, mode);
    assert.equal(wrapper.attention, q.attention);
    assert.equal(wrapper.max_new_tokens, outputs);
    assert.deepEqual(wrapper.errors, []);
    assert.equal(wrapper.all_eight_idle_before, true);
    assert.equal(wrapper.all_eight_idle_after, true);
    assert.equal(closed.execution_completed, true);
    assert.equal(closed.reference_passed, true);
    assert.equal(closed.worker_exited, true);
    assert.equal(closed.completed_batches, batches);
    assert.deepEqual(closed.completed_packets, [packets]);
    assert(Number.isSafeInteger(setup.worker_pid) && setup.worker_pid > 0);
    assert.equal(closed.worker_pid, setup.worker_pid);
    assert.equal(checked.worker_pid, setup.worker_pid);
    assert.deepEqual(timing.setup, setup);
    assert.deepEqual(timing.closed, closed);
    const process = json("owned-process.json");
    assert.equal(process.pid, timing.controller_pid);
    assert.equal(process.pgid, process.pid);
    assert(Number.isSafeInteger(process.pid) && process.pid > 0);
    assert.deepEqual(json("group-cleanup.json"), { absent: true, cleanup_error: null,
      controller_returncode: 0, forced: false, reason: null });
    for (const name of ["gpu-before.json", "gpu-after.json"]) {
      const cards = json(name);
      assert.deepEqual(Object.keys(cards).sort(), devices.map((_, index) => `card${index}`));
      devices.forEach((device, index) => {
        const card = cards[`card${index}`];
        assert.equal(card["Unique ID"], device);
        for (const field of ["GPU use (%)", "GPU Memory Allocated (VRAM%)", "GPU Memory Read/Write Activity (%)"])
          assert.equal(card[field], "0");
      });
    }
  }
  const a = value.abba;
  const summary = await pinned("abba/summary.json", a.summarySha256);
  const manifest = await pinned("abba/manifest.json", a.manifestSha256);
  const replay = await pinned("abba/replay.json", a.replaySha256);
  const before = await pinned("abba/before.json", a.auditSha256, false);
  assert.deepEqual(await pinned("abba/after.json", a.auditSha256, false), before);
  await pinned("abba/summarize_wave_submission.py", a.reducerSha256, false);
  assert.equal(summary.schema, "FerricWaveSubmissionAbbaSummaryV1");
  assert.equal(manifest.schema, "FerricWaveSubmissionAbbaEvidenceV1");
  assert.equal(replay.schema, "FerricWaveSubmissionReplayGateV1");
  assert.equal(replay.passed, true);
  assert.deepEqual(replay.errors, []);
  assert.deepEqual(replay.commands.map((command) => command.exit_code), [0, 1]);
  assert.equal(summary.evidence_manifest_sha256, a.manifestSha256);
  assert.equal(summary.experiment_plan_sha256, value.planSha256);
  assert.equal(summary.reducer_sha256, a.reducerSha256);
  assert.equal(summary.checker_sha256, q.checkerSha256);
  assert.equal(summary.wrapper_sha256, q.wrapperSha256);
  assert.deepEqual(summary.controller, manifest.controller);
  assert.equal(summary.controller.source, value.source);
  assert.equal(summary.controller.tree, value.tree);
  assert.equal(summary.controller.sha256, value.controllerSha256);
  for (const item of [summary, manifest, replay]) {
    assert.equal(item.authority, "none");
    assert.equal(item.performance_qualified, false);
  }
  for (const flag of ["serving_qualified", "http_measurement", "gpu_clock_measurement", "confidence_qualified",
    "competitive_ranking", "default_promotion", "m1_completion", "new_verus_proof", "old_cohort_mixed",
    "additive_gain_claim", "stable_performance_gain_claimed", "operation_group_cross_mode_comparison"])
    assert.equal(summary[flag], false);
  assert.deepEqual(summary.runs.map((run) => run.id), a.runs);
  assert.deepEqual(manifest.runs.map((run) => run.id), a.runs);
  assert(summary.stage_accounting.includes("no cross-mode group speed ratios"));
  const close = (actual, expectedValue) => {
    assert(Number.isFinite(actual) && Number.isFinite(expectedValue));
    assert(Math.abs(actual - expectedValue) <= 1e-10 * Math.max(1, Math.abs(expectedValue)),
      `arithmetic mismatch: ${actual} versus ${expectedValue}`);
  };
  const metrics = { ttftSeconds: "diagnostic_ttft_seconds", tpotSeconds: "diagnostic_tpot_seconds",
    workloadSeconds: "diagnostic_workload_seconds", outputTokensPerSecond: "diagnostic_output_tokens_per_second" };
  const perRun = [], sessions = new Set();
  let previousEnd = 0;
  for (const [index, item] of summary.runs.entries()) {
    const mode = ["synchronous", "ordered", "ordered", "synchronous"][index];
    const suffix = ["a1", "b1", "b2", "a2"][index];
    const directory = `wave-submission-7cb6522-${mode}-128-${suffix}`;
    const entry = manifest.runs[index];
    assert.equal(entry.directory, directory);
    assert.deepEqual(Object.keys(entry.files).sort(), [...names].sort());
    assert.deepEqual(item.evidence_files, entry.files);
    const bytes = new Map();
    for (const name of names) {
      const pin = entry.files[name];
      assert.deepEqual(Object.keys(pin).sort(), ["bytes", "sha256"]);
      const file = await pinned(`abba/${directory}/${name}`, pin.sha256, false);
      assert.equal(file.length, pin.bytes);
      bytes.set(name, file);
    }
    const records = bytes.get("results.jsonl").toString("utf8").trim().split("\n").map((line) => JSON.parse(line));
    assert.equal(records.length, 4);
    const [setup, , observation, closed] = records;
    const wrapper = JSON.parse(bytes.get("wrapper-result.json"));
    const checked = item.checked_trace;
    assert.deepEqual(checked, wrapper.checked_trace);
    assert.equal(item.submission, mode);
    assert.equal(item.outputs, a.outputs);
    assert.equal(item.repetition, index < 2 ? 1 : 2);
    assert.equal(item.attention, q.attention);
    assert.equal(item.argmax_mode, q.argmax);
    assert.equal(setup.controller_sha256, value.controllerSha256);
    assert.equal(setup.runtime_ordered_batches, mode === "ordered");
    assert.equal(setup.attention, q.attention);
    assert.equal(setup.argmax_mode, q.argmax);
    assert.equal(setup.context, q.context);
    assert.deepEqual(setup.prompt_token_ids, reference.prompt_token_ids);
    for (const result of [observation, checked]) {
      assert.equal(result.reference_passed, true);
      assert.deepEqual(result.generated_token_ids, reference.generated_token_ids);
      assert.equal(result.generated_utf8_hex, reference.generated_utf8_hex);
      assert.equal(result.completed_packets, a.packets);
      assert.equal(result.completed_batches, a.batches);
      assert.equal(result.committed_inputs_before_retirement, a.cursor);
    }
    assert.equal(observation.pool_retired, true);
    assert.equal(closed.worker_exited, true);
    assert.equal(closed.worker_pid, checked.worker_pid);
    assert.equal(wrapper.passed, true);
    assert.deepEqual(wrapper.errors, []);
    assert.equal(wrapper.all_eight_idle_before, true);
    assert.equal(wrapper.all_eight_idle_after, true);
    assert.equal(checked.submission_host.attention_groups_measure,
      mode === "ordered" ? "command-preparation-only" : "submit-and-wait");
    assert.deepEqual(JSON.parse(bytes.get("group-cleanup.json")), { absent: true, cleanup_error: null,
      controller_returncode: 0, forced: false, reason: null });
    const offsets = observation.elapsed_seconds;
    assert.equal(offsets.length, a.outputs);
    offsets.forEach((offset, i) => assert(Number.isFinite(offset) && offset > (i ? offsets[i - 1] : 0)));
    const derived = { ttftSeconds: offsets[0], tpotSeconds: (offsets.at(-1) - offsets[0]) / (a.outputs - 1),
      workloadSeconds: observation.workload_seconds, outputTokensPerSecond: a.outputs / observation.workload_seconds };
    assert(Number.isFinite(derived.workloadSeconds) && derived.workloadSeconds >= offsets.at(-1));
    for (const [name, metric] of Object.entries(metrics)) close(checked[metric], derived[name]);
    assert.equal(item.session, setup.session);
    assert(!sessions.has(item.session));
    sessions.add(item.session);
    const wall = JSON.parse(bytes.get("wall-clock.json"));
    assert.deepEqual(item.child_wall, wall);
    // Python replay and pinned bytes retain exact u64/ns custody; JS checks coarse chronology only.
    assert(wall.started_unix_ns > previousEnd && wall.completed_unix_ns > wall.started_unix_ns);
    previousEnd = wall.completed_unix_ns;
    perRun.push({ mode, ...derived });
  }
  const cohort = summary.cohort;
  assert.equal(cohort.repetitions_per_submission, a.repetitionsPerMode);
  assert.equal(cohort.inference, "descriptive n=2 only; no confidence interval or statistical qualification");
  assert.deepEqual(Object.keys(cohort.ratio_of_arithmetic_means).sort(), Object.values(metrics).sort());
  assert(!Object.hasOwn(cohort, "stage_ratios"));
  for (const mode of ["synchronous", "ordered"]) {
    for (const [field, metric] of Object.entries(metrics)) {
      const values = perRun.filter((run) => run.mode === mode).map((run) => run[field]);
      const mean = values.reduce((total, x) => total + x, 0) / values.length;
      close(a[mode][field], mean);
      close(cohort.arithmetic_metric_means[mode][metric], mean);
      const range = cohort.within_submission_variability[mode][metric];
      assert.equal(range.n, 2);
      range.values.forEach((x, i) => close(x, values[i]));
      close(range.minimum, Math.min(...values));
      close(range.maximum, Math.max(...values));
      if (field === "tpotSeconds" || field === "outputTokensPerSecond") {
        const displayed = a[mode][field === "tpotSeconds" ? "tpotRangeSeconds" : "outputRateRange"];
        close(displayed[0], Math.min(...values));
        close(displayed[1], Math.max(...values));
      }
    }
  }
  for (const [field, metric] of Object.entries(metrics)) {
    const synchronous = a.synchronous[field], ordered = a.ordered[field];
    const higher = field === "outputTokensPerSecond";
    const ratio = higher ? ordered / synchronous : synchronous / ordered;
    const gain = 100 * (higher ? ordered / synchronous - 1 : 1 - ordered / synchronous);
    close(cohort.ratio_of_arithmetic_means[metric].speed_ratio, ratio);
    close(cohort.ratio_of_arithmetic_means[metric].improvement_percent, gain);
    const displayed = { ttftSeconds: "ttftReductionPercent", tpotSeconds: "tpotReductionPercent",
      outputTokensPerSecond: "meanOutputRateGainPercent" }[field];
    if (displayed) close(a[displayed], gain);
    for (const [pairIndex, [syncIndex, orderedIndex]] of [[0, 1], [3, 2]].entries()) {
      const sync = perRun[syncIndex][field], orderedValue = perRun[orderedIndex][field];
      const pair = cohort.repetition_pairs[pairIndex];
      assert.equal(pair.synchronous_id, a.runs[syncIndex]);
      assert.equal(pair.ordered_id, a.runs[orderedIndex]);
      close(pair.relative_changes[metric].speed_ratio, higher ? orderedValue / sync : sync / orderedValue);
      const pairedGain = 100 * (higher ? orderedValue / sync - 1 : 1 - orderedValue / sync);
      close(pair.relative_changes[metric].improvement_percent, pairedGain);
      if (field === "tpotSeconds") close(a.pairedTpotReductionPercent[pairIndex], pairedGain);
      if (higher) close(a.pairedOutputRateGainPercent[pairIndex], pairedGain);
    }
  }
  console.log("PASS: exact submission host gate, four native correctness cases and separate forty-file full128 ABBA means/ranges/pairs; no cross-mode group comparison or stable/HTTP/GPU claim.");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const context = { window: {} };
  vm.runInNewContext(await readFile(join(dirname(fileURLToPath(import.meta.url)), "data/project.js"), "utf8"), context);
  const value = context.window.FERRIC_PROJECT.submissionCheckpoint;
  validateSubmissionCheckpoint(value);
  testSubmissionCheckpointRejections(value);
  if (process.argv[2]) await validateSubmissionEvidence(process.argv[2], value);
}
