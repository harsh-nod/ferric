import assert from "node:assert/strict";

function exactKeys(value, expected) {
  assert(value && typeof value === "object" && !Array.isArray(value));
  assert.deepEqual(Object.keys(value).sort(), [...expected].sort());
}

export function validateResidentCheckpoint(value, updated) {
  const checkpoint = JSON.parse(JSON.stringify(value));
  exactKeys(checkpoint, ["date", "scope", "implementationPrivate", "m1OpenGates",
    "nativeServingQualified", "fullAcceptanceCatchupImplemented", "newGpuObservation",
    "newPerformanceMeasurement", "overview", "integration", "followup", "selection", "host", "compiler", "remaining"]);
  assert.equal(updated, "2026-09-14");
  assert.equal(checkpoint.date, updated);
  assert.equal(checkpoint.implementationPrivate, true);
  assert.equal(checkpoint.m1OpenGates, 33);
  assert.equal(checkpoint.fullAcceptanceCatchupImplemented, true);
  assert.equal(checkpoint.newGpuObservation, true);
  for (const key of ["nativeServingQualified", "newPerformanceMeasurement"]) {
    assert.equal(checkpoint[key], false, key);
  }
  assert.equal(checkpoint.scope, "Private resident integration; source-bound engineering observations, not qualification");
  assert(checkpoint.overview.includes("draft KV catch-up is integrated and host-tested"));
  assert(checkpoint.overview.includes("All 33 M1 gates remain open; no new serving or performance result is claimed"));
  assert(checkpoint.overview.includes("one-prompt historical reference spot check"));

  const integration = checkpoint.integration;
  exactKeys(integration, ["compiler", "latestNativeSmoke", "nativeSmoke", "currentHost", "currentCoverage", "resident", "host", "coverage", "sourcePolicy", "prepack"]);
  const currentCompiler = integration.compiler;
  exactKeys(currentCompiler, ["source", "ferricSource", "backendPassed", "kernelHostPassed", "emissionState",
    "emissionExit", "target", "codeObjectVersion", "kernelEntries", "kernelDescriptors", "imageProduced",
    "imageBytes", "imageSha256", "manifestSha256", "nativeGpuTested", "exactOutputReplay", "authority", "grants",
    "evidenceSha256", "inspectionSha256", "detail", "differential"]);
  assert.equal(currentCompiler.source, "ccfd43d6bc58b27e0efe510b0ee1b8463166dfc2");
  assert.equal(currentCompiler.ferricSource, "bb2b0123f4f96410269a098d7cba8aa7bb950008");
  assert.deepEqual([currentCompiler.backendPassed, currentCompiler.kernelHostPassed], [543, 37]);
  assert.equal(currentCompiler.emissionState, "engineering-emitted");
  assert.equal(currentCompiler.emissionExit, 0);
  assert.equal(currentCompiler.target, "gfx942:xnack-");
  assert.equal(currentCompiler.codeObjectVersion, 6);
  assert.deepEqual([currentCompiler.kernelEntries, currentCompiler.kernelDescriptors], [12, 12]);
  assert.equal(currentCompiler.imageProduced, true);
  assert.equal(currentCompiler.imageBytes, 103872);
  assert.equal(currentCompiler.imageSha256, "46335b09a921b33ed66392e415ce3d2348dd63042242c0387d5a58362ba3c84c");
  assert.equal(currentCompiler.manifestSha256, "339ea2c2c4528a28d7ca16085a31c57070a304b4a2b5454ef118e4ce0a0d66a2");
  assert.equal(currentCompiler.nativeGpuTested, true);
  assert.equal(currentCompiler.exactOutputReplay, true);
  assert.equal(currentCompiler.authority, "none");
  assert.deepEqual(currentCompiler.grants, { publication: false, load: false, launch: false });
  assert.equal(currentCompiler.evidenceSha256, "3eb1c0834b2ab957efa13b063921a25516713c4bb1a0c89cb0cd5fe1fa828107");
  assert.equal(currentCompiler.inspectionSha256, "4e760c7cd716a3d73a11d68a19efe62ab03e8f8fb508f8b0e5861181bad11a3a");
  assert(currentCompiler.detail.includes("all 12 kernel entries and 12 matching descriptors"));
  assert(currentCompiler.detail.includes("structural symbol sets, not Ferric policy descriptor-table order or numerical execution"));
  assert(currentCompiler.detail.includes("Publication, load and launch grants all remain false; engineering authority is none"));
  assert(currentCompiler.detail.includes("newer image differs from c6 and has its own separate four-token native observation"));
  assert(currentCompiler.detail.includes("earlier native smoke is not relabeled as ccfd"));
  assert(currentCompiler.detail.includes("Emission alone establishes no model-parity, serving or performance result"));
  const differential = currentCompiler.differential;
  exactKeys(differential, ["comparedAssertions", "proofGains", "proofLosses", "priorUnprovedBlock",
    "priorUnprovedLine", "attributedExpression", "evidenceSha256", "detail"]);
  assert.deepEqual([differential.comparedAssertions, differential.proofGains, differential.proofLosses], [49, 1, 0]);
  assert.deepEqual([differential.priorUnprovedBlock, differential.priorUnprovedLine], ["bb97", 373]);
  assert.equal(differential.attributedExpression, "committed_tokens + query_token");
  assert.equal(differential.evidenceSha256, "6916aeae89906f74bef47c7ad422dbfe34fd4185e77d7b96728ff1750c4231a7");
  assert(differential.detail.includes("49 assertions, one false-to-true proof gain and zero true-to-false losses"));
  assert(differential.detail.includes("already unproved, not a regression introduced by 852"));
  assert(differential.detail.includes("all producer assertions and limits remain mandatory"));
  assert(differential.detail.includes("earlier 12 GiB resource stop and rejected extractions remain retained separately"));
  const { detail: latestNativeDetail, cleanupDetail, ...latestNativeFields } = integration.latestNativeSmoke;
  assert.deepEqual(latestNativeFields, {
    source: "bb2b0123f4f96410269a098d7cba8aa7bb950008",
    compilerSource: "ccfd43d6bc58b27e0efe510b0ee1b8463166dfc2",
    binarySha256: "fe475d52f5b3b6f3ee5c35a3c947874b03f35c463f0e71c10f118de581759eba",
    imageSha256: "46335b09a921b33ed66392e415ce3d2348dd63042242c0387d5a58362ba3c84c",
    evidenceSha256: "2a9b13d81098a878ad1f299a7f4f999c368c16aca1a7f906cd9dc283f1e9dffc",
    reportSha256: "4d02407107a46ac88c1b621fa64d099779f77d9f341a77b9e1136c65e548c00e",
    referenceResultSha256: "1ed868663df52a146dd7921f9fbb2cf1e1d0c0ceed8a7bfda030d65f8ec5b094",
    target: "gfx942:xnack-", promptIds: [785, 6722, 315, 9625, 374],
    outputIds: [12095, 13, 576, 6722], outputText: " Paris. The capital",
    termination: "max-new-tokens", exit: 0, hardwareCompletionObserved: true,
    historicalReferencePrefixMatched: true, authority: "none", benchmarkComparable: false,
    workerV3Authenticated: false, compilerOriginAuthenticated: false, currentPublicationSelected: false,
    generalNumericalParity: false, r29LogitComparison: false, r33Qualified: false,
    immediateDeviceMemoryBytes: 56838090752, laterDeviceMemoryBytes: 298647552,
    immediateBaselineRestored: false, laterBaselineRestored: true,
    cleanupObservation: "later-independent-sysfs-check", deviceReset: false,
  });
  assert.equal(latestNativeFields.source, currentCompiler.ferricSource);
  assert.equal(latestNativeFields.imageSha256, currentCompiler.imageSha256);
  assert(latestNativeDetail.includes("four IDs [12095, 13, 576, 6722], text \" Paris. The capital\""));
  assert(latestNativeDetail.includes("one-prompt token-prefix spot check, not an R29 logit comparison"));
  assert(latestNativeDetail.includes("general numerical validation, speculative catch-up or R33 serving qualification"));
  assert(latestNativeDetail.includes("Controller offsets are not comparable TTFT, TPOT or throughput"));
  assert(cleanupDetail.includes("immediate archived after-state still reports 56,838,090,752 VRAM bytes"));
  assert(cleanupDetail.includes("later independent direct sysfs check observes return to the exact 298,647,552-byte baseline"));
  assert(cleanupDetail.includes("delayed check is separate from the immutable archive; the original after-state is not rewritten"));
  assert(cleanupDetail.includes("No device reset or foreign-process termination was performed"));
  const { detail: nativeDetail, ...nativeFields } = integration.nativeSmoke;
  assert.deepEqual(nativeFields, {
    binarySource: "6651f2d1a1739f30c35a4a5c34d2c900be1c5d06",
    compilerSource: "c6b4050dd6c18e1868b24e4580da3eed8a3d19fa",
    artifactSource: "f17efe83d37105c0da78ea2c8bbbe166a0ee4f43",
    binarySha256: "18ff100fde890a25bbeb0588c0723e741674f737a6a31af5a37e4e2255a29e43",
    imageSha256: "9e04fe0c8c7682146b9e42d21c83d5d77336d08b7d599cbd86a012eb9eef69a1",
    evidenceSha256: "0dc8ca2a724bde837750bae565a4224feae03e6f88fde50913ed35c58199999d",
    reportSha256: "e6cf51add84b633e57b05f4a2a92ba71e6ce5a91642c58f3ac27a4f658171d62",
    target: "gfx942:xnack-", prompt: "The capital of France is",
    promptIds: [785, 6722, 315, 9625, 374], outputIds: [12095], outputText: " Paris", exit: 0,
    hardwareCompletionObserved: true, deviceMemoryBaselineRestored: true,
    authority: "none", benchmarkComparable: false, workerV3Authenticated: false,
    compilerOriginAuthenticated: false, currentPublicationSelected: false,
    generalNumericalParity: false, servingQualified: false,
  });
  assert(nativeDetail.includes("five-token prompt \"The capital of France is\" produces one token: ID 12095, \" Paris\""));
  assert(nativeDetail.includes("earlier f17/c6 image, not the newer bb2/ccfd image"));
  assert(nativeDetail.includes("do not establish general numerical parity, speculative catch-up, serving readiness, performance or M1 qualification"));
  assert(nativeDetail.includes("Raw controller offsets are not serving TTFT/TPOT and are excluded from the performance data"));
  const { detail: currentHostDetail, ...currentHostFields } = integration.currentHost;
  assert.deepEqual(currentHostFields, {
    source: "bb2b0123f4f96410269a098d7cba8aa7bb950008",
    compilerSource: "ccfd43d6bc58b27e0efe510b0ee1b8463166dfc2",
    phasesPassed: 71, specPassed: 123, enginePassed: 697, engineIgnored: 9,
    binaryPassed: [11, 2, 75, 2], binaryIgnored: [0, 0, 2, 0], engineDoctestsPassed: 171,
    checkerPassed: 81, checkerIntegrationPassed: 27, checkerIgnored: 3, checkerDoctestsPassed: 6,
    normalHostFeatures: [], adapterPassed: 109, adapterIgnored: 1, adapterPoliciesPassed: 37,
    numericalHostPassed: 7, ownerPassed: 16, ownerPoliciesPassed: 3, lockedGraphs: 32,
    sourceGatePassed: 38, verifierPoliciesPassed: 31, sourcePinPoliciesPassed: 6, dependencyInventoriesEqual: 3,
    strictClippyPackages: ["spec", "engine", "checker", "engineering-adapter", "protected-owner"],
    releaseBinaryNames: ["ferric-m1-engineering-target-smoke", "ferric-m1-engineering-speculative-smoke", "ferric-m1-engineering-r29-capture"],
    releaseBinarySha256: ["fe475d52f5b3b6f3ee5c35a3c947874b03f35c463f0e71c10f118de581759eba", "3756e98b53833c9cda5603ddf572a66c0c69a926fa9d77b825fcda6dd4040a05", "fbc0e505782252308e0920f9b1c4edd47ff3d937e90905eb31da8580dad66e9c"],
    evidenceSha256: "4848bf1306627d8c4b932ac75decf83d9efd20252b2183bb8699e89cf7536fe9",
    releaseEvidenceSha256: "4a18fe27f2449896169dfc2b9e9426a959695ac9253b37ad0cab40e7866fa0b2",
    gpuExecution: false, servingQualified: false,
  });
  assert(currentHostDetail.includes("all 71 host and metadata phases with exit 0"));
  assert(currentHostDetail.includes("normal fe2o3_host artifact has features=[]"));
  assert(currentHostDetail.includes("building them is not GPU execution or qualification"));
  assert(currentHostDetail.includes("Earlier cohorts below retain their own source identities"));
  const { detail: currentCoverageDetail, ...currentCoverageFields } = integration.currentCoverage;
  assert.deepEqual(currentCoverageFields, {
    source: "d09430897232759fb9bbb2a3485f60da69bef890",
    compilerSource: "ccfd43d6bc58b27e0efe510b0ee1b8463166dfc2",
    modules: 173, identities: 8435, verifiedLabels: 723, previousVerifiedPreserved: 722,
    unverified: 7712, metadataPhasesPassed: 8, committedManifestEqual: true,
    wholeCrateVerified: false, physicalCatchupVerified: false,
    evidenceSha256: "209c1e49d96e91a93010e248a444db0f2b2e92bbba8ee7a801aee899b04d179d",
  });
  assert(currentCoverageDetail.includes("helper and six executed callees verified individually"));
  assert(currentCoverageDetail.includes("seven sensitive executable mutations rejected"));
  assert(currentCoverageDetail.includes("not whole-crate verification or GPU/page-lease/queue-completion refinement"));
  assert(currentCoverageDetail.includes("engine preflight and complete physical catch-up chain remain unverified"));
  const currentResident = integration.resident;
  exactKeys(currentResident, ["windows", "dispatchIntegrated", "focusedPassed", "focusedCompilerSource",
    "maintenanceServedTokens", "warmedAllocationValidated", "detail"]);
  assert.deepEqual(currentResident.windows, [4, 8, 16]);
  assert.equal(currentResident.dispatchIntegrated, true);
  assert.equal(currentResident.focusedPassed, 20);
  assert.equal(currentResident.focusedCompilerSource, "82950a3cfc7b0192159fb7afcd1b89fb48d56053");
  assert.equal(currentResident.maintenanceServedTokens, 0);
  assert.equal(currentResident.warmedAllocationValidated, false);
  assert(currentResident.detail.includes("missing last draft candidate before the next speculative round"));
  assert(currentResident.detail.includes("not a native model-serving or warmed allocation-free qualification"));
  const currentHost = integration.host;
  exactKeys(currentHost, ["source", "compilerSource", "enginePassed", "engineIgnored", "binaryPassed",
    "binaryIgnored", "doctestsPassed", "strictEngineClippyPassed", "strictAdapterClippyPassed", "strictOwnerClippyPassed",
    "generatedRunnerSourceEqualityPassed",
    "adapterPassed", "adapterIgnored", "adapterPoliciesPassed", "ownerPassed", "ownerPoliciesPassed", "detail"]);
  assert.equal(currentHost.source, "d55fd00a44a01dce3f4c8c790b4736e2440a69fa");
  assert.equal(currentHost.compilerSource, "85255498a021ed2b7a1a511f99f577ee4b11d175");
  assert.deepEqual([currentHost.enginePassed, currentHost.engineIgnored, currentHost.doctestsPassed], [697, 9, 171]);
  assert.deepEqual(currentHost.binaryPassed, [11, 2, 75, 2]);
  assert.deepEqual(currentHost.binaryIgnored, [0, 0, 2, 0]);
  assert.equal(currentHost.strictEngineClippyPassed, true);
  assert.equal(currentHost.strictAdapterClippyPassed, true);
  assert.equal(currentHost.strictOwnerClippyPassed, true);
  assert.equal(currentHost.generatedRunnerSourceEqualityPassed, true);
  assert.deepEqual([currentHost.adapterPassed, currentHost.adapterIgnored, currentHost.adapterPoliciesPassed,
    currentHost.ownerPassed, currentHost.ownerPoliciesPassed], [109, 1, 37, 16, 3]);
  assert(currentHost.detail.includes("generated runner source equality; this is not a native runner or image"));
  assert(currentHost.detail.includes("do not include the earlier producer's prepack work or constitute hardware qualification"));
  const coverage = integration.coverage;
  exactKeys(coverage, ["modules", "identities", "existingVerified", "unverified", "addedPending",
    "removed", "changedStatus", "newProofQualification", "evidenceSha256", "detail"]);
  assert.deepEqual([coverage.modules, coverage.identities, coverage.existingVerified, coverage.unverified,
    coverage.addedPending, coverage.removed, coverage.changedStatus], [173, 8433, 722, 7711, 181, 0, 0]);
  assert.equal(coverage.identities, coverage.existingVerified + coverage.unverified);
  assert.equal(coverage.newProofQualification, false);
  assert.equal(coverage.evidenceSha256, "5001522110d67c40ac177aae320da36f842092e148dce0f752818eac87190ad7");
  assert(coverage.detail.includes("722 existing verified labels remain unchanged"));
  assert(coverage.detail.includes("not fresh whole-crate verification or a proof of physical catch-up"));
  const sourcePolicy = integration.sourcePolicy;
  exactKeys(sourcePolicy, ["compilerSource", "lockedGraphs", "sourceGatePassed", "verifierPassed",
    "sourcePinPassed", "dependencyInventoriesEqual", "evidenceSha256", "detail"]);
  assert.equal(sourcePolicy.compilerSource, "85255498a021ed2b7a1a511f99f577ee4b11d175");
  assert.deepEqual([sourcePolicy.lockedGraphs, sourcePolicy.sourceGatePassed, sourcePolicy.verifierPassed,
    sourcePolicy.sourcePinPassed, sourcePolicy.dependencyInventoriesEqual], [32, 38, 31, 6, 3]);
  assert.equal(sourcePolicy.evidenceSha256, "1074fbf60b9643f13044c13ae2cd12a00502644371ab69a98efec38fce327f63");
  assert(sourcePolicy.detail.includes("No dependency admission or proof-status upgrade was needed"));
  const prepack = integration.prepack;
  exactKeys(prepack, ["producerSource", "compilerSource", "roles", "fullCanonicalPrepackPassed",
    "reopenVerificationPassed", "hardwareExecution", "detail"]);
  assert.equal(prepack.producerSource, "bed11d00e49baae326a71cf3a7df36bf22336548");
  assert.equal(prepack.compilerSource, "82950a3cfc7b0192159fb7afcd1b89fb48d56053");
  assert.deepEqual(prepack.roles, ["Target8B", "Draft06B"]);
  assert.equal(prepack.fullCanonicalPrepackPassed, true);
  assert.equal(prepack.reopenVerificationPassed, true);
  assert.equal(prepack.hardwareExecution, false);
  assert(prepack.detail.includes("exact frozen bed11/829 CLI, not a relabeled 852 binary"));
  assert(prepack.detail.includes("do not execute GPU kernels, establish model output parity or provide latency measurements"));

  const followup = checkpoint.followup;
  exactKeys(followup, ["compilerDiagnostic", "coordinateRepair", "runtimeGetter", "cursorProof", "catchup"]);
  const diagnostic = followup.compilerDiagnostic;
  exactKeys(diagnostic, ["source", "rebaseBase", "backendPassed", "lineagePassed", "analysisPassed", "irPassed", "detail"]);
  assert.equal(diagnostic.source, "ae441734f27ef35c26baf7b3a6281852cd544de9");
  assert.equal(diagnostic.rebaseBase, "836afb2841f92fae4f9675a91947de23a3faa1a9");
  assert.deepEqual([diagnostic.backendPassed, diagnostic.lineagePassed, diagnostic.analysisPassed, diagnostic.irPassed], [534, 30, 97, 291]);
  assert(diagnostic.detail.includes("without weakening the nonzero-divisor check"));
  assert(diagnostic.detail.includes("paged GQA coordinate calculation at line 322, rather than GEMM"));
  const coordinate = followup.coordinateRepair;
  exactKeys(coordinate, ["source", "hostPassed", "equivalentProfiles", "emissionExit", "emissionBlock", "emissionSourceFingerprint", "emissionLine", "exactRootEmitted", "imageProduced", "compilerFixValidated", "evidenceSha256", "detail"]);
  assert.equal(coordinate.source, "6da025e3017c555f4425a4fc26c0cab0805d7811");
  assert.deepEqual([coordinate.hostPassed, coordinate.equivalentProfiles, coordinate.emissionExit], [35, 14, 1]);
  assert.deepEqual([coordinate.emissionBlock, coordinate.emissionSourceFingerprint, coordinate.emissionLine], ["bb81", "09ab2715abb3", 378]);
  assert.equal(coordinate.exactRootEmitted, false);
  assert.equal(coordinate.imageProduced, false);
  assert.equal(coordinate.compilerFixValidated, false);
  assert.equal(coordinate.evidenceSha256, "cf6e686834c3f7ce714d3dfc83c8e49e1c7cc734cc5ddcbe4c1e1e3300258b1e");
  assert(coordinate.detail.includes("actual AST equivalence across 14 profiles"));
  assert(coordinate.detail.includes("diagnostic attribution, not a validated compiler repair"));
  assert(coordinate.detail.includes("No image, replay or GPU result was produced"));
  const getter = followup.runtimeGetter;
  exactKeys(getter, ["source", "ferricPinSource", "servicePassed", "qualificationPassed", "pureKfdPassed", "combinedEngineValidationPending", "detail"]);
  assert.equal(getter.source, "b75096cf947fcb45dd9a579b68b0372882282e6d");
  assert.equal(getter.ferricPinSource, "7c3f9729e6af1747a02e2392a9728305f5f53078");
  assert.deepEqual([getter.servicePassed, getter.qualificationPassed, getter.pureKfdPassed], [43, 45, 3]);
  assert.equal(getter.combinedEngineValidationPending, true);
  assert(getter.detail.includes("earlier green host suites are not relabeled as current results"));
  const proof = followup.cursorProof;
  exactKeys(proof, ["source", "compilerSource", "module", "verusVersion", "verified", "errors", "closureFiles", "wholeCrateVerified", "physicalCatchupVerified", "evidenceSha256", "detail"]);
  assert.equal(proof.source, "bd2f2464f7583e8b35d7d250a761f40b8808fb3c");
  assert.equal(proof.compilerSource, "e6cfa668f21e5b80d2e70730ed61602c1aaf5804");
  assert.equal(proof.module, "kv_physical");
  assert.equal(proof.verusVersion, "0.2026.08.02.b677dd5");
  assert.deepEqual([proof.verified, proof.errors, proof.closureFiles], [4, 0, 190]);
  assert.equal(proof.wholeCrateVerified, false);
  assert.equal(proof.physicalCatchupVerified, false);
  assert.equal(proof.evidenceSha256, "4d58e31661411697cc82bd09eb0ce055374f71f2cea65b911d64a88b0457dc57");
  assert(proof.detail.includes("not whole-crate verification or a proof of physical catch-up, queue ownership or device execution"));
  const catchup = followup.catchup;
  exactKeys(catchup, ["windows", "privateComponentsImplemented", "residentDispatchIntegrated", "warmedAllocationValidated", "detail"]);
  assert.deepEqual(catchup.windows, [4, 8, 16]);
  assert.equal(catchup.privateComponentsImplemented, true);
  assert.equal(catchup.residentDispatchIntegrated, false);
  assert.equal(catchup.warmedAllocationValidated, false);
  assert(catchup.detail.includes("advances draft KV by one and emits no served token"));
  assert(catchup.detail.includes("No warmed allocation-free catch-up or end-to-end serving result is claimed"));

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
  assert(checkpoint.remaining.includes("Current-pin engine and checker host validation pass"));
  assert(checkpoint.remaining.includes("neither target-only observation qualifies resident speculative serving"));
}

export function testResidentCheckpointRejections(value, updated) {
  for (const mutate of [
    (x) => { x.implementationPrivate = false; },
    (x) => { x.m1OpenGates = 32; },
    (x) => { x.nativeServingQualified = true; },
    (x) => { x.fullAcceptanceCatchupImplemented = false; },
    (x) => { x.newGpuObservation = false; },
    (x) => { x.newPerformanceMeasurement = true; },
    (x) => { x.integration.compiler.source = x.followup.runtimeGetter.source; },
    (x) => { x.integration.compiler.backendPassed += 1; },
    (x) => { x.integration.compiler.kernelHostPassed += 1; },
    (x) => { x.integration.compiler.emissionState = "produced"; },
    (x) => { x.integration.compiler.ferricSource = x.integration.host.source; },
    (x) => { x.integration.compiler.emissionExit = 1; },
    (x) => { x.integration.compiler.target = "gfx950:xnack-"; },
    (x) => { x.integration.compiler.codeObjectVersion = 5; },
    (x) => { x.integration.compiler.kernelEntries += 1; },
    (x) => { x.integration.compiler.kernelDescriptors -= 1; },
    (x) => { x.integration.compiler.imageProduced = false; },
    (x) => { x.integration.compiler.imageBytes += 1; },
    (x) => { x.integration.compiler.imageSha256 = x.integration.compiler.manifestSha256; },
    (x) => { x.integration.compiler.manifestSha256 = x.integration.compiler.imageSha256; },
    (x) => { x.integration.compiler.nativeGpuTested = false; },
    (x) => { x.integration.compiler.exactOutputReplay = false; },
    (x) => { x.integration.compiler.authority = "qualified"; },
    (x) => { x.integration.compiler.grants.publication = true; },
    (x) => { x.integration.compiler.grants.load = true; },
    (x) => { x.integration.compiler.grants.launch = true; },
    (x) => { x.integration.compiler.evidenceSha256 = x.integration.compiler.inspectionSha256; },
    (x) => { x.integration.compiler.inspectionSha256 = x.integration.compiler.evidenceSha256; },
    (x) => { x.integration.compiler.differential.comparedAssertions += 1; },
    (x) => { x.integration.compiler.differential.proofGains = 0; },
    (x) => { x.integration.compiler.differential.proofLosses = 1; },
    (x) => { x.integration.compiler.differential.priorUnprovedBlock = "bb81"; },
    (x) => { x.integration.compiler.differential.priorUnprovedLine = 378; },
    (x) => { x.integration.compiler.differential.attributedExpression = "query_position + 1"; },
    (x) => { x.integration.compiler.differential.evidenceSha256 = x.integration.compiler.evidenceSha256; },
    (x) => { x.integration.latestNativeSmoke.source = x.integration.nativeSmoke.binarySource; },
    (x) => { x.integration.latestNativeSmoke.compilerSource = x.integration.nativeSmoke.compilerSource; },
    (x) => { x.integration.latestNativeSmoke.binarySha256 = x.integration.nativeSmoke.binarySha256; },
    (x) => { x.integration.latestNativeSmoke.imageSha256 = x.integration.nativeSmoke.imageSha256; },
    (x) => { x.integration.latestNativeSmoke.evidenceSha256 = x.integration.latestNativeSmoke.reportSha256; },
    (x) => { x.integration.latestNativeSmoke.reportSha256 = x.integration.latestNativeSmoke.evidenceSha256; },
    (x) => { x.integration.latestNativeSmoke.referenceResultSha256 = x.integration.latestNativeSmoke.reportSha256; },
    (x) => { x.integration.latestNativeSmoke.target = "gfx950:xnack-"; },
    (x) => { x.integration.latestNativeSmoke.promptIds.pop(); },
    (x) => { x.integration.latestNativeSmoke.outputIds.pop(); },
    (x) => { x.integration.latestNativeSmoke.outputText = " Paris"; },
    (x) => { x.integration.latestNativeSmoke.termination = "eos"; },
    (x) => { x.integration.latestNativeSmoke.exit = 1; },
    (x) => { x.integration.latestNativeSmoke.hardwareCompletionObserved = false; },
    (x) => { x.integration.latestNativeSmoke.historicalReferencePrefixMatched = false; },
    (x) => { x.integration.latestNativeSmoke.authority = "qualified"; },
    (x) => { x.integration.latestNativeSmoke.benchmarkComparable = true; },
    (x) => { x.integration.latestNativeSmoke.workerV3Authenticated = true; },
    (x) => { x.integration.latestNativeSmoke.compilerOriginAuthenticated = true; },
    (x) => { x.integration.latestNativeSmoke.currentPublicationSelected = true; },
    (x) => { x.integration.latestNativeSmoke.generalNumericalParity = true; },
    (x) => { x.integration.latestNativeSmoke.r29LogitComparison = true; },
    (x) => { x.integration.latestNativeSmoke.r33Qualified = true; },
    (x) => { x.integration.latestNativeSmoke.immediateDeviceMemoryBytes = x.integration.latestNativeSmoke.laterDeviceMemoryBytes; },
    (x) => { x.integration.latestNativeSmoke.laterDeviceMemoryBytes += 1; },
    (x) => { x.integration.latestNativeSmoke.immediateBaselineRestored = true; },
    (x) => { x.integration.latestNativeSmoke.laterBaselineRestored = false; },
    (x) => { x.integration.latestNativeSmoke.cleanupObservation = "immediate-archived-after-state"; },
    (x) => { x.integration.latestNativeSmoke.deviceReset = true; },
    (x) => { x.integration.latestNativeSmoke.detail = "General model parity established."; },
    (x) => { x.integration.latestNativeSmoke.cleanupDetail = "Immediate archive showed baseline memory."; },
    (x) => { x.integration.nativeSmoke.binarySource = x.integration.currentHost.source; },
    (x) => { x.integration.nativeSmoke.compilerSource = x.integration.compiler.source; },
    (x) => { x.integration.nativeSmoke.artifactSource = x.integration.compiler.ferricSource; },
    (x) => { x.integration.nativeSmoke.binarySha256 = x.integration.currentHost.releaseBinarySha256[0]; },
    (x) => { x.integration.nativeSmoke.imageSha256 = x.integration.compiler.imageSha256; },
    (x) => { x.integration.nativeSmoke.evidenceSha256 = x.integration.nativeSmoke.reportSha256; },
    (x) => { x.integration.nativeSmoke.reportSha256 = x.integration.nativeSmoke.evidenceSha256; },
    (x) => { x.integration.nativeSmoke.target = "gfx950:xnack-"; },
    (x) => { x.integration.nativeSmoke.prompt = "Paris"; },
    (x) => { x.integration.nativeSmoke.promptIds.pop(); },
    (x) => { x.integration.nativeSmoke.outputIds[0] += 1; },
    (x) => { x.integration.nativeSmoke.outputIds.push(13); },
    (x) => { x.integration.nativeSmoke.outputText = "Paris"; },
    (x) => { x.integration.nativeSmoke.exit = 1; },
    (x) => { x.integration.nativeSmoke.hardwareCompletionObserved = false; },
    (x) => { x.integration.nativeSmoke.deviceMemoryBaselineRestored = false; },
    (x) => { x.integration.nativeSmoke.authority = "qualified"; },
    (x) => { x.integration.nativeSmoke.benchmarkComparable = true; },
    (x) => { x.integration.nativeSmoke.workerV3Authenticated = true; },
    (x) => { x.integration.nativeSmoke.compilerOriginAuthenticated = true; },
    (x) => { x.integration.nativeSmoke.currentPublicationSelected = true; },
    (x) => { x.integration.nativeSmoke.generalNumericalParity = true; },
    (x) => { x.integration.nativeSmoke.servingQualified = true; },
    (x) => { x.integration.nativeSmoke.detail = "Numerically qualified serving."; },
    (x) => { x.integration.currentHost.source = x.integration.host.source; },
    (x) => { x.integration.currentHost.compilerSource = x.integration.host.compilerSource; },
    (x) => { x.integration.currentHost.phasesPassed += 1; },
    (x) => { x.integration.currentHost.specPassed += 1; },
    (x) => { x.integration.currentHost.enginePassed += 1; },
    (x) => { x.integration.currentHost.engineIgnored = 0; },
    (x) => { x.integration.currentHost.binaryPassed[2] += 2; },
    (x) => { x.integration.currentHost.binaryIgnored[2] = 0; },
    (x) => { x.integration.currentHost.engineDoctestsPassed += 1; },
    (x) => { x.integration.currentHost.checkerPassed += 1; },
    (x) => { x.integration.currentHost.checkerIntegrationPassed += 1; },
    (x) => { x.integration.currentHost.checkerIgnored = 0; },
    (x) => { x.integration.currentHost.checkerDoctestsPassed += 1; },
    (x) => { x.integration.currentHost.normalHostFeatures.push("testing"); },
    (x) => { x.integration.currentHost.numericalHostPassed += 1; },
    (x) => { x.integration.currentHost.dependencyInventoriesEqual = 2; },
    (x) => { x.integration.currentHost.strictClippyPackages.pop(); },
    (x) => { x.integration.currentHost.releaseBinaryNames.pop(); },
    (x) => { x.integration.currentHost.releaseBinarySha256[0] = x.integration.nativeSmoke.binarySha256; },
    (x) => { x.integration.currentHost.evidenceSha256 = x.integration.currentHost.releaseEvidenceSha256; },
    (x) => { x.integration.currentHost.gpuExecution = true; },
    (x) => { x.integration.currentHost.servingQualified = true; },
    (x) => { x.integration.currentHost.detail = "Production serving qualified."; },
    (x) => { x.integration.currentCoverage.source = x.integration.currentHost.source; },
    (x) => { x.integration.currentCoverage.identities += 1; },
    (x) => { x.integration.currentCoverage.verifiedLabels += 6; },
    (x) => { x.integration.currentCoverage.previousVerifiedPreserved += 1; },
    (x) => { x.integration.currentCoverage.unverified = 0; },
    (x) => { x.integration.currentCoverage.metadataPhasesPassed += 1; },
    (x) => { x.integration.currentCoverage.committedManifestEqual = false; },
    (x) => { x.integration.currentCoverage.wholeCrateVerified = true; },
    (x) => { x.integration.currentCoverage.physicalCatchupVerified = true; },
    (x) => { x.integration.currentCoverage.evidenceSha256 = x.integration.currentHost.evidenceSha256; },
    (x) => { x.integration.currentCoverage.detail = "Whole physical catch-up verified."; },
    (x) => { x.integration.resident.windows.push(32); },
    (x) => { x.integration.resident.dispatchIntegrated = false; },
    (x) => { x.integration.resident.focusedPassed += 1; },
    (x) => { x.integration.resident.focusedCompilerSource = x.integration.compiler.source; },
    (x) => { x.integration.resident.maintenanceServedTokens = 1; },
    (x) => { x.integration.resident.warmedAllocationValidated = true; },
    (x) => { x.integration.host.source = x.integration.prepack.producerSource; },
    (x) => { x.integration.host.compilerSource = x.integration.prepack.compilerSource; },
    (x) => { x.integration.host.compilerSource = x.integration.compiler.source; },
    (x) => { x.integration.host.enginePassed += 1; },
    (x) => { x.integration.host.engineIgnored = 0; },
    (x) => { x.integration.host.binaryPassed[2] += 2; },
    (x) => { x.integration.host.binaryIgnored[2] = 0; },
    (x) => { x.integration.host.doctestsPassed += 1; },
    (x) => { x.integration.host.strictEngineClippyPassed = false; },
    (x) => { x.integration.host.strictAdapterClippyPassed = false; },
    (x) => { x.integration.host.strictOwnerClippyPassed = false; },
    (x) => { x.integration.host.generatedRunnerSourceEqualityPassed = false; },
    (x) => { x.integration.host.adapterPassed += 1; },
    (x) => { x.integration.host.adapterIgnored = 0; },
    (x) => { x.integration.host.adapterPoliciesPassed += 1; },
    (x) => { x.integration.host.ownerPassed += 1; },
    (x) => { x.integration.host.ownerPoliciesPassed += 1; },
    (x) => { x.integration.coverage.existingVerified += 181; },
    (x) => { x.integration.coverage.unverified -= 181; },
    (x) => { x.integration.coverage.addedPending = 0; },
    (x) => { x.integration.coverage.removed = 1; },
    (x) => { x.integration.coverage.changedStatus = 181; },
    (x) => { x.integration.coverage.newProofQualification = true; },
    (x) => { x.integration.coverage.evidenceSha256 = x.integration.sourcePolicy.evidenceSha256; },
    (x) => { x.integration.sourcePolicy.compilerSource = x.integration.prepack.compilerSource; },
    (x) => { x.integration.sourcePolicy.compilerSource = x.integration.compiler.source; },
    (x) => { x.integration.sourcePolicy.lockedGraphs += 1; },
    (x) => { x.integration.sourcePolicy.sourceGatePassed += 1; },
    (x) => { x.integration.sourcePolicy.verifierPassed += 1; },
    (x) => { x.integration.sourcePolicy.sourcePinPassed += 1; },
    (x) => { x.integration.sourcePolicy.dependencyInventoriesEqual = 2; },
    (x) => { x.integration.prepack.producerSource = x.integration.host.source; },
    (x) => { x.integration.prepack.compilerSource = x.integration.compiler.source; },
    (x) => { x.integration.prepack.roles.pop(); },
    (x) => { x.integration.prepack.fullCanonicalPrepackPassed = false; },
    (x) => { x.integration.prepack.reopenVerificationPassed = false; },
    (x) => { x.integration.prepack.hardwareExecution = true; },
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
    (x) => { x.followup.compilerDiagnostic.backendPassed += 1; },
    (x) => { x.followup.compilerDiagnostic.rebaseBase = x.followup.compilerDiagnostic.source; },
    (x) => { x.followup.coordinateRepair.hostPassed += 1; },
    (x) => { x.followup.coordinateRepair.equivalentProfiles += 1; },
    (x) => { x.followup.coordinateRepair.emissionExit = 0; },
    (x) => { x.followup.coordinateRepair.exactRootEmitted = true; },
    (x) => { x.followup.coordinateRepair.imageProduced = true; },
    (x) => { x.followup.coordinateRepair.compilerFixValidated = true; },
    (x) => { x.followup.runtimeGetter.combinedEngineValidationPending = false; },
    (x) => { x.followup.cursorProof.compilerSource = x.followup.runtimeGetter.source; },
    (x) => { x.followup.cursorProof.verified += 1; },
    (x) => { x.followup.cursorProof.wholeCrateVerified = true; },
    (x) => { x.followup.cursorProof.physicalCatchupVerified = true; },
    (x) => { x.followup.catchup.residentDispatchIntegrated = true; },
    (x) => { x.followup.catchup.warmedAllocationValidated = true; },
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
