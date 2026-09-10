import assert from "node:assert/strict";

function keys(value, expected) {
  assert(value && typeof value === "object" && !Array.isArray(value));
  assert.deepEqual(Object.keys(value).sort(), [...expected].sort());
}

function range(value) {
  assert(Array.isArray(value) && value.length === 2);
  assert(value.every((number) => Number.isFinite(number) && number > 0));
  assert(value[0] <= value[1]);
}

function digest(value) {
  assert(typeof value === "string" && /^[a-f0-9]{64}$/.test(value));
  assert(!/^0+$/.test(value));
}

export function validatePerformance(data) {
  keys(data, ["updated", "scope", "interpretation", "correctness", "statistics", "definitions",
    "variants", "requests", "pruningReuse", "runtimeProfile", "fixtures", "identities",
    "provenance", "publication", "ablations", "mfmaPair", "deviceTp1Pair", "peerObservations", "peerSourceControls",
    "hostTranspose", "replicaCohorts", "wideRowPair", "transposeModelPair", "mfmaRepeated", "mfmaPruning", "deviceTp1Repeated", "currentCompatibility"]);
  assert.match(data.updated, /^\d{4}-\d{2}-\d{2}$/);
  assert(data.scope.includes("Not steady-state serving throughput"));
  assert(data.scope.includes("eight output tokens including one from the cancelled request"));
  assert(data.statistics.includes("Request identities are never pooled"));
  assert(data.correctness.includes("status 0") && data.correctness.includes("authority is none"));
  assert(data.publication.includes("implementation-local") && data.publication.includes("site only"));
  assert.equal(data.variants.length, 3);
  const names = ["Frozen baseline", "Output-head pruning only", "Operational runtime only"];
  data.variants.forEach((variant, index) => {
    keys(variant, ["name", "kind", "repetitions", "outputHeadPruning", "operationalCurrentness",
      "outputTokensPerSecond", "workloadSeconds", "setupSeconds", "wholeSeconds",
      "controllerSha256", "workerSha256"]);
    assert.equal(variant.name, names[index]);
    assert.equal(variant.kind, index === 0 ? "baseline" : "standalone");
    assert.equal(variant.repetitions, 2);
    assert.equal(variant.outputHeadPruning, index === 1);
    assert.equal(variant.operationalCurrentness, index === 2);
    for (const field of ["outputTokensPerSecond", "workloadSeconds", "setupSeconds", "wholeSeconds"]) {
      range(variant[field]);
    }
    for (const endpoint of [0, 1]) {
      assert(Math.abs(variant.outputTokensPerSecond[endpoint] * variant.workloadSeconds[1 - endpoint] - 8) < 1e-9);
      assert(variant.wholeSeconds[endpoint] > variant.setupSeconds[endpoint]);
    }
    digest(variant.controllerSha256);
    digest(variant.workerSha256);
  });
  assert.equal(data.variants[0].workerSha256, data.variants[1].workerSha256);
  assert.notEqual(data.variants[0].workerSha256, data.variants[2].workerSha256);
  assert.equal(data.requests.length, 4);
  data.requests.forEach((request, index) => {
    keys(request, ["name", "gapsPerRepetition", "baselineTtft", "baselineTpot", "operationalTtft", "operationalTpot"]);
    assert.equal(request.name, ["seed-prefix", "arriving-short", "cancel-between-batches", "reuse-prefix"][index]);
    assert.equal(request.gapsPerRepetition, [1, 2, 0, 1][index]);
    range(request.baselineTtft);
    range(request.operationalTtft);
    for (const field of ["baselineTpot", "operationalTpot"]) {
      if (index === 2) assert.equal(request[field], null);
      else range(request[field]);
    }
  });
  keys(data.pruningReuse, ["ttftSeconds", "tpotSeconds", "gapsPerRepetition"]);
  range(data.pruningReuse.ttftSeconds);
  range(data.pruningReuse.tpotSeconds);
  assert.equal(data.pruningReuse.gapsPerRepetition, 1);
  assert.deepEqual(JSON.parse(JSON.stringify(data.runtimeProfile)), {
    runtime_cache_admission: false, runtime_operational: true,
    dispatch_sequences: false, queue_rollover: false,
    projection: "baseline", attention: "baseline", runtime_profiling: false,
  });
  const ablations = data.ablations;
  keys(ablations, ["scope", "interpretation", "controllerSha256", "workerSha256", "comparatorSha256",
    "baselineLedgerSha256", "controlLedgerSha256", "profiles"]);
  assert(ablations.scope.includes("one repetition each") && ablations.scope.includes("row/chunk budget 16"));
  assert(ablations.interpretation.includes("Sequence-only is slower") && ablations.interpretation.includes("limit attribution"));
  for (const key of ["controllerSha256", "workerSha256", "comparatorSha256", "baselineLedgerSha256", "controlLedgerSha256"]) {
    digest(ablations[key]);
  }
  assert.equal(ablations.workerSha256, data.variants[2].workerSha256);
  assert.notEqual(ablations.controllerSha256, data.variants[2].controllerSha256);
  assert.equal(ablations.profiles.length, 5);
  const ids = ["runtime-control", "runtime-combined", "admission-cache", "sequences", "host-reuse"];
  ablations.profiles.forEach((profile, index) => {
    keys(profile, ["id", "name", "kind", "repetitions", "admissionCache", "operationalCurrentness",
      "dispatchSequences", "hostWorkspaceReuse", "outputTokensPerSecond", "workloadSeconds",
      "setupSeconds", "wholeSeconds", "requestLatencies"]);
    assert.equal(profile.id, ids[index]);
    assert.equal(profile.repetitions, 1);
    assert.equal(profile.kind, index === 0 ? "control" : index === 1 ? "cumulative" : "standalone");
    assert.equal(profile.admissionCache, index === 1 || index === 2);
    assert.equal(profile.operationalCurrentness, index === 1);
    assert.equal(profile.dispatchSequences, index === 1 || index === 3);
    assert.equal(profile.hostWorkspaceReuse, index === 4);
    for (const key of ["outputTokensPerSecond", "workloadSeconds", "setupSeconds", "wholeSeconds"]) {
      assert(Number.isFinite(profile[key]) && profile[key] > 0);
    }
    assert(Math.abs(profile.outputTokensPerSecond * profile.workloadSeconds - 8) < 1e-9);
    assert(profile.wholeSeconds > profile.workloadSeconds + profile.setupSeconds);
    assert.equal(profile.requestLatencies.length, 4);
    profile.requestLatencies.forEach((values, requestIndex) => {
      assert(Array.isArray(values) && values.length === 2);
      assert(Number.isFinite(values[0]) && values[0] > 0);
      if (requestIndex === 2) assert.equal(values[1], null);
      else assert(Number.isFinite(values[1]) && values[1] > 0);
    });
  });
  assert(ablations.profiles[3].outputTokensPerSecond < ablations.profiles[0].outputTokensPerSecond);
  assert(ablations.profiles[1].outputTokensPerSecond < data.variants[2].outputTokensPerSecond[0]);
  keys(data.identities, ["hsacoSha256", "manifestSha256", "handoffSha256", "workloadSha256",
    "referenceSha256", "operationalLedgerSha256", "pruningLedgerSha256",
    "waveImageControlHsacoSha256", "waveImageControlManifestSha256",
    "waveImageControlHandoffSha256", "waveImageControlComparisonSha256",
    "waveProjectionOperationalRejectionSha256", "waveAttentionOperationalRejectionSha256"]);
  Object.values(data.identities).forEach(digest);
  assert.equal(data.definitions.length, 5);
  assert.equal(data.fixtures.length, 4);
  for (const pair of [...data.definitions, ...data.fixtures]) {
    assert(Array.isArray(pair) && pair.length === 2 && pair.every((text) => typeof text === "string" && text.length > 0));
  }
  assert(data.fixtures[1][1].includes("correctness failure"));
  assert(data.fixtures[1][1].includes("both wave-projection-only and wave-attention-only"));
  assert(data.fixtures[2][1].includes("No matched host control"));
  const singleProfile = (profile, extraKeys) => {
    keys(profile, ["id", "name", "repetitions", "outputTokensPerSecond", "workloadSeconds",
      "setupSeconds", "wholeSeconds", "requestLatencies", "comparisonSha256", ...extraKeys]);
    assert.equal(profile.repetitions, 1);
    digest(profile.comparisonSha256);
    for (const field of ["outputTokensPerSecond", "workloadSeconds", "setupSeconds", "wholeSeconds"]) {
      assert(Number.isFinite(profile[field]) && profile[field] > 0);
    }
    assert(Math.abs(profile.outputTokensPerSecond * profile.workloadSeconds - 8) < 1e-9);
    assert(profile.wholeSeconds > profile.setupSeconds + profile.workloadSeconds);
    assert.equal(profile.requestLatencies.length, 4);
    profile.requestLatencies.forEach((values, index) => {
      assert(Array.isArray(values) && values.length === 2);
      assert(Number.isFinite(values[0]) && values[0] > 0);
      if (index === 2) assert.equal(values[1], null);
      else assert(Number.isFinite(values[1]) && values[1] > 0);
    });
  };
  const current = data.currentCompatibility;
  keys(current, ["scope", "interpretation", "pins", "accepted", "rejected"]);
  keys(current.pins, ["controllerSha256", "workerSha256", "controllerFe2o3Revision", "workerRebuildFe2o3Revision"]);
  digest(current.pins.controllerSha256);
  digest(current.pins.workerSha256);
  assert.equal(current.pins.workerSha256, data.replicaCohorts.pins.workerSha256);
  assert.equal(current.pins.controllerFe2o3Revision, "7528e7345cef0158d7034cdbae23011e3c6fb5d2");
  assert.equal(current.pins.workerRebuildFe2o3Revision, "1b262ac3dd23ee63067e40587d62a124f40b9fc9");
  assert(current.scope.includes("byte-identical"));
  assert(current.interpretation.includes("at most 17 rows, not 32"));
  assert(current.interpretation.includes("no rejected-run timing"));
  assert.equal(current.accepted.length, 3);
  current.accepted.forEach((profile, index) => {
    singleProfile(profile, ["world", "imageProfile", "projection", "outputHeadPruning", "collective", "rowCapacity", "actualBatchRows", "metricsFileSha256"]);
    assert.equal(profile.id, ["latest-tp1-control-r1", "latest-tp8-wide-cumulative-r1", "latest-tp1-residual-pruning-r1"][index]);
    assert.equal(profile.world, [1, 8, 1][index]);
    assert.equal(profile.imageProfile, index === 1 ? "v5-mfma32" : "v3-mfma");
    assert.equal(profile.projection, index === 1 ? "mfma" : "baseline");
    assert.equal(profile.outputHeadPruning, index !== 0);
    assert.equal(profile.collective, index === 2 ? "device-tp1-v3" : null);
    assert.equal(profile.rowCapacity, index === 1 ? 32 : 16);
    assert.deepEqual([...profile.actualBatchRows], index === 1 ? [17, 6, 6, 4, 1] : [16, 6, 7, 4, 1]);
    digest(profile.metricsFileSha256);
  });
  assert.equal(current.rejected.length, 2);
  current.rejected.forEach((profile, index) => {
    keys(profile, ["id", "name", "repetitions", "world", "projection", "outputHeadPruning", "collective", "requestName", "expectedTokens", "observedTokens", "rejectionSha256"]);
    assert.equal(profile.id, ["latest-tp1-cumulative-r1", "latest-tp1-mfma-only-r1"][index]);
    assert.equal(profile.repetitions, 1);
    assert.equal(profile.world, 1);
    assert.equal(profile.projection, "mfma");
    assert.equal(profile.outputHeadPruning, index === 0);
    assert.equal(profile.collective, index === 0 ? "device-tp1-v3" : null);
    assert.equal(profile.requestName, "seed-prefix");
    assert.deepEqual([...profile.expectedTokens], [17689, 374]);
    assert.deepEqual([...profile.observedTokens], [9856, 374]);
    digest(profile.rejectionSha256);
  });
  const mfma = data.mfmaPair;
  keys(mfma, ["scope", "interpretation", "pins", "profiles"]);
  assert(mfma.scope.includes("Only projection selection changes"));
  assert(mfma.interpretation.includes("Startup regresses"));
  keys(mfma.pins, ["controllerSha256", "workerSha256", "hsacoSha256", "manifestSha256",
    "handoffSha256", "comparatorSha256", "ledgerFileSha256", "ledgerCanonicalId"]);
  Object.values(mfma.pins).forEach(digest);
  assert.equal(mfma.pins.controllerSha256, ablations.controllerSha256);
  assert.equal(mfma.pins.workerSha256, ablations.workerSha256);
  assert.equal(mfma.profiles.length, 2);
  mfma.profiles.forEach((profile, index) => {
    singleProfile(profile, ["projection"]);
    assert.equal(profile.id, ["mfma-image-control", "mfma-projection"][index]);
    assert.equal(profile.projection, ["baseline", "mfma"][index]);
  });
  assert(mfma.profiles[1].setupSeconds > mfma.profiles[0].setupSeconds);
  assert(mfma.profiles[1].wholeSeconds > mfma.profiles[0].wholeSeconds);
  assert(mfma.profiles[1].outputTokensPerSecond > mfma.profiles[0].outputTokensPerSecond);
  const repeated = data.mfmaRepeated;
  keys(repeated, ["scope", "interpretation", "repetitions", "ledgerFileSha256", "secondProfiles"]);
  assert.equal(repeated.repetitions, 2);
  digest(repeated.ledgerFileSha256);
  assert(repeated.scope.includes("R1 is the previously published pair"));
  assert(repeated.interpretation.includes("not a stable tail"));
  assert(repeated.interpretation.includes("Request identities are never pooled"));
  assert.equal(repeated.secondProfiles.length, 2);
  repeated.secondProfiles.forEach((profile, index) => {
    singleProfile(profile, ["projection"]);
    assert.equal(profile.id, `${mfma.profiles[index].id}-r2`);
    assert.equal(profile.projection, mfma.profiles[index].projection);
    assert.notEqual(profile.comparisonSha256, mfma.profiles[index].comparisonSha256);
  });
  assert(repeated.secondProfiles[1].setupSeconds > repeated.secondProfiles[0].setupSeconds);
  const cumulative = data.mfmaPruning;
  keys(cumulative, ["scope", "interpretation", "ledgerFileSha256", "profiles"]);
  digest(cumulative.ledgerFileSha256);
  assert(cumulative.scope.includes("adds pruning to MFMA"));
  assert(cumulative.interpretation.includes("does not show an additive pruning gain"));
  assert.equal(cumulative.profiles.length, 1);
  singleProfile(cumulative.profiles[0], ["projection", "outputHeadPruning"]);
  assert.equal(cumulative.profiles[0].projection, "mfma");
  assert.equal(cumulative.profiles[0].outputHeadPruning, true);
  assert.equal(cumulative.profiles[0].id, "mfma-and-pruning");
  const tp1 = data.deviceTp1Pair;
  keys(tp1, ["scope", "interpretation", "pins", "profiles"]);
  keys(tp1.pins, ["controllerSha256", "workerSha256", "hsacoSha256", "manifestSha256",
    "handoffSha256", "comparatorSha256", "ledgerFileSha256"]);
  Object.values(tp1.pins).forEach(digest);
  assert.equal(tp1.pins.hsacoSha256, data.identities.waveImageControlHsacoSha256);
  assert(tp1.scope.includes("not multi-device peer transport"));
  assert(tp1.interpretation.includes("too few samples for a robust gain claim"));
  assert.equal(tp1.profiles.length, 2);
  tp1.profiles.forEach((profile, index) => {
    singleProfile(profile, ["collective"]);
    assert.equal(profile.id, ["device-tp1-control", "device-tp1-residual"][index]);
    assert.equal(profile.collective, ["host_staged_fp32_rank_order_reduce_bf16_residual", "device-tp1-v3"][index]);
  });
  const tp1Repeated = data.deviceTp1Repeated;
  keys(tp1Repeated, ["scope", "interpretation", "repetitions", "ledgerFileSha256", "secondProfiles"]);
  assert.equal(tp1Repeated.repetitions, 2);
  digest(tp1Repeated.ledgerFileSha256);
  assert(tp1Repeated.scope.includes("R1 is the previously published pair"));
  assert(tp1Repeated.interpretation.includes("not a stable tail"));
  assert.equal(tp1Repeated.secondProfiles.length, 2);
  tp1Repeated.secondProfiles.forEach((profile, index) => {
    singleProfile(profile, ["collective"]);
    assert.equal(profile.id, `${tp1.profiles[index].id}-r2`);
    assert.equal(profile.collective, tp1.profiles[index].collective);
    assert.notEqual(profile.comparisonSha256, tp1.profiles[index].comparisonSha256);
  });
  const peer = data.peerObservations;
  keys(peer, ["scope", "interpretation", "matchedControl", "speedupClaimed", "collective", "pins", "profiles"]);
  assert.equal(peer.matchedControl, null);
  assert.equal(peer.speedupClaimed, false);
  assert.equal(peer.collective, "device-peer-serial-v4");
  assert(peer.interpretation.includes("negative performance evidence"));
  keys(peer.pins, ["controllerSha256", "workerSha256", "baseHsacoSha256", "baseManifestSha256", "baseHandoffSha256",
    "peerHsacoSha256", "peerManifestSha256", "peerHandoffSha256", "comparatorSha256"]);
  Object.values(peer.pins).forEach(digest);
  assert.equal(peer.pins.baseHsacoSha256, data.identities.hsacoSha256);
  assert.equal(peer.profiles.length, 2);
  peer.profiles.forEach((profile, index) => {
    singleProfile(profile, ["world", "metricsFileSha256"]);
    assert.equal(profile.world, [2, 8][index]);
    assert.equal(profile.id, `peer-tp${profile.world}-operational-r1`);
    digest(profile.metricsFileSha256);
  });
  const peerControls = data.peerSourceControls;
  keys(peerControls, ["scope", "interpretation", "sourceRevision", "controllerSha256", "hostWorkerSha256", "peerWorkerSha256", "controls"]);
  assert.equal(peerControls.sourceRevision, "902fef6e1478b3ac677e5456b2a2d1f917456fba");
  for (const key of ["controllerSha256", "hostWorkerSha256", "peerWorkerSha256"]) digest(peerControls[key]);
  assert.equal(peerControls.controllerSha256, peer.pins.controllerSha256);
  assert.equal(peerControls.peerWorkerSha256, peer.pins.workerSha256);
  assert.notEqual(peerControls.hostWorkerSha256, peerControls.peerWorkerSha256);
  assert(peerControls.scope.includes("not byte-identical workers"));
  assert(peerControls.interpretation.includes("not counted as new repetitions"));
  assert.equal(peerControls.controls.length, 2);
  peerControls.controls.forEach((control, index) => {
    singleProfile(control, ["world", "ledgerFileSha256"]);
    assert.equal(control.world, [2, 8][index]);
    assert.equal(control.id, `host-source-matched-tp${control.world}`);
    digest(control.ledgerFileSha256);
    assert(control.outputTokensPerSecond > peer.profiles[index].outputTokensPerSecond);
  });
  const host = data.hostTranspose;
  keys(host, ["scope", "interpretation", "modelTiming", "allocationHashUploadIncluded", "repetitions",
    "fullArrayComparisons", "source", "summarySha256", "rawLogSha256", "groups"]);
  assert.equal(host.modelTiming, false);
  assert.equal(host.allocationHashUploadIncluded, false);
  assert.equal(host.repetitions, 3);
  assert.equal(host.fullArrayComparisons, 240);
  assert.match(host.source, /^[0-9a-f]{7}$/);
  digest(host.summarySha256);
  digest(host.rawLogSha256);
  assert(host.interpretation.includes("not a measured model setup"));
  assert.equal(host.groups.length, 3);
  host.groups.forEach((group, index) => {
    keys(group, ["world", "cases", "baselineSeconds", "tiledSeconds", "helperSpeedup"]);
    assert.equal(group.world, [1, 2, 8][index]);
    assert.equal(group.cases, [8, 15, 57][index]);
    assert(group.baselineSeconds > 0 && group.tiledSeconds > 0);
    assert(Math.abs(group.baselineSeconds / group.tiledSeconds - group.helperSpeedup) < 1e-12);
  });
  const transposeModel = data.transposeModelPair;
  keys(transposeModel, ["scope", "interpretation", "causalDecodeSpeedupClaimed", "pins", "profiles"]);
  assert.equal(transposeModel.causalDecodeSpeedupClaimed, false);
  assert(transposeModel.interpretation.includes("not a causal decode-speed claim"));
  keys(transposeModel.pins, ["workerSha256", "hsacoSha256", "manifestSha256", "handoffSha256", "ledgerFileSha256"]);
  Object.values(transposeModel.pins).forEach(digest);
  assert.equal(transposeModel.pins.hsacoSha256, mfma.pins.hsacoSha256);
  assert.notEqual(transposeModel.pins.workerSha256, mfma.pins.workerSha256);
  assert.equal(transposeModel.profiles.length, 2);
  transposeModel.profiles.forEach((profile, index) => {
    singleProfile(profile, ["controllerSha256"]);
    assert.equal(profile.id, ["transpose-untiled", "transpose-tiled"][index]);
    digest(profile.controllerSha256);
  });
  assert.notEqual(transposeModel.profiles[0].controllerSha256, transposeModel.profiles[1].controllerSha256);
  const cohorts = data.replicaCohorts;
  keys(cohorts, ["scope", "interpretation", "clockScope", "limits", "pins", "profiles"]);
  assert(cohorts.scope.includes("64 total outputs") && cohorts.scope.includes("not steady-state serving"));
  assert(cohorts.clockScope.includes("not a sum of instance rates"));
  assert(cohorts.clockScope.includes("first spawn to all Ready, excluding the one-second release lead"));
  assert(cohorts.interpretation.includes("16-row budget is per instance"));
  assert(cohorts.limits.includes("not host RSS or full GPU usage"));
  assert(cohorts.limits.includes("canonical JSON, not original file bytes"));
  keys(cohorts.pins, ["controllerSha256", "workerSha256", "hsacoSha256", "manifestSha256", "handoffSha256",
    "workloadSha256", "referenceSha256", "cohortComparatorSha256", "traceComparatorSha256", "batchComparatorSha256",
    "allocationComparisonSha256"]);
  Object.values(cohorts.pins).forEach(digest);
  assert.notEqual(cohorts.pins.workloadSha256, data.identities.workloadSha256);
  assert.equal(cohorts.pins.referenceSha256, data.identities.referenceSha256);
  assert.equal(cohorts.profiles.length, 3);
  cohorts.profiles.forEach((profile, index) => {
    keys(profile, ["layout", "replicas", "world", "repetitions", "perInstanceRows", "totalRowBudget",
      "outputTokensPerSecond", "releaseToLastOutputNs", "barrierSetupNs", "spawnToReapNs", "releaseEpochNs",
      "maximumLatenessNs", "physicalTokenRows", "hostWeightBytes", "gpuBaseWeightBytes", "gpuTransposedWeightBytes",
      "requestLatencies", "comparisonSha256", "expectationSha256"]);
    assert.equal(profile.layout, ["1xTP8", "4xTP2", "8xTP1"][index]);
    assert.equal(profile.replicas, [1, 4, 8][index]);
    assert.equal(profile.replicas * profile.world, 8);
    assert.equal(profile.repetitions, 1);
    assert.equal(profile.perInstanceRows, 16);
    assert.equal(profile.totalRowBudget, 16 * profile.replicas);
    for (const key of ["releaseToLastOutputNs", "barrierSetupNs", "spawnToReapNs", "releaseEpochNs",
      "maximumLatenessNs", "physicalTokenRows", "hostWeightBytes", "gpuBaseWeightBytes", "gpuTransposedWeightBytes"]) {
      assert(Number.isSafeInteger(profile[key]) && profile[key] >= 0);
    }
    assert.equal(profile.physicalTokenRows, 96);
    assert.equal(profile.gpuTransposedWeightBytes, 0);
    assert(profile.hostWeightBytes > 0 && profile.gpuBaseWeightBytes > 0);
    assert(profile.maximumLatenessNs <= 100000000);
    assert(Math.abs(profile.outputTokensPerSecond * profile.releaseToLastOutputNs / 1e9 - 64) < 1e-9);
    assert(profile.spawnToReapNs > profile.barrierSetupNs + profile.releaseToLastOutputNs);
    digest(profile.comparisonSha256);
    digest(profile.expectationSha256);
    assert.equal(profile.requestLatencies.length, 8);
    profile.requestLatencies.forEach((values) => {
      assert(Array.isArray(values) && values.length === 4);
      assert(Number.isSafeInteger(values[0]) && values[0] >= 0 && values[0] < profile.replicas);
      assert(Number.isSafeInteger(values[1]) && values[1] > 0);
      assert(Number.isSafeInteger(values[2]) && values[2] >= values[1] && values[2] < profile.releaseToLastOutputNs);
      assert(Number.isFinite(values[3]) && values[3] > 0);
    });
  });
  const wide = data.wideRowPair;
  keys(wide, ["scope", "interpretation", "timing", "pins", "profiles"]);
  assert(wide.scope.includes("Batch budget and prefill chunk change together"));
  assert(wide.scope.includes("selectors are baseline"));
  assert(wide.interpretation.includes("admission TTFT worsens"));
  keys(wide.pins, ["controllerSha256", "workerSha256", "hsacoSha256", "manifestSha256", "handoffSha256", "pairComparisonSha256"]);
  Object.values(wide.pins).forEach(digest);
  assert.equal(wide.pins.controllerSha256, cohorts.pins.controllerSha256);
  assert.equal(wide.pins.workerSha256, cohorts.pins.workerSha256);
  assert.notEqual(wide.pins.hsacoSha256, cohorts.pins.hsacoSha256);
  assert.equal(wide.profiles.length, 2);
  wide.profiles.forEach((profile, index) => {
    keys(profile, ["name", "rows", "prefillChunk", "repetitions", "outputTokensPerSecond", "releaseToLastOutputNs",
      "barrierSetupNs", "spawnToReapNs", "releaseEpochNs", "maximumLatenessNs", "actualBatchRows", "requestLatencies",
      "comparisonSha256", "expectationSha256"]);
    assert.equal(profile.rows, [16, 32][index]);
    assert.equal(profile.prefillChunk, profile.rows);
    assert.equal(profile.repetitions, 1);
    for (const key of ["releaseToLastOutputNs", "barrierSetupNs", "spawnToReapNs", "releaseEpochNs", "maximumLatenessNs"]) {
      assert(Number.isSafeInteger(profile[key]) && profile[key] > 0);
    }
    assert(profile.maximumLatenessNs <= 100000000);
    assert(Math.abs(profile.outputTokensPerSecond * profile.releaseToLastOutputNs / 1e9 - 64) < 1e-9);
    assert(profile.spawnToReapNs > profile.barrierSetupNs + profile.releaseToLastOutputNs);
    assert.deepEqual([...profile.actualBatchRows], index === 0 ? [16, 16, 16, 8, 8, 8, 8, 8, 5, 3] : [32, 14, 8, 8, 8, 8, 8, 8, 2]);
    digest(profile.comparisonSha256);
    digest(profile.expectationSha256);
    assert.equal(profile.requestLatencies.length, 8);
    profile.requestLatencies.forEach((values) => {
      assert(Array.isArray(values) && values.length === 3);
      assert(Number.isSafeInteger(values[0]) && values[0] > 0);
      assert(Number.isSafeInteger(values[1]) && values[1] >= values[0] && values[1] < profile.releaseToLastOutputNs);
      assert(Number.isFinite(values[2]) && values[2] > 0);
    });
  });
}

export function testPerformanceRejections(data) {
  const mutations = [
    (copy) => { copy.currentCompatibility.accepted[2].collective = null; },
    (copy) => { copy.currentCompatibility.accepted[2].projection = "mfma"; },
    (copy) => { copy.currentCompatibility.rejected[1].outputHeadPruning = true; },
    (copy) => { copy.currentCompatibility.accepted[1].actualBatchRows[0] = 32; },
    (copy) => { copy.currentCompatibility.accepted[1].repetitions = 2; },
    (copy) => { copy.currentCompatibility.rejected[0].outputTokensPerSecond = 1; },
    (copy) => { copy.currentCompatibility.rejected[0].observedTokens = [17689, 374]; },
    (copy) => { copy.currentCompatibility.rejected[0].collective = null; },
    (copy) => { copy.currentCompatibility.pins.workerRebuildFe2o3Revision = copy.currentCompatibility.pins.controllerFe2o3Revision; },
    (copy) => { copy.currentCompatibility.interpretation = "All combinations qualified"; },
    (copy) => { copy.deviceTp1Repeated.repetitions = 4; },
    (copy) => { copy.deviceTp1Repeated.secondProfiles[1].collective = "device-peer-serial-v4"; },
    (copy) => { copy.deviceTp1Repeated.secondProfiles[0].comparisonSha256 = copy.deviceTp1Pair.profiles[0].comparisonSha256; },
    (copy) => { copy.deviceTp1Repeated.interpretation = "Stable p95 established"; },
    (copy) => { copy.mfmaPruning.profiles[0].outputHeadPruning = false; },
    (copy) => { copy.mfmaPruning.profiles[0].repetitions = 2; },
    (copy) => { copy.mfmaPruning.interpretation = "Pruning gives an additive gain"; },
    (copy) => { copy.mfmaRepeated.repetitions = 4; },
    (copy) => { copy.mfmaRepeated.secondProfiles[1].requestLatencies[2][1] = 1; },
    (copy) => { copy.mfmaRepeated.secondProfiles[0].comparisonSha256 = copy.mfmaPair.profiles[0].comparisonSha256; },
    (copy) => { copy.mfmaRepeated.secondProfiles[1].projection = "wave"; },
    (copy) => { copy.mfmaRepeated.interpretation = "Stable p95 established"; },
    (copy) => { copy.variants[0].repetitions = 20; },
    (copy) => { copy.variants[0].outputTokensPerSecond[0] *= 2; },
    (copy) => { copy.variants[2].outputHeadPruning = true; },
    (copy) => { copy.variants[0].workloadSeconds.reverse(); },
    (copy) => { copy.requests[2].baselineTpot = [1, 1]; },
    (copy) => { copy.requests[0].name = "pooled"; },
    (copy) => { copy.runtimeProfile.runtime_cache_admission = true; },
    (copy) => { copy.identities.referenceSha256 = "0".repeat(64); },
    (copy) => { copy.fixtures[1][1] = "Wave model performance passed"; },
    (copy) => { copy.extra = true; },
    (copy) => { copy.ablations.profiles[0].repetitions = 2; },
    (copy) => { copy.ablations.profiles[1].kind = "standalone"; },
    (copy) => { copy.ablations.profiles[2].outputTokensPerSecond *= 2; },
    (copy) => { copy.ablations.profiles[3].requestLatencies[2][1] = 0; },
    (copy) => { copy.ablations.profiles[4].id = "wave-attention"; },
    (copy) => { copy.ablations.profiles[0].dispatchSequences = true; },
    (copy) => { copy.ablations.controlLedgerSha256 = "0".repeat(64); },
    (copy) => { copy.mfmaPair.profiles[0].repetitions = 2; },
    (copy) => { copy.mfmaPair.profiles[0].projection = "wave"; },
    (copy) => { copy.mfmaPair.profiles[1].outputTokensPerSecond *= 2; },
    (copy) => { copy.mfmaPair.profiles[1].requestLatencies[2][1] = 1; },
    (copy) => { copy.mfmaPair.interpretation = "Faster everywhere"; },
    (copy) => { copy.mfmaPair.pins.ledgerFileSha256 = "0".repeat(64); },
    (copy) => { copy.peerObservations.matchedControl = "historical"; },
    (copy) => { copy.peerObservations.speedupClaimed = true; },
    (copy) => { copy.peerObservations.profiles[1].world = 2; },
    (copy) => { copy.peerObservations.profiles[0].metricsFileSha256 = "0".repeat(64); },
    (copy) => { copy.hostTranspose.modelTiming = true; },
    (copy) => { copy.hostTranspose.allocationHashUploadIncluded = true; },
    (copy) => { copy.hostTranspose.fullArrayComparisons = 80; },
    (copy) => { copy.hostTranspose.groups[0].helperSpeedup *= 2; },
    (copy) => { copy.deviceTp1Pair.profiles[1].collective = "device-peer-serial-v4"; },
    (copy) => { copy.deviceTp1Pair.interpretation = "Robust gain established"; },
    (copy) => { copy.replicaCohorts.profiles[0].outputTokensPerSecond *= 8; },
    (copy) => { copy.replicaCohorts.profiles[0].totalRowBudget = 128; },
    (copy) => { copy.replicaCohorts.profiles[0].requestLatencies[0][2] = 0; },
    (copy) => { copy.replicaCohorts.profiles[0].requestLatencies[0][0] = 1; },
    (copy) => { copy.replicaCohorts.profiles[0].maximumLatenessNs = 100000001; },
    (copy) => { copy.replicaCohorts.pins.workloadSha256 = copy.identities.workloadSha256; },
    (copy) => { copy.wideRowPair.profiles[1].prefillChunk = 16; },
    (copy) => { copy.wideRowPair.profiles[1].actualBatchRows[0] = 16; },
    (copy) => { copy.wideRowPair.profiles[0].outputTokensPerSecond *= 2; },
    (copy) => { copy.wideRowPair.profiles[1].requestLatencies[0][0] = -1; },
    (copy) => { copy.wideRowPair.interpretation = "Latency always improves"; },
    (copy) => { copy.peerSourceControls.hostWorkerSha256 = copy.peerSourceControls.peerWorkerSha256; },
    (copy) => { copy.peerSourceControls.controls[0].world = 8; },
    (copy) => { copy.peerSourceControls.controls[1].repetitions = 2; },
    (copy) => { copy.transposeModelPair.causalDecodeSpeedupClaimed = true; },
    (copy) => { copy.transposeModelPair.profiles[1].repetitions = 2; },
    (copy) => { copy.transposeModelPair.profiles[1].controllerSha256 = copy.transposeModelPair.profiles[0].controllerSha256; },
  ];
  for (const mutate of mutations) {
    const copy = JSON.parse(JSON.stringify(data));
    mutate(copy);
    assert.throws(() => validatePerformance(copy));
  }
}
