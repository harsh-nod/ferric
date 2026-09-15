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
  assert.equal(updated, "2026-09-15");
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
  assert(checkpoint.overview.includes("rows are not byte-identical and no tolerance is accepted"));
  assert(checkpoint.overview.includes("that image is not GPU-tested"));
  assert(checkpoint.overview.includes("actual GPU comparisons remain 1 of 7"));
  assert(checkpoint.overview.includes("completes five real K4 rounds and eight tokens with zero/partial acceptance"));
  assert(checkpoint.overview.includes("full-acceptance catch-up/restore is not observed"));
  assert(checkpoint.overview.includes("no new MFMA image or GPU result exists"));

  const integration = checkpoint.integration;
  exactKeys(integration, ["hostProgress", "sourceCoverage", "nativeResidentAttempt", "optimizedResidentBuild", "nativeResident", "selectedNumerical", "latestCompiler", "compiler", "latestNativeSmoke", "nativeSmoke", "currentHost", "currentCoverage", "resident", "host", "coverage", "sourcePolicy", "prepack"]);
  const progress = integration.hostProgress;
  exactKeys(progress, ["r29", "resident", "mfma", "compilerCandidate", "compiler", "boundary"]);
  const expectedProgress = {
    r29: {
      source: "940d37e253841a17741904e814860e110c230b07",
      integratedSource: "10cc6587d198a315678c8e1d939aa8e5163a32f5",
      evidenceSha256: "a05d9b1a63950a01be2334f087b22c7477ccc1f259d75df8d449cbf09bc064dc",
      comparisonTests: 24, engineeringReferenceTests: 15, legacyReferenceTests: 23,
      strictClippyPassed: true, supportedCases: 7, outputRows: 52, gpuComparedCases: 1,
      fullGpuSuite: false, authority: "none",
    },
    resident: {
      engineSource: "dfb4c5140b9c50ad936e27f1850541043135130b",
      adapterSource: "9421a6a887046749961f5e2cbf5e4d4e3e93df75",
      integratedSource: "c5ad1ba47ffb940bfc0b69f1acb89fd259e60e56",
      engineEvidenceSha256: "f6aedb5d76b2fa5f5661834c548d2dc7a780035de6430f1adb8f782059256d2c",
      enginePassed: 707, engineIgnored: 9, engineStrictClippyPassed: true,
      adapterPassed: 109, adapterIgnored: 1, adapterCliPassed: 12, adapterPolicyPassed: 37,
      finalAdapterValidated: true, nativeRepeatedRounds: true,
      authority: "none",
    },
    mfma: {
      source: "ce2d3c1fb57ac4e2a883f1a1a329a17118bd9427",
      compilerOverlay: "b86d7749a7951864ed8c59863674891a94b2b186",
      compilerOverlayPublished: false,
      engineEvidenceSha256: "20045976f0abc71e65e9a86a132cb685c29b67fce8e494de7c78353c308ac92c",
      adapterEvidenceSha256: "e7fdbc1f3a0d21c28cf8e2a3de9cdaed052e55e38e8745d3d0c8a375cb566239",
      hostLibraryPassed: 709, hostLibraryIgnored: 9, adapterPassed: 110, adapterIgnored: 1,
      capturePassed: 87, captureIgnored: 2, policyPassed: 38, sourceContractPassed: 11,
      clippySource: "1fc91418773be1a08f084f1b9e2b16e75b6ce759", focusedCatalogPassed: 8,
      clippyEvidenceSha256: "8fb01aefc93b2c19f85c6efd288320a44f3e713593bed4ad65a4e8b7e3b0f201",
      strictClippyPassed: true, deviceCompiled: false,
      gpuTested: false, performanceMeasured: false, authority: "none",
    },
    compilerCandidate: {
      source: "5602c4889142571d7d82041b8a9bdef369799af1", published: false,
      directGeometryTestsPassed: 4, pairedStorageExportPassed: true, releaseToolsBuilt: true,
      imageProduced: false, gpuTested: false, authority: "none",
    },
    compiler: {
      source: "0fe50455b2fb65744048789bfc05f720313a12c5",
      ferricSource: "f9ff2f3c2dc8e467387f6c5204ff2aedfa952aa9",
      lineagePassed: 42, mirPassed: 84, plironPassed: 1029, backendPassed: 543,
      aggregateHostPassed: 37, baselineStrictClippyPassed: false,
      emissionStarted: true, imageProduced: true, emissionExit: 0, exactOutputReplay: true,
      target: "gfx942:xnack-", codeObjectVersion: 6, kernelEntries: 12, kernelDescriptors: 12,
      imageBytes: 103616,
      imageSha256: "e17bf955d3de70c44d721cb798785f539915cb003c7b046efd2cce787e2df7c3",
      manifestSha256: "6e80c84941e7c7d7eef2d611cf130ab290b69495a448f6e7ec73ff8659c47138",
      inspectionSha256: "ae952c85deed06f58e76e251c27153db531bae6c6e0775768b46d8dfe4fff817",
      evidenceSha256: "539970f202a048cc5cd3664b6fadde528da13fc7e99249bdac7bae82ffe3e45f",
      grants: { publication: false, load: false, launch: false },
      gpuTested: false, authority: "none",
    },
  };
  const progressClaims = {
    r29: ["seven canonical cases and 52 output rows", "real 8,192-token decode histories",
      "All 24 comparison tests, 15 engineering and 23 legacy reference tests",
      "Actual GPU comparisons remain 1 of 7", "remaining six GPU cases have not run",
      "no tolerance acceptance or qualification authority"],
    resident: ["707 engine library tests (nine existing ignores)", "2242-to-425-to-2242",
      "strict all-target Clippy", "exact 942 results: 109 library tests (one ignore)",
      "12 speculative CLI tests, 37 source-policy tests", "failures remain retained separately",
      "without changing the parser or proven initialization helpers",
      "Host checks alone do not establish native execution or qualification",
      "not full-acceptance catch-up"],
    mfma: ["complete unpublished b86 compiler overlay", "709 engine library tests (nine ignores)",
      "110 adapter library tests (one ignore)", "87 capture tests (two ignores) and 38 policies",
      "Exact successor 1fc", "do not relabel the full ce2 cohorts",
      "No MFMA device image, GPU execution or performance improvement has been demonstrated"],
    compilerCandidate: ["Unpublished compiler candidate 5602", "actual paired row-major/column-major storage exports",
      "bounded unconstrained launch inputs", "does not substitute maximum-grid constants or bypass convergence checks",
      "HSACO emission and the real 13-root aggregate remain pending", "no GPU or qualification result"],
    compiler: ["42 lineage, 84 MIR, 1,029 Pliron and 543 backend tests",
      "37 exact f9 aggregate host tests", "unchanged dependency failure",
      "emit and exactly replay the 103,616-byte gfx942 image",
      "12 kernel entries and 12 descriptors", "differs from 4f6 and has no GPU validation",
      "earlier pre-emission resource stop remains separate",
      "publication, load and launch grants remain false", "actual older producer identities"],
  };
  for (const [key, expected] of Object.entries(expectedProgress)) {
    const { detail, ...facts } = progress[key];
    assert.deepEqual(facts, expected, key);
    for (const claim of progressClaims[key]) assert(detail.includes(claim), `${key}: ${claim}`);
  }
  for (const claim of ["No new TTFT, TPOT or throughput measurement", "No accepted matched SGLang result is available",
    "Historical performance data is unchanged", "all 33 M1 gates remain open"]) {
    assert(progress.boundary.includes(claim), claim);
  }
  const { detail: coverageDetail, ...currentSourceCoverage } = integration.sourceCoverage;
  assert.deepEqual(currentSourceCoverage, {
    source: "c5ad1ba47ffb940bfc0b69f1acb89fd259e60e56",
    compilerSource: "4f6f65ce22222bae9ece5c5f08c66e56e022a9e4",
    evidenceSha256: "11670d90819a8ed3a731ec733e94434805ad9b929e4b2efe55ed8666b2b775f4",
    lockedGraphs: 32, dependencyTcbs: 3, modules: 177, bodies: 8557,
    verifiedBodies: 724, unverifiedBodies: 7833, proofUpgrades: 0, committedEqualityPassed: true,
  });
  for (const claim of ["committed coverage equality", "92 new structural-runtime/comparator rows remain proof-pending",
    "7,741 surviving full rows are unchanged", "No proof label or M1 gate is upgraded",
    "does not relabel the earlier dfb host tests"]) assert(coverageDetail.includes(claim), claim);
  const { detail: attemptDetail, ...attempt } = integration.nativeResidentAttempt;
  assert.deepEqual(attempt, {
    source: "c5ad1ba47ffb940bfc0b69f1acb89fd259e60e56",
    compilerSource: "4f6f65ce22222bae9ece5c5f08c66e56e022a9e4",
    artifactSource: "8113e2314b2e030f93bc27d1281b4caee7e66216",
    binarySha256: "4151c135ff693e1d66a1b8004a3d31488dd227ee0c50ce45b90a025f0e488e2a",
    evidenceSha256: "66575e9bd00b950ed0a592c931de6e7ae672da2ca27e50017d4f853d76af89ae",
    profile: "debug", requestedOutputTokens: 8, exit: 126, stop: "selected-gpu-use-guard",
    deadlineExpired: false, processAttribution: "unresolved", prefillOutputObserved: false,
    speculativeRoundOutputObserved: false, catchupObserved: false,
    deviceMemoryBaselineRestored: true, authority: "none", qualification: false, performanceMeasurement: false,
  });
  for (const claim of ["unchanged 8113/4f6 image", "604.307 seconds", "exit 126",
    "not a deadline or resource-cap stop", "ownership remains unresolved", "Stdout is empty",
    "no prefill, speculative-round, token or catch-up/restore result is claimed",
    "298,647,552-byte GPU baseline", "without reset or foreign intervention",
    "failed engineering attempt, not a native continuation or performance result"]) assert(attemptDetail.includes(claim), claim);
  const { detail: buildDetail, ...optimizedBuild } = integration.optimizedResidentBuild;
  assert.deepEqual(optimizedBuild, {
    source: "c5ad1ba47ffb940bfc0b69f1acb89fd259e60e56",
    compilerSource: "4f6f65ce22222bae9ece5c5f08c66e56e022a9e4",
    binarySha256: "d6d6f7af56957d0987d0e2cb01d4d2b75766fcc1f95f43fb19da39c7ff294819",
    evidenceSha256: "5c5399489e1d208e90fa5e82f2bc0843f9fe956c72e6d5dc72aa5329b4db6a4a",
    binaryBytes: 14133320, profile: "release", features: [], exit: 0,
    buildGpuEnabled: false, qualification: false, authority: "none",
  });
  for (const claim of ["separate optimized c5/4f6", "GPU access was disabled during the build phase",
    "does not replace or relabel the debug guard-stop record", "close any M1 gate"]) assert(buildDetail.includes(claim), claim);
  const { detail: residentNativeDetail, ...nativeResident } = integration.nativeResident;
  assert.deepEqual(nativeResident, {
    source: "c5ad1ba47ffb940bfc0b69f1acb89fd259e60e56",
    compilerSource: "4f6f65ce22222bae9ece5c5f08c66e56e022a9e4",
    artifactSource: "8113e2314b2e030f93bc27d1281b4caee7e66216",
    binarySha256: "d6d6f7af56957d0987d0e2cb01d4d2b75766fcc1f95f43fb19da39c7ff294819",
    evidenceSha256: "c46c0023872f404ddda8140f9162fa2d36785618916266ad41d0240234908870",
    profile: "release", exit: 0, speculativeRounds: 5, acceptedDraftCounts: [0, 0, 1, 2, 0],
    epochs: [2, 3, 4, 5, 6], publishedTokenIds: [4710, 32313, 11, 358, 1184, 311, 7071, 700],
    prefillAnchor: 12, physicalPromptTokens: 128, rawPromptTokens: 5, activeSuffixFill: true,
    completedDraftCatchups: 0, fullAcceptanceContinuationObserved: false,
    finalDraftCursor: 136, finalTargetCursor: 136, hardwareCompletionObserved: true,
    nativeQueueDestroyed: true, deviceMemoryBaselineRestored: true, workerV3Authenticated: false,
    compilerOriginAuthenticated: false, currentPublicationSelected: false, benchmarkComparable: false,
    numericalQualification: false, authority: "none",
  });
  for (const claim of ["five real K4 rounds", "actual structural registry, bridge and coordinator",
    "Accepted draft counts are [0, 0, 1, 2, 0]", "full-acceptance maintenance/restore remains unobserved",
    "actively EOT-filled to 128 tokens, not attention-mask padded", "prefill anchor 12 is excluded",
    "not ordinary raw-prompt serving or a numerical comparison", "no performance gain or M1 gate closure is claimed"]) {
    assert(residentNativeDetail.includes(claim), claim);
  }
  const { detail: numericalDetail, boundary: numericalBoundary, ...numerical } = integration.selectedNumerical;
  assert.deepEqual(numerical, {
    source: "4ac1250735c1e774356c901aede91066d05816f1",
    compilerSource: "ccfd43d6bc58b27e0efe510b0ee1b8463166dfc2",
    artifactSource: "bb2b0123f4f96410269a098d7cba8aa7bb950008",
    imageSha256: "46335b09a921b33ed66392e415ce3d2348dd63042242c0387d5a58362ba3c84c",
    evidenceSha256: "816f162e5a2d3229239dc305e8658f8c570a269b44a077787d916030ec60335c",
    comparisonSha256: "38a39f82dd2df55060acdf27e68813ad13f09f3d4cdb76d9ccbbdf61ddaba665",
    diagnosticsSha256: "7f2ec61f0d47a3bc0ab738f5e94f049161b29e6364f88a0d029c6a55ead673df",
    caseId: "prefill-s1-t128.001", inputTokens: 128, completedCases: 1, fullRosterCases: 7,
    target: "gfx942:xnack-", captureExit: 0, referenceExit: 0, comparisonExit: 0,
    comparedLogits: 151936, finite: true, ferricToken: 198, referenceToken: 198, tokenMismatches: 0,
    maximumLogitUlpError: 31121, maximumAbsoluteError: 0.0703125,
    rootMeanSquaredError: 0.01524191977257529, cosineSimilarity: 0.9999202347605426,
    exactBf16Matches: 20968, oppositeNonzeroSigns: 619,
    top10TokenOrder: [198, 220, 63477, 271, 151645, 80126, 2529, 46556, 33179, 279],
    identicalTop10Order: true, logitRowsByteIdentical: false, authority: "none",
    toleranceAccepted: false, fullR29Comparison: false, qualification: false, benchmarkComparable: false,
  });
  for (const claim of ["one generated 128-token input", "not the earlier five-token English prompt or speculative padded prompt",
    "151,936 compared logits are finite", "both select token 198", "rows are not byte-identical",
    "maximum BF16 ULP error is 31,121", "maximum absolute error 0.0703125",
    "RMSE 0.01524191977257529", "cosine similarity 0.9999202347605426",
    "20,968 exact BF16 matches and 619 opposite nonzero signs",
    "within one model load, not two fresh launches"]) assert(numericalDetail.includes(claim), claim);
  for (const claim of ["one of seven planned R29 cases", "not a tolerance acceptance or full R29 comparison",
    "Authority is none", "qualification and benchmark comparability remain false",
    "does not GPU-validate the newer 4f6 image", "Historical performance data remains unchanged"]) {
    assert(numericalBoundary.includes(claim), claim);
  }
  const { detail: latestCompilerDetail, ...latestCompiler } = integration.latestCompiler;
  assert.deepEqual(latestCompiler, {
    source: "4f6f65ce22222bae9ece5c5f08c66e56e022a9e4",
    ferricSource: "8113e2314b2e030f93bc27d1281b4caee7e66216",
    kernelIrPassed: 298, kernelAnalysisPassed: 125, backendPassed: 543, kernelHostPassed: 37,
    emissionExit: 0, target: "gfx942:xnack-", codeObjectVersion: 6, kernelEntries: 12, kernelDescriptors: 12,
    imageBytes: 103616,
    imageSha256: "6f77d6813e6a2c9fd20c8b50eaffe0feb4435f00ac65192e9574a3637b6e5284",
    manifestSha256: "39cf7d5a915f759cb289e2f48b63acbd725904bb22aa14439bbda575074b3d3f",
    evidenceSha256: "6208de3c1329e000c5393739b600d6e04b49b4b20dff4fb8dfc5443a8e65d3cb",
    inspectionSha256: "70625b50d7be703fa5eee1256f9879380d5d8d248437ddcc984604fd06e7f216",
    exactOutputReplay: true, nativeGpuTested: true, authority: "none",
    grants: { publication: false, load: false, launch: false },
  });
  for (const claim of ["543 backend tests and 37 aggregate host tests", "12 kernel entries and 12 matching descriptors",
    "HSACO is 103,616 bytes", "unchanged aggregate kernel bodies", "not policy descriptor-table ordering",
    "separate c5 resident observation now exercises this image on a GPU", "without numerical qualification or relabeling its 8113 producer", "publication, load and launch grants remain false",
    "actual older producer identities, not 4f6 labels"]) assert(latestCompilerDetail.includes(claim), claim);
  assert.notEqual(numerical.imageSha256, latestCompiler.imageSha256);
  assert.notEqual(numerical.compilerSource, latestCompiler.source);
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
  assert(checkpoint.remaining.includes("Latest combined host validation is separate from the retained bb2/ccfd cohorts"));
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
    (x) => { x.integration.hostProgress.r29.source = x.integration.hostProgress.r29.integratedSource; },
    (x) => { x.integration.hostProgress.r29.comparisonTests = 25; },
    (x) => { x.integration.hostProgress.r29.engineeringReferenceTests = 14; },
    (x) => { x.integration.hostProgress.r29.legacyReferenceTests = 22; },
    (x) => { x.integration.hostProgress.r29.supportedCases = 1; },
    (x) => { x.integration.hostProgress.r29.outputRows = 7; },
    (x) => { x.integration.hostProgress.r29.gpuComparedCases = 7; },
    (x) => { x.integration.hostProgress.r29.fullGpuSuite = true; },
    (x) => { x.integration.hostProgress.r29.authority = "qualified"; },
    (x) => { x.integration.hostProgress.r29.detail = "All seven GPU cases passed."; },
    (x) => { x.integration.hostProgress.resident.engineSource = x.integration.hostProgress.resident.adapterSource; },
    (x) => { x.integration.hostProgress.resident.pendingAdapterSource = x.integration.hostProgress.resident.adapterSource; },
    (x) => { x.integration.hostProgress.resident.enginePassed = 705; },
    (x) => { x.integration.hostProgress.resident.engineIgnored = 0; },
    (x) => { x.integration.hostProgress.resident.adapterPassed = 13; },
    (x) => { x.integration.hostProgress.resident.finalAdapterValidated = false; },
    (x) => { x.integration.hostProgress.resident.nativeRepeatedRounds = false; },
    (x) => { x.integration.sourceCoverage.verifiedBodies = 725; },
    (x) => { x.integration.sourceCoverage.proofUpgrades = 1; },
    (x) => { x.integration.sourceCoverage.committedEqualityPassed = false; },
    (x) => { x.integration.nativeResidentAttempt.exit = 0; },
    (x) => { x.integration.nativeResidentAttempt.deadlineExpired = true; },
    (x) => { x.integration.nativeResidentAttempt.processAttribution = "owned-helper"; },
    (x) => { x.integration.nativeResidentAttempt.profile = "release"; },
    (x) => { x.integration.nativeResidentAttempt.artifactSource = x.integration.nativeResidentAttempt.source; },
    (x) => { x.integration.nativeResidentAttempt.prefillOutputObserved = true; },
    (x) => { x.integration.nativeResidentAttempt.speculativeRoundOutputObserved = true; },
    (x) => { x.integration.nativeResidentAttempt.catchupObserved = true; },
    (x) => { x.integration.nativeResidentAttempt.qualification = true; },
    (x) => { x.integration.nativeResidentAttempt.performanceMeasurement = true; },
    (x) => { x.integration.nativeResidentAttempt.detail = "Successful repeated GPU serving."; },
    (x) => { x.integration.optimizedResidentBuild.binarySha256 = x.integration.nativeResidentAttempt.binarySha256; },
    (x) => { x.integration.optimizedResidentBuild.buildGpuEnabled = true; },
    (x) => { x.integration.optimizedResidentBuild.features = ["test-support"]; },
    (x) => { x.integration.optimizedResidentBuild.qualification = true; },
    (x) => { x.integration.nativeResident.source = x.integration.nativeResident.artifactSource; },
    (x) => { x.integration.nativeResident.binarySha256 = x.integration.nativeResidentAttempt.binarySha256; },
    (x) => { x.integration.nativeResident.evidenceSha256 = x.integration.optimizedResidentBuild.evidenceSha256; },
    (x) => { x.integration.nativeResident.exit = 126; },
    (x) => { x.integration.nativeResident.speculativeRounds = 4; },
    (x) => { x.integration.nativeResident.acceptedDraftCounts[0] = 4; },
    (x) => { x.integration.nativeResident.epochs[2] = 5; },
    (x) => { x.integration.nativeResident.publishedTokenIds[0] = 12; },
    (x) => { x.integration.nativeResident.prefillAnchor = 4710; },
    (x) => { x.integration.nativeResident.physicalPromptTokens = 5; },
    (x) => { x.integration.nativeResident.activeSuffixFill = false; },
    (x) => { x.integration.nativeResident.completedDraftCatchups = 1; },
    (x) => { x.integration.nativeResident.fullAcceptanceContinuationObserved = true; },
    (x) => { x.integration.nativeResident.finalDraftCursor = 135; },
    (x) => { x.integration.nativeResident.hardwareCompletionObserved = false; },
    (x) => { x.integration.nativeResident.nativeQueueDestroyed = false; },
    (x) => { x.integration.nativeResident.deviceMemoryBaselineRestored = false; },
    (x) => { x.integration.nativeResident.workerV3Authenticated = true; },
    (x) => { x.integration.nativeResident.compilerOriginAuthenticated = true; },
    (x) => { x.integration.nativeResident.currentPublicationSelected = true; },
    (x) => { x.integration.nativeResident.benchmarkComparable = true; },
    (x) => { x.integration.nativeResident.numericalQualification = true; },
    (x) => { x.integration.nativeResident.authority = "qualified"; },
    (x) => { x.integration.nativeResident.detail = "Full-acceptance serving and performance are qualified."; },
    (x) => { x.integration.hostProgress.resident.detail = "Repeated serving is qualified."; },
    (x) => { x.integration.hostProgress.mfma.hostLibraryPassed = 80; },
    (x) => { x.integration.hostProgress.mfma.strictClippyPassed = false; },
    (x) => { x.integration.hostProgress.mfma.deviceCompiled = true; },
    (x) => { x.integration.hostProgress.mfma.gpuTested = true; },
    (x) => { x.integration.hostProgress.mfma.performanceMeasured = true; },
    (x) => { x.integration.hostProgress.mfma.detail = "The MFMA image is GPU-tested."; },
    (x) => { x.integration.hostProgress.mfma.compilerOverlayPublished = true; },
    (x) => { x.integration.hostProgress.mfma.clippySource = x.integration.hostProgress.mfma.source; },
    (x) => { x.integration.hostProgress.compilerCandidate.published = true; },
    (x) => { x.integration.hostProgress.compilerCandidate.imageProduced = true; },
    (x) => { x.integration.hostProgress.compilerCandidate.gpuTested = true; },
    (x) => { x.integration.hostProgress.compiler.source = x.integration.latestCompiler.source; },
    (x) => { x.integration.hostProgress.compiler.plironPassed = 1028; },
    (x) => { x.integration.hostProgress.compiler.baselineStrictClippyPassed = true; },
    (x) => { x.integration.hostProgress.compiler.emissionStarted = false; },
    (x) => { x.integration.hostProgress.compiler.imageProduced = false; },
    (x) => { x.integration.hostProgress.compiler.imageSha256 = x.integration.latestCompiler.imageSha256; },
    (x) => { x.integration.hostProgress.compiler.exactOutputReplay = false; },
    (x) => { x.integration.hostProgress.compiler.kernelEntries = 11; },
    (x) => { x.integration.hostProgress.compiler.kernelDescriptors = 13; },
    (x) => { x.integration.hostProgress.compiler.grants.launch = true; },
    (x) => { x.integration.hostProgress.compiler.gpuTested = true; },
    (x) => { x.integration.hostProgress.compiler.detail = "0fe GPU qualification passed."; },
    (x) => { x.integration.hostProgress.boundary = "Matched SGLang superiority accepted."; },
    (x) => { x.integration.selectedNumerical.compilerSource = x.integration.latestCompiler.source; },
    (x) => { x.integration.selectedNumerical.source = x.integration.latestCompiler.ferricSource; },
    (x) => { x.integration.selectedNumerical.imageSha256 = x.integration.latestCompiler.imageSha256; },
    (x) => { x.integration.selectedNumerical.evidenceSha256 = x.integration.selectedNumerical.comparisonSha256; },
    (x) => { x.integration.selectedNumerical.comparisonSha256 = x.integration.selectedNumerical.diagnosticsSha256; },
    (x) => { x.integration.selectedNumerical.caseId = "prefill-s1-t2048.001"; },
    (x) => { x.integration.selectedNumerical.inputTokens = 5; },
    (x) => { x.integration.selectedNumerical.completedCases = 7; },
    (x) => { x.integration.selectedNumerical.comparedLogits -= 1; },
    (x) => { x.integration.selectedNumerical.ferricToken = 12095; },
    (x) => { x.integration.selectedNumerical.maximumLogitUlpError = 0; },
    (x) => { x.integration.selectedNumerical.maximumAbsoluteError = 0; },
    (x) => { x.integration.selectedNumerical.rootMeanSquaredError = 0; },
    (x) => { x.integration.selectedNumerical.cosineSimilarity = 1; },
    (x) => { x.integration.selectedNumerical.exactBf16Matches = 151936; },
    (x) => { x.integration.selectedNumerical.oppositeNonzeroSigns = 0; },
    (x) => { x.integration.selectedNumerical.top10TokenOrder.reverse(); },
    (x) => { x.integration.selectedNumerical.logitRowsByteIdentical = true; },
    (x) => { x.integration.selectedNumerical.toleranceAccepted = true; },
    (x) => { x.integration.selectedNumerical.fullR29Comparison = true; },
    (x) => { x.integration.selectedNumerical.qualification = true; },
    (x) => { x.integration.selectedNumerical.benchmarkComparable = true; },
    (x) => { x.integration.selectedNumerical.authority = "qualified"; },
    (x) => { x.integration.selectedNumerical.detail = "All logits match exactly."; },
    (x) => { x.integration.selectedNumerical.boundary = "R29 acceptance passed."; },
    (x) => { x.integration.latestCompiler.source = x.integration.compiler.source; },
    (x) => { x.integration.latestCompiler.ferricSource = x.integration.compiler.ferricSource; },
    (x) => { x.integration.latestCompiler.imageSha256 = x.integration.compiler.imageSha256; },
    (x) => { x.integration.latestCompiler.imageBytes = x.integration.compiler.imageBytes; },
    (x) => { x.integration.latestCompiler.kernelEntries = 11; },
    (x) => { x.integration.latestCompiler.kernelDescriptors = 13; },
    (x) => { x.integration.latestCompiler.nativeGpuTested = false; },
    (x) => { x.integration.latestCompiler.exactOutputReplay = false; },
    (x) => { x.integration.latestCompiler.authority = "qualified"; },
    (x) => { x.integration.latestCompiler.grants.launch = true; },
    (x) => { x.integration.latestCompiler.detail = "Latest compiler image GPU-qualified."; },
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
