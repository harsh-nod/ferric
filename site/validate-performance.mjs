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
    "provenance", "publication", "ablations", "mfmaPair", "deviceTp1Pair", "peerObservations", "hostTranspose", "replicaCohorts"]);
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
}

export function testPerformanceRejections(data) {
  const mutations = [
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
  ];
  for (const mutate of mutations) {
    const copy = JSON.parse(JSON.stringify(data));
    mutate(copy);
    assert.throws(() => validatePerformance(copy));
  }
}
