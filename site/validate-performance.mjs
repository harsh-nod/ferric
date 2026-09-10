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
    "provenance", "publication"]);
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
  keys(data.identities, ["hsacoSha256", "manifestSha256", "handoffSha256", "workloadSha256",
    "referenceSha256", "operationalLedgerSha256", "pruningLedgerSha256"]);
  Object.values(data.identities).forEach(digest);
  assert.equal(data.definitions.length, 5);
  assert.equal(data.fixtures.length, 4);
  for (const pair of [...data.definitions, ...data.fixtures]) {
    assert(Array.isArray(pair) && pair.length === 2 && pair.every((text) => typeof text === "string" && text.length > 0));
  }
  assert(data.fixtures[1][1].includes("correctness failure"));
  assert(data.fixtures[2][1].includes("No peer-transport Qwen model result"));
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
  ];
  for (const mutate of mutations) {
    const copy = JSON.parse(JSON.stringify(data));
    mutate(copy);
    assert.throws(() => validatePerformance(copy));
  }
}
