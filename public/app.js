(function renderFerricStatus() {
  "use strict";

  const project = window.FERRIC_PROJECT;
  if (!project) {
    return;
  }

  const stateLabels = {
    implemented: "Implemented",
    integration: "In integration",
    observed: "Hardware observed",
    verified: "Source verified",
    qualified: "Qualified",
    open: "Open",
  };

  const capabilityGroups = [
    ["runnable", "Implemented surface", "Available for the exact admitted path"],
    [
      "experimental",
      "Integration surface",
      "Scoped source or hardware evidence exists",
    ],
    ["roadmap", "Still blocked", "Required before authenticated R33 serving and M1"],
  ];

  function element(tag, className, text) {
    const node = document.createElement(tag);
    if (className) {
      node.className = className;
    }
    if (text !== undefined) {
      node.textContent = text;
    }
    return node;
  }

  function stateTag(state) {
    return element(
      "span",
      `state-tag state-${state}`,
      stateLabels[state] || state,
    );
  }

  function commitLink(commit, repository = project.repository) {
    const link = element("a", "commit-link", commit);
    link.href = `${repository}/commit/${commit}`;
    link.setAttribute("aria-label", `View source commit ${commit}`);
    return link;
  }

  document.querySelectorAll("[data-repository]").forEach((link) => {
    link.href = project.repository;
  });

  const roadmapLink = document.querySelector("[data-roadmap-link]");
  roadmapLink.href = `${project.repository}/blob/main/docs/ROADMAP.md`;

  document.querySelector("[data-milestone-name]").textContent = project.milestone.name;
  document.querySelector("[data-milestone-label]").textContent = project.milestone.label;
  document.querySelector("[data-milestone-summary]").textContent = project.residentCheckpoint.overview;
  document.querySelector("[data-milestone-dot]").classList.add(
    `dot-${project.milestone.state}`,
  );

  const updated = document.querySelector("[data-updated]");
  updated.dateTime = project.updated;
  updated.textContent = `Updated ${project.updated}`;

  const resident = project.residentCheckpoint;
  const residentProgress = document.querySelector("[data-resident-progress]");
  residentProgress.append(element("p", "performance-scope", resident.scope),
    element("h3", "", "Matching abd8 kernels: all 13 emission phases passed"),
    element("p", "", resident.integration.emissionAbd8.detail),
    element("h3", "", "Vendor semantics checked; original formation remains failed"),
    element("p", "", resident.integration.vendorSemanticsAbd8.detail),
    element("h3", "", "Compiler bd1 adopted; GPU validation held"),
    element("p", "", resident.integration.upstreamAndNativeHold.detail),
    element("p", "performance-scope", "The retained checkpoints below preserve their original source identities and then-pending states. Later abd8 emission does not relabel earlier validation or close a qualification gate."),
    element("h3", "", "Prefill EOS: authenticated terminal ownership, host checks passed"),
    element("p", "", resident.integration.prefillEos.detail),
    element("h3", "", "Canonical prepack publication: atomic no-replace fix"),
    element("p", "", resident.integration.prepackPublication.detail),
    element("h3", "", "Compiler 9f8 adopted: dependency and source checks passed"),
    element("p", "", resident.integration.compilerAdoption9f8.detail),
    element("h3", "", "Retained abd8 diagnostic: vendor formation stopped"),
    element("p", "", resident.integration.vendorAttemptAbd8.detail),
    element("p", "performance-scope", "The retained host diagnostics, compiler tools and optimized release below preserve their original source identities; they are not 9f8 tool builds or native validation."),
    element("h3", "", "Final-RMS diagnostics: host and protocol checks passed"),
    element("p", "", resident.integration.finalRmsDiagnostics.detail),
    element("h3", "", "Compiler abd8 adopted: 50 dependency and source phases passed"),
    element("p", "", resident.integration.compilerAdoptionAbd8.detail),
    element("h3", "", "Matching abd8 compiler tools: six build phases passed"),
    element("p", "", resident.integration.compilerToolsAbd8.detail),
    element("h3", "", "Optimized diagnostic release and device metadata passed"),
    element("p", "", resident.integration.releaseMetadataAbd8.detail),
    element("p", "performance-scope", "The following model-bundle and compiler records describe earlier checkpoints; their source identities and then-pending states are preserved."),
    element("h3", "", "Model-bundle component proof: scoped result, separate assessment"),
    element("p", "", resident.integration.modelBundleComponent119.detail),
    element("h3", "", "Host checks and source inventory passed"),
    element("p", "", resident.integration.modelBundleHostInventory.detail),
    element("h3", "", "Registered proof timed out; protected qualification unavailable"),
    element("p", "", resident.integration.registeredBundleTimeout.detail),
    element("h3", "", "Retained M5 logits: CPU-only ranking replay"),
    element("p", "", resident.integration.m5RankReplay.detail),
    element("p", "performance-scope", "The retained compiler and M5 records below preserve their earlier source identities. This checkpoint adds no GPU observation or performance measurement."),
    element("h3", "", "Compiler 7f90 adopted: 47 dependency and source phases passed"),
    element("p", "", resident.integration.compilerAdoption7f90.detail),
    element("h3", "", "Matching 7f90 compiler tools: all six tool phases passed"),
    element("p", "", resident.integration.compilerTools7f90.detail),
    element("h3", "", "RMSNorm: 167 tests pass; strict Clippy stops the campaign"),
    element("p", "", resident.integration.rmsnormHostR3.detail),
    element("h3", "", "R4: tests and build-script parity pass; Clippy stops"),
    element("p", "", resident.integration.rmsnormFollowupR4.detail),
    element("h3", "", "R5: 24 tests and all six strict Clippy selections pass"),
    element("p", "", resident.integration.rmsnormFollowupR5.detail),
    element("h3", "", "Compiler 7f90: optimized release and vendor formation passed"),
    element("p", "", resident.integration.releaseAndVendor7f90.detail),
    element("h3", "", "Matching MFMA13 emission: all 13 phases passed"),
    element("p", "", resident.integration.m5Emission7f90.detail),
    element("h3", "", "Native capture stopped by the shared-GPU guard"),
    element("p", "", resident.integration.nativeAttempt7f90.detail),
    element("h3", "", "Current five-position comparison: one greedy-token mismatch"),
    element("p", "", resident.integration.m5Comparison7f90.detail),
    element("p", "performance-scope", "The retained a0cc9929 / compiler 1fadb7e0 observations below predate these source changes. They do not validate compiler 7f90 or establish an RMSNorm numerical improvement."),
    element("h3", "", "Retained M5: matching tools, executable and MFMA13 image"),
    element("p", "", resident.integration.m5Artifacts1fad.detail),
    element("h3", "", "Five-position native capture: guarded run completed"),
    element("p", "", resident.integration.m5Capture1fad.detail),
    element("h3", "", "Five matching tokens; logit differences remain unqualified"),
    element("p", "", resident.integration.m5Reference1fad.detail),
    element("p", "performance-scope", "The adoption, host and proof records below retain their original campaign scope. The later compiler-1fad native capture does not relabel compiler-349 host/proof or compiler-111 numerical results."),
    element("h3", "", "Compiler 1fadb7e0 adopted: dependency and source checks passed"),
    element("p", "", resident.integration.compilerAdoption1fad.detail),
    element("h3", "", "Combined M5 capture: host checks passed on compiler 349"),
    element("p", "", resident.integration.combinedM5Host349.detail),
    element("h3", "", "Compiler 349: focused production catch-up proof"),
    element("p", "", resident.integration.catchupProof349.detail),
    element("h3", "", "Core compiler boundary: 52 tests pass, strict Clippy remains open"),
    element("p", "", resident.integration.compilerBoundaryR5.detail),
    element("p", "performance-scope", "Retained native and numerical observations below use compiler 111. They are not new M5 or compiler-349 GPU results."),
    element("h3", "", "Compiler 111: 32 tokens and completed draft catch-up"),
    element("p", "", resident.integration.native111.detail),
    element("h3", "", "Prefill: matching token, logit differences still unqualified"),
    element("p", "", resident.integration.prefill111.detail),
    element("h3", "", "Earlier compiler 111 proof: nine negative mutations rejected"),
    element("p", "performance-scope", "These earlier proof and dependency records preserve their then-pending integration and validation states. The combined-source host and compiler-349 proof results above supersede those pending states, not their original evidence identities."),
    element("p", "", resident.integration.catchupProof111.detail),
    element("h3", "", "Earlier compiler 349 adoption and 45d0 observation"),
    element("p", "", resident.integration.compiler349.detail),
    element("h3", "", "Earlier compiler 111 adoption: dependency checks passed"),
    element("p", "performance-scope", "The following records preserve their original source identities and then-pending work. They do not supersede the native, numerical, proof or compiler checkpoints above."),
    element("p", "", resident.integration.compilerAdoption.detail),
    element("h3", "", "Earlier 8af smoke checkpoint: scoped host checks passed"),
    element("p", "performance-scope", "This smoke and verifier-publication record predates the adoption above. Its 8af test attribution and then-pending adoption status are preserved, not current 111 engine-validation claims."),
    element("p", "", resident.integration.engineeringSmoke.detail),
    element("h3", "", "Earlier selected-proof and native checkpoints"),
    element("p", "performance-scope", "The following records preserve their original sources and dated pending states. They do not supersede the current smoke or compiler results above."),
    element("h3", "", "14 selected proofs; maintenance fix host-tested"),
    element("p", "", resident.integration.latestProgress.detail),
    element("h3", "", "Earlier retained checkpoints, including the R4 snapshot"),
    element("p", "performance-scope", "The following records keep their original source identities and dated pending states. They do not supersede the current proof or maintenance results above."),
    element("h3", "", "Earlier 55-pinned MFMA host checkpoint; consumer proof then open"),
    element("p", "", resident.integration.currentValidation.detail),
    element("h3", "", "Earlier e3 dependency refresh; caller then unvalidated"),
    element("p", "", resident.integration.dependencyRefresh.detail),
    element("h3", "", "Frozen native runtime: tested diagnostics, maintenance still blocked"),
    element("p", "", resident.integration.runtimeSuccessor.detail),
    element("h3", "", "32-token attempts: separate failures, no inferred success"),
    element("p", "", resident.integration.sustainedNativeFailures.detail),
    element("h3", "", "Ferric MFMA image emitted; GPU validation pending"),
    element("p", "", resident.integration.compilerFollowup.detail),
    element("h3", "", "Retirement proof: scoped positives pass, composition still open"),
    element("p", "", resident.integration.retirementProof.detail),
    element("h3", "", "Remote validation: fresh resource admission required"),
    element("p", "", resident.integration.resourceHold.detail),
    element("h3", "", "Five native K4 rounds: zero and partial acceptance"),
    element("p", "", resident.integration.nativeResident.detail),
    element("h3", "", "Earlier debug attempt: stopped before inference"),
    element("p", "", resident.integration.nativeResidentAttempt.detail),
    element("h3", "", "Optimized resident executable: build only"),
    element("p", "", resident.integration.optimizedResidentBuild.detail),
    element("h3", "", "Retained c5 source coverage: no proof upgrades"),
    element("p", "", resident.integration.sourceCoverage.detail),
    element("h3", "", "Seven-case tooling; GPU comparisons remain 1 of 7"),
    element("p", "", resident.integration.hostProgress.r29.detail),
    element("h3", "", "Repeated resident rounds: host validation"),
    element("p", "", resident.integration.hostProgress.resident.detail),
    element("h3", "", "Matrix-kernel strategy: separate host validation"),
    element("p", "", resident.integration.hostProgress.mfma.detail),
    element("h3", "", "Earlier compiler checkpoint: paired exports before emission"),
    element("p", "", resident.integration.hostProgress.compilerCandidate.detail),
    element("h3", "", "Retained twelve-kernel image: emitted, not GPU-tested"),
    element("p", "", resident.integration.hostProgress.compiler.detail),
    element("p", "performance-scope", resident.integration.hostProgress.boundary),
    element("h3", "", "One-case numerical comparison"),
    element("p", "", resident.integration.selectedNumerical.detail),
    element("p", "performance-scope", resident.integration.selectedNumerical.boundary),
    element("h3", "", "Retained 4f6 image: separate structural GPU observation"),
    element("p", "", resident.integration.latestCompiler.detail),
    element("h3", "", "Retained four-token native target observation"),
    element("p", "", resident.integration.latestNativeSmoke.detail),
    element("p", "performance-scope", resident.integration.latestNativeSmoke.cleanupDetail),
    element("h3", "", "Retained c6 native target smoke"),
    element("p", "", resident.integration.nativeSmoke.detail),
    element("h3", "", "Retained ccfd compiler and catch-up checkpoint"),
    element("p", "", resident.integration.compiler.detail),
    element("h3", "", "Resolved assertion differential"),
    element("p", "", resident.integration.compiler.differential.detail),
    element("p", "", resident.integration.resident.detail),
    element("h3", "", "Retained bb2 host validation and engineering binaries"),
    element("p", "", resident.integration.currentHost.detail),
    element("h3", "", "Retained earlier host cohort"),
    element("p", "", resident.integration.host.detail),
    element("h3", "", "Canonical target and draft preparation"),
    element("p", "", resident.integration.prepack.detail),
    element("h3", "", "Retained d094 scoped source coverage"),
    element("p", "", resident.integration.currentCoverage.detail),
    element("h3", "", "Retained source coverage and dependency checks"),
    element("p", "", resident.integration.coverage.detail),
    element("p", "", resident.integration.sourcePolicy.detail),
    element("h3", "", "Earlier diagnostic and getter checkpoints"),
    element("p", "performance-scope", "The following cohorts retain their original compiler and Ferric identities; their pending states describe those earlier checkpoints."),
    element("p", "", resident.followup.compilerDiagnostic.detail),
    element("p", "", resident.followup.coordinateRepair.detail),
    element("p", "", resident.followup.runtimeGetter.detail),
    element("h3", "", "Scoped cursor proof and resident work"),
    element("p", "", resident.followup.cursorProof.detail),
    element("p", "", resident.followup.catchup.detail),
    element("h3", "", "Finite singleton windows: K4, K8 and K16"),
    element("p", "", resident.selection.detail),
    element("h3", "", "Earlier host validation and qualification boundary"),
    element("p", "", resident.host.detail),
    element("h3", "", "Earlier compiler and canonical gfx942 extraction"),
    element("p", "performance-scope", "The following eaa observations predate the attributed divisor and overflow diagnostics above; migration was pending at that checkpoint."),
    element("p", "", resident.compiler.detail),
    element("h3", "", "Earlier R18 compiler and M5 summary"),
    element("p", "", resident.earlierOverview),
    element("p", "", resident.overview),
    element("p", "", resident.remaining));
  const residentDetails = element("details", "performance-identities");
  residentDetails.append(element("summary", "", "Checkpoint source identities"));
  const residentPins = element("dl", "observation-facts");
  for (const [label, value] of [
    ["Latest emission tracker source", resident.integration.emissionAbd8.trackerSource],
    ["Matching abd8 HSACO SHA-256", resident.integration.emissionAbd8.hsacoSha256],
    ["Matching abd8 inspection SHA-256", resident.integration.emissionAbd8.inspectionSha256],
    ["Matching abd8 emission evidence SHA-256", resident.integration.emissionAbd8.evidenceSha256],
    ["Separate vendor source assessment receipt SHA-256", resident.integration.vendorSemanticsAbd8.assessmentReceiptSha256],
    ["Separate Cargo semantic receipt SHA-256", resident.integration.vendorSemanticsAbd8.semanticReceiptSha256],
    ["Completed-tool cleanup receipt SHA-256", resident.integration.vendorSemanticsAbd8.cleanupReceiptSha256],
    ["Current integrated compiler", resident.integration.upstreamAndNativeHold.integratedCompiler],
    ["Compiler bd1 adoption source", resident.integration.upstreamAndNativeHold.source],
    ["Compiler bd1 validation archive SHA-256", resident.integration.upstreamAndNativeHold.validationArchiveSha256],
    ["Compiler bd1 retention receipt SHA-256", resident.integration.upstreamAndNativeHold.retentionReceiptSha256],
    ["Compiler bd1 external ledger SHA-256", resident.integration.upstreamAndNativeHold.externalLedgerSha256],
    ["Prefill EOS integration source", resident.integration.prefillEos.source],
    ["Prefill EOS R4 host evidence SHA-256", resident.integration.prefillEos.evidenceSha256],
    ["Prepack publication fix source", resident.integration.prepackPublication.source],
    ["Prepack publication evidence SHA-256", resident.integration.prepackPublication.evidenceSha256],
    ["Current compiler adoption source", resident.integration.compilerAdoption9f8.adoptedSource],
    ["Current adopted compiler", resident.integration.compilerAdoption9f8.compilerSource],
    ["Compiler 9f8 adoption evidence SHA-256", resident.integration.compilerAdoption9f8.evidenceSha256],
    ["Retained abd8 release tracker source", resident.integration.releaseMetadataAbd8.trackerSource],
    ["Final-RMS host evidence ledger SHA-256", resident.integration.finalRmsDiagnostics.hostEvidenceLedgerSha256],
    ["Final-RMS protocol evidence ledger SHA-256", resident.integration.finalRmsDiagnostics.protocolEvidenceLedgerSha256],
    ["Model-bundle integration source", resident.integration.modelBundleHostInventory.integrationSource],
    ["Component Verus evidence SHA-256; original campaign failed", resident.integration.modelBundleComponent119.evidenceSha256],
    ["Separate component assessment SHA-256", resident.integration.modelBundleComponent119.assessmentSha256],
    ["Source-gate R5 evidence SHA-256; generation then failed", resident.integration.modelBundleHostInventory.sourceGateEvidenceSha256],
    ["Scoped engine-host evidence SHA-256", resident.integration.modelBundleHostInventory.engineEvidenceSha256],
    ["R7 inventory evidence SHA-256", resident.integration.modelBundleHostInventory.inventoryEvidenceSha256],
    ["Integrated executable inventory SHA-256", resident.integration.modelBundleHostInventory.generatedInventorySha256],
    ["Registered theorem timeout evidence SHA-256", resident.integration.registeredBundleTimeout.evidenceSha256],
    ["CPU-only M5 ranking replay SHA-256", resident.integration.m5RankReplay.resultSha256],
    ["Earlier abd8 compiler adoption source", resident.integration.compilerAdoptionAbd8.adoptedSource],
    ["Compiler abd8 adoption evidence SHA-256", resident.integration.compilerAdoptionAbd8.evidenceSha256],
    ["Compiler abd8 normal source-gate SHA-256", resident.integration.compilerAdoptionAbd8.normalSourceGateSha256],
    ["Compiler abd8 test source-gate SHA-256", resident.integration.compilerAdoptionAbd8.testSourceGateSha256],
    ["Compiler abd8 matching-tool evidence SHA-256", resident.integration.compilerToolsAbd8.evidenceSha256],
    ["Retained abd8 diagnostic source", resident.integration.compilerToolsAbd8.ferricImplementationSource],
    ["Optimized abd8 diagnostic release evidence SHA-256", resident.integration.releaseMetadataAbd8.evidenceSha256],
    ["Retained abd8 device metadata SHA-256", resident.integration.releaseMetadataAbd8.metadataSha256],
    ["Earlier compiler 7f90 adoption source", resident.integration.compilerAdoption7f90.source],
    ["Retained abd8 dependency pin", resident.integration.compilerAdoptionAbd8.compilerSource],
    ["Compiler 7f90 adoption evidence SHA-256", resident.integration.compilerAdoption7f90.evidenceSha256],
    ["Compiler 7f90 matching-tool evidence SHA-256", resident.integration.compilerTools7f90.evidenceSha256],
    ["RMSNorm R3 tested source; campaign 101", resident.integration.rmsnormHostR3.source],
    ["RMSNorm R3 retained evidence SHA-256", resident.integration.rmsnormHostR3.evidenceSha256],
    ["RMSNorm R4 source; campaign 101, parity passed", resident.integration.rmsnormFollowupR4.source],
    ["RMSNorm R4 retained evidence SHA-256", resident.integration.rmsnormFollowupR4.evidenceSha256],
    ["RMSNorm R5 source; focused campaign passed", resident.integration.rmsnormFollowupR5.source],
    ["RMSNorm R5 retained evidence SHA-256", resident.integration.rmsnormFollowupR5.evidenceSha256],
    ["Compiler 7f90 optimized-release evidence SHA-256", resident.integration.releaseAndVendor7f90.releaseEvidenceSha256],
    ["Compiler 7f90 release executable SHA-256", resident.integration.releaseAndVendor7f90.releaseBinarySha256],
    ["Compiler 7f90 vendor-formation receipt SHA-256", resident.integration.releaseAndVendor7f90.vendorReceiptSha256],
    ["Compiler 7f90 vendor raw-custody manifest SHA-256", resident.integration.releaseAndVendor7f90.vendorCustodyManifestSha256],
    ["Compiler 7f90 MFMA13 inspection SHA-256", resident.integration.m5Emission7f90.inspectionSha256],
    ["Compiler 7f90 MFMA13 image SHA-256", resident.integration.m5Emission7f90.hsacoSha256],
    ["Compiler 7f90 emission evidence SHA-256", resident.integration.m5Emission7f90.evidenceSha256],
    ["M5 comparison tracker source", resident.integration.m5Comparison7f90.trackerSource],
    ["Compiler 7f90 native-attempt failure evidence SHA-256", resident.integration.nativeAttempt7f90.evidenceSha256],
    ["Compiler 7f90 second-capture evidence SHA-256", resident.integration.m5Comparison7f90.captureEvidenceSha256],
    ["Compiler 7f90 five-position comparison SHA-256", resident.integration.m5Comparison7f90.comparisonSha256],
    ["Compiler 7f90 pre-comparison identity SHA-256", resident.integration.m5Comparison7f90.beforeIdentitySha256],
    ["Compiler 7f90 reference evidence SHA-256", resident.integration.m5Comparison7f90.referenceEvidenceSha256],
    ["Compiler 7f90 post-comparison identity SHA-256", resident.integration.m5Comparison7f90.afterIdentitySha256],
    ["Earlier 1fad status tracker source", resident.integration.m5Artifacts1fad.trackerSource],
    ["Retained M5 engineering source", resident.integration.m5Artifacts1fad.source],
    ["Compiler 1fad matching-tool evidence SHA-256", resident.integration.m5Artifacts1fad.toolsEvidenceSha256],
    ["Compiler 1fad optimized-release evidence SHA-256", resident.integration.m5Artifacts1fad.releaseEvidenceSha256],
    ["Compiler 1fad MFMA13 emission evidence SHA-256", resident.integration.m5Artifacts1fad.emissionEvidenceSha256],
    ["Retained five-position capture archive SHA-256", resident.integration.m5Capture1fad.evidenceSha256],
    ["Retained five-position comparison SHA-256", resident.integration.m5Reference1fad.comparisonSha256],
    ["Retained independent reference logits SHA-256", resident.integration.m5Reference1fad.referenceLogitsSha256],
    ["Retained five-position reference archive SHA-256", resident.integration.m5Reference1fad.evidenceSha256],
    ["Upstream compiler observed at the 1fad checkpoint", resident.integration.m5Artifacts1fad.latestObservedCompiler],
    ["Earlier compiler 1fad adoption source", resident.integration.compilerAdoption1fad.source],
    ["Earlier compiler 1fad dependency pin", resident.integration.compilerAdoption1fad.compilerSource],
    ["Compiler 1fad adoption R2 evidence SHA-256", resident.integration.compilerAdoption1fad.evidenceSha256],
    ["Compiler 1fad adoption first-attempt failure SHA-256", resident.integration.compilerAdoption1fad.firstAttemptEvidenceSha256],
    ["Combined M5 host and proof source", resident.integration.combinedM5Host349.source],
    ["Combined M5 private integration", resident.integration.combinedM5Host349.integrationSource],
    ["Combined M5 validation compiler", resident.integration.combinedM5Host349.compilerSource],
    ["Combined M5 host evidence SHA-256", resident.integration.combinedM5Host349.evidenceSha256],
    ["Compiler 349 catch-up proof evidence SHA-256", resident.integration.catchupProof349.evidenceSha256],
    ["Compiler boundary R5 tested tree", resident.integration.compilerBoundaryR5.testedTree],
    ["Compiler boundary R5 tested parent", resident.integration.compilerBoundaryR5.testedParent],
    ["Compiler boundary R5 evidence SHA-256", resident.integration.compilerBoundaryR5.evidenceSha256],
    ["Published compiler correction after rebase", resident.integration.compilerBoundaryR5.rebasedSource],
    ["Rebased compiler parent; not a Ferric adoption", resident.integration.compilerBoundaryR5.rebasedParent],
    ["Earlier observed upstream compiler", resident.integration.compiler349.latestObservedCompiler],
    ["Native and prefill Ferric source", resident.integration.native111.source],
    ["Native and prefill compiler", resident.integration.native111.compilerSource],
    ["Earlier compiler 111 catch-up proof candidate", resident.integration.catchupProof111.source],
    ["Earlier compiler adoption source", resident.integration.compilerAdoption.source],
    ["Earlier smoke tracker source", resident.integration.engineeringSmoke.trackerSource],
    ["MFMA13 engineering smoke source", resident.integration.engineeringSmoke.source],
    ["MFMA13 smoke validation compiler", resident.integration.engineeringSmoke.compilerPin],
    ["Earlier published correction; adoption then pending", resident.integration.engineeringSmoke.publishedVerifierSource],
    ["Earlier proof/native tracker source", resident.integration.latestProgress.trackerSource],
    ["R9 selected proof source", resident.integration.latestProgress.proofSource],
    ["Host-tested maintenance fix", resident.integration.latestProgress.fixSource],
    ["Earlier fetched compiler; adoption then pending", resident.integration.latestProgress.latestFetchedCompiler],
    ["Earlier validated compiler pin", resident.integration.currentValidation.validatedCompilerPin],
    ["Earlier R4 checkpoint compiler fetch", resident.integration.currentValidation.latestFetchedCompiler],
    ["55-pinned dependency/source-gate evidence SHA-256", resident.integration.currentValidation.precheckEvidenceSha256],
    ["Host R1 source; 1,141 passes / 14 ignored", resident.integration.currentValidation.r1Source],
    ["Host R2 source; 54 aggregate passes", resident.integration.currentValidation.r2Source],
    ["Host R3 source; 11 scoped passes", resident.integration.currentValidation.r3Source],
    ["Host R4 source; core Clippy passes, other lint work open", resident.integration.currentValidation.r4Source],
    ["Host R4 evidence SHA-256", resident.integration.currentValidation.r4EvidenceSha256],
    ["Adapter ordering correction; lint retry pending", resident.integration.currentValidation.adapterFixSource],
    ["Failed R4 batch-consumer proof evidence SHA-256", resident.integration.currentValidation.consumerProofEvidenceSha256],
    ["Ghost-only correction; actual R4 proof retry failed", resident.integration.currentValidation.ghostFixSource],
    ["Earlier private integration source", resident.integration.dependencyRefresh.source],
    ["Earlier private integration tree", resident.integration.dependencyRefresh.tree],
    ["Earlier integration compiler dependency", resident.integration.dependencyRefresh.compilerSource],
    ["Dependency-only validation base", resident.integration.dependencyRefresh.metadataBaseSource],
    ["Dependency refresh evidence SHA-256", resident.integration.dependencyRefresh.evidenceSha256],
    ["Refreshed combined dependency inventory SHA-256", resident.integration.dependencyRefresh.combinedTcbSha256],
    ["Original contracted caller; then unvalidated", resident.integration.dependencyRefresh.callerSource],
    ["Frozen native runtime source", resident.integration.runtimeSuccessor.source],
    ["Frozen native runtime host evidence SHA-256", resident.integration.runtimeSuccessor.hostEvidenceSha256],
    ["Frozen native runtime release evidence SHA-256", resident.integration.runtimeSuccessor.releaseEvidenceSha256],
    ["R3 generation failure evidence SHA-256", resident.integration.sustainedNativeFailures.r3EvidenceSha256],
    ["R4 maintenance failure evidence SHA-256", resident.integration.sustainedNativeFailures.r4EvidenceSha256],
    ["R5 guard stop evidence SHA-256", resident.integration.sustainedNativeFailures.r5EvidenceSha256],
    ["Guard mock-fixture evidence SHA-256", resident.integration.sustainedNativeFailures.guardMockEvidenceSha256],
    ["Earlier published compiler source", resident.integration.compilerFollowup.currentSource],
    ["Earlier compiler observed upstream", resident.integration.compilerFollowup.newerObservedUpstreamSource],
    ["Earlier compiler R12 host evidence SHA-256", resident.integration.compilerFollowup.currentHostEvidenceSha256],
    ["Initial R12 pre-build failure evidence SHA-256", resident.integration.compilerFollowup.initialHostFailureEvidenceSha256],
    ["Earlier tested compiler R10 source", resident.integration.compilerFollowup.previousSource],
    ["Earlier compiler R10 host evidence SHA-256", resident.integration.compilerFollowup.previousHostEvidenceSha256],
    ["Earlier tested compiler R9 source", resident.integration.compilerFollowup.successorSource],
    ["Tested compiler R9 host evidence SHA-256", resident.integration.compilerFollowup.successorHostEvidenceSha256],
    ["Frozen e0 aggregate emission evidence SHA-256", resident.integration.compilerFollowup.frozenEmissionEvidenceSha256],
    ["Helper integration source", resident.integration.runtimeSuccessor.helperIntegrationSource],
    ["Helper integration host evidence SHA-256", resident.integration.runtimeSuccessor.helperIntegrationEvidenceSha256],
    ["Earlier d3 compiler host source", resident.integration.compilerFollowup.testedHostSource],
    ["Earlier d3 compiler host evidence SHA-256", resident.integration.compilerFollowup.hostEvidenceSha256],
    ["Routing-test source with strict Clippy failure", resident.integration.runtimeSuccessor.routingTestSource],
    ["Host-validated routing successor", resident.integration.runtimeSuccessor.routingSuccessorSource],
    ["Routing successor host evidence SHA-256", resident.integration.runtimeSuccessor.routingSuccessorEvidenceSha256],
    ["Scoped retirement-proof successor", resident.integration.retirementProof.successorSource],
    ["Scoped commit-proof evidence SHA-256", resident.integration.retirementProof.successorCommitEvidenceSha256],
    ["Scoped positive/negative proof evidence SHA-256", resident.integration.retirementProof.currentSourcePositiveNegativeEvidenceSha256],
    ["Selected global-index core R6 proof evidence SHA-256", resident.integration.retirementProof.indexSuccessorEvidenceSha256],
    ["Selected global-index core R7 negative evidence SHA-256", resident.integration.retirementProof.indexNegativeEvidenceSha256],
    ["Integrated index source", resident.integration.retirementProof.indexIntegrationSuccessorSource],
    ["Integrated index host evidence SHA-256", resident.integration.retirementProof.indexIntegrationSuccessorEvidenceSha256],
    ["Ledger candidate failed host evidence SHA-256", resident.integration.retirementProof.ledgerCandidateEvidenceSha256],
    ["Ledger successor host evidence SHA-256", resident.integration.retirementProof.ledgerSuccessorHostEvidenceSha256],
    ["Ledger successor R9 proof evidence SHA-256", resident.integration.retirementProof.ledgerSuccessorProofEvidenceSha256],
    ["Ledger successor R10 negative evidence SHA-256", resident.integration.retirementProof.ledgerSuccessorNegativeEvidenceSha256],
    ["Verified ledger helper source", resident.integration.retirementProof.ledgerSuccessorSource],
    ["Ledger helper integration source", resident.integration.retirementProof.ledgerIntegrationSource],
    ["Retained c5 source coverage", resident.integration.sourceCoverage.source],
    ["Retained c5 coverage evidence SHA-256", resident.integration.sourceCoverage.evidenceSha256],
    ["Five-round native source", resident.integration.nativeResident.source],
    ["Five-round native evidence SHA-256", resident.integration.nativeResident.evidenceSha256],
    ["Failed resident attempt source", resident.integration.nativeResidentAttempt.source],
    ["Failed resident attempt binary SHA-256", resident.integration.nativeResidentAttempt.binarySha256],
    ["Failed resident attempt evidence SHA-256", resident.integration.nativeResidentAttempt.evidenceSha256],
    ["Optimized resident binary SHA-256", resident.integration.optimizedResidentBuild.binarySha256],
    ["Optimized resident build evidence SHA-256", resident.integration.optimizedResidentBuild.evidenceSha256],
    ["Unpublished paired-export compiler candidate", resident.integration.hostProgress.compilerCandidate.source],
    ["Retained 0fe compiler pin", resident.integration.hostProgress.compiler.source],
    ["Retained 0fe aggregate snapshot", resident.integration.hostProgress.compiler.ferricSource],
    ["Retained 0fe HSACO SHA-256", resident.integration.hostProgress.compiler.imageSha256],
    ["Retained 0fe manifest SHA-256", resident.integration.hostProgress.compiler.manifestSha256],
    ["Retained 0fe compiler evidence SHA-256", resident.integration.hostProgress.compiler.evidenceSha256],
    ["Retained 0fe ELF inspection SHA-256", resident.integration.hostProgress.compiler.inspectionSha256],
    ["Seven-case tooling tested source", resident.integration.hostProgress.r29.source],
    ["Seven-case tooling evidence SHA-256", resident.integration.hostProgress.r29.evidenceSha256],
    ["Repeated resident engine tested source", resident.integration.hostProgress.resident.engineSource],
    ["Retained adapter tested source", resident.integration.hostProgress.resident.adapterSource],
    ["Matrix-kernel host-tested candidate", resident.integration.hostProgress.mfma.source],
    ["Matrix-kernel unpublished compiler overlay", resident.integration.hostProgress.mfma.compilerOverlay],
    ["Matrix-kernel scoped Clippy candidate", resident.integration.hostProgress.mfma.clippySource],
    ["Retained 4f6 compiler pin", resident.integration.latestCompiler.source],
    ["Retained 4f6 aggregate snapshot", resident.integration.latestCompiler.ferricSource],
    ["Retained 4f6 HSACO SHA-256", resident.integration.latestCompiler.imageSha256],
    ["Retained 4f6 manifest SHA-256", resident.integration.latestCompiler.manifestSha256],
    ["Retained 4f6 evidence SHA-256", resident.integration.latestCompiler.evidenceSha256],
    ["Retained 4f6 inspection SHA-256", resident.integration.latestCompiler.inspectionSha256],
    ["Selected numerical capture source", resident.integration.selectedNumerical.source],
    ["Selected numerical compiler pin", resident.integration.selectedNumerical.compilerSource],
    ["Selected numerical evidence SHA-256", resident.integration.selectedNumerical.evidenceSha256],
    ["Selected comparison SHA-256", resident.integration.selectedNumerical.comparisonSha256],
    ["Selected explanatory metrics SHA-256", resident.integration.selectedNumerical.diagnosticsSha256],
    ["Retained ccfd compiler pin", resident.integration.compiler.source],
    ["Retained bb2 aggregate snapshot", resident.integration.compiler.ferricSource],
    ["Retained ccfd HSACO SHA-256", resident.integration.compiler.imageSha256],
    ["Retained ccfd manifest SHA-256", resident.integration.compiler.manifestSha256],
    ["Compiler and aggregate evidence SHA-256", resident.integration.compiler.evidenceSha256],
    ["Independent ELF inspection SHA-256", resident.integration.compiler.inspectionSha256],
    ["Latest four-token native evidence SHA-256", resident.integration.latestNativeSmoke.evidenceSha256],
    ["Latest four-token native report SHA-256", resident.integration.latestNativeSmoke.reportSha256],
    ["Frozen historical reference result SHA-256", resident.integration.latestNativeSmoke.referenceResultSha256],
    ["Native c6 smoke binary source", resident.integration.nativeSmoke.binarySource],
    ["Native c6 smoke binary SHA-256", resident.integration.nativeSmoke.binarySha256],
    ["Native c6 smoke artifact source", resident.integration.nativeSmoke.artifactSource],
    ["Native c6 smoke HSACO SHA-256", resident.integration.nativeSmoke.imageSha256],
    ["Native c6 smoke evidence SHA-256", resident.integration.nativeSmoke.evidenceSha256],
    ["Native c6 smoke report SHA-256", resident.integration.nativeSmoke.reportSha256],
    ["Retained bb2 host snapshot", resident.integration.currentHost.source],
    ["Retained bb2 host evidence SHA-256", resident.integration.currentHost.evidenceSha256],
    ["Retained bb2 binaries evidence SHA-256", resident.integration.currentHost.releaseEvidenceSha256],
    ["Retained d094 coverage source", resident.integration.currentCoverage.source],
    ["Retained d094 coverage evidence SHA-256", resident.integration.currentCoverage.evidenceSha256],
    ["Assertion differential evidence SHA-256", resident.integration.compiler.differential.evidenceSha256],
    ["Earlier private engine host snapshot", resident.integration.host.source],
    ["Earlier source-policy evidence SHA-256", resident.integration.sourcePolicy.evidenceSha256],
    ["Catch-up inventory evidence SHA-256", resident.integration.coverage.evidenceSha256],
    ["Frozen canonical model producer", resident.integration.prepack.producerSource],
    ["Canonical producer compiler pin", resident.integration.prepack.compilerSource],
    ["Published typed compiler diagnostic", resident.followup.compilerDiagnostic.source],
    ["Private literal-coordinate repair", resident.followup.coordinateRepair.source],
    ["Coordinate and getter evidence SHA-256", resident.followup.coordinateRepair.evidenceSha256],
    ["Published completed-read generation getter", resident.followup.runtimeGetter.source],
    ["Earlier private repin; then-pending combined validation", resident.followup.runtimeGetter.ferricPinSource],
    ["Private selected cursor-proof source", resident.followup.cursorProof.source],
    ["Cursor-proof compiler pin", resident.followup.cursorProof.compilerSource],
    ["Selected Verus evidence SHA-256", resident.followup.cursorProof.evidenceSha256],
    ["Private resident implementation", resident.selection.residentSource],
    ["Private sealed owner-plan selection", resident.selection.ownerSource],
    ["Private combined host snapshot", resident.host.engine.source],
    ["Private service identity repair", resident.host.service.source],
    ["Private focused source-policy checks", resident.host.policySource],
    ["Private allocation-free planning check", resident.host.allocationFreePlanningSource],
    ["Private source-inventory snapshot", resident.host.sourceInventory.source],
    ["Source-inventory tree", resident.host.sourceInventory.tree],
    ["Verified-modules inventory SHA-256", resident.host.sourceInventory.manifestSha256],
    ["Final source-gate evidence SHA-256", resident.host.sourceInventory.evidenceSha256],
    ["Earlier compiler build and extraction", resident.compiler.testedSource],
    ["Compiler repair base", resident.compiler.repairBase],
    ["Published compiler repair", resident.compiler.publishedSource],
    ["Upstream observed at the earlier eaa checkpoint", resident.compiler.observedLatestSource],
    ["Compiler test evidence SHA-256", resident.compiler.evidenceSha256],
    ["Private aggregate snapshot", resident.compiler.aggregateSource],
    ["Aggregate build and emission evidence SHA-256", resident.compiler.aggregateEvidenceSha256],
    ["Earlier unattributed emission failure log SHA-256", resident.compiler.emissionLogSha256],
  ]) {
    residentPins.append(element("dt", "", label), element("dd", "", value));
  }
  residentDetails.append(residentPins);
  residentProgress.append(residentDetails);

  const performance = window.FERRIC_PERFORMANCE;
  const measured = document.querySelector("[data-performance]");
  function range(values, digits = 3) {
    return values === null ? "n/a" : values.map((value) => value.toFixed(digits)).join(" to ");
  }
  function performanceTable(caption, headings, rows, parent = measured) {
    const wrap = element("div", "transition-table-wrap");
    wrap.tabIndex = 0;
    wrap.setAttribute("role", "region");
    wrap.setAttribute("aria-label", caption);
    const table = element("table", "transition-table performance-table");
    table.append(element("caption", "visually-hidden", caption));
    const head = element("thead", "");
    const heading = element("tr", "");
    headings.forEach((label) => {
      const cell = element("th", "", label);
      cell.scope = "col";
      heading.append(cell);
    });
    head.append(heading);
    const body = element("tbody", "");
    rows.forEach((values) => {
      const row = element("tr", "");
      values.forEach((value) => row.append(element("td", "", value)));
      body.append(row);
    });
    table.append(head, body);
    wrap.append(table);
    parent.append(wrap);
  }
  const attentionProgress = document.querySelector("[data-attention-progress]");
  const attention = project.attentionCheckpoint;
  attentionProgress.append(element("h3", "", "Seven attention operation groups"),
    element("p", "performance-scope", "One TP1 run: 128 input tokens, 128 outputs, baseline attention, wave-v11 argmax, no ordered batches or prefix caching. Exact reference and clean teardown passed."),
    element("p", "", "The attention parent spans 56.580 host seconds. Each group has 4,860 observations across 135 batches and 36 layers. These host intervals include dispatch and waiting; they are not isolated GPU-kernel durations."));
  performanceTable("Single-run attention operation groups: host intervals, not GPU durations",
    ["Operation group", "Host span (s)", "Share of parent (%)"],
    attention.attribution.groups.map(([, label, nanoseconds]) => [label,
      (nanoseconds / 1e9).toFixed(3), (100 * nanoseconds / attention.attribution.parentNs).toFixed(2)]), attentionProgress);
  attentionProgress.append(element("p", "", "Sibling groups are disjoint; parent and IPC spans overlap and must not be added together. The GQA group identifies the largest recorded host interval in this profile, not a measured optimization gain or a cross-run comparison."),
    element("h3", "", "Eight finite TP1 attention fixtures"),
    element("p", "", project.attentionReadiness[1].detail),
    element("p", "", project.attentionReadiness[2].detail));
  const attentionMetrics = attention.composition.metrics;
  attentionProgress.append(element("h3", "", "Combined attention / argmax: full128 ABBA"),
    element("p", "performance-scope", "Instrumented native TP1, context 256, 128 input / 128 output tokens, fixed wave-v11 argmax. Baseline/wave/wave/baseline; n=2 per mode. These host diagnostics are not HTTP serving or GPU durations."));
  performanceTable("Combined attention full128 ABBA: native host means and run ranges",
    ["Attention", "Mean TTFT (s)", "Mean TPOT (ms)", "Mean per-run output (tok/s)", "TPOT min / max (ms)", "Output min / max (tok/s)"],
    ["baseline", "wave"].map((mode) => {
      const row = attentionMetrics[mode];
      return [mode, row.ttftSeconds.toFixed(6), (row.tpotSeconds * 1000).toFixed(3),
        row.outputTokensPerSecond.toFixed(6), row.tpotRangeSeconds.map((x) => (x * 1000).toFixed(3)).join(" / "),
        row.outputRateRange.map((x) => x.toFixed(6)).join(" / ")];
    }), attentionProgress);
  attentionProgress.append(element("p", "", project.attentionReadiness[3].detail),
    element("p", "", "Output rate is the arithmetic mean of each run's 128 / workload seconds, not 256 divided by pooled duration. The mean-rate ratio is distinct from the average paired ratio. Qualification-run timings and the earlier six-run argmax cohort are excluded. Nested host intervals and IPC spans overlap; no isolated kernel speedup or additive gain follows."));
  const attentionDetails = element("details", "performance-identities");
  attentionDetails.append(element("summary", "", "Attention evidence identities"));
  const attentionPins = element("dl", "observation-facts");
  for (const [label, value] of [["Instrumented source", attention.attribution.source],
    ["Host-group diagnostic SHA-256", attention.attribution.reportSha256],
    ["Fixture source", attention.fixtures.source], ["Frozen v5 image SHA-256", attention.fixtures.imageSha256],
    ["Native fixture report SHA-256", attention.fixtures.reportSha256],
    ["Independent fixture replay SHA-256", attention.fixtures.replaySha256],
    ["Combined model qualification source", attention.composition.source],
    ["Combined model qualification note SHA-256", attention.composition.qualification.noteSha256],
    ["Four-case qualification roster SHA-256", attention.composition.qualification.localRosterSha256],
    ["Full128 ABBA summary SHA-256", attentionMetrics.summarySha256],
    ["Full128 ABBA manifest SHA-256", attentionMetrics.manifestSha256],
    ["Full128 ABBA raw archive SHA-256", attentionMetrics.rawArchiveSha256]]) {
    attentionPins.append(element("dt", "", label), element("dd", "", value));
  }
  attentionDetails.append(attentionPins);
  attentionProgress.append(attentionDetails);

  const submission = project.submissionCheckpoint;
  const submissionProgress = document.querySelector("[data-submission-progress]");
  submissionProgress.append(element("h3", "", "Ordered submission: host and correctness gates passed"),
    element("p", "", "The integrated opt-in submission path passes 51 host commands without retry, with 745 passed adapter test invocations, 38 ignored and eight doctests. Strict Clippy, 27 adapter policies, 38 source-gate tests, 31 protected policies and five unchanged inventories pass. All 104 custody ledgers match; the build reused a warm target. Separate checker/wrapper and reducer gates pass 26 and 18 synthetic CPU methods."),
    element("p", "performance-scope", "Both synchronous and ordered submission pass exact IDs and UTF-8 at 8 and 128 outputs. Fixed native TP1: 128 input tokens, context 256, wave attention and wave-v11 argmax, with prefix caching and speculation off."));
  performanceTable("Ordered-submission correctness: fixed native packet and ownership bounds",
    ["Submission", "Outputs", "Packets", "Batches", "Committed inputs"],
    submission.qualification.cases.map(([, mode, outputs, packets, batches, cursor]) =>
      [mode, String(outputs), packets.toLocaleString("en-US"), String(batches), String(cursor)]), submissionProgress);
  submissionProgress.append(element("p", "", "All four runs retire their pools, close workers normally and leave all eight GPUs idle. These are correctness cases, not comparison samples; their timings are excluded."),
    element("h3", "", "Ordered submission: full128 ABBA diagnostic"),
    element("p", "performance-scope", "Instrumented native TP1, context 256, 128 input / 128 output tokens, fixed wave attention and wave-v11 argmax. Synchronous / ordered / ordered / synchronous; n=2 per mode. These controller-wall diagnostics are not HTTP serving or GPU durations."));
  performanceTable("Submission full128 ABBA: native host means and run ranges",
    ["Submission", "Mean TTFT (s)", "Mean TPOT (ms)", "Mean per-run output (tok/s)", "TPOT min / max (ms)", "Output min / max (tok/s)"],
    ["synchronous", "ordered"].map((mode) => {
      const row = submission.abba[mode];
      return [mode, row.ttftSeconds.toFixed(6), (row.tpotSeconds * 1000).toFixed(3),
        row.outputTokensPerSecond.toFixed(6), row.tpotRangeSeconds.map((x) => (x * 1000).toFixed(3)).join(" / "),
        row.outputRateRange.map((x) => x.toFixed(6)).join(" / ")];
    }), submissionProgress);
  submissionProgress.append(element("p", "", "All four separate ABBA runs pass the exact 128-output reference and normal teardown. Ratios of arithmetic means describe 8.91% lower TTFT, 15.45% lower TPOT and 17.56% higher mean per-run output rate. Ordered TPOT ranges from 187.289 to 202.566 ms and output rate from 4.472915 to 4.813659 tokens/s. Paired rate gains are 21.03% and 14.03%; paired TPOT reductions are 18.20% and 12.75%. These small-sample observations are not stable estimates or confidence intervals."),
    element("p", "", "Output rate is the arithmetic mean of per-run rates, not 128 divided by mean duration. Ratios of arithmetic means and mean paired ratios are distinct. The four correctness timings and all older cohorts are excluded; no gains are added together. Setup, final detokenization, retirement and worker close are outside the timing boundary."),
    element("p", "", "Operation-group timings are not comparable across submission modes: synchronous spans include submission and waiting, while ordered spans measure command preparation only. Parent and IPC spans overlap. No GPU-duration, HTTP-serving, stable-gain, competitive, default, new Verus or M1 claim follows; all historical measurements remain separate."));
  const submissionDetails = element("details", "performance-identities");
  submissionDetails.append(element("summary", "", "Submission checkpoint identities"));
  const submissionPins = element("dl", "observation-facts");
  for (const [label, value] of [["Exact source", submission.source],
    ["Controller SHA-256", submission.controllerSha256], ["Experiment plan SHA-256", submission.planSha256],
    ["Host gate receipt SHA-256", submission.host.noteSha256],
    ["Correctness note SHA-256", submission.qualification.noteSha256],
    ["Forty-file correctness roster SHA-256", submission.qualification.rosterSha256],
    ["Forty-file correctness sizes SHA-256", submission.qualification.sizesSha256],
    ["Separate ABBA summary SHA-256", submission.abba.summarySha256],
    ["Separate ABBA manifest SHA-256", submission.abba.manifestSha256],
    ["Independent replay receipt SHA-256", submission.abba.replaySha256]]) {
    submissionPins.append(element("dt", "", label), element("dd", "", value));
  }
  submissionDetails.append(submissionPins);
  submissionProgress.append(submissionDetails);

  const liveHttp = project.liveHttpCheckpoint;
  const liveHttpProgress = document.querySelector("[data-live-http-progress]");
  liveHttpProgress.append(element("h3", "", "Wave attention / v11: context8192 HTTP"),
    element("p", "performance-scope", liveHttp.scope));
  performanceTable("Live wave/v11 HTTP: separate admitted context8192 cohorts",
    ["Submission", "Mean TTFT (ms)", "Mean TPOT (ms)", "Output tok/s"],
    liveHttp.cohorts.map((row) => [row.mode, row.ttftMeanMs.toFixed(3), row.tpotMeanMs.toFixed(3),
      row.outputTokensPerSecond.toFixed(6)]), liveHttpProgress);
  liveHttpProgress.append(element("p", "", liveHttp.measurement),
    element("p", "", liveHttp.correctness), element("p", "", liveHttp.imageScope),
    element("p", "", liveHttp.interpretation));
  if (liveHttp.orderedMeasurementState !== "independent-replay-passed") {
    liveHttpProgress.append(element("p", "", "Ordered HTTP measurement is awaiting completed independent replay; no ordered timing is published yet."));
  }
  const liveHttpDetails = element("details", "performance-identities");
  liveHttpDetails.append(element("summary", "", "Live HTTP percentiles and evidence"));
  performanceTable("Live wave/v11 HTTP: descriptive single-cohort percentiles",
    ["Submission", "TTFT p50 / p99 (ms)", "TPOT p50 / p99 (ms)", "Cohort (s)"],
    liveHttp.cohorts.map((row) => [row.mode, `${row.ttftP50Ms.toFixed(3)} / ${row.ttftP99Ms.toFixed(3)}`,
      `${row.tpotP50Ms.toFixed(3)} / ${row.tpotP99Ms.toFixed(3)}`, row.windowSeconds.toFixed(6)]), liveHttpDetails);
  const liveHttpPins = element("dl", "observation-facts");
  for (const [label, value] of [["Live controller source", liveHttp.source],
    ["Live controller SHA-256", liveHttp.controllerSha256], ["Controller dependency / v11 compiler source", liveHttp.compilerSource],
    ["Actual runtime worker source", liveHttp.workerSource], ["Actual runtime worker SHA-256", liveHttp.workerSha256],
    ["Unchanged HTTP client SHA-256", liveHttp.clientSha256], ["Independent reference SHA-256", liveHttp.referenceSha256],
    ...liveHttp.qualification.map((row) => [`${row.mode} context8192 qualification receipt SHA-256`, row.receiptSha256]),
    ...liveHttp.cohorts.flatMap((row) => [[`${row.mode} HTTP receipt SHA-256`, row.receiptSha256],
      [`${row.mode} independent replay SHA-256`, row.replaySummarySha256],
      [`${row.mode} twice-verified raw archive SHA-256`, row.archiveSha256]])]) {
    liveHttpPins.append(element("dt", "", label), element("dd", "", value));
  }
  liveHttpDetails.append(liveHttpPins);
  liveHttpProgress.append(liveHttpDetails);
  const liveHttpTeams = document.querySelector("[data-live-http-teams]");
  for (const team of liveHttp.teams) {
    const article = element("article", "team-item");
    const heading = element("div", "team-heading");
    const identity = element("div", "team-identity");
    identity.append(element("div", "team-scope-label", "Private source checkpoint"), element("h3", "", team.name));
    heading.append(identity, team.name === "Live HTTP" ? stateTag("observed")
      : element("span", "state-tag state-open", team.label));
    const facts = element("dl", "team-facts");
    for (const [label, value] of [["Source", team.source], ["Validation", team.detail]]) {
      facts.append(element("dt", "", label), element("dd", "", value));
    }
    article.append(heading, facts);
    liveHttpTeams.append(article);
  }

  const liveC1 = project.liveC1Checkpoint;
  const liveC1Progress = document.querySelector("[data-live-c1-checkpoint]");
  liveC1Progress.append(element("h3", "", "C1 live route: context8192 HTTP"),
    element("p", "performance-scope", liveC1.qualificationScope));
  performanceTable("C1 live route: same-binary finite HTTP cohorts",
    ["Layer projection", "Mean TTFT (ms)", "Mean TPOT (ms)", "Output tok/s"],
    liveC1.matchedCohorts.map((row) => [row.layerProjection, row.ttftMeanMs.toFixed(3),
      row.tpotMeanMs.toFixed(3), row.outputTokensPerSecond.toFixed(6)]), liveC1Progress);
  for (const text of [liveC1.measurement, liveC1.interpretation, liveC1.correctness, liveC1.exclusions])
    liveC1Progress.append(element("p", "", text));
  const liveC1Details = element("details", "performance-identities");
  liveC1Details.append(element("summary", "", "C1 live HTTP percentiles and evidence"));
  performanceTable("C1 live route: retained HTTP percentiles",
    ["Layer projection", "TTFT p50 / p99 (ms)", "TPOT p50 / p99 (ms)", "Window (s)"],
    liveC1.matchedCohorts.map((row) => [row.layerProjection,
      `${row.ttftP50Ms.toFixed(3)} / ${row.ttftP99Ms.toFixed(3)}`,
      `${row.tpotP50Ms.toFixed(3)} / ${row.tpotP99Ms.toFixed(3)}`, row.windowSeconds.toFixed(6)]), liveC1Details);
  const liveC1Pins = element("dl", "observation-facts");
  for (const [label, value] of [["Same live source", liveC1.source], ["Same controller SHA-256", liveC1.controllerSha256],
    ["Unchanged HTTP client SHA-256", liveC1.clientSha256], ["Actual worker SHA-256", liveC1.workerSha256],
    ...liveC1.qualification.map((row) => [`${row.layerProjection} qualification receipt SHA-256`, row.receiptSha256]),
    ...liveC1.matchedCohorts.flatMap((row) => [[`${row.layerProjection} matched receipt SHA-256`, row.receiptSha256],
      [`${row.layerProjection} replay SHA-256`, row.replaySha256], [`${row.layerProjection} archive SHA-256`, row.archiveSha256]]),
    ["Excluded zero-request attempt archive SHA-256", liveC1.excludedAttempt.archiveSha256]])
    liveC1Pins.append(element("dt", "", label), element("dd", "", value));
  liveC1Details.append(liveC1Pins);
  liveC1Progress.append(liveC1Details);
  const liveC1Teams = element("div", "team-grid");
  for (const [name, label, team] of [["Sharded argmax v13: static image checks", "ABI/resources passed", liveC1.v13],
    ["Query-hoist v14: finite native parity", "8 cases + CPU replay passed", liveC1.v14],
    ["Wave RMSNorm v15: model correctness", "4 full-Qwen cases passed", liveC1.v15],
    ["V14 canary and current runtime", "4 full-Qwen cases passed", liveC1.development]]) {
    const article = element("article", "team-item");
    const heading = element("div", "team-heading");
    heading.append(element("h3", "", name), element("span", "state-tag state-open", label));
    const facts = element("dl", "team-facts");
    for (const [key, value] of [["Source", team === liveC1.v15 ? team.current.source : team.source], ["Validation", team.detail]])
      facts.append(element("dt", "", key), element("dd", "", value));
    if (team.native) {
      for (const [key, value] of [["Native report SHA-256", team.native.reportSha256],
        ["CPU replay archive SHA-256", team.native.cpuReplayArchiveSha256]])
        facts.append(element("dt", "", key), element("dd", "", value));
    }
    if (team === liveC1.v15) {
      facts.prepend(element("dt", "", "Model source"), element("dd", "", team.model.source),
        element("dt", "", "Model checks"), element("dd", "", team.model.detail));
      facts.append(element("dt", "", "Emission SHA-256"), element("dd", "", team.current.emissionArchiveSha256),
        element("dt", "", "Image SHA-256"), element("dd", "", team.current.imageSha256),
        element("dt", "", "Native SHA-256"), element("dd", "", team.current.nativeArchiveSha256),
        element("dt", "", "Replay SHA-256"), element("dd", "", team.current.replayArchiveSha256),
        element("dt", "", "Model SHA-256"), element("dd", "", team.model.archiveSha256),
        element("dt", "", "Model replay"), element("dd", "", team.model.replayArchiveSha256),
        element("dt", "", "ABBA"), element("dd", "", team.abba.detail),
        element("dt", "", "ABBA summary"), element("dd", "", team.abba.summarySha256));
    }
    if (team === liveC1.development) {
      facts.append(element("dt", "", "Tested pin"), element("dd", "", team.fe2o3Source),
        element("dt", "", "Active pin"), element("dd", "", `${team.activeDependencySource}; V15 emission and scoped model checks passed`),
        element("dt", "", "V15 canary"), element("dd", "", `${team.v15ModelSource}; twenty host phases and four fixed-prompt cases passed; descriptive ABBA replay passed; no HTTP qualification`));
      facts.append(element("dt", "", "Native SHA-256"), element("dd", "", team.model.archiveSha256),
        element("dt", "", "Replay SHA-256"), element("dd", "", team.model.replayArchiveSha256));
    }
    article.append(heading, facts);
    liveC1Teams.append(article);
  }
  liveC1Progress.append(liveC1Teams);

  const c1 = project.c1Checkpoint;
  const c1Progress = document.querySelector("[data-c1-checkpoint]");
  c1Progress.append(element("h3", "", "C1 layer projection: variable native diagnostic"),
    element("p", "performance-scope", c1.scope));
  performanceTable("C1 layer projection: same-binary n=2 diagnostic",
    ["Layer projection", "Mean TTFT (ms)", "Mean TPOT (ms)", "Mean output tok/s"],
    c1.rows.map((row) => [row.label, (row.ttftMeanSeconds * 1000).toFixed(3),
      (row.tpotMeanSeconds * 1000).toFixed(3), row.meanPerRunOutputTokensPerSecond.toFixed(6)]), c1Progress);
  c1Progress.append(element("p", "", c1.interpretation),
    element("p", "", `TPOT ranges: MFMA ${c1.rows[0].tpotRangeSeconds.map((x) => (x * 1000).toFixed(3)).join(" to ")} ms; C1 ${c1.rows[1].tpotRangeSeconds.map((x) => (x * 1000).toFixed(3)).join(" to ")} ms.`),
    element("p", "", c1.limitations), element("p", "", c1.correctness),
    element("h3", "", "v13 ownership: standalone integer proof"),
    element("p", "", c1.integerProof.scope));
  const c1Teams = element("div", "team-grid");
  for (const team of c1.teams) {
    const article = element("article", "team-item");
    const heading = element("div", "team-heading");
    heading.append(element("h3", "", team.name), element("span", "state-tag state-open", team.label));
    const facts = element("dl", "team-facts");
    for (const [label, value] of [["Source", team.source], ["Validation", team.detail]])
      facts.append(element("dt", "", label), element("dd", "", value));
    article.append(heading, facts);
    c1Teams.append(article);
  }
  c1Progress.append(c1Teams);
  const c1Details = element("details", "performance-identities");
  c1Details.append(element("summary", "", "C1 diagnostic and integer-proof evidence"));
  const c1Pins = element("dl", "observation-facts");
  for (const [label, value] of [["Same controller source", c1.source], ["Controller SHA-256", c1.controllerSha256],
    ["Independent summary SHA-256", c1.summarySha256], ["Forty-file manifest SHA-256", c1.manifestSha256],
    ["Replay receipt SHA-256", c1.replaySha256], ["Complete native archive SHA-256", c1.nativeArchiveSha256],
    ["Complete replay archive SHA-256", c1.replayArchiveSha256], ["Integer proof source", c1.integerProof.source],
    ["Integer proof source SHA-256", c1.integerProof.sourceSha256], ["Raw Verus JSON SHA-256", c1.integerProof.rawSha256],
    ["Complete proof archive SHA-256", c1.integerProof.archiveSha256]])
    c1Pins.append(element("dt", "", label), element("dd", "", value));
  c1Details.append(c1Pins);
  c1Progress.append(c1Details);

  const residual = project.residualDiagnostic;
  const residualProgress = document.querySelector("[data-residual-diagnostic]");
  residualProgress.append(element("h3", "", "Residual-tail diagnostic: decode regression"),
    element("p", "performance-scope", residual.scope));
  performanceTable("Residual-tail native diagnostic: n=2 per exact binary",
    ["Ordered residual policy", "Mean TTFT (ms)", "Mean TPOT (ms)", "Mean output tok/s"],
    residual.rows.map((row) => [row.label, (row.ttftMeanSeconds * 1000).toFixed(3),
      (row.tpotMeanSeconds * 1000).toFixed(3), row.meanPerRunOutputTokensPerSecond.toFixed(6)]),
    residualProgress);
  residualProgress.append(element("p", "", residual.interpretation),
    element("p", "", `TPOT ranges: old ${residual.rows[0].tpotRangeSeconds.map((x) => (x * 1000).toFixed(3)).join(" to ")} ms; new ${residual.rows[1].tpotRangeSeconds.map((x) => (x * 1000).toFixed(3)).join(" to ")} ms.`),
    element("p", "", residual.limitations), element("p", "", residual.correctness));
  const residualDetails = element("details", "performance-identities");
  residualDetails.append(element("summary", "", "Residual diagnostic evidence"));
  const residualPins = element("dl", "observation-facts");
  for (const [label, value] of [
    ...residual.rows.flatMap((row) => [[`${row.profile} source`, row.source],
      [`${row.profile} controller SHA-256`, row.controllerSha256]]),
    ["Independent summary SHA-256", residual.summarySha256],
    ["Forty-file manifest SHA-256", residual.manifestSha256],
    ["Independent replay receipt SHA-256", residual.replaySha256],
    ["Complete native archive SHA-256", residual.nativeArchiveSha256],
    ["Complete replay archive SHA-256", residual.replayArchiveSha256]]) {
    residualPins.append(element("dt", "", label), element("dd", "", value));
  }
  residualDetails.append(residualPins);
  residualProgress.append(residualDetails);

  const matched = project.matched128;
  measured.append(element("h3", "", "First matched Ferric / vLLM cell"),
    element("p", "performance-scope", matched.scope), element("p", "", matched.interpretation));
  performanceTable("Matched 128/128: client latency and output rate",
    ["Engine", "Mean TTFT (ms)", "Mean TPOT (ms)", "Output tok/s"],
    matched.metrics.map((row) => [row.engine, row.ttftMeanMs.toFixed(3), row.tpotMeanMs.toFixed(3),
      row.outputTokensPerSecond.toFixed(6)]));
  measured.append(element("p", "", matched.measurement), element("p", "", matched.correctness),
    element("p", "", matched.sglangNote));
  const matchedDetails = element("details", "performance-identities");
  matchedDetails.append(element("summary", "", "Matched-cell percentiles and evidence"));
  performanceTable("Matched 128/128: descriptive single-cohort percentiles",
    ["Engine", "TTFT p50 / p99 (ms)", "TPOT p50 / p99 (ms)", "Cohort (s)"],
    matched.metrics.map((row) => [row.engine, `${row.ttftP50Ms.toFixed(3)} / ${row.ttftP99Ms.toFixed(3)}`,
      `${row.tpotP50Ms.toFixed(3)} / ${row.tpotP99Ms.toFixed(3)}`, row.windowSeconds.toFixed(6)]), matchedDetails);
  const matchedPins = element("dl", "observation-facts");
  for (const [label, value] of [["Read-only pair replay SHA-256", matched.sourceSummarySha256],
    ["Timed client SHA-256", matched.clientSha256], ["Independent reference SHA-256", matched.referenceSha256],
    ...matched.metrics.map((row) => [`${row.engine} receipt SHA-256`, row.receiptSha256])]) {
    matchedPins.append(element("dt", "", label), element("dd", "", value));
  }
  matchedDetails.append(matchedPins);
  measured.append(matchedDetails, element("h3", "", "Earlier engineering measurements"));
  measured.append(
    element("p", "performance-scope", performance.scope),
    element("p", "", performance.interpretation),
  );
  function singleRunTables(section, title, caption) {
    measured.append(element("h3", "", title), element("p", "", section.scope));
    performanceTable(`${caption}: process windows`,
      ["Profile / Reps", "Output tok/s", "Workload Window (s)", "Setup (s)", "Whole Process (s)"],
      section.profiles.map((profile) => [
        `${profile.name} / n=1`, profile.outputTokensPerSecond.toFixed(6),
        profile.workloadSeconds.toFixed(3), profile.setupSeconds.toFixed(3), profile.wholeSeconds.toFixed(3),
      ]));
    measured.append(element("p", "", section.interpretation));
    performanceTable(`${caption}: request latencies`,
      ["Profile", "Request / Decode Gaps", "TTFT (s)", "TPOT (s)"],
      section.profiles.flatMap((profile) => profile.requestLatencies.map((values, index) => [
        `${profile.name} / n=1`, `${performance.requests[index].name} / ${performance.requests[index].gapsPerRepetition}`,
        values[0].toFixed(3), values[1] === null ? "n/a" : values[1].toFixed(3),
      ])));
  }
  const fp32 = performance.fp32HeadV7;
  measured.append(element("h3", "", "FP32 head on TP1: faster requests, slower startup"),
    element("p", "", fp32.scope), element("p", "", fp32.interpretation));
  performanceTable("FP32 head: three separate process-window observations",
    ["TP1 Profile / Reps", "Output tok/s", "Workload (s)", "Setup (s)", "Whole (s)", "Reuse TTFT / TPOT (s)"],
    fp32.profiles.map((profile) => [`${profile.label} / n=1`, profile.outputTokensPerSecond.toFixed(6),
      profile.workloadSeconds.toFixed(3), profile.setupSeconds.toFixed(3), profile.wholeSeconds.toFixed(3),
      `${profile.requestLatencies[3][0].toFixed(3)} / ${profile.requestLatencies[3][1].toFixed(3)}`]));
  performanceTable("FP32 head: only the two approved candidate-to-baseline pairs",
    ["Candidate / Baseline", "Output Rate Ratio", "Setup Ratio", "Whole Ratio", "Reuse TTFT Ratio", "Reuse TPOT Ratio"],
    fp32.pairs.map((pair) => [`${pair.label}: ${pair.candidate} / ${pair.baseline}`,
      ...[pair.rateRatio, pair.setupRatio, pair.wholeRatio, pair.reuseTtftRatio, pair.reuseTpotRatio]
        .map((value) => `${value.toFixed(3)}x`)]));
  measured.append(element("p", "", "Ratios use candidate divided by baseline. Higher output rate is better; higher setup, whole-process or latency ratios are worse. Pair summaries reuse the same three observations, not extra repetitions."));
  const headLatencies = element("details", "performance-identities");
  headLatencies.append(element("summary", "", "All FP32-head request latencies"));
  performanceTable("FP32 head: all 12 named request latencies",
    ["Profile / Request / Decode Gaps", "TTFT (s)", "TPOT (s)"],
    fp32.profiles.flatMap((profile) => profile.requestLatencies.map((values, index) => [
      `${profile.name} / ${performance.requests[index].name} / ${performance.requests[index].gapsPerRepetition}`,
      values[0].toFixed(3), values[1] === null ? "n/a" : values[1].toFixed(3),
    ])), headLatencies);
  measured.append(headLatencies, element("p", "", fp32.precisionScope), element("p", "", fp32.sourceScope),
    element("p", "", fp32.limits));
  const rounds = performance.concurrentRounds;
  if (rounds) {
    measured.append(element("h3", "", "Concurrent peer rounds: matched TP2 and TP8"),
      element("p", "", rounds.scope), element("p", "", rounds.interpretation));
    performanceTable("Concurrent peer matrix: six process-window observations",
      ["TP / Mode / Reps", "Output tok/s", "Workload (s)", "Setup (s)", "Whole (s)", "Reuse TTFT / TPOT (s)"],
      rounds.profiles.map((profile) => [`TP${profile.world} / ${profile.name} / n=1`,
        profile.outputTokensPerSecond.toFixed(6), profile.workloadSeconds.toFixed(3), profile.setupSeconds.toFixed(3),
        profile.wholeSeconds.toFixed(3), `${profile.requestLatencies[3][0].toFixed(3)} / ${profile.requestLatencies[3][1].toFixed(3)}`]));
    performanceTable("Concurrent peer matrix: separate workload-rate baselines",
      ["TP / Reps per Mode", "Round / Serial", "Round / Host Staged"],
      rounds.worlds.map((world) => [`TP${world.world} / n=1`, `${world.roundOverSerial.toFixed(3)}x`, `${world.roundOverHost.toFixed(3)}x`]));
    const roundLatencies = element("details", "performance-identities");
    roundLatencies.append(element("summary", "", "All concurrent-matrix request latencies"));
    performanceTable("Concurrent peer matrix: all 24 named request latencies",
      ["TP / Mode / Request / Gaps", "TTFT (s)", "TPOT (s)"],
      rounds.profiles.flatMap((profile) => profile.requestLatencies.map((values, index) => [
        `TP${profile.world} / ${profile.name} / ${performance.requests[index].name} / ${performance.requests[index].gapsPerRepetition}`,
        values[0].toFixed(3), values[1] === null ? "n/a" : values[1].toFixed(3),
      ])), roundLatencies);
    measured.append(roundLatencies, element("p", "", rounds.measurementScope), element("p", "", rounds.sourceScope),
      element("p", "", rounds.limits));
  }
  const current = performance.currentCompatibility;
  measured.append(element("h3", "", "Current-controller combinations: mixed correctness"),
    element("p", "", current.scope), element("p", "", current.interpretation));
  performanceTable("Current-controller compatibility: rejected profiles",
    ["Rejected Profile / Reps", "Request", "Expected Token IDs", "Observed Token IDs"],
    current.rejected.map((profile) => [`${profile.name} / n=1`, profile.requestName,
      profile.expectedTokens.join(", "), profile.observedTokens.join(", ")]));
  performanceTable("Current-controller compatibility: accepted process windows",
    ["Accepted Profile / Reps", "Output tok/s", "Workload Window (s)", "Setup (s)", "Whole Process (s)"],
    current.accepted.map((profile) => [`${profile.name} / n=1`, profile.outputTokensPerSecond.toFixed(6),
      profile.workloadSeconds.toFixed(3), profile.setupSeconds.toFixed(3), profile.wholeSeconds.toFixed(3)]));
  performanceTable("Current-controller compatibility: observed rows and profiles",
    ["Profile", "Image / Projection / Pruning / Collective", "Row Capacity / Actual Batch Rows"],
    current.accepted.map((profile) => [profile.name,
      `${profile.imageProfile} / ${profile.projection} / ${profile.outputHeadPruning ? "on" : "off"} / ${profile.collective ?? "host-staged"}`,
      `${profile.rowCapacity} / ${profile.actualBatchRows.join(", ")}`]));
  const currentLatencies = element("details", "performance-identities");
  currentLatencies.append(element("summary", "", "All accepted current-controller request latencies"));
  performanceTable("Current-controller compatibility: accepted named request latencies",
    ["Profile / Request / Gaps", "TTFT (s)", "TPOT (s)"],
    current.accepted.flatMap((profile) => profile.requestLatencies.map((values, index) => [
      `${profile.name} / ${performance.requests[index].name} / ${performance.requests[index].gapsPerRepetition}`,
      values[0].toFixed(3), values[1] === null ? "n/a" : values[1].toFixed(3),
    ])), currentLatencies);
  measured.append(currentLatencies);
  const cohorts = performance.replicaCohorts;
  measured.append(element("h3", "", "Eight-GPU allocation cohorts: 64 outputs"), element("p", "", cohorts.scope));
  performanceTable("Replica cohorts: common-release throughput and process windows",
    ["Layout / Reps", "Output tok/s", "Release to Last Output (s)", "Barrier Setup (s)", "Spawn to Reap (s)"],
    cohorts.profiles.map((profile) => [`${profile.layout} / n=1`, profile.outputTokensPerSecond.toFixed(6),
      (profile.releaseToLastOutputNs / 1e9).toFixed(3), (profile.barrierSetupNs / 1e9).toFixed(3),
      (profile.spawnToReapNs / 1e9).toFixed(3)]));
  measured.append(element("p", "", cohorts.interpretation), element("p", "", cohorts.clockScope));
  performanceTable("Replica cohorts: row policy and loaded weight payloads",
    ["Layout", "Rows / Instance / Total", "Physical Token Rows", "Host Weight Bytes", "GPU Base Weight Bytes", "GPU Transposed Bytes"],
    cohorts.profiles.map((profile) => [profile.layout, `${profile.perInstanceRows} / ${profile.totalRowBudget}`,
      profile.physicalTokenRows, profile.hostWeightBytes, profile.gpuBaseWeightBytes, profile.gpuTransposedWeightBytes]));
  const replicaLatencies = element("details", "performance-identities");
  replicaLatencies.append(element("summary", "", "All replica-cohort request latencies"));
  performanceTable("Replica cohorts: eight named requests per layout",
    ["Layout / Request / Instance", "Admission TTFT (s)", "Release to First Token (s)", "TPOT (s) / 7 Gaps"],
    cohorts.profiles.flatMap((profile) => profile.requestLatencies.map((values, index) => [
      `${profile.layout} / replica-request-${String(index).padStart(2, "0")} / replica-${String(values[0]).padStart(2, "0")}`,
      (values[1] / 1e9).toFixed(3), (values[2] / 1e9).toFixed(3), values[3].toFixed(3),
    ])), replicaLatencies);
  measured.append(replicaLatencies, element("p", "", cohorts.limits));
  const wide = performance.wideRowPair;
  measured.append(element("h3", "", "True 32-row execution: a latency tradeoff"), element("p", "", wide.scope));
  performanceTable("Wide row-policy pair: common-release process windows",
    ["Row / Chunk Policy / Reps", "Output tok/s", "Release to Last Output (s)", "Barrier Setup (s)", "Spawn to Reap (s)"],
    wide.profiles.map((profile) => [`${profile.rows} / ${profile.prefillChunk} / n=1`, profile.outputTokensPerSecond.toFixed(6),
      (profile.releaseToLastOutputNs / 1e9).toFixed(3), (profile.barrierSetupNs / 1e9).toFixed(3),
      (profile.spawnToReapNs / 1e9).toFixed(3)]));
  measured.append(element("p", "", wide.interpretation));
  performanceTable("Wide row-policy pair: observed batch rows",
    ["Policy", "Actual Batch Rows / 96 Total"],
    wide.profiles.map((profile) => [profile.name, profile.actualBatchRows.join(", ")]));
  const wideLatencies = element("details", "performance-identities");
  wideLatencies.append(element("summary", "", "All wide-policy request latencies"));
  performanceTable("Wide row-policy pair: eight named requests per policy",
    ["Policy / Request", "Admission TTFT (s)", "Release to First Token (s)", "TPOT (s) / 7 Gaps"],
    wide.profiles.flatMap((profile) => profile.requestLatencies.map((values, index) => [
      `${profile.rows} rows / replica-request-${String(index).padStart(2, "0")}`,
      (values[0] / 1e9).toFixed(3), (values[1] / 1e9).toFixed(3), values[2].toFixed(3),
    ])), wideLatencies);
  measured.append(wideLatencies, element("p", "", wide.timing));
  const repeated = performance.mfmaRepeated;
  const summaryRange = (values, digits = 3) => values[0] === null ? "n/a"
    : `${((values[0] + values[1]) / 2).toFixed(digits)} [${Math.min(...values).toFixed(digits)}, ${Math.max(...values).toFixed(digits)}]`;
  function repeatedTables(section, firstProfiles, title, label) {
    const groups = firstProfiles.map((first, index) => [first, section.secondProfiles[index]]);
    measured.append(element("h3", "", title), element("p", "", section.scope));
    performanceTable(`Repeated ${label} pair: mean and observed range`,
      ["Profile / Reps", "Output tok/s", "Workload Window (s)", "Setup (s)", "Whole Process (s)"],
      groups.map((runs) => [`${runs[0].name} / n=2`,
        ...["outputTokensPerSecond", "workloadSeconds", "setupSeconds", "wholeSeconds"].map((key, index) =>
          summaryRange(runs.map((run) => run[key]), index === 0 ? 6 : 3))]));
    measured.append(element("p", "", section.interpretation));
    const latencies = element("details", "performance-identities");
    latencies.append(element("summary", "", `All repeated ${label} request latencies`));
    performanceTable(`Repeated ${label} pair: named request means and ranges`,
      ["Profile / Request / Gaps Per Run", "TTFT (s)", "TPOT (s)"],
      groups.flatMap((runs) => performance.requests.map((request, index) => [
        `${runs[0].name} / ${request.name} / ${request.gapsPerRepetition}`,
        summaryRange(runs.map((run) => run.requestLatencies[index][0])),
        summaryRange(runs.map((run) => run.requestLatencies[index][1])),
      ])), latencies);
    measured.append(latencies);
  }
  repeatedTables(repeated, performance.mfmaPair.profiles, "MFMA repeated: request gains, startup cost", "MFMA");
  singleRunTables(performance.mfmaPruning, "MFMA plus pruning: no additive gain observed", "Cumulative MFMA and pruning");
  singleRunTables(performance.mfmaPair, "MFMA R1 checkpoint: faster requests, slower startup", "Matched MFMA pair");
  repeatedTables(performance.deviceTp1Repeated, performance.deviceTp1Pair.profiles, "TP1 residual repeated: small decode change", "TP1 residual");
  singleRunTables(performance.deviceTp1Pair, "TP1 device residual pair", "Matched TP1 residual pair");
  const peerControls = performance.peerSourceControls;
  measured.append(element("h3", "", "Source-matched peer controls: a regression"),
    element("p", "", peerControls.scope), element("p", "", peerControls.interpretation));
  performanceTable("Source-matched peer controls: workload and process windows",
    ["World / Path / Reps", "Output tok/s", "Workload Window (s)", "Setup (s)", "Whole Process (s)"],
    peerControls.controls.flatMap((control, index) => [control, performance.peerObservations.profiles[index]].map((profile) => [
      `${profile.name} / n=1`, profile.outputTokensPerSecond.toFixed(6), profile.workloadSeconds.toFixed(3),
      profile.setupSeconds.toFixed(3), profile.wholeSeconds.toFixed(3),
    ])));
  const peerControlLatencies = element("details", "performance-identities");
  peerControlLatencies.append(element("summary", "", "All source-matched peer request latencies"));
  performanceTable("Source-matched peer controls: per-request latency",
    ["World / Path / Request", "TTFT (s)", "TPOT (s)"],
    peerControls.controls.flatMap((control, index) => [control, performance.peerObservations.profiles[index]].flatMap((profile) =>
      profile.requestLatencies.map((values, requestIndex) => [
        `${profile.name} / ${performance.requests[requestIndex].name}`,
        values[0].toFixed(3), values[1] === null ? "n/a" : values[1].toFixed(3),
      ]))), peerControlLatencies);
  measured.append(peerControlLatencies);
  singleRunTables(performance.peerObservations, "Original peer checkpoint: correctness, not a speedup", "Original peer checkpoint");
  measured.append(element("h3", "", "CPU transpose helper only"), element("p", "", performance.hostTranspose.scope));
  performanceTable("CPU-only transpose helper, sums of per-case medians",
    ["Shard World / Cases", "Baseline Helper (s)", "Tiled Helper (s)", "Helper Speedup"],
    performance.hostTranspose.groups.map((group) => [
      `TP${group.world} / ${group.cases}`, group.baselineSeconds.toFixed(6),
      group.tiledSeconds.toFixed(6), `${group.helperSpeedup.toFixed(3)}x`,
    ]));
  measured.append(element("p", "", performance.hostTranspose.interpretation));
  singleRunTables(performance.transposeModelPair, "Setup transpose: full-model observation", "Setup transpose model pair");
  measured.append(element("h3", "", "Historical repeated profiles"));
  performanceTable("Standalone profile ranges, two repetitions each",
    ["Profile / Reps", "Output tok/s", "Workload Window (s)", "Setup (s)", "Whole Process (s)"],
    performance.variants.map((variant) => [
      `${variant.name} / n=${variant.repetitions}`,
      range(variant.outputTokensPerSecond, 6), range(variant.workloadSeconds),
      range(variant.setupSeconds), range(variant.wholeSeconds),
    ]));
  measured.append(
    element("p", "", performance.statistics),
    element("h3", "", "Latency by request identity"),
  );
  performanceTable("Baseline and operational-only request latency ranges, two repetitions each",
    ["Request / Decode Gaps per Rep", "Baseline TTFT (s)", "Operational TTFT (s)", "Baseline TPOT (s)", "Operational TPOT (s)"],
    performance.requests.map((request) => [
      `${request.name} / ${request.gapsPerRepetition}`,
      range(request.baselineTtft), range(request.operationalTtft),
      range(request.baselineTpot), range(request.operationalTpot),
    ]));
  measured.append(
    element("p", "", `Pruning-only reuse-prefix: TTFT ${range(performance.pruningReuse.ttftSeconds)} s; TPOT ${range(performance.pruningReuse.tpotSeconds)} s, one decode gap per repetition. Large variation prevents an isolated speedup claim.`),
    element("h3", "", "Single-repetition ablations"),
    element("p", "", performance.ablations.scope),
  );
  performanceTable("Single-repetition ablations, throughput and process windows",
    ["Profile / Kind / Reps", "Output tok/s", "Workload Window (s)", "Setup (s)", "Whole Process (s)"],
    performance.ablations.profiles.map((profile) => [
      `${profile.name} / ${profile.kind} / n=1`, profile.outputTokensPerSecond.toFixed(6),
      profile.workloadSeconds.toFixed(3), profile.setupSeconds.toFixed(3), profile.wholeSeconds.toFixed(3),
    ]));
  performanceTable("Single-repetition reuse-prefix latency, one decode gap per run",
    ["Profile / Reps", "Reuse TTFT (s)", "Reuse TPOT (s)", "Workload Rate / Control"],
    performance.ablations.profiles.map((profile) => [
      `${profile.name} / n=1`, ...profile.requestLatencies[3].map((value) => value.toFixed(3)),
      `${(profile.outputTokensPerSecond / performance.ablations.profiles[0].outputTokensPerSecond).toFixed(3)}x`,
    ]));
  measured.append(element("p", "", performance.ablations.interpretation));
  const allLatencies = element("details", "performance-identities");
  allLatencies.append(element("summary", "", "All single-run request latencies"));
  performanceTable("Single-run ablation latencies by request identity",
    ["Profile", "Request / Decode Gaps", "TTFT (s)", "TPOT (s)"],
    performance.ablations.profiles.flatMap((profile) => profile.requestLatencies.map((values, index) => [
      `${profile.name} / n=1`, `${performance.requests[index].name} / ${performance.requests[index].gapsPerRepetition}`,
      values[0].toFixed(3), values[1] === null ? "n/a" : values[1].toFixed(3),
    ])), allLatencies);
  measured.append(allLatencies, element("p", "", performance.correctness));
  const definitions = element("dl", "observation-facts");
  performance.definitions.forEach(([label, detail]) => {
    definitions.append(element("dt", "", label), element("dd", "", detail));
  });
  measured.append(definitions, element("h3", "", "Fixtures and remaining ablations"));
  const fixtures = element("dl", "observation-facts");
  performance.fixtures.forEach(([label, detail]) => {
    fixtures.append(element("dt", "", label), element("dd", "", detail));
  });
  measured.append(fixtures, element("p", "", performance.publication));
  const provenance = element("details", "performance-identities");
  provenance.append(element("summary", "", "Exact identities and archived evidence"));
  provenance.append(element("p", "", performance.provenance));
  const pins = element("dl", "observation-facts");
  const headPinLabels = {
    generatorSourceSha256: "summary generator source SHA-256",
    comparatorSourceSha256: "comparator source SHA-256",
    summaryFileSha256: "generated three-case summary SHA-256",
    summaryManifestSha256: "three-case input manifest SHA-256",
    precisionPairFileSha256: "generated precision-pair summary SHA-256",
    mfmaPairFileSha256: "generated MFMA-pair summary SHA-256",
  };
  for (const [key, value] of Object.entries(fp32.pins)) {
    pins.append(element("dt", "", `FP32 head ${headPinLabels[key] || key}`), element("dd", "", value));
  }
  for (const profile of fp32.profiles) {
    for (const key of ["expectationFileSha256", "comparisonFileSha256", "rawTimingSha256"]) {
      pins.append(element("dt", "", `FP32 head ${profile.name} ${key}`), element("dd", "", profile[key]));
    }
  }
  Object.entries(performance.identities).forEach(([label, digest]) => {
    pins.append(element("dt", "", label), element("dd", "", digest));
  });
  if (rounds) {
    for (const [key, value] of Object.entries(rounds.pins)) {
      const label = key === "ledgerGeneratorSourceSha256" ? "performance-ledger generator source SHA-256"
        : key === "hostTimingGeneratorSourceSha256" ? "host-timing generator source SHA-256"
          : key === "comparatorSourceSha256" ? "comparator source SHA-256" : key;
      pins.append(element("dt", "", `Concurrent matrix ${label}`), element("dd", "", value));
    }
    for (const world of rounds.worlds) {
      pins.append(element("dt", "", `Concurrent matrix TP${world.world} generated three-mode report SHA-256`),
        element("dd", "", world.ledgerFileSha256),
        element("dt", "", `Concurrent matrix TP${world.world} generated serial-baseline report SHA-256`),
        element("dd", "", world.serialLedgerFileSha256));
    }
    for (const profile of rounds.profiles) {
      pins.append(element("dt", "", `${profile.id} case report SHA-256`), element("dd", "", profile.caseFileSha256),
        element("dt", "", `${profile.id} comparison report SHA-256`), element("dd", "", profile.comparisonFileSha256));
    }
  }
  performance.variants.forEach((variant) => {
    pins.append(element("dt", "", `${variant.name} controller`), element("dd", "", variant.controllerSha256),
      element("dt", "", `${variant.name} worker`), element("dd", "", variant.workerSha256));
  });
  pins.append(element("dt", "", "Operational-only profile"),
    element("dd", "", JSON.stringify(performance.runtimeProfile)));
  for (const key of ["controllerSha256", "workerSha256", "comparatorSha256", "baselineLedgerSha256", "controlLedgerSha256"]) {
    pins.append(element("dt", "", `Single-run ablations ${key}`), element("dd", "", performance.ablations[key]));
  }
  performance.ablations.profiles.forEach((profile) => {
    const flags = { ...performance.runtimeProfile, runtime_operational: profile.operationalCurrentness,
      runtime_cache_admission: profile.admissionCache, dispatch_sequences: profile.dispatchSequences };
    pins.append(element("dt", "", `${profile.name} profile`), element("dd", "", JSON.stringify(flags)),
      element("dt", "", `${profile.name} collective`), element("dd", "", profile.hostWorkspaceReuse
        ? "host-staged-reuse-v3" : "host_staged_fp32_rank_order_reduce_bf16_residual"));
  });
  for (const [label, section] of [["Matched MFMA", performance.mfmaPair],
    ["Matched TP1 residual", performance.deviceTp1Pair], ["Serial peer", performance.peerObservations]]) {
    Object.entries(section.pins).forEach(([key, value]) => {
      const caption = key === "ledgerCanonicalId" ? "ledger generator source SHA-256" : key;
      pins.append(element("dt", "", `${label} ${caption}`), element("dd", "", value));
    });
    section.profiles.forEach((profile) => {
      pins.append(element("dt", "", `${profile.name} comparison SHA-256`), element("dd", "", profile.comparisonSha256));
      if (profile.metricsFileSha256) {
        pins.append(element("dt", "", `${profile.name} metrics file SHA-256`), element("dd", "", profile.metricsFileSha256));
      }
    });
  }
  for (const key of ["source", "summarySha256", "rawLogSha256"]) {
    pins.append(element("dt", "", `CPU transpose ${key}`), element("dd", "", performance.hostTranspose[key]));
  }
  Object.entries(cohorts.pins).forEach(([key, value]) => {
    pins.append(element("dt", "", `Replica cohorts ${key}`), element("dd", "", value));
  });
  cohorts.profiles.forEach((profile) => {
    for (const key of ["comparisonSha256", "expectationSha256", "releaseEpochNs", "maximumLatenessNs"]) {
      const label = key === "expectationSha256" ? "canonical expectation SHA-256" : key;
      pins.append(element("dt", "", `${profile.layout} ${label}`), element("dd", "", profile[key]));
    }
  });
  Object.entries(wide.pins).forEach(([key, value]) => {
    pins.append(element("dt", "", `Wide row policy ${key}`), element("dd", "", value));
  });
  wide.profiles.forEach((profile) => {
    for (const key of ["comparisonSha256", "expectationSha256", "releaseEpochNs", "maximumLatenessNs"]) {
      const label = key === "expectationSha256" ? "canonical expectation SHA-256" : key;
      pins.append(element("dt", "", `${profile.name} ${label}`), element("dd", "", profile[key]));
    }
  });
  for (const key of ["sourceRevision", "controllerSha256", "hostWorkerSha256", "peerWorkerSha256"]) {
    pins.append(element("dt", "", `Source-matched peer ${key}`), element("dd", "", peerControls[key]));
  }
  peerControls.controls.forEach((control) => {
    for (const key of ["comparisonSha256", "ledgerFileSha256"]) {
      pins.append(element("dt", "", `${control.name} ${key}`), element("dd", "", control[key]));
    }
  });
  Object.entries(performance.transposeModelPair.pins).forEach(([key, value]) => {
    pins.append(element("dt", "", `Transpose model pair ${key}`), element("dd", "", value));
  });
  pins.append(element("dt", "", "Repeated MFMA ledger SHA-256"), element("dd", "", repeated.ledgerFileSha256));
  pins.append(element("dt", "", "Cumulative MFMA/pruning ledger SHA-256"), element("dd", "", performance.mfmaPruning.ledgerFileSha256),
    element("dt", "", "Cumulative MFMA/pruning comparison SHA-256"), element("dd", "", performance.mfmaPruning.profiles[0].comparisonSha256));
  repeated.secondProfiles.forEach((profile) => {
    pins.append(element("dt", "", `${profile.name} comparison SHA-256`), element("dd", "", profile.comparisonSha256));
  });
  pins.append(element("dt", "", "Repeated TP1 residual ledger SHA-256"), element("dd", "", performance.deviceTp1Repeated.ledgerFileSha256));
  performance.deviceTp1Repeated.secondProfiles.forEach((profile) => {
    pins.append(element("dt", "", `${profile.name} comparison SHA-256`), element("dd", "", profile.comparisonSha256));
  });
  Object.entries(current.pins).forEach(([key, value]) => {
    pins.append(element("dt", "", `Current-controller ${key}`), element("dd", "", value));
  });
  current.accepted.forEach((profile) => {
    for (const key of ["comparisonSha256", "metricsFileSha256"]) {
      pins.append(element("dt", "", `${profile.name} ${key}`), element("dd", "", profile[key]));
    }
  });
  current.rejected.forEach((profile) => {
    pins.append(element("dt", "", `${profile.name} rejection SHA-256`), element("dd", "", profile.rejectionSha256));
  });
  performance.transposeModelPair.profiles.forEach((profile) => {
    for (const key of ["controllerSha256", "comparisonSha256"]) {
      pins.append(element("dt", "", `${profile.name} ${key}`), element("dd", "", profile[key]));
    }
  });
  provenance.append(pins);
  measured.append(provenance);

  const readiness = document.querySelector("[data-readiness]");
  const residentRow = element("div", "readiness-row");
  const residentHeading = element("div", "readiness-row-heading");
  residentHeading.append(element("strong", "", "Authenticated singleton serving"), stateTag("integration"));
  residentRow.append(residentHeading, element("p", "", resident.overview));
  readiness.append(residentRow, element("h3", "", "Earlier attention checkpoints"));
  const routeCount = project.attentionReadiness.length + project.routeReadiness.length;
  const currentCount = routeCount + project.latestReadiness.length;
  [...project.attentionReadiness, ...project.routeReadiness, ...project.latestReadiness, ...project.readiness].forEach((item, index) => {
    if (index === project.attentionReadiness.length) {
      readiness.append(element("h3", "", "Earlier argmax checkpoints"));
    }
    if (index === routeCount) {
      readiness.append(element("h3", "", "Earlier native checkpoints"));
    }
    if (index === currentCount) {
      readiness.append(element("h3", "", "Earlier sprint checkpoints"));
    }
    if (index === currentCount + project.competitivenessRecovery.currentReadinessCount) {
      readiness.append(element("h3", "", "Earlier validated checkpoints"));
    }
    const row = element("div", "readiness-row");
    const heading = element("div", "readiness-row-heading");
    heading.append(element("strong", "", item.label), stateTag(item.state));
    row.append(heading, element("p", "", item.detail));
    readiness.append(row);
  });

  const envelope = document.querySelector("[data-envelope]");
  project.envelope.forEach(([term, definition]) => {
    const item = element("div", "envelope-item");
    item.append(element("dt", "", term), element("dd", "", definition));
    envelope.append(item);
  });

  const capabilities = document.querySelector("[data-capabilities]");
  capabilityGroups.forEach(([key, title, description]) => {
    const group = element("section", `capability-group capability-${key}`);
    const heading = element("div", "capability-group-heading");
    heading.append(element("h3", "", title), element("p", "", description));
    const list = element("ul", "capability-list");
    project.capabilities[key].forEach((item) => {
      const entry = element("li", "");
      entry.append(element("strong", "", item.name), element("p", "", item.detail));
      list.append(entry);
    });
    group.append(heading, list);
    capabilities.append(group);
  });

  const validation = document.querySelector("[data-validation]");
  [
    ["host", "Host validation"],
    ["proof", "Proof policy"],
    ["hardware", "Hardware validation"],
  ].forEach(([key, label]) => {
    const item = project.validation[key];
    const article = element("article", `validation-item validation-${key}`);
    const heading = element("div", "validation-item-heading");
    const title = element("div", "");
    title.append(
      element("div", "validation-label", label),
      element("h3", "", item.title),
    );
    heading.append(title, stateTag(item.state));

    const facts = element("dl", "validation-facts");
    const sourceValue = element("dd", "");
    if (item.source) {
      sourceValue.append(commitLink(item.source, item.repository));
    } else {
      sourceValue.textContent = item.sourceStatus || "No current-source observation";
    }
    facts.append(
      element("dt", "", "Source"),
      sourceValue,
    );
    if (item.closureSha256) {
      const closureValue = element("dd", "");
      closureValue.append(
        element("code", "closure-digest", item.closureSha256),
      );
      facts.append(element("dt", "", "Source closure"), closureValue);
    }
    facts.append(
      element("dt", "", "Result"),
      element("dd", "", item.result),
    );
    article.append(
      heading,
      facts,
      element("p", "validation-detail", item.detail),
    );
    validation.append(article);
  });

  const transitions = document.querySelector("[data-transitions]");
  project.validation.transitions.forEach(([prior, next, state]) => {
    const row = element("tr", "");
    const priorCell = element("td", "", prior);
    const nextCell = element("td", "", next);
    const stateCell = element("td", "");
    stateCell.append(stateTag(state));
    row.append(priorCell, nextCell, stateCell);
    transitions.append(row);
  });
  document.querySelector("[data-transition-limitation]").textContent =
    project.validation.limitation;

  const teams = document.querySelector("[data-teams]");
  project.teams.forEach((team) => {
    const article = element("article", "team-item");
    const heading = element("div", "team-heading");
    const identity = element("div", "team-identity");
    identity.append(
      element("div", "team-scope-label", team.scope),
      element("h3", "", team.name),
    );
    const state = element("div", "team-state");
    state.append(stateTag(team.state), element("strong", "", team.status));
    heading.append(identity, state);

    const facts = element("dl", "team-facts");
    [
      ["Completed", team.completed],
      ["In progress", team.current],
      ["Dependencies", team.blockedBy],
      ["Next", team.next],
      ["Validation", team.validation],
    ].forEach(([label, value]) => {
      facts.append(element("dt", "", label), element("dd", "", value));
    });
    article.append(heading, facts);
    teams.append(article);
  });

  const boundaries = document.querySelector("[data-boundaries]");
  [
    ["ferric", "Ferric owns"],
    ["fe2o3", "fe2o3 owns"],
  ].forEach(([key, title]) => {
    const section = element("section", `boundary boundary-${key}`);
    const heading = element("h3", "", title);
    const list = element("ul", "");
    project.boundaries[key].forEach((item) => list.append(element("li", "", item)));
    section.append(heading, list);
    boundaries.append(section);
  });

  const tpObservations = document.querySelector("[data-tp-observations]");
  const batch = project.batchEngineeringObservations;
  tpObservations.append(
    element("div", "observation-label", "Batched engineering observation"),
    element("h3", "", batch.title),
    element("p", "", batch.scope),
  );
  for (const [headings, rows] of [
    [["Profile", "Rows / Batches", "Cached Tokens", "Seed TTFT", "Reuse TTFT", "Reuse Decode Gap", "Setup"],
      batch.runs.map((run) => [
        `TP${run.worldSize}, cache ${run.prefixCache ? "on" : "off"}`,
        `${run.physicalTokenRows} / ${run.batches}`, String(run.cachedTokens),
        `${(run.seedTtftNs / 1e9).toFixed(3)} s`, `${(run.reuseTtftNs / 1e9).toFixed(3)} s`,
        `${(run.reuseDecodeIntervalNs / 1e9).toFixed(3)} s`, `${run.setupSeconds.toFixed(3)} s`,
      ])],
    [["Request", "Terminal State", "Exact Output IDs", "Decoded Text"],
      batch.outputs.map((output) => [output.name, output.state, output.tokenIds.join(", "), output.text])],
  ]) {
    const wrap = element("div", "transition-table-wrap");
    const table = element("table", "transition-table");
    const head = element("thead", "");
    const heading = element("tr", "");
    headings.forEach((label) => {
      const cell = element("th", "", label);
      cell.scope = "col";
      heading.append(cell);
    });
    head.append(heading);
    const body = element("tbody", "");
    rows.forEach((values) => {
      const row = element("tr", "");
      values.forEach((value) => row.append(element("td", "", value)));
      body.append(row);
    });
    table.append(head, body);
    wrap.append(table);
    tpObservations.append(wrap);
  }
  const batchFacts = element("dl", "observation-facts tp-facts");
  const batchIdentities = [
    ["Authority", `${batch.authority}; Contracted engineering observation only`],
    ["Private implementation", `${batch.implementationSource}; comparator ${batch.comparatorSource}`],
    ["Private kernel source", batch.kernelSource],
    ["Controller SHA-256", batch.controllerSha256],
    ["KFD worker SHA-256", batch.workerSha256],
    ["Nine-root HSACO SHA-256", batch.hsacoSha256],
    ["Frozen reference SHA-256", batch.referenceSha256],
    ["Fixed workload SHA-256", batch.workloadSha256],
  ];
  batch.runs.forEach((run) => {
    const name = `TP${run.worldSize} cache ${run.prefixCache ? "on" : "off"}`;
    batchIdentities.push(
      [`${name} dispatches`, `${run.rankDispatchCounts.join(", ")}; workers closed and reaped`],
      [`${name} whole run`, `${run.wholeSeconds.toFixed(3)} s, including setup and teardown`],
      [`${name} JSONL SHA-256`, run.resultSha256],
      [`${name} comparison SHA-256`, run.comparisonSha256],
    );
  });
  batchIdentities.forEach(([name, value]) => {
    batchFacts.append(element("dt", "", name), element("dd", "", value));
  });
  tpObservations.append(batchFacts);
  const tp = project.engineeringObservations;
  tpObservations.append(
    element("div", "observation-label", "Engineering Qwen observations"),
    element("h3", "", tp.title),
    element("p", "", tp.scope),
  );
  if (tp.single32) {
    const sequence = tp.single32;
    const facts = element("dl", "observation-facts tp-facts");
    [["TTFT", `${sequence.ttftSeconds.toFixed(3)} s`],
      ["Single-sequence TPOT", `${sequence.tpotSeconds.toFixed(3)} s (mean of 31 intervals)`],
      ["Setup", `${sequence.setupSeconds.toFixed(3)} s, excluded from TTFT`],
      ["Dispatches by rank", sequence.rankDispatchCounts.join(", ")],
      ["KV positions", `${sequence.kvTokensProcessed}; all eight workers closed and reaped`],
      ["Generated IDs", sequence.generatedTokenIds.join(", ")],
      ["Generated text", sequence.generatedText],
      ["JSONL SHA-256", sequence.resultSha256],
      ["Comparison SHA-256", sequence.comparisonSha256]]
      .forEach(([name, value]) => facts.append(element("dt", "", name), element("dd", "", value)));
    tpObservations.append(
      element("h4", "", "TP8: 32-token single-sequence result"),
      element("p", "", "32/32 IDs and decoded text match both frozen reference passes. One unwarmed sequence, no repeated-run statistics or controlled speed comparison."),
      facts,
    );
  }
  tpObservations.append(element("h4", "", "Separate two-token smokes"));
  const smokeWrap = element("div", "transition-table-wrap");
  const smokeTable = element("table", "transition-table");
  const smokeHead = element("thead", "");
  const smokeHeading = element("tr", "");
  ["Mode", "Output", "TTFT", "Single Decode Interval", "Setup"].forEach((label) => {
    const cell = element("th", "", label);
    cell.scope = "col";
    smokeHeading.append(cell);
  });
  smokeHead.append(smokeHeading);
  const smokeBody = element("tbody", "");
  tp.smokes.forEach((smoke) => {
    const row = element("tr", "");
    [`TP${smoke.worldSize}`, "2/2 IDs match", `${smoke.ttftSeconds.toFixed(3)} s`,
      `${smoke.singleDecodeIntervalSeconds.toFixed(3)} s`, `${smoke.setupSeconds.toFixed(3)} s`]
      .forEach((value) => row.append(element("td", "", value)));
    smokeBody.append(row);
  });
  smokeTable.append(smokeHead, smokeBody);
  smokeWrap.append(smokeTable);
  tpObservations.append(smokeWrap, element("p", "", tp.timing));
  const tpFacts = element("dl", "observation-facts tp-facts");
  [["Shared prompt", tp.prompt], ["Smoke token IDs", tp.generatedTokenIds.join(", ")],
    ["Smoke text", tp.generatedText], ["Controller SHA-256", tp.controllerSha256],
    ["Worker SHA-256", tp.workerSha256], ["HSACO SHA-256", tp.hsacoSha256],
    ...tp.smokes.map((smoke) => [`TP${smoke.worldSize} JSONL SHA-256`, smoke.resultSha256])]
    .forEach(([name, value]) => tpFacts.append(element("dt", "", name), element("dd", "", value)));
  tpObservations.append(tpFacts, element("p", "authority-note", tp.authority));

  const observation = document.querySelector("[data-observation]");
  const observationHeader = element("div", "observation-heading");
  const observationTitle = element("div", "");
  observationTitle.append(
    element("div", "observation-label", "Historical MI300X Qwen observation"),
    element("h3", "", project.latestObservation.title),
  );
  observationHeader.append(observationTitle, stateTag(project.latestObservation.state));
  const observationFacts = element("dl", "observation-facts");
  const observationEntries = [
    [
      "Source",
      project.latestObservation.commit
        ? commitLink(project.latestObservation.commit)
        : project.latestObservation.sourceStatus,
    ],
    ["Environment", project.latestObservation.environment],
    ["Result", project.latestObservation.result],
    ["Artifact identity", project.latestObservation.buildId],
  ];
  if (project.latestObservation.generatedTokenIds.length > 0) {
    observationEntries.splice(3, 0, [
      "Token IDs",
      project.latestObservation.generatedTokenIds.join(", "),
    ]);
  }
  observationEntries.forEach(([term, value]) => {
    const dd = element("dd", "");
    if (value instanceof Node) {
      dd.append(value);
    } else {
      dd.textContent = value;
    }
    observationFacts.append(element("dt", "", term), dd);
  });
  observation.append(
    observationHeader,
    observationFacts,
    element("p", "authority-note", project.latestObservation.authority),
  );

  const progress = document.querySelector("[data-progress]");
  project.recentProgress.forEach((item) => {
    const entry = element("li", "timeline-entry");
    const marker = element("span", `timeline-marker marker-${item.state}`);
    marker.setAttribute("aria-hidden", "true");
    const body = element("div", "timeline-body");
    const heading = element("div", "timeline-heading");
    const title = element("h3", "");
    const source = item.commit
      ? commitLink(item.commit, item.repository)
      : element("code", "commit-link", item.sourceStatus);
    title.append(
      source,
      document.createTextNode(` ${item.title}`),
    );
    heading.append(title, stateTag(item.state));
    body.append(heading, element("p", "", item.detail));
    entry.append(marker, body);
    progress.append(entry);
  });

  document.querySelector("[data-evidence-summary]").textContent = project.evidence.summary;
  const legend = document.querySelector("[data-authority-legend]");
  project.evidence.legend.forEach(([state, detail]) => {
    const item = element("div", "authority-item");
    item.append(stateTag(state), element("p", "", detail));
    legend.append(item);
  });

  const gates = document.querySelector("[data-gates]");
  project.evidence.gates.forEach(([label, count, state]) => {
    const row = element("div", "gate-row");
    row.append(
      element("span", "gate-count", count),
      element("span", "gate-label", label),
      stateTag(state),
    );
    gates.append(row);
  });
})();
