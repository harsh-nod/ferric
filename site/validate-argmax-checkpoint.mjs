import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const expected = {
  schema: "FerricPagesArgmaxCheckpointV1",
  source: "d9a2705e6b2f3a8d8dd173b9700a63446b611016",
  fe2o3: "8efd4fd416d1ffae7a718144e4d299fe3c8f7590",
  controllerSha256: "d298c5ea6b0dfd64c66722b12ffba1f5e8498bf215b15b9ca6d0d88da56f890f",
  hostReceiptSha256: "497de7d70a785c39d99f9b919e7906542896928e27559fad31bb0f9f7e7df861",
  hostTests: 637, doctests: 8, sourceGateTests: 38, protectedPolicies: 31,
  unchangedInventories: 5, sourceFiles: 1059, remainingImageSkips: 28,
  explicitImageAndReferenceFixtures: 2,
  adapterIntegrated: true, optInOnly: true, defaultArgmax: "serial",
  nativeState: "passed-six-run-diagnostic", nativeMetrics: "argmaxNative",
  priorRejection: {
    source: "f662d541e1ab9a47ef8dada410fffd964851d297",
    wrapperSha256: "cf9543fec83c68a5d88cbd14b808896b87c5a8f47d31e55138e6e25b2cf488ee",
    traceSha256: "fd339708320124ad96e0a6201dd9108637b5d7a10c782adef53e1604c244a391",
    state: "retirement-harness-rejected", packets: 83139, batches: 135,
    observationPublished: false, parityAdmitted: false, timingAdmitted: false,
    workerClosed: true, allEightIdleAfter: true,
  },
  runtimeDiagnostic: {
    source: "f65c4603f936b3cf5d009991bc19a9645ecdb604",
    reportSha256: "d4ebc18f20fb1697237503a7f94b38ccadac4853b127157bf5e106466cdb91c8",
    wrapperSha256: "076c028176cec0390cb04c6562b7096ceef921bf4fa29a4638862d17ceaa7036",
    passed: true, packets: 3077, batches: 5, generatedTokens: 8,
    commandNs: 1358311936, operationalCurrentnessNs: 139591676,
    fullCurrentnessNs: 7309285, dispatchWaitNs: 1217955723,
    operationalToCommandPercent: 10.28,
    exclusiveDurations: false, gpuDurations: false, benchmarkAdmitted: false,
  },
  authority: "none", competitiveWin: false, newVerusProof: false, m1Complete: false,
};
const rows = [
  ["Native argmax: full 128-output pair", "observed", "n=1 per mode",
    "553.419547 / 528.962031 ms", "not a stable estimate, HTTP serving or GPU-clock timing"],
  ["Native argmax: short ABBA diagnostics", "observed", "n=2 per mode", "12.49% lower TPOT",
    "10.16% higher mean output rate", "16.79% to 7.72%", "are not additive",
    "not isolated GPU-kernel durations", "matched HTTP table is unchanged"],
  ["Opt-in argmax route: host gate passed", "integration", "637 adapter test invocations",
    "separate six-run native diagnostic passes", "No serving qualification or new Verus proof"],
  ["Retirement harness: rejected attempt preserved", "open", "never used",
    "no parity or timing result", "never be recategorized as a successful run"],
  ["Ordered runtime: diagnostic counters only", "observed", "10.28% ratio",
    "durations overlap", "not isolated GPU execution", "not an HTTP comparison or a new win"],
];
const plain = (value) => JSON.parse(JSON.stringify(value));

export function validateArgmaxCheckpoint(value, readiness) {
  assert.deepEqual(plain(value), expected, "argmax checkpoint identity or scope drifted");
  assert.equal(readiness.length, rows.length);
  rows.forEach(([label, state, ...phrases], index) => {
    assert.deepEqual(Object.keys(readiness[index]).sort(), ["detail", "label", "state"]);
    assert.equal(readiness[index].label, label);
    assert.equal(readiness[index].state, state);
    for (const phrase of phrases) assert(readiness[index].detail.includes(phrase), phrase);
  });
}

export function testArgmaxCheckpointRejections(value, readiness) {
  let mutations = 0;
  const visit = (object, path = []) => {
    for (const [key, child] of Object.entries(object)) {
      const next = [...path, key];
      if (child !== null && typeof child === "object") visit(child, next);
      else {
        const altered = plain(value);
        next.slice(0, -1).reduce((node, field) => node[field], altered)[key] =
          typeof child === "boolean" ? !child : String(child) + "-changed";
        assert.throws(() => validateArgmaxCheckpoint(altered, readiness));
        mutations += 1;
      }
    }
  };
  visit(expected);
  for (const key of Object.keys(expected)) {
    const altered = plain(value);
    delete altered[key];
    assert.throws(() => validateArgmaxCheckpoint(altered, readiness));
    mutations += 1;
  }
  rows.forEach(([, , ...phrases], index) => {
    for (const phrase of phrases) {
      const altered = plain(readiness);
      altered[index].detail = altered[index].detail.replace(phrase, "changed");
      assert.throws(() => validateArgmaxCheckpoint(value, altered));
      mutations += 1;
    }
  });
  assert.throws(() => validateArgmaxCheckpoint({ ...value, qualification: true }, readiness));
  console.log(`Argmax checkpoint: ${mutations + 1} mutations rejected.`);
}

export async function validateArgmaxCheckpointEvidence(root, value) {
  const pins = {
    "host-aggregate.json": value.hostReceiptSha256,
    "runtime-diagnostic.json": value.runtimeDiagnostic.reportSha256,
    "runtime-wrapper.json": value.runtimeDiagnostic.wrapperSha256,
    "rejected-wrapper.json": value.priorRejection.wrapperSha256,
    "rejected-results.jsonl": value.priorRejection.traceSha256,
  };
  const records = {};
  for (const [file, hash] of Object.entries(pins)) {
    const raw = await readFile(join(root, file));
    assert(raw.length > 0 && raw.length < 4 * 1024 * 1024);
    assert.equal(createHash("sha256").update(raw).digest("hex"), hash, file);
    records[file] = file.endsWith(".jsonl")
      ? raw.toString("utf8").trim().split("\n").map((line) => JSON.parse(line)) : JSON.parse(raw);
  }
  const host = records["host-aggregate.json"];
  assert.equal(host.source_commit, value.source);
  assert.equal(host.fe2o3_revision, value.fe2o3);
  assert.equal(host.binary.sha256, value.controllerSha256);
  assert.equal(host.source_files, value.sourceFiles);
  assert.equal(host.source_unchanged, true);
  assert.equal(host.steps.length, 29);
  assert(host.steps.every((step) => step.exit_code === 0));
  for (const [key, field] of Object.entries({ adapter_passed_invocations: "hostTests",
    doctests_passed: "doctests", source_gate_tests: "sourceGateTests",
    protected_verifier_source_policies: "protectedPolicies", generated_inventories_unchanged: "unchangedInventories",
    remaining_image_gated_ignored_invocations: "remainingImageSkips",
    explicit_ignored_fixtures_executed: "explicitImageAndReferenceFixtures" }))
    assert.equal(host.tests[key], value[field]);
  for (const key of ["gpu_execution", "verus_proof_execution", "native_model_success", "performance_qualified"])
    assert.equal(host[key], false);
  const rejected = records["rejected-wrapper.json"];
  assert.equal(rejected.passed, false);
  assert.equal(rejected.checked_trace, null);
  assert.equal(rejected.performance_qualified, false);
  assert.equal(rejected.all_eight_idle_after, true);
  const trace = records["rejected-results.jsonl"];
  assert(!trace.some((row) => row.schema === "FerricArgmaxCanaryObservationV1"));
  const closed = trace.find((row) => row.schema === "FerricArgmaxCanaryClosedV1");
  assert.equal(closed.execution_completed, false);
  assert.equal(closed.reference_passed, null);
  assert.equal(closed.worker_exited, true);
  assert.equal(closed.completed_batches, value.priorRejection.batches);
  assert.deepEqual(closed.completed_packets, [value.priorRejection.packets]);
  const report = records["runtime-diagnostic.json"];
  const wrapper = records["runtime-wrapper.json"];
  assert.equal(report.source_identity.controller, value.runtimeDiagnostic.source);
  for (const row of [report, wrapper]) {
    assert.equal(row.passed, true);
    assert.equal(row.authority, "none");
    assert.equal(row.performance_qualified, false);
  }
  assert.equal(wrapper.all_eight_idle_after, true);
  assert.equal(report.clean_teardown_recorded, true);
  assert.equal(report.fixed_reference_passed, true);
  assert.deepEqual(report.counts.rank_dispatch_counts, [value.runtimeDiagnostic.packets]);
  assert.equal(report.counts.batch_count, value.runtimeDiagnostic.batches);
  assert.equal(report.counts.generated_tokens, value.runtimeDiagnostic.generatedTokens);
  for (const [key, field] of Object.entries({command_ns: "commandNs", operational_currentness_ns: "operationalCurrentnessNs",
    full_currentness_ns: "fullCurrentnessNs", dispatch_wait_ns: "dispatchWaitNs"}))
    assert.equal(report.delta_counters[key], value.runtimeDiagnostic[field]);
  assert.equal(Number((100 * report.delta_counters.operational_currentness_ns / report.delta_counters.command_ns).toFixed(2)),
    value.runtimeDiagnostic.operationalToCommandPercent);
  assert(report.counter_semantics.includes("first/periodic currentness and sleep are included in wait time, not isolated device time"));
  console.log("PASS: pinned correction CPU gate, permanently rejected f662 trace, and nonbenchmark runtime counters.");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const context = { window: {} };
  vm.runInNewContext(await readFile(join(dirname(fileURLToPath(import.meta.url)), "data/project.js"), "utf8"), context);
  const project = context.window.FERRIC_PROJECT;
  validateArgmaxCheckpoint(project.routeCheckpoint, project.routeReadiness);
  testArgmaxCheckpointRejections(project.routeCheckpoint, project.routeReadiness);
  if (process.argv[2]) await validateArgmaxCheckpointEvidence(process.argv[2], project.routeCheckpoint);
}
