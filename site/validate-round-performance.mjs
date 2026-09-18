import assert from "node:assert/strict";

const requestNames = ["seed-prefix", "arriving-short", "cancel-between-batches", "reuse-prefix"];
const modes = ["host", "serial", "round"];
const profileKeys = ["id", "name", "world", "mode", "repetitions", "outputTokensPerSecond",
  "workloadSeconds", "setupSeconds", "wholeSeconds", "requestLatencies", "caseFileSha256",
  "comparisonFileSha256"];

function keys(value, expected) {
  assert(value && typeof value === "object" && !Array.isArray(value));
  assert.deepEqual(Object.keys(value).sort(), [...expected].sort());
}

function digest(value) {
  assert(typeof value === "string" && /^[a-f0-9]{64}$/.test(value) && !/^0+$/.test(value));
}

export function validateRoundPerformance(section) {
  keys(section, ["scope", "interpretation", "sourceScope", "measurementScope", "limits", "pins", "worlds", "profiles"]);
  assert(section.scope.includes("n=1 per mode") && section.scope.includes("four requests and eight outputs"));
  assert(section.measurementScope.includes("not GPU timestamps") && section.measurementScope.includes("excludes setup"));
  assert(section.limits.includes("not steady-state serving") && section.limits.includes("no physical overlap measurement"));
  assert(section.sourceScope.includes("4fbc0a34") && section.sourceScope.includes("79706b43"));
  keys(section.pins, ["controllerSha256", "controllerSourceRevision", "controllerFe2o3Revision", "workerFe2o3Revision",
    "hostWorkerSha256", "peerWorkerSha256", "hsacoSha256", "manifestSha256", "handoffSha256",
    "peerHsacoSha256", "peerManifestSha256", "peerHandoffSha256", "workloadSha256", "referenceSha256",
    "prelaunchSha256", "comparatorSourceSha256", "ledgerGeneratorSourceSha256", "hostTimingGeneratorSourceSha256"]);
  assert.equal(section.pins.controllerSourceRevision, "abecc46662ed884162407a4825899b212fb63db0");
  assert.equal(section.pins.controllerFe2o3Revision, "4fbc0a34c7938474406d37a45e7972ea8b1ec277");
  assert.equal(section.pins.workerFe2o3Revision, "79706b43a177a2fd3fa43ec328221fa3e5041af5");
  for (const [key, value] of Object.entries(section.pins)) if (!key.endsWith("Revision")) digest(value);
  assert.equal(section.pins.controllerSha256, "d9d378e0378758f41b22b3112ccf113cc0d86f266f992a5626b062ad2399d800");
  assert.equal(section.pins.hostWorkerSha256, "aaa0216a77de0d5f12c2d668b31ca8c340d8975407c2b446bb5e20b5d820bd6e");
  assert.equal(section.pins.peerWorkerSha256, "2032e31b6d7fbc22aae6e5b7092f9a4e37047360ca5025fb10b2e0119999ea2e");
  assert.equal(section.pins.hsacoSha256, "8c81d3fe869d3346d95486354f366988ce99210111cd0fc712dfb2194450b125");
  assert.equal(section.pins.peerHsacoSha256, "9d4f1b922967521b05199ce7d1e0269eaae3afa4fe558eafc4ba0bda3c4a4b65");
  assert.equal(section.pins.prelaunchSha256, "3675ad678cfb1427d0ec84b79d7ab3141992aebdc3d1c8cd8ab399a894128115");
  assert.equal(section.pins.comparatorSourceSha256, "1be4f2b3b0f8d1c96ef0b5bf21388da232d6b23e5700c7bef6295117b6093b6a");
  assert.equal(section.pins.ledgerGeneratorSourceSha256, "6da3b95acda3332bfa27a1c51685dda316d5f188615b9b939554d5eef8c9085e");
  assert.equal(section.pins.hostTimingGeneratorSourceSha256, "057f4a6dfb929db44714f8192bb1e373184f4170ce3946dcc31be811637b2402");
  assert.equal(section.worlds.length, 2);
  assert.equal(section.profiles.length, 6, "incomplete matrix cannot pass the publication gate");
  section.profiles.forEach((profile, index) => {
    keys(profile, profileKeys);
    const world = index < 3 ? 2 : 8;
    const mode = modes[index % 3];
    assert.equal(profile.world, world);
    assert.equal(profile.mode, mode);
    assert.equal(profile.name, ["Host-staged", "Serial peer", "Concurrent peer round"][index % 3]);
    assert.equal(profile.id, `tp${world}-${mode}-profile-r1`);
    assert.equal(profile.repetitions, 1);
    for (const key of ["outputTokensPerSecond", "workloadSeconds", "setupSeconds", "wholeSeconds"]) {
      assert(Number.isFinite(profile[key]) && profile[key] > 0);
    }
    assert(Math.abs(profile.outputTokensPerSecond * profile.workloadSeconds - 8) < 1e-9);
    assert(profile.wholeSeconds > profile.setupSeconds + profile.workloadSeconds);
    assert.equal(profile.requestLatencies.length, requestNames.length);
    profile.requestLatencies.forEach((latency, requestIndex) => {
      assert(Array.isArray(latency) && latency.length === 2);
      assert(Number.isFinite(latency[0]) && latency[0] > 0);
      if (requestIndex === 2) assert.equal(latency[1], null);
      else assert(Number.isFinite(latency[1]) && latency[1] > 0);
    });
    digest(profile.caseFileSha256);
    digest(profile.comparisonFileSha256);
  });
  section.worlds.forEach((entry, index) => {
    keys(entry, ["world", "ledgerFileSha256", "serialLedgerFileSha256", "roundOverSerial", "roundOverHost"]);
    assert.equal(entry.world, [2, 8][index]);
    digest(entry.ledgerFileSha256);
    digest(entry.serialLedgerFileSha256);
    assert.notEqual(entry.ledgerFileSha256, section.pins.ledgerGeneratorSourceSha256);
    assert.notEqual(entry.serialLedgerFileSha256, section.pins.ledgerGeneratorSourceSha256);
    const [host, serial, round] = section.profiles.slice(index * 3, index * 3 + 3);
    assert.equal(entry.roundOverSerial, round.outputTokensPerSecond / serial.outputTokensPerSecond);
    assert.equal(entry.roundOverHost, round.outputTokensPerSecond / host.outputTokensPerSecond);
  });
}

export function testRoundPerformanceRejections(section) {
  for (const mutate of [
    (copy) => { copy.profiles.pop(); },
    (copy) => { copy.profiles[0].repetitions = 2; },
    (copy) => { copy.profiles[0].world = 8; },
    (copy) => { copy.profiles[1].mode = "round"; },
    (copy) => { copy.profiles[1].name = "Host-staged"; },
    (copy) => { copy.profiles[0].outputTokensPerSecond *= 2; },
    (copy) => { copy.profiles[0].requestLatencies[2][1] = 1; },
    (copy) => { copy.profiles[0].comparisonFileSha256 = "0".repeat(64); },
    (copy) => { copy.worlds[0].roundOverHost = copy.worlds[0].roundOverSerial; },
    (copy) => { copy.worlds[0].ledgerFileSha256 = copy.pins.ledgerGeneratorSourceSha256; },
    (copy) => { copy.pins.workerFe2o3Revision = copy.pins.controllerFe2o3Revision; },
    (copy) => { copy.measurementScope = "GPU time including setup"; },
  ]) {
    const copy = JSON.parse(JSON.stringify(section));
    mutate(copy);
    assert.throws(() => validateRoundPerformance(copy));
  }
}

export { requestNames };
