import assert from "node:assert/strict";

function exactKeys(value, expected) {
  assert(value && typeof value === "object" && !Array.isArray(value));
  assert.deepEqual(Object.keys(value).sort(), [...expected].sort());
}

export function validateResidentCheckpoint(value, updated) {
  const checkpoint = JSON.parse(JSON.stringify(value));
  exactKeys(checkpoint, ["date", "scope", "implementationPrivate", "m1OpenGates",
    "nativeServingQualified", "fullAcceptanceCatchupImplemented", "newGpuMeasurement",
    "newPerformanceMeasurement", "overview", "selection", "host", "compiler", "remaining"]);
  assert.equal(updated, "2026-09-14");
  assert.equal(checkpoint.date, updated);
  assert.equal(checkpoint.implementationPrivate, true);
  assert.equal(checkpoint.m1OpenGates, 33);
  for (const key of ["nativeServingQualified", "fullAcceptanceCatchupImplemented",
    "newGpuMeasurement", "newPerformanceMeasurement"]) {
    assert.equal(checkpoint[key], false, key);
  }
  assert.equal(checkpoint.scope, "Private authenticated resident serving; host and compiler progress only");
  assert(checkpoint.overview.includes("Continuing full acceptance fails closed because authenticated draft KV catch-up is not implemented"));
  assert(checkpoint.overview.includes("All 33 M1 gates remain open; this checkpoint adds no GPU or performance result"));

  const selection = checkpoint.selection;
  exactKeys(selection, ["windows", "prefill", "residentSource", "ownerSource", "legacyCanonicalBytesPreserved", "detail"]);
  assert.deepEqual(selection.windows, [4, 8, 16]);
  assert.equal(selection.prefill, "S1/T128");
  assert.equal(selection.residentSource, "5c74b6f787a23e9afd70d2ab8e600992a0418a7c");
  assert.equal(selection.ownerSource, "624d9a2b6eacd90d49183738c5bdfce9274d63fb");
  assert.equal(selection.legacyCanonicalBytesPreserved, true);
  assert(selection.detail.includes("This does not add S8, continuous admission or new public authority"));

  const host = checkpoint.host;
  exactKeys(host, ["engine", "adapter", "verifierPassed", "numericalPassed", "strictClippyTargetsPassed",
    "policySource", "fastPolicyChecksPassed", "metadataGraphsPassed", "allocationFreePlanningSource",
    "allocationFreePlanningWindows", "sourceInventory", "service", "serviceQualified", "detail"]);
  assert.deepEqual(host.engine, { source: "9e4d818d5d7165453d005560f6c55ecd2eaca101", passed: 659, ignored: 9 });
  assert.deepEqual(host.adapter, { source: "9e4d818d5d7165453d005560f6c55ecd2eaca101", passed: 109, ignored: 1, policies: 37 });
  assert.equal(host.verifierPassed, 31);
  assert.equal(host.numericalPassed, 7);
  assert.equal(host.strictClippyTargetsPassed, 4);
  assert.equal(host.policySource, "2e073b548d1d398fede1cc574c310c73cdace5d7");
  assert.equal(host.fastPolicyChecksPassed, true);
  assert.equal(host.metadataGraphsPassed, 32);
  assert.equal(host.allocationFreePlanningSource, "8d6813f15383fb825b02cd38a84335995f77ecd8");
  assert.deepEqual(host.allocationFreePlanningWindows, [4, 8, 16]);
  assert.deepEqual(host.sourceInventory, {
    source: "dd5ade014af937f78ea20b8d317565ae9b0a615e",
    tree: "2e64f042aad3764f1769815dc81f8df90ad7843d",
    manifestSha256: "dc776165f3fc8e63d1d1afd1f1408d843b8c03cc60f9a42bd3e234866ed2bdbd",
    evidenceSha256: "f2d73e96953334cc7d9cf934288a6c15ea8b1d533d2a25a7ba02e82bfa17c5f0",
    equalityPassed: true, modules: 173, bodies: 8252, existingVerified: 722,
    unverified: 7530, addedUnverified: 18, removedUnverified: 1, newProofQualification: false,
  });
  assert.deepEqual(host.service, { source: "8d29d708c07170992d68af7a1b50146bc15b762f", passed: 16, policies: 3 });
  assert.equal(host.serviceQualified, false);
  assert(host.detail.includes("These checks do not qualify serving, new Verus proofs or GPU execution"));

  const compiler = checkpoint.compiler;
  exactKeys(compiler, ["testedSource", "passed", "ignored", "repairBase", "publishedSource",
    "observedLatestSource", "latestMigrationPending", "latestRetested",
    "publishedPassed", "publishedIgnored", "focusedSubsetPassed", "strictClippyPassed",
    "evidenceSha256", "aggregateSource", "aggregateHostPassed", "aggregateEvidenceSha256",
    "emissionLogSha256", "emissionExit", "emissionFailure", "emissionFailureRoot",
    "canonicalInvocationCount", "canonicalImageProduced", "detail"]);
  assert.equal(compiler.testedSource, "dc8e7f84fdcbc18fd85012f70f053f43c41f8fbe");
  assert.equal(compiler.passed, 964);
  assert.equal(compiler.ignored, 1);
  assert.equal(compiler.repairBase, "c76f844bb63c2b60cfc6e943612a51d450c4487c");
  assert.equal(compiler.publishedSource, "eaa057ac34f8fc3cfda3dd2c2cb7df058560de06");
  assert.equal(compiler.observedLatestSource, "e6cfa668f21e5b80d2e70730ed61602c1aaf5804");
  assert.equal(compiler.latestMigrationPending, true);
  assert.equal(compiler.latestRetested, false);
  assert.equal(compiler.publishedPassed, 1006);
  assert.equal(compiler.publishedIgnored, 0);
  assert.equal(compiler.focusedSubsetPassed, 13);
  assert.equal(compiler.strictClippyPassed, true);
  assert.equal(compiler.evidenceSha256, "de075f4d916170b7144f92b0d55b130fe2a5089cd861d0d7168a9f9c78021310");
  assert.equal(compiler.aggregateSource, "0b2d53a08bc8cf0639abfc3e4b833ac8e2a90148");
  assert.equal(compiler.aggregateHostPassed, 33);
  assert.equal(compiler.aggregateEvidenceSha256, "4655edd6b1cab3aca826509a49afa1d4a1d8d63c2ce28e134df80ef833a89653");
  assert.equal(compiler.emissionLogSha256, "8529eed10c7507dcdda9d8cf28e95fb2bdf97f1fcecf70250010bee2f3e09e05");
  assert.equal(compiler.emissionExit, 1);
  assert.equal(compiler.emissionFailure, "a division or remainder used for deterministic control lacks a statically nonzero divisor");
  assert.equal(compiler.emissionFailureRoot, null);
  assert.equal(compiler.canonicalInvocationCount, 77791232);
  assert.equal(compiler.canonicalImageProduced, false);
  assert(compiler.detail.includes("before producing an image"));
  assert(compiler.detail.includes("Kernel attribution is unknown; this is not identified as a GEMM failure"));
  assert(compiler.detail.includes("Exit 1 produces no image, replay or GPU result"));
  assert(compiler.detail.includes("The eaa results do not validate e6cfa668"));
  assert(checkpoint.remaining.includes("No native K8/K16 qualification, new TTFT/TPOT/throughput measurement or competitiveness claim"));
  assert(checkpoint.remaining.includes("Historical measurements below are unchanged"));
}

export function testResidentCheckpointRejections(value, updated) {
  for (const mutate of [
    (x) => { x.implementationPrivate = false; },
    (x) => { x.m1OpenGates = 32; },
    (x) => { x.nativeServingQualified = true; },
    (x) => { x.fullAcceptanceCatchupImplemented = true; },
    (x) => { x.newGpuMeasurement = true; },
    (x) => { x.newPerformanceMeasurement = true; },
    (x) => { x.selection.windows.push(32); },
    (x) => { x.selection.prefill = "S8/T128"; },
    (x) => { x.selection.legacyCanonicalBytesPreserved = false; },
    (x) => { x.host.engine.passed += 1; },
    (x) => { x.host.serviceQualified = true; },
    (x) => { x.host.service.passed += 1; },
    (x) => { x.host.allocationFreePlanningWindows.push(32); },
    (x) => { x.host.sourceInventory.newProofQualification = true; },
    (x) => { x.host.sourceInventory.equalityPassed = false; },
    (x) => { x.host.sourceInventory.existingVerified += 18; },
    (x) => { x.compiler.canonicalImageProduced = true; },
    (x) => { x.compiler.testedSource = x.compiler.repairBase; },
    (x) => { x.compiler.passed += 1; },
    (x) => { x.compiler.publishedPassed += x.compiler.focusedSubsetPassed; },
    (x) => { x.compiler.publishedSource = x.compiler.observedLatestSource; },
    (x) => { x.compiler.latestMigrationPending = false; },
    (x) => { x.compiler.latestRetested = true; },
    (x) => { x.compiler.strictClippyPassed = false; },
    (x) => { x.compiler.aggregateHostPassed += 1; },
    (x) => { x.compiler.emissionExit = 0; },
    (x) => { x.compiler.emissionFailureRoot = "GEMM"; },
    (x) => { x.overview = "Catch-up and all M1 gates are complete."; },
    (x) => { x.remaining = "New competitive performance result."; },
    (x) => { x.uncheckedAuthority = true; },
  ]) {
    const changed = JSON.parse(JSON.stringify(value));
    mutate(changed);
    assert.throws(() => validateResidentCheckpoint(changed, updated));
  }
  assert.throws(() => validateResidentCheckpoint(value, "2026-09-13"));
}
