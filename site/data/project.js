window.FERRIC_PROJECT = Object.freeze({
  updated: "2026-09-06",
  repository: "https://github.com/harsh-nod/ferric",
  fe2o3Repository: "https://github.com/harsh-nod/fe2o3",
  current: {
    siteRefreshBase: "d57242e978a2b215a33ea1e48cd43fea16e9cdc1",
    integrationCommit: "28b925a1c4de75aa4ea35175a9f76d6071f4ca86",
    integrationTree: "5a6d77564d96e9fdcdc2c124f42b37215988dae9",
    rmsnormUniformFoldCommit: "7521cdcdfebf76dce5f5499aa25f2d90fb81033a",
    r33ExecutorCommit: "c1b9590b548acea2450136d52ad37a586d03bfed",
    intermediateIntegrationCommit: "23f326a3133ef4b5e0da19b9a170bf05f8cb7a6b",
    r33LifecycleCommit: "eebdb38dab764143b023b33311363a452a1238ee",
    prefillProofSourceCommit: "240bb3d1ce394436cc62244f51888d7737ea6b9c",
    prefillProofIntegratedCommit: "f709a5da2f0130087bcfc0605a4c84ebd966ea0c",
    directReadbackSourceCommit: "18eed253d30b40a23f3984d7249d24b4db7318d2",
    pairedPrefillExecutorCommit: "e8b9908e48313ee43cbeec8092e4ba62006a3dfb",
    engineeringIdentityCommit: "6df1f2fa99a409adbafd8c3e138f8eae2a728260",
    radixPrefixCommit: "1dc659beda81d37d746cb05a16d35fd7788e29ef",
    kernelRepinSourceCommit: "5ecad80658f27909fa2477d6f73e47e9f95407ae",
    compactCompletionSourceCommit: "c4f63ca071a4012c09bdff349c11d69af0f18b18",
    hoistedWitnessSourceCommit: "17b282f77ec16f598054d9983287094457be723c",
    immediateBranchSourceCommit: "fb991b83e2a0f9d1cf4f11058c51b372ee96acff",
    flattenedLoopSourceCommit: "aad3b3aae05a261ac011b3318b786790c1f08320",
    canonicalBoundSourceCommit: "96df9a33eaf0ef0d97c176e5aaa870ff336747c5",
    kernelCandidateSourceCommit: "72c78c6fb3766ae3de1a4a393f6a8fa358498ca4",
    kernelCandidateSourceTree: "6b3a5ab219216fe2d7adf797d7ea890f94c76a60",
    fe2o3V71Main: "6492c8fa85a00d93aa6ca2a4a77675fefad2fee6",
    fe2o3V71Tree: "68573bf31789625ecc2489491711ad9153eb1cac",
    fe2o3LatestMain: "1ddcd36b8f8b758e0d75780fe27813b4cd0581b1",
    fe2o3LatestTree: "553334d69d51c4ccffea383cd72cf9105df74130",
    formalVerified: 81,
    formalErrors: 0,
    proofTestsPassed: 26,
    sourceGateTestsPassed: 28,
    scopedUnadmittedRuntimeBodies: 23,
    combinedInventoryCurrent: false,
    focusedRmsnormTestsPassed: 21,
    focusedRmsnormLogShaPrefix: "4d2bcd1",
    exactCompilerAttempt: "v76",
    exactCompilerLogSha256: "338edfa1bfa3d7c8346a1b0eec2b64911b91eea149676d27a701651988198a65",
    exactCompilerExitStatus: 0,
    exactCompilerOutputs: 2,
    exactCompilerHandoffBytes: 415660,
    exactCompilerHandoffSha256: "31a15c036261f0d7d2ab7027e709ea6c01c16f9654bc3d6e248dca3994a12282",
    exactCompilerGuardedStores: 26,
    exactCompilerHsacoBytes: 103616,
    exactCompilerHsacoSha256: "c270a528439df199cdae59b4cffda5566ad8a1d24d7029b2b58a519a4aef5b94",
    exactCompilerManifestSha256: "6af8bd8292c5c108ea77e2416fb1325f11507d471254f2cc9607b6ae8be5035a",
    exactCompilerDescriptorSha256: "fc8469775585a165c9674ff25683ee0676fd0e6c11a1885e1dde26cdcea392d5",
    exactCompilerKernelCount: 12,
    exactCompilerReplayExact: true,
    exactCompilerPublicationGrant: false,
    exactCompilerLoadGrant: false,
    exactCompilerLaunchGrant: false,
    engineeringSmokeStop: "CurrentFerricDescriptorRoster",
    engineeringSmokeReachedKfd: false,
    engineeringSmokeGpuStateChanged: false,
    engineeringSmokeVramStateChanged: false,
    r33AdapterTestsPassed: 67,
    r33AdapterHardwareIgnored: 2,
    integratedEngineTestsPassed: 689,
    integratedEngineHardwareIgnored: 7,
    engineValidationLogSha256: "ac5c60dfde5bdb4eae1d5ce1f1f8cc6b096afae2a7c71895bdc0c8ceddfc669f",
    adapterValidationLogSha256: "d79f354d37d65fa47ff7cc1deeff43de6f30070f7d0c487fe15d60994ad8731a",
    remoteTestFailures: 0,
    r33ExecutableWindows: 1,
    r33RequiredWindows: 20,
    r33OutputTokensPerWindow: 128,
    radixVerusVerified: 608,
    radixVerusErrors: 0,
    openM1Gates: 33,
    engineeringSmokeBinaryStaged: true,
    canonicalQwenSnapshotVerified: true,
    radixPrefixIntegrated: true,
    currentAggregateHsaco: true,
    qwenTokenObserved: false,
    servingEndpointAvailable: false,
    baselineRunsAvailable: false,
    dockerAccessible: false,
    authority: "none",
  },
  milestone: {
    name: "M1",
    label: "Qwen3 speculative inference on one gfx942",
    state: "integration",
    summary:
      "Ferric integration 28b925a repins to public fe2o3 1ddcd36 and retains the uniform serial RMSNorm fold, one-window authenticated R33 executor, paired-prefill/readback, and radix work. Exact v76 completes a 415,660-byte Kernel IR V9 handoff containing 26 GuardedStore operations and produces a 103,616-byte aggregate HSACO for all 12 kernels with exact replay. The observation and every grant remain authority none/false. The first smoke stops at CurrentFerricDescriptorRoster before KFD; GPU and VRAM state are unchanged. No Qwen token, TTFT, TPOT, serving endpoint, baseline, or qualification evidence exists, and all 33 M1 exit gates remain open.",
  },
  readiness: [
    {
      label: "Authenticated engine lifecycle",
      state: "integration",
      detail:
        "28b925a retains the authenticated queue path and paired-prefill executor, including the one-window target executor added at c1b9590. It owns queue creation, bounded completion, checked direct readback, semantic completion, KV settlement, page release, and fail-closed teardown custody.",
    },
    {
      label: "R33 lifecycle",
      state: "integration",
      detail:
        "c1b9590 executes exactly one preadmitted R33 row with 128 output tokens, checked-token causality, and CLOCK_MONOTONIC_RAW offsets, then retains terminal custody in Faulted and rejects a second window. This is not the required 20-window qualification. Repinned remote all-target checks report 67 adapter tests passed, 2 ignored, zero failures; log SHA d79f354d37d65fa47ff7cc1deeff43de6f30070f7d0c487fe15d60994ad8731a. Authority is none.",
    },
    {
      label: "Engineering Qwen smoke path",
      state: "integration",
      detail:
        "6df1f2f adds authority-free, domain-separated engineering identity derivation. The first smoke using the exact v76 observation stops at CurrentFerricDescriptorRoster before entering KFD. GPU and VRAM state remain unchanged, and no token or timing result exists.",
    },
    {
      label: "Formal bootstrap model",
      state: "verified",
      detail:
        "Pinned Verus proves 81 obligations with 0 errors for the proof crate, with 26/26 proof tests. The added pure model covers exact S1/T128 accounting, monotone prepublication-or-quarantine phases, precompletion silence, and exact-stop behavior only.",
    },
    {
      label: "Aggregate kernel artifact",
      state: "integration",
      detail:
        "Exact v76 from Ferric 28b925a and public fe2o3 1ddcd36 completes a 415,660-byte Kernel IR V9 handoff with 26 GuardedStore operations and exact replay, then emits a 103,616-byte aggregate HSACO containing all 12 kernels. HSACO SHA c270a528439df199cdae59b4cffda5566ad8a1d24d7029b2b58a519a4aef5b94; observation manifest SHA 6af8bd8292c5c108ea77e2416fb1325f11507d471254f2cc9607b6ae8be5035a; canonical descriptor SHA fc8469775585a165c9674ff25683ee0676fd0e6c11a1885e1dde26cdcea392d5. Authority is none and publication/load/launch grants are false.",
    },
    {
      label: "Radix prefix reuse",
      state: "integration",
      detail:
        "28b925a retains bounded live-source reuse of logical committed prefixes through generational Engine custody. Remote all-target checks report 689 engine tests passed with 7 ignored and zero failures; pinned Verus reports 608 verified and 0 errors. S1/T128 correctly remains NoMatch at logical width 256, with no persistent device-KV or speed claim.",
    },
    {
      label: "Qwen execution and serving",
      state: "open",
      detail:
        "Exact v76 produces an authority-free aggregate HSACO, but the first engineering smoke stops at CurrentFerricDescriptorRoster before KFD. GPU and VRAM state are unchanged. The production protected receipt/verifier service is undeployed. No hardware execution, Qwen token, endpoint, TTFT, TPOT, or baseline comparison exists. vLLM and SGLang baselines are absent, Docker is inaccessible to this account, and no baseline launch was attempted.",
    },
  ],
  envelope: [
    ["Target", "Qwen3-8B"],
    ["Draft", "Qwen3-0.6B"],
    ["Device", "1 x gfx942"],
    ["Precision", "BF16 with FP32 accumulation"],
    ["Context", "up to 8K tokens"],
    ["Concurrency", "up to 32 sequences"],
    ["Ferric integration candidate", "28b925a1c4de75aa4ea35175a9f76d6071f4ca86; tree 5a6d77564d96e9fdcdc2c124f42b37215988dae9; repinned to fe2o3 1ddcd36 and exact-compiled as v76; not public main, a release, a serving result, or qualification"],
    ["Ferric intermediate integration", "23f326a3133ef4b5e0da19b9a170bf05f8cb7a6b; audited seven-file exact kernel/host ABI delta plus authenticated prefill, readback, and radix; not final or public product integration"],
    ["fe2o3 exact v76 input", "public main 1ddcd36b8f8b758e0d75780fe27813b4cd0581b1; tree 553334d69d51c4ccffea383cd72cf9105df74130; native gfx942 sqrt plus outlined-helper geometry lowering produce the authority-free aggregate"],
  ],
  capabilities: {
    runnable: [
      {
        name: "Authenticated model and plan admission",
        detail:
          "Ferric has exact Qwen3 target/draft configuration, tokenizer, weight-manifest, graph, plan, and program-identity checks with fail-closed ownership transitions.",
      },
      {
        name: "Generational scheduling and paged KV",
        detail:
          "The engine implements fixed-capacity scheduling, request generations, completion epochs, page-generation custody, cancellation, retirement, and explicit queue fault handling.",
      },
      {
        name: "Authenticated long-lived queue rounds",
        detail:
          "Typed owners cross prepare, submit, bounded wait, recycle, readback, completion, page release, rearm, and teardown without a raw queue escape.",
      },
      {
        name: "Authenticated new-window transition",
        detail:
          "All-terminal speculative K4/K8/K16 rounds can enter queued paired prefill and produce exact successor seed material while retaining bounded predecessor evidence and resetting active round history.",
      },
      {
        name: "Authenticated paired-prefill execution",
        detail:
          "The exact S1/T128 bootstrap now feeds an integrated executor that submits, waits, recycles, observes completed device copies, commits one first token, settles device KV, and releases prefill pages. This is implemented source, not a completed GPU run.",
      },
      {
        name: "Authenticated one-window target execution",
        detail:
          "c1b9590 can execute exactly one preadmitted 128-output R33 window. It derives token events only from checked device completions, records CLOCK_MONOTONIC_RAW offsets from the accepted measurement boundary, settles all owned state, and then fail-closes in Faulted. Authority is none, and this is not a serving or qualification result.",
      },
      {
        name: "Authority-free engineering identity derivation",
        detail:
          "The engineering smoke adapter can derive distinct domain-separated identities from the actual observation, authenticated target/draft model plans, descriptors, and program catalog. It grants no KFD, protected-verification, benchmark, or qualification authority.",
      },
    ],
    experimental: [
      {
        name: "Ferric-owned Qwen kernel set",
        detail:
          "The model kernels remain in Ferric and are written through fe2o3 compiler APIs. Exact v76 emits a 415,660-byte Kernel IR V9 handoff with 26 GuardedStore operations and a 103,616-byte authority-free aggregate HSACO containing all 12 kernels. Exact replay is true; publication, load, and launch grants are false.",
      },
      {
        name: "Bounded live radix prefix reuse",
        detail:
          "The integrated radix slice indexes token prefixes and revalidates a live Ready source before sharing committed logical Engine pages. It deliberately returns NoMatch for S1/T128 at the current 256-token logical width and makes no persistent-cache, physical device-KV reuse, or speed claim.",
      },
      {
        name: "Authenticated R33 backend lifecycle",
        detail:
          "The R33 lifecycle owns admitted engine capabilities and exact start/ready/measure/stop ordering. c1b9590 executes exactly one 128-output window with a real monotonic clock and checked-token causality, then fail-closes and rejects a second window. The required 20-window path and qualification evidence remain absent.",
      },
      {
        name: "Pure lifecycle proofs",
        detail:
          "Separate Verus models cover the finite window cap and exact S1/T128 prepublication lifecycle. They do not prove Rust refinement, liveness, timing, KFD, allocation, device memory, queue publication, or serving.",
      },
      {
        name: "Event-backed comparison schema",
        detail:
          "Ferric has structures for paired per-request E2E, TTFT, and TPOT collection across Ferric, vLLM, and SGLang, but no workload has been measured. vLLM and SGLang baselines are absent; Docker is inaccessible to this account, so no baseline launch was attempted.",
      },
    ],
    roadmap: [
      {
        name: "Bind the current descriptor roster",
        detail:
          "Update the staged engineering smoke's CurrentFerricDescriptorRoster binding to the exact v76 canonical descriptor and preserve the fail-closed identity checks. The first attempt stopped here before KFD and changed no GPU or VRAM state.",
      },
      {
        name: "Run the staged engineering smoke",
        detail:
          "Use the exact aggregate observation, staged smoke binary, verified Qwen snapshot, and an exclusive gfx942 slot to obtain the first authority-free diagnostic token. This will not by itself be a production or benchmark result.",
      },
      {
        name: "Deploy protected artifact admission",
        detail:
          "A concrete checker, signer, currentness store, supervisor, and protected execution profile must accept one exact aggregate before production KFD execution.",
      },
      {
        name: "Run Qwen and collect timings",
        detail:
          "Only an authenticated end-to-end GPU run can establish a generated token, TTFT, TPOT, numerical behavior, or a result comparable with vLLM and SGLang.",
      },
      {
        name: "Launch the comparison baselines",
        detail:
          "Restore container access, then launch and authenticate the exact vLLM and SGLang baseline configurations. Docker is currently inaccessible to this account, no baseline was launched, and no comparison result exists.",
      },
      {
        name: "Extend prefix caching after first execution",
        detail:
          "The integrated radix slice covers live logical page sharing. Persistent device-KV prefix caching, symmetric memory, and MTP remain deferred and do not delay the initial single-GPU smoke.",
      },
      {
        name: "Close M1 evidence",
        detail:
          "All 33 M1 exit gates remain open pending their exact proof, artifact, runtime, hardware, performance, independent-review, and receipt evidence.",
      },
    ],
  },
  validation: {
    host: {
      title: "Integrated one-window executor and engine source",
      state: "integration",
      source: "28b925a1c4de75aa4ea35175a9f76d6071f4ca86",
      result: "PASS: remote all-target checks report engine 689 passed / 7 ignored and adapter 67 passed / 2 ignored; zero failures",
      detail:
        "28b925a repins Ferric to fe2o3 1ddcd36 and retains c1b9590's one authenticated 128-output R33 target window with checked-token causality and CLOCK_MONOTONIC_RAW timing. Engine log SHA ac5c60dfde5bdb4eae1d5ce1f1f8cc6b096afae2a7c71895bdc0c8ceddfc669f; adapter log SHA d79f354d37d65fa47ff7cc1deeff43de6f30070f7d0c487fe15d60994ad8731a. These are non-hardware checks: authority is none, and they are not GPU, token, serving, speed, 20-window, performance, or qualification evidence.",
    },
    proof: {
      title: "Authenticated bootstrap and finite-window models",
      state: "verified",
      source: "f709a5da2f0130087bcfc0605a4c84ebd966ea0c",
      result: "PASS: strict Verus 81 verified / 0 errors; proof tests 26/26; source gate passes 28/28",
      detail:
        "Source 240bb3d, integrated as f709a5d, adds the bootstrap model. The radix slice's separate pinned Verus run reports 608 verified and 0 errors. The latest scoped inventory diagnostic found 23 unadmitted bootstrap runtime bodies; combined regeneration remains pending, so no current whole-tree verified/unverified total is claimed. The pure proofs do not prove runtime refinement or effects.",
    },
    hardware: {
      title: "Exact v76 emits an authority-free aggregate",
      state: "integration",
      sourceStatus: "Exact compiler artifact exists; first smoke stops before KFD",
      result: "PASS: 415,660-byte handoff; exact replay; 103,616-byte HSACO with all 12 kernels. OPEN: descriptor-roster smoke admission, GPU execution, and tokens",
      detail:
        "Exact v76 from Ferric 28b925a1c4de75aa4ea35175a9f76d6071f4ca86, tree 5a6d77564d96e9fdcdc2c124f42b37215988dae9, uses public fe2o3 1ddcd36b8f8b758e0d75780fe27813b4cd0581b1, tree 553334d69d51c4ccffea383cd72cf9105df74130. It emits a 415,660-byte Kernel IR V9 handoff with 26 GuardedStore operations and SHA 31a15c036261f0d7d2ab7027e709ea6c01c16f9654bc3d6e248dca3994a12282. Exact replay produces a 103,616-byte HSACO with SHA c270a528439df199cdae59b4cffda5566ad8a1d24d7029b2b58a519a4aef5b94 and all 12 kernels; manifest SHA 6af8bd8292c5c108ea77e2416fb1325f11507d471254f2cc9607b6ae8be5035a; descriptor SHA fc8469775585a165c9674ff25683ee0676fd0e6c11a1885e1dde26cdcea392d5. Authority is none and every grant is false. The first smoke stops at CurrentFerricDescriptorRoster before KFD, with GPU and VRAM state unchanged; no token or timing exists.",
    },
    transitions: [
      ["Speculative S1/K4 (all terminal)", "Paired prefill new window", "implemented"],
      ["Speculative S1/K8 (all terminal)", "Paired prefill new window", "implemented"],
      ["Speculative S1/K16 (all terminal)", "Paired prefill new window", "implemented"],
      ["Authenticated paired-prefill readback", "Fresh speculative coordinator", "implemented"],
      ["Authenticated S1/T128 bootstrap", "Queue-ready prepublication custody", "implemented"],
      ["Queue-ready paired prefill", "Engine and device-KV completion", "implemented"],
      ["Authenticated R33 128-output window", "Faulted custody; second window rejected", "implemented"],
      ["Live Ready radix source", "Shared committed logical prefix", "integration"],
    ],
    limitation:
      "These are source-level transitions in integration candidate 28b925a, with the R33 executor integrated at c1b9590. R33 supports exactly one 128-output window, then fail-closes; it does not provide the required 20-window qualification. Radix reuse is live-source and logical only; S1/T128 is NoMatch at logical width 256. Exact v76 produced an authority-free aggregate HSACO, but it has not been accepted, loaded, or launched. The smoke stopped before KFD, and no row is evidence of a Qwen run, serving endpoint, or performance result.",
  },
  teams: [
    {
      name: "Integration",
      scope: "Ferric integration, review, publication, and end-to-end evidence",
      state: "integration",
      status: "Progressing, no team blocker",
      completed:
        "Candidate 28b925a repins the uniform serial RMSNorm fold, one-window authenticated R33 target execution, paired-prefill executor, authority-free engineering identities, and bounded live-source radix reuse to public fe2o3 1ddcd36.",
      current:
        "Exact v76 produces an authority-free 103,616-byte aggregate HSACO with exact replay and all 12 kernels. Integration 28b925a is not public main or a release. The first smoke stops at CurrentFerricDescriptorRoster before KFD and changes no GPU or VRAM state.",
      blockedBy:
        "No team-local blocker. M1 still depends on current descriptor-roster admission, an exclusive GPU run, deployed protected artifact admission, and authenticated baselines.",
      next:
        "Bind the exact v76 descriptor roster, rerun the staged one-token smoke, and stop on any identity or KFD preflight failure.",
      validation:
        "Repinned remote all-target checks report engine 689 passed / 7 ignored and adapter 67 passed / 2 ignored, zero failures. They establish no hardware, token, timing, serving, baseline, or qualification result.",
    },
    {
      name: "Kernels",
      scope: "Ferric-owned Qwen kernels compiled with fe2o3",
      state: "integration",
      status: "Progressing, no team blocker",
      completed:
        "7521cdc implements a uniform serial RMSNorm fold and passes focused RMSNorm validation 21/21, log SHA prefix 4d2bcd1. Exact v76 clears the prior convergence, helper-geometry, and native-sqrt failures and emits all 12 kernels.",
      current:
        "The exact aggregate is 103,616 bytes with SHA c270a528439df199cdae59b4cffda5566ad8a1d24d7029b2b58a519a4aef5b94; exact replay is true and the canonical descriptor SHA is fc8469775585a165c9674ff25683ee0676fd0e6c11a1885e1dde26cdcea392d5.",
      blockedBy:
        "No team-local blocker. Runtime progress depends on admitting the exact descriptor roster before KFD; artifact authority remains none.",
      next:
        "Preserve the exact v76 artifact identity while integration fixes the CurrentFerricDescriptorRoster smoke admission boundary.",
      validation:
        "Exact v76 status 0; 415,660-byte handoff; 26 GuardedStore operations; 103,616-byte HSACO; all 12 kernels; exact replay true; authority none and publication/load/launch grants false.",
    },
    {
      name: "Inference engine",
      scope: "Scheduling, KV, queue custody, R33 lifecycle, and bootstrap",
      state: "integration",
      status: "Progressing, no team blocker",
      completed:
        "Authenticated rollover, paired-prefill execution, authority-free engineering identities, live-source logical radix reuse, and one checked 128-output target window are integrated.",
      current:
        "The staged smoke and verified Qwen snapshot now consume the exact v76 observation, but the first attempt stops at CurrentFerricDescriptorRoster before KFD. The R33 executor intentionally supports only one window before Faulted.",
      blockedBy:
        "No team-local blocker. End-to-end execution depends on current descriptor-roster admission and protected runtime admission; the required 20-window R33 path remains absent.",
      next:
        "Extend authenticated custody across all 20 required windows after the exact artifact and first diagnostic execution are available.",
      validation:
        "Remote all-target checks report engine 689 passed / 7 ignored and adapter 67 passed / 2 ignored, zero failures. Real monotonic timing and checked-token causality are source-validated, not hardware-observed.",
    },
    {
      name: "Formal verification",
      scope: "Verus models, source coverage, and hostile mutation policy",
      state: "verified",
      status: "Progressing, no team blocker",
      completed:
        "The combined proof crate passes strict Verus 81/0, proof tests 26/26, and source-gate tests 28/28; the radix slice separately passes pinned Verus 608/0.",
      current:
        "Keeping runtime effects outside the pure proof claim and preparing final combined inventory regeneration after active source integration.",
      blockedBy:
        "No team-local blocker. Final inventory identity depends on the combined kernel and executor head; runtime refinement remains explicitly unproved.",
      next:
        "Regenerate and audit exact body identities, then rerun the full formal and hostile-policy matrix on the settled combined head.",
      validation:
        "Latest scoped diagnostic: 23 bootstrap runtime bodies unadmitted pending combined regeneration; radix Verus is 608/0; no current whole-tree closure count is claimed.",
    },
  ],
  boundaries: {
    ferric: [
      "Qwen model, tokenizer, weight, graph, plan, and artifact admission policy",
      "All model-specific kernels and inference semantics",
      "Scheduling, speculative coordination, paged KV ownership, and queue lifecycle composition",
      "Authenticated new-window custody and the R33 lifecycle",
      "Authenticated paired-prefill execution and authority-free engineering smoke identity derivation",
      "Bounded live-source logical radix prefix reuse",
      "Ferric-specific Verus models, source policy, hostile mutations, and M1 evidence",
      "Integration candidate 28b925a1c4de75aa4ea35175a9f76d6071f4ca86, tree 5a6d77564d96e9fdcdc2c124f42b37215988dae9; one-window R33 executor source c1b9590b548acea2450136d52ad37a586d03bfed; exact v76 aggregate exists with authority none; no Qwen result",
      "Intermediate integration 23f326a3133ef4b5e0da19b9a170bf05f8cb7a6b contains the audited seven-file exact kernel/host ABI delta plus authenticated prefill, readback, and radix; it is not final or public product integration",
    ],
    fe2o3: [
      "Reusable Rust-to-KIR-to-LLVM compiler infrastructure",
      "Generic artifact, descriptor, and compiler-lineage types",
      "Direct-KFD runtime, allocations, AQL queues, completion, and bounded waits",
      "Generic protected verification transport",
      "Public main 1ddcd36b8f8b758e0d75780fe27813b4cd0581b1, tree 553334d69d51c4ccffea383cd72cf9105df74130, contains reusable native gfx942 sqrt plus outlined-device-helper geometry lowering",
      "Exact Ferric v76 uses public main 1ddcd36b8f8b758e0d75780fe27813b4cd0581b1, tree 553334d69d51c4ccffea383cd72cf9105df74130; the compiler emits the complete authority-free aggregate",
      "No Ferric model kernel or inference policy is moved upstream",
    ],
  },
  latestObservation: {
    title: "Exact v76 emits all 12 kernels",
    state: "integration",
    sourceStatus: "Ferric 28b925a1c4de75aa4ea35175a9f76d6071f4ca86, tree 5a6d77564d96e9fdcdc2c124f42b37215988dae9",
    environment: "mi300x exact compilation against public fe2o3 1ddcd36, tree 553334d; first smoke stopped before KFD",
    result:
      "Status 0. A 415,660-byte handoff with 26 GuardedStore operations replays exactly into a 103,616-byte HSACO containing all 12 kernels. Handoff SHA 31a15c036261f0d7d2ab7027e709ea6c01c16f9654bc3d6e248dca3994a12282; manifest SHA 6af8bd8292c5c108ea77e2416fb1325f11507d471254f2cc9607b6ae8be5035a; descriptor SHA fc8469775585a165c9674ff25683ee0676fd0e6c11a1885e1dde26cdcea392d5.",
    buildId: "SHA-256 c270a528439df199cdae59b4cffda5566ad8a1d24d7029b2b58a519a4aef5b94",
    generatedTokenIds: [],
    authority:
      "Authority: none; publication, load, and launch grants are false. The first smoke stops at CurrentFerricDescriptorRoster before KFD, and GPU and VRAM state are unchanged. No Qwen token, TTFT, TPOT, numerical result, serving endpoint, vLLM baseline, SGLang baseline, or qualification evidence exists.",
  },
  recentProgress: [
    {
      commit: "28b925a1c4de75aa4ea35175a9f76d6071f4ca86",
      title: "Produced the exact authority-free aggregate",
      state: "integration",
      detail:
        "Exact v76 status 0 emits a 103,616-byte aggregate HSACO with all 12 kernels and exact replay. Handoff SHA 31a15c036261f0d7d2ab7027e709ea6c01c16f9654bc3d6e248dca3994a12282; HSACO SHA c270a528439df199cdae59b4cffda5566ad8a1d24d7029b2b58a519a4aef5b94; manifest SHA 6af8bd8292c5c108ea77e2416fb1325f11507d471254f2cc9607b6ae8be5035a; descriptor SHA fc8469775585a165c9674ff25683ee0676fd0e6c11a1885e1dde26cdcea392d5. Authority is none and all grants are false. The first smoke stops at CurrentFerricDescriptorRoster before KFD with GPU and VRAM unchanged.",
    },
    {
      commit: "1ddcd36b8f8b758e0d75780fe27813b4cd0581b1",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Published native gfx942 sqrt lowering",
      state: "implemented",
      detail:
        "Public fe2o3 main, tree 553334d69d51c4ccffea383cd72cf9105df74130, emits strict native llvm.sqrt.f32 on gfx942. Exact Ferric v76 confirms the prior native-worker abort is cleared and the aggregate is emitted.",
    },
    {
      commit: "01b2cb2ae9100dc28a729481d7c8ef660fef5b76",
      title: "Advanced exact compilation into the native LLVM worker",
      state: "integration",
      detail:
        "Historical exact v75 cleared both prior outlined-helper geometry gaps and produced a 415,659-byte Kernel IR V9 handoff with 26 GuardedStore operations. The native AMDGPU LLVM worker then exited SIGABRT before HSACO; status 1, empty output manifest, log SHA 7d7fbb57a113f27ec42fcf919751466b783706242f12949950a0b2bd80db7d0e. Exact v76 supersedes this compiler terminal.",
    },
    {
      commit: "629465f1a85a1af331a4e285062991f7ab59a5ce",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Published complete outlined-helper geometry lowering",
      state: "implemented",
      detail:
        "Public fe2o3 main, tree 12da211265860aeec3235e49b631238ce3e618cf, adds reusable WorkgroupSize and Workgroup lowering to the existing WorkgroupCount helper support. Exact Ferric v75 confirms that both prior helper geometry terminals clear.",
    },
    {
      commit: "e8d0c889b710d2cb97f70b043e2ff67385bb4b75",
      title: "Advanced exact compilation to WorkgroupSize helper lowering",
      state: "integration",
      detail:
        "Exact v74 clears the prior WorkgroupCount helper gap and next fails closed on generic fe2 WorkgroupSize X lowering in outlined device helper f15 bb0 op2. Status is 1 with no outputs; log SHA 6dd1e79bcc88d24fb1a42779e567985a329cf65ee6a995fc9e3511f2ed88fe42. This compiler diagnostic grants no artifact, hardware, token, timing, or qualification authority.",
    },
    {
      commit: "b1d45a00b52fc76d74f32a61cef66ee13f6da080",
      title: "Repinned Ferric to the public device-helper fix",
      state: "integration",
      detail:
        "Integration tree 6f50ebdbbfef97a9aa98daeaf465513b6edd06d2 pins public fe2o3 be5668e. Remote all-target checks report engine 689 passed / 7 ignored and adapter 67 passed / 2 ignored, zero failures. Engine log SHA cde5a597108aa90784e04d2397cdd5090378a67e98cc5c05f95da9f157bf4991; adapter log SHA 6dbc57fc05b06935a24f29b68c1fc4718df4dd541a187103327e36f5c7b36215. This is non-hardware validation, not token or performance evidence.",
    },
    {
      commit: "be5668eaa71f8d60a0a5041891d25ce2ed9c2e6e",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Published generic workgroup-count lowering for device helpers",
      state: "implemented",
      detail:
        "Public fe2o3 main, tree f8b09ddeaaed81b0f9f49e5d17020fdb5a434367, lowers workgroup-count intrinsics in outlined device helpers. Ferric b1d45a0 later repinned to it, and exact v74 confirms that the v71 WorkgroupCount terminal clears.",
    },
    {
      commit: "c1b9590b548acea2450136d52ad37a586d03bfed",
      title: "Integrated one authenticated R33 target window",
      state: "integration",
      detail:
        "The executor handles exactly one 128-output window with CLOCK_MONOTONIC_RAW offsets and checked-token causality, settles owned state, then fail-closes and rejects another window. Remote all-target checks report 689 engine and 67 adapter tests passed. Authority is none; the required 20-window qualification remains open.",
    },
    {
      commit: "7521cdcdfebf76dce5f5499aa25f2d90fb81033a",
      title: "Integrated a uniform serial RMSNorm fold",
      state: "integration",
      detail:
        "Focused RMSNorm validation passes 21/21, log SHA prefix 4d2bcd1. Exact v71 clears the prior convergence failure and next fails closed on generic fe2 WorkgroupCount X lowering in an outlined device helper, log SHA prefix 07a6180, with no HSACO.",
    },
    {
      commit: "23f326a3133ef4b5e0da19b9a170bf05f8cb7a6b",
      title: "Audited the intermediate exact ABI integration",
      state: "integration",
      detail:
        "This intermediate Ferric head contains the seven-file exact kernel/host ABI delta plus authenticated prefill, readback, and radix work. It is not final or public product integration and remains pending the final fe2o3 repin and RMSNorm fix.",
    },
    {
      commit: "6492c8fa85a00d93aa6ca2a4a77675fefad2fee6",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Published general private-slot lowering",
      state: "implemented",
      detail:
        "Public fe2o3 main, tree 68573bf31789625ecc2489491711ad9153eb1cac, contains the reusable compiler fix. In the patched exact compile it clears Ferric retained-borrow locals 178 and 40, allowing progress into AMDGPU LLVM lowering.",
    },
    {
      commit: "72c78c6fb3766ae3de1a4a393f6a8fa358498ca4",
      title: "Cleared the exact-total loop-bound gate",
      state: "implemented",
      detail:
        "This Ferric candidate, tree 6b3a5ab219216fe2d7adf797d7ea890f94c76a60, passes focused Rope/KV 27/27, the full ferric-qwen-kernels suite, and engine physical-recipe 7/7. With the private-slot fix it advances to the RMSNorm convergence diagnostic and still produces no artifact.",
    },
    {
      commit: "42e959710d830c6394413ff861c41dcbf61fd54d",
      title: "Formatted the canonical flattened traversal",
      state: "implemented",
      detail:
        "This format-only DCO tip has tree 2af4b8672051740b26e2d8c0c81c0238a52f2114. It passes cargo fmt and focused v50 27/27; exact v13 rejects the induction-bound form with no output.",
    },
    {
      commit: "96df9a33eaf0ef0d97c176e5aaa870ff336747c5",
      title: "Canonicalized the flattened KV loop bound",
      state: "implemented",
      detail:
        "This DCO source replaced the induction-bound form exact v11 rejected after the retained-borrow class cleared. Exact v13 still rejected that form; later candidate 72c78c6 cleared the gate.",
    },
    {
      commit: "aad3b3aae05a261ac011b3318b786790c1f08320",
      title: "Flattened paged-KV row and component traversal",
      state: "implemented",
      detail:
        "This clean DCO source, tree 9bc5e0677041d8d9a4c776de50de1a7ef5cc4242, uses one bounded loop. Corrected focused v46 passes 27/27; exact v11 cleared the retained-borrow class and advanced to the induction-bound gate.",
    },
    {
      commit: "fb991b83e2a0f9d1cf4f11058c51b372ee96acff",
      title: "Branched immediately on each paged-KV write",
      state: "implemented",
      detail:
        "This clean DCO source, tree 75e19bab4346ab1bf1d4f1ea82e5ecd8c3cce911, implements the MIR shape identified by compiler attribution and passes focused v44 27/27. Exact v10 later rejected local 201/type 63 and produced no outputs.",
    },
    {
      commit: "17b282f77ec16f598054d9983287094457be723c",
      title: "Hoisted the paged-KV row witness",
      state: "implemented",
      detail:
        "This DCO source, tree 9cb8e0cc53f2faef3d6d6fb7ad0ebc9c2bf2c0a5, passes focused v43 27/27. Exact v9 moved the retained local to 48/type 63 but still failed closed with no outputs.",
    },
    {
      commit: "96c3f077e75bd059d7e66f26d61173453a22871e",
      title: "Isolated independent paged-KV write witnesses",
      state: "implemented",
      detail:
        "This clean DCO source, tree 963ae4d724baa2157d5e91b2046e345909e75843, passes focused Rope/KV v42 27/27. Exact v8 moves the retained compiler-intrinsic borrow from local 88 to local 341 but still fails closed before artifact production.",
    },
    {
      commit: "2360a11bbac4e3376ba004472c2fda7aee2d26b1",
      title: "Ended the paged-KV witness scope before loop control flow",
      state: "implemented",
      detail:
        "This clean DCO source ends the local row-striped witness block before the write-result branch and loop latch. The v41 matrix passes 27/27; the source was superseded by the independent key/value witness isolation in 96c3f07.",
    },
    {
      commit: "c4f63ca071a4012c09bdff349c11d69af0f18b18",
      title: "Localized compact-completion row witnesses",
      state: "implemented",
      detail:
        "The DCO source proactively ends compact-completion row witnesses before surrounding control flow. Focused logits checks pass 24/24; the source remains unintegrated.",
    },
    {
      commit: "5ecad80658f27909fa2477d6f73e47e9f95407ae",
      title: "Repinned Ferric to current fe2o3 main",
      state: "implemented",
      detail:
        "This clean DCO source commit updates 54 Ferric dependency, lock, TCB, policy, and documentation files for fe2o3 3d10825. Focused v39 checks pass 27/27 and TCB gates pass 28/28; it remains unintegrated and has produced no HSACO.",
    },
    {
      commit: "1dc659beda81d37d746cb05a16d35fd7788e29ef",
      title: "Integrated bounded live-source radix prefix reuse",
      state: "integration",
      detail:
        "The slice revalidates a live Ready source before sharing committed logical Engine pages. Integrated engine gates report 597 passed, 5 hardware ignores, and 164 doctests; pinned radix Verus reports 608 verified and 0 errors. It does not claim persistent device-KV reuse or speedup.",
    },
    {
      commit: "6df1f2fa99a409adbafd8c3e138f8eae2a728260",
      title: "Integrated authority-free engineering smoke identities",
      state: "integration",
      detail:
        "The adapter derives domain-separated identities from the exact observation and authenticated model plan while granting no KFD, protected-verification, benchmark, or qualification authority.",
    },
    {
      commit: "e8b9908e48313ee43cbeec8092e4ba62006a3dfb",
      title: "Integrated authenticated paired-prefill execution",
      state: "integration",
      detail:
        "The executor joins queue submission, bounded wait and recycle, completed-copy readback, semantic completion, device-KV settlement, page release, and fail-closed teardown custody.",
    },
    {
      commit: "f709a5da2f0130087bcfc0605a4c84ebd966ea0c",
      title: "Integrated the authenticated prefill bootstrap proof",
      state: "verified",
      detail:
        "Source 240bb3d adds the pure S1/T128 lifecycle model. Strict Verus reports 81 verified and 0 errors; proof tests pass 26/26 and source-gate tests pass 28/28.",
    },
    {
      commit: "3d10825df93a86644cc5a5b006cadd45f71afb91",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Selected current fe2o3 main for comprehensive repin",
      state: "implemented",
      detail:
        "Public main 3d10825 includes the earlier deterministic reachability repair and is the selected compiler/runtime/KFD baseline. Ferric's clean DCO source repin is complete and its scoped gates pass; Ferric retains all model kernels and inference policy.",
    },
  ],
  evidence: {
    summary:
      "Ferric separates implemented source, integration, pure proofs, compiler-rooted coverage, artifact acceptance, GPU observation, performance measurement, and M1 qualification. Exact v76 produces one 103,616-byte authority-free aggregate HSACO containing all 12 kernels from a 415,660-byte handoff with 26 GuardedStore operations; exact replay is true. Publication, load, and launch grants are false. The first smoke stops at CurrentFerricDescriptorRoster before KFD, with GPU and VRAM unchanged. The authenticated R33 path executes one 128-output window with authority none, not the required 20 windows. No token, TTFT, TPOT, serving endpoint, vLLM baseline, SGLang baseline, or qualification evidence exists; Docker is inaccessible to this account, no baseline was launched, and all 33 M1 exit gates remain open.",
    legend: [
      ["implemented", "The named source path exists and passes scoped checks."],
      ["integration", "Reviewed components are joined, but end-to-end authority remains open."],
      ["verified", "Pinned Verus proves only the stated pure model and postconditions."],
      ["open", "Required artifact, runtime, hardware, performance, or receipt evidence is absent."],
    ],
    gates: [
      ["M1 exit gates", "33 / 33", "open"],
      ["Authority-free aggregate HSACO", "1", "integration"],
      ["Authenticated Qwen tokens", "0", "open"],
      ["TTFT / TPOT measurements", "0", "open"],
      ["vLLM / SGLang baselines", "0", "open"],
      ["Serving endpoints", "0", "open"],
    ],
  },
});
