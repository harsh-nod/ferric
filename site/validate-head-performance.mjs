import assert from "node:assert/strict";

export const headProfiles = ["bf16-control", "fp32-baseline", "fp32-mfma"];
export const headRequests = ["seed-prefix", "arriving-short", "cancel-between-batches", "reuse-prefix"];
export const headPinKeys = ["controllerSourceRevision", "fe2o3Revision", "controllerSha256", "workerSha256",
  "hsacoSha256", "manifestSha256", "handoffSha256", "fp32HsacoSha256", "fp32ManifestSha256", "fp32HandoffSha256",
  "workloadSha256", "referenceSha256", "approvedPrelaunchSha256", "summaryFileSha256", "summaryManifestSha256",
  "precisionPairFileSha256", "precisionPairManifestSha256", "mfmaPairFileSha256", "mfmaPairManifestSha256",
  "generatorSourceSha256", "comparatorSourceSha256", "legacyComparatorSourceSha256", "legacyTimingSourceSha256", "legacyLedgerSourceSha256"];

function keys(value, expected) {
  assert(value && typeof value === "object" && !Array.isArray(value));
  assert.deepEqual(Object.keys(value).sort(), [...expected].sort());
}
function digest(value) {
  assert(typeof value === "string" && /^[a-f0-9]{64}$/.test(value) && !/^0+$/.test(value));
}

export function validateHeadPerformance(section) {
  keys(section, ["scope", "interpretation", "precisionScope", "sourceScope", "limits", "pins", "profiles", "pairs"]);
  assert(section.scope.includes("TP1") && section.scope.includes("n=1 per profile") && section.scope.includes("eight outputs"));
  assert(section.interpretation.includes("slower fresh full process") && section.interpretation.includes("precision-only"));
  assert(section.precisionScope.includes("not an all-FP32 model") && section.precisionScope.includes("9,723,904"));
  assert(section.limits.includes("not broad numerical qualification") && section.limits.includes("not GPU duration"));
  assert(section.sourceScope.includes("separate from the frozen BF16 and transport ledgers"));
  keys(section.pins, headPinKeys);
  for (const [key, value] of Object.entries(section.pins)) if (!key.endsWith("Revision")) digest(value);
  assert.equal(section.pins.controllerSourceRevision, "00fa58d6a1a9394433083b0f37b4f6365d5edb63");
  assert.equal(section.pins.fe2o3Revision, "5110577a6d8c45390dfb353386cde748efd5d76c");
  assert.equal(section.pins.controllerSha256, "3d039c49bf4a4610f3b704d52c17cc46398f0c0f010c060af66c369b8ea48192");
  assert.equal(section.pins.approvedPrelaunchSha256, "4cfe035142fdd4751106cef6781d2238a1316c25f9e22625bd3975eb1daeb199");
  assert.equal(section.pins.summaryFileSha256, "38ba909a53c3b6d4ae491b22ffbe56927ff50b484dd445fc048eb8184f55b97a");
  assert.equal(section.pins.generatorSourceSha256, "1d0cecc87021798aa7aba7dca2fa4085a8c92fed66da5bb230f06e6e017ae6d0");
  assert.equal(section.pins.comparatorSourceSha256, "3c69090b82fe578a25f4ef7f870b9cc2a634e41f9934f9714cc5fa6f9e07fdbc");
  assert.equal(section.profiles.length, 3);
  section.profiles.forEach((profile, index) => {
    keys(profile, ["name", "label", "headPrecision", "projection", "repetitions", "workspaceBytes", "outputTokensPerSecond",
      "workloadSeconds", "setupSeconds", "wholeSeconds", "requestLatencies", "expectationFileSha256", "comparisonFileSha256", "rawTimingSha256"]);
    assert.equal(profile.name, headProfiles[index]);
    assert.equal(profile.label, ["BF16 head control", "FP32 head / baseline projection", "FP32 head / MFMA projection"][index]);
    assert.equal(profile.headPrecision, index === 0 ? "bf16-v7-control" : "fp32-v7");
    assert.equal(profile.projection, index === 2 ? "mfma" : "baseline");
    assert.equal(profile.repetitions, 1);
    assert.equal(profile.workspaceBytes, index === 0 ? 0 : 9723904);
    for (const field of ["outputTokensPerSecond", "workloadSeconds", "setupSeconds", "wholeSeconds"]) {
      assert(Number.isFinite(profile[field]) && profile[field] > 0);
    }
    assert(Math.abs(profile.outputTokensPerSecond * profile.workloadSeconds - 8) < 1e-9);
    assert(profile.wholeSeconds > profile.setupSeconds + profile.workloadSeconds);
    assert.equal(profile.requestLatencies.length, 4);
    profile.requestLatencies.forEach((latency, request) => {
      assert(Array.isArray(latency) && latency.length === 2 && Number.isFinite(latency[0]) && latency[0] > 0);
      if (request === 2) assert.equal(latency[1], null);
      else assert(Number.isFinite(latency[1]) && latency[1] > 0);
    });
    for (const key of ["expectationFileSha256", "comparisonFileSha256", "rawTimingSha256"]) digest(profile[key]);
  });
  assert.equal(section.pairs.length, 2);
  section.pairs.forEach((pair, index) => {
    keys(pair, ["baseline", "candidate", "label", "rateRatio", "setupRatio", "wholeRatio", "reuseTtftRatio", "reuseTpotRatio"]);
    assert.equal(pair.baseline, headProfiles[index]);
    assert.equal(pair.candidate, headProfiles[index + 1]);
    assert.equal(pair.label, ["Head precision only", "MFMA projection under FP32 head"][index]);
    const [base, next] = section.profiles.slice(index, index + 2);
    for (const [ratio, field] of [["rateRatio", "outputTokensPerSecond"], ["setupRatio", "setupSeconds"], ["wholeRatio", "wholeSeconds"]]) {
      assert.equal(pair[ratio], next[field] / base[field]);
    }
    assert.equal(pair.reuseTtftRatio, next.requestLatencies[3][0] / base.requestLatencies[3][0]);
    assert.equal(pair.reuseTpotRatio, next.requestLatencies[3][1] / base.requestLatencies[3][1]);
  });
}

export function testHeadPerformanceRejections(section) {
  for (const mutate of [
    (copy) => { copy.profiles.pop(); },
    (copy) => { copy.profiles[0].repetitions = 2; },
    (copy) => { copy.profiles[0].workspaceBytes = 9723904; },
    (copy) => { copy.profiles[2].projection = "baseline"; },
    (copy) => { copy.profiles[1].headPrecision = "bf16-v7-control"; },
    (copy) => { copy.profiles[0].label = copy.profiles[2].label; },
    (copy) => { copy.profiles[2].outputTokensPerSecond *= 2; },
    (copy) => { copy.profiles[1].requestLatencies[2][1] = 1; },
    (copy) => { copy.pairs[1].baseline = "bf16-control"; },
    (copy) => { copy.pairs[0].setupRatio = copy.pairs[0].rateRatio; },
    (copy) => { copy.pins.summaryFileSha256 = copy.pins.generatorSourceSha256; },
    (copy) => { copy.pins.controllerSha256 = "0".repeat(64); },
    (copy) => { copy.limits = "Broad serving and numerical qualification"; },
  ]) {
    const copy = JSON.parse(JSON.stringify(section));
    mutate(copy);
    assert.throws(() => validateHeadPerformance(copy));
  }
}
