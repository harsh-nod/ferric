import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { lstat, readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const expected = {
  schema: "FerricPagesArgmaxNativeV1",
  summarySha256: "064e1ced84a0ac4e275225bcc9243a161462c19f907f9ca2080048daafc7eaab",
  manifestSha256: "d7efd64eb1aad2f55e5639b97fdae52b9bd60e2a75c56d5c4a346c12fd3448f1",
  planSha256: "5781935439ab23f4e234135ab14fa4ea425e71d210919980afe6de46a6f10c79",
  reducerSha256: "d112d5113657867417bc8358ddbbf19a2c1a918a658e146c0522b1860b7863d7",
  checkerSha256: "eb7a5f7d243675fac25c318d640e50c6a5aad1d39b7bc3eedefc5f5316964a76",
  wrapperSha256: "186e2de9c64a278d2e16458ca4e24b0f8eea2b0ed20fec73e460634df7ab2daf",
  source: "d9a2705e6b2f3a8d8dd173b9700a63446b611016",
  tree: "523833a9a3218dff216b05429460ec68acad2b3f",
  controllerSha256: "d298c5ea6b0dfd64c66722b12ffba1f5e8498bf215b15b9ca6d0d88da56f890f",
  runsPassed: 6, rawFilesAuthenticated: 60, exactIdsAndUtf8: true,
  normalWorkerCloses: true, allEightIdleAfter: true,
  inputTokens: 128, tp: 1, context: 256, pages: 16, prefillChunk: 16,
  attention: "baseline", ordered: false, prefixCaching: false,
  clock: "controller-std-instant", aggregation: "ratio-of-arithmetic-means",
  full128: {
    outputs: 128, repetitionsPerMode: 1, packets: 83139, batches: 135, cursor: 255,
    serial: { ttftSeconds: 4.18202292, tpotSeconds: 0.5534195468897639, outputTokensPerSecond: 1.7188982218505306 },
    wave: { ttftSeconds: 3.966503252, tpotSeconds: 0.528962031488189, outputTokensPerSecond: 1.7991506557805377 },
    tpotReductionPercent: 4.419344336322595, outputRateGainPercent: 4.668829888229742,
  },
  short8: {
    outputs: 8, repetitionsPerMode: 2, packets: 9219, batches: 15, cursor: 135,
    serial: { ttftSeconds: 4.3379974820000005, tpotSeconds: 0.4892928130714286, outputTokensPerSecond: 1.0327211131918654 },
    wave: { ttftSeconds: 4.03498096, tpotSeconds: 0.42817899171428564, outputTokensPerSecond: 1.1376929437114325 },
    tpotReductionPercent: 12.490234829633874, outputRateGainPercent: 10.164586467601811,
    pairedTpotReductionPercent: [16.792596746663612, 7.723137419584891],
  },
  authority: "none", performanceQualified: false, httpMeasurement: false,
  gpuClockMeasurement: false, confidenceQualified: false, competitiveRanking: false,
  defaultPromotion: false, speculativeServing: false, exclusiveHostStages: false,
};
const plain = (value) => JSON.parse(JSON.stringify(value));
const close = (actual, wanted) => {
  assert(Number.isFinite(actual) && Number.isFinite(wanted));
  assert(Math.abs(actual - wanted) <= 1e-12 * Math.max(1, Math.abs(wanted)), `${actual} != ${wanted}`);
};
const metrics = {
  ttftSeconds: "diagnostic_ttft_seconds", tpotSeconds: "diagnostic_tpot_seconds",
  outputTokensPerSecond: "diagnostic_output_tokens_per_second",
};
const ids = ["full128-serial-r1", "full128-wave-r1", "short8-serial-r1",
  "short8-wave-r1", "short8-wave-r2", "short8-serial-r2"];
const rawNames = ["gpu-after.json", "gpu-before.json", "group-cleanup.json", "host-timing.json",
  "owned-process.json", "prelaunch.json", "resources-before.json", "results.jsonl", "wall-clock.json", "wrapper-result.json"];

export function validateArgmaxNative(value) {
  assert.deepEqual(plain(value), expected, "native argmax diagnostic identity or scope drifted");
}

export function testArgmaxNativeRejections(value) {
  let rejected = 0;
  const visit = (node, path = []) => {
    for (const [key, child] of Object.entries(node)) {
      const fields = [...path, key];
      const changed = plain(value);
      const parent = path.reduce((part, field) => part[field], changed);
      delete parent[key];
      assert.throws(() => validateArgmaxNative(changed));
      rejected += 1;
      if (child !== null && typeof child === "object") visit(child, fields);
      else {
        parent[key] = typeof child === "boolean" ? !child : `${child}-changed`;
        assert.throws(() => validateArgmaxNative(changed));
        rejected += 1;
      }
    }
  };
  visit(expected);
  assert.throws(() => validateArgmaxNative({ ...value, confidence: "qualified" }));
  console.log(`Native argmax diagnostic: ${rejected + 1} mutations rejected.`);
}

async function pinnedFile(root, file, digest) {
  const path = join(root, file);
  const stat = await lstat(path);
  assert(stat.isFile() && !stat.isSymbolicLink() && stat.size > 0 && stat.size <= 8 * 1024 * 1024);
  const raw = await readFile(path);
  assert.equal(createHash("sha256").update(raw).digest("hex"), digest, file);
  return raw;
}

export async function validateArgmaxNativeEvidence(root, value) {
  validateArgmaxNative(value);
  const summary = JSON.parse(await pinnedFile(root, "native-summary-r1.json", value.summarySha256));
  const manifest = JSON.parse(await pinnedFile(root, "evidence-manifest.json", value.manifestSha256));
  await pinnedFile(root, "PLAN.md", value.planSha256);
  await pinnedFile(root, "summarize_argmax.py", value.reducerSha256);
  assert.equal(summary.schema, "FerricArgmaxSixRunSummaryV1");
  for (const item of [summary, manifest]) {
    assert.equal(item.authority, "none");
    assert.equal(item.performance_qualified, false);
    assert.equal(item.controller.source, value.source);
    assert.equal(item.controller.tree, value.tree);
    assert.equal(item.controller.sha256, value.controllerSha256);
    assert.equal(item.checker_sha256, value.checkerSha256);
    assert.equal(item.wrapper_sha256, value.wrapperSha256);
    assert.equal(item.expectations_sha256, value.planSha256);
    assert.deepEqual(item.runs.map((row) => row.id), ids);
  }
  for (const field of ["http_measurement", "gpu_clock_measurement", "confidence_qualified", "competitive_ranking", "default_promotion"])
    assert.equal(summary[field], false);
  assert.equal(summary.stage_accounting,
    "nested host scopes and IPC categories overlap; do not add them into elapsed workload or GPU duration");
  let authenticated = 0;
  const firstByCohort = new Map();
  let artifacts;
  for (let index = 0; index < summary.runs.length; index += 1) {
    const run = summary.runs[index];
    const entry = manifest.runs[index];
    const cohort = value[run.cohort];
    const modeName = run.mode === "serial" ? "serial" : "wave";
    assert.equal(entry.directory, `argmax-model-d9a2705-${modeName}-${run.outputs}-r${run.repetition}`);
    const directory = join(root, entry.directory);
    const stat = await lstat(directory);
    assert(stat.isDirectory() && !stat.isSymbolicLink());
    assert.deepEqual(Object.keys(entry.files).sort(), rawNames);
    assert.deepEqual(run.evidence_sha256, entry.files);
    const raw = {};
    for (const file of rawNames) {
      const bytes = await pinnedFile(directory, file, entry.files[file]);
      raw[file] = file.endsWith(".jsonl")
        ? bytes.toString("utf8").trim().split("\n").map((line) => JSON.parse(line)) : JSON.parse(bytes);
      authenticated += 1;
    }
    const checked = run.checked_trace;
    const wrapper = raw["wrapper-result.json"];
    assert.deepEqual(wrapper.checked_trace, checked);
    for (const field of ["passed", "all_eight_idle_before", "all_eight_idle_after"]) assert.equal(wrapper[field], true);
    assert.equal(wrapper.performance_qualified, false);
    assert.equal(wrapper.serving_qualified, false);
    assert.deepEqual(wrapper.errors, []);
    assert.deepEqual(raw["group-cleanup.json"], {
      absent: true, cleanup_error: null, controller_returncode: 0, forced: false, reason: null,
    });
    assert.equal(checked.reference_passed, true);
    assert.equal(checked.completed_packets, cohort.packets);
    assert.equal(checked.completed_batches, cohort.batches);
    assert.equal(checked.committed_inputs_before_retirement, cohort.cursor);
    assert.equal(checked.generated_token_ids.length, cohort.outputs);
    assert.equal(checked.argmax_mode, run.mode);
    assert.equal(checked.controller_sha256, value.controllerSha256);
    assert.equal(checked.host_timing.clock, value.clock);
    assert.equal(checked.host_timing.aggregation, "overlapping-not-additive");
    for (const field of ["performance_qualified", "http_measurement", "gpu_clock_measurement", "speculative_serving"])
      assert.equal(checked[field], false);
    const first = firstByCohort.get(run.cohort) ?? checked;
    assert.deepEqual(checked.generated_token_ids, first.generated_token_ids);
    assert.equal(checked.generated_utf8_hex, first.generated_utf8_hex);
    firstByCohort.set(run.cohort, first);
    const setup = raw["results.jsonl"][0];
    const closed = raw["results.jsonl"].at(-1);
    assert.equal(setup.schema, "FerricArgmaxCanarySetupV1");
    for (const [key, field] of Object.entries({ context: "context", pages: "pages", prefill_chunk: "prefillChunk",
      attention: "attention", runtime_ordered_batches: "ordered", prefix_cache: "prefixCaching" }))
      assert.equal(setup[key], value[field]);
    assert.equal(setup.prompt_token_ids.length, value.inputTokens);
    assert.equal(setup.collective, "device-tp1-v3");
    assert.equal(setup.head_precision, "fp32-v8");
    assert.equal(setup.projection, "mfma");
    const loaded = [setup.target_artifact, setup.target_head_artifact, setup.argmax_artifact];
    artifacts ??= loaded;
    assert.deepEqual(loaded, artifacts);
    assert.equal(closed.schema, "FerricArgmaxCanaryClosedV1");
    for (const field of ["execution_completed", "reference_passed", "worker_exited"]) assert.equal(closed[field], true);
  }
  assert.equal(authenticated, value.rawFilesAuthenticated);
  for (const name of ["full128", "short8"]) {
    const displayed = value[name];
    const reduced = summary.cohorts[name];
    assert.equal(reduced.repetitions_per_mode, displayed.repetitionsPerMode);
    for (const [label, mode] of [["serial", "serial"], ["wave", "wave-v11"]]) {
      const selected = summary.runs.filter((run) => run.cohort === name && run.mode === mode);
      assert.equal(selected.length, displayed.repetitionsPerMode);
      for (const [field, metric] of Object.entries(metrics)) {
        const mean = selected.reduce((sum, run) => sum + run.checked_trace[metric], 0) / selected.length;
        close(mean, displayed[label][field]);
        close(mean, reduced.arithmetic_metric_means[mode][metric]);
      }
    }
    close(100 * (1 - displayed.wave.tpotSeconds / displayed.serial.tpotSeconds), displayed.tpotReductionPercent);
    close(100 * (displayed.wave.outputTokensPerSecond / displayed.serial.outputTokensPerSecond - 1), displayed.outputRateGainPercent);
    close(reduced.ratio_of_arithmetic_means.diagnostic_tpot_seconds.improvement_percent, displayed.tpotReductionPercent);
    close(reduced.ratio_of_arithmetic_means.diagnostic_output_tokens_per_second.improvement_percent, displayed.outputRateGainPercent);
  }
  summary.cohorts.short8.repetition_pairs.forEach((pair, index) => {
    const serial = summary.runs.find((run) => run.id === pair.serial_id).checked_trace;
    const wave = summary.runs.find((run) => run.id === pair.wave_id).checked_trace;
    close(100 * (1 - wave.diagnostic_tpot_seconds / serial.diagnostic_tpot_seconds), value.short8.pairedTpotReductionPercent[index]);
  });
  console.log("PASS: pinned native summary, manifest and all 60 raw files; six exact-output clean runs and noncompetitive host means.");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const context = { window: {} };
  vm.runInNewContext(await readFile(join(dirname(fileURLToPath(import.meta.url)), "data/project.js"), "utf8"), context);
  const value = context.window.FERRIC_PROJECT.argmaxNative;
  validateArgmaxNative(value);
  testArgmaxNativeRejections(value);
  if (process.argv[2]) await validateArgmaxNativeEvidence(process.argv[2], value);
}
