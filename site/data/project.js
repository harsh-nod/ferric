window.FERRIC_PROJECT = Object.freeze({
  updated: "2026-09-06",
  repository: "https://github.com/harsh-nod/ferric",
  fe2o3Repository: "https://github.com/harsh-nod/fe2o3",
  current: {
    siteRefreshBase: "4d5fd39b11a10152524518f34bdc00267a858753",
    integrationCommit: "1dc659beda81d37d746cb05a16d35fd7788e29ef",
    integrationTree: "5b6cb64b72a0fefe8a92cd76a3129d63fa53a8ad",
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
    fe2o3LatestMain: "6492c8fa85a00d93aa6ca2a4a77675fefad2fee6",
    fe2o3LatestTree: "68573bf31789625ecc2489491711ad9153eb1cac",
    formalVerified: 81,
    formalErrors: 0,
    proofTestsPassed: 26,
    sourceGateTestsPassed: 28,
    scopedUnadmittedRuntimeBodies: 23,
    combinedInventoryCurrent: false,
    r33AdapterTestsPassed: 55,
    r33AdapterHardwareIgnored: 2,
    r33AdapterDoctestsPassed: 3,
    integratedEngineTestsPassed: 597,
    integratedEngineHardwareIgnored: 5,
    integratedEngineDoctestsPassed: 164,
    radixVerusVerified: 608,
    radixVerusErrors: 0,
    openM1Gates: 33,
    engineeringSmokeBinaryStaged: true,
    canonicalQwenSnapshotVerified: true,
    radixPrefixIntegrated: true,
    currentAggregateHsaco: false,
    qwenTokenObserved: false,
    servingEndpointAvailable: false,
  },
  milestone: {
    name: "M1",
    label: "Qwen3 speculative inference on one gfx942",
    state: "integration",
    summary:
      "Intermediate Ferric integration 23f326a contains the audited seven-file exact kernel/host ABI delta plus authenticated prefill, readback, and radix work, but it is not final or public product integration. The fe2o3 private-slot fix is public at 6492c8f. An exact patched compile of Ferric candidate 72c78c6 clears retained-borrow locals 178 and 40 and reaches AMDGPU LLVM lowering. It then fails closed at qwen3_rmsnorm_v1 bb6 op0 with UnprovenBarrierConvergence: Subgroup uniform required, Varying found. Status is 1 and no HSACO was produced. The final fe2o3 repin and RMSNorm fix remain pending; no Qwen token, TTFT, or TPOT exists, and every M1 exit gate remains open.",
  },
  readiness: [
    {
      label: "Authenticated engine lifecycle",
      state: "integration",
      detail:
        "1dc659b retains the authenticated multi-window queue path and the paired-prefill executor integrated at e8b9908. It owns queue creation, bounded completion, direct readback, semantic completion, KV settlement, page release, and fail-closed teardown custody.",
    },
    {
      label: "R33 lifecycle",
      state: "integration",
      detail:
        "eebdb38 adds the authenticated R33 ownership and measurement lifecycle. The standalone adapter passes 55 tests with 2 hardware-dependent ignores and 3 compile-fail doctests. Those adapter bodies are outside the root-workspace formal inventory.",
    },
    {
      label: "Engineering Qwen smoke path",
      state: "integration",
      detail:
        "6df1f2f adds authority-free, domain-separated engineering identity derivation. A smoke binary is staged and the canonical Qwen snapshot is verified, but this path is explicitly non-production and has produced no GPU token or timing result.",
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
        "Ferric candidate 72c78c6, tree 6b3a5ab, retains its green focused Rope/KV 27/27 and full ferric-qwen-kernels suite; engine physical-recipe checks pass 7/7. The private-slot fix used by the patched exact compile is now public in fe2o3 at 6492c8f, tree 68573bf. That compile clears retained-borrow locals 178 and 40, reaches AMDGPU LLVM lowering, and fails closed at qwen3_rmsnorm_v1 bb6 op0: UnprovenBarrierConvergence: Subgroup uniform required, Varying found. The run exits status 1 with no HSACO.",
    },
    {
      label: "Radix prefix reuse",
      state: "integration",
      detail:
        "1dc659b integrates bounded live-source reuse of logical committed prefixes through generational Engine custody. Integrated engine gates report 597 passed with 5 hardware ignores and 164 doctests; pinned Verus reports 608 verified and 0 errors. S1/T128 correctly remains NoMatch at logical width 256, with no persistent device-KV or speed claim.",
    },
    {
      label: "Qwen execution and serving",
      state: "open",
      detail:
        "The engineering smoke executable and verified model snapshot are staged, but exact compilation has not produced the aggregate HSACO. The production protected receipt/verifier service is undeployed. No hardware execution, Qwen token, endpoint, timing measurement, or baseline comparison exists.",
    },
  ],
  envelope: [
    ["Target", "Qwen3-8B"],
    ["Draft", "Qwen3-0.6B"],
    ["Device", "1 x gfx942"],
    ["Precision", "BF16 with FP32 accumulation"],
    ["Context", "up to 8K tokens"],
    ["Concurrency", "up to 32 sequences"],
    ["Ferric integration candidate", "1dc659beda81d37d746cb05a16d35fd7788e29ef; reviewed integration evidence, not public main, a release, or a serving result"],
    ["Ferric intermediate integration", "23f326a3133ef4b5e0da19b9a170bf05f8cb7a6b; audited seven-file exact kernel/host ABI delta plus authenticated prefill, readback, and radix; not final or public product integration"],
    ["fe2o3 main", "6492c8fa85a00d93aa6ca2a4a77675fefad2fee6; tree 68573bf31789625ecc2489491711ad9153eb1cac; reusable compiler, runtime, and KFD"],
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
        name: "Authority-free engineering identity derivation",
        detail:
          "The engineering smoke adapter can derive distinct domain-separated identities from the actual observation, authenticated target/draft model plans, descriptors, and program catalog. It grants no KFD, protected-verification, benchmark, or qualification authority.",
      },
    ],
    experimental: [
      {
        name: "Ferric-owned Qwen kernel set",
        detail:
          "The model kernels remain in Ferric and are written through fe2o3 compiler APIs. Candidate 72c78c6 retains green focused Rope/KV 27/27, full kernel-suite, and engine physical-recipe 7/7 checks. With private-slot lowering fixed, exact compilation clears retained-borrow locals 178 and 40 and reaches AMDGPU LLVM lowering. The current kernel obligation is RMSNorm barrier convergence: qwen3_rmsnorm_v1 bb6 op0 requires subgroup-uniform control flow but receives a varying value.",
      },
      {
        name: "Bounded live radix prefix reuse",
        detail:
          "The integrated radix slice indexes token prefixes and revalidates a live Ready source before sharing committed logical Engine pages. It deliberately returns NoMatch for S1/T128 at the current 256-token logical width and makes no persistent-cache, physical device-KV reuse, or speed claim.",
      },
      {
        name: "Authenticated R33 backend lifecycle",
        detail:
          "The R33 lifecycle owns admitted engine capabilities, exact start/ready/measure/stop ordering, bounded twenty-window operation, and fail-closed cleanup. It has not produced a serving or comparison run.",
      },
      {
        name: "Pure lifecycle proofs",
        detail:
          "Separate Verus models cover the finite window cap and exact S1/T128 prepublication lifecycle. They do not prove Rust refinement, liveness, timing, KFD, allocation, device memory, queue publication, or serving.",
      },
      {
        name: "Event-backed comparison schema",
        detail:
          "Ferric has structures for paired per-request E2E, TTFT, and TPOT collection across Ferric, vLLM, and SGLang, but no workload has been measured.",
      },
    ],
    roadmap: [
      {
        name: "Produce the exact aggregate HSACO",
        detail:
          "Repair qwen3_rmsnorm_v1 subgroup barrier convergence, complete the final fe2o3 6492c8f repin, rerun exact compilation, then inspect an artifact only if one is produced.",
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
      title: "Integrated executor, engineering identities, and radix source",
      state: "integration",
      source: "1dc659beda81d37d746cb05a16d35fd7788e29ef",
      result: "PASS: integrated engine 597 passed / 5 hardware ignored; 164 doctests; strict workspace Clippy clean",
      detail:
        "1dc659b integrates the authenticated paired-prefill executor, authority-free engineering identity mode, and live-source logical radix reuse. The engineering smoke binary and verified snapshot are staged, not executed. These source checks are not a Qwen, serving, speed, or qualification result.",
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
      title: "No current aggregate hardware result",
      state: "open",
      sourceStatus: "No exact aggregate artifact or authenticated Qwen run",
      result: "OPEN: retained-borrow locals 178 and 40 cleared; RMSNorm barrier convergence fails in AMDGPU LLVM lowering; no HSACO",
      detail:
        "The private-slot fix used by this patched exact compile is public in fe2o3 at 6492c8fa85a00d93aa6ca2a4a77675fefad2fee6, tree 68573bf31789625ecc2489491711ad9153eb1cac; Ferric's final repin remains pending. Ferric candidate 72c78c6fb3766ae3de1a4a393f6a8fa358498ca4, tree 6b3a5ab219216fe2d7adf797d7ea890f94c76a60, clears retained-borrow locals 178 and 40 and reaches AMDGPU LLVM lowering. The current terminal is qwen3_rmsnorm_v1 bb6 op0 UnprovenBarrierConvergence: Subgroup uniform required, Varying found. The run exits status 1 with no HSACO; log SHA 8bb0e6cb1e87f3c22a4328fd3aeebcb6285197d33cab830bf6afedad91653be0. No artifact, hardware execution, token, or timing authority exists.",
    },
    transitions: [
      ["Speculative S1/K4 (all terminal)", "Paired prefill new window", "implemented"],
      ["Speculative S1/K8 (all terminal)", "Paired prefill new window", "implemented"],
      ["Speculative S1/K16 (all terminal)", "Paired prefill new window", "implemented"],
      ["Authenticated paired-prefill readback", "Fresh speculative coordinator", "implemented"],
      ["Authenticated S1/T128 bootstrap", "Queue-ready prepublication custody", "implemented"],
      ["Queue-ready paired prefill", "Engine and device-KV completion", "implemented"],
      ["Live Ready radix source", "Shared committed logical prefix", "integration"],
    ],
    limitation:
      "These are source-level transitions in integration candidate 1dc659b. Radix reuse is live-source and logical only; S1/T128 is NoMatch at logical width 256. No aggregate artifact has been produced or accepted, and no row is evidence of a Qwen run, serving endpoint, or performance result.",
  },
  teams: [
    {
      name: "Integration",
      scope: "Ferric integration, review, publication, and end-to-end evidence",
      state: "integration",
      status: "Progressing, no team blocker",
      completed:
        "Candidate 1dc659b joins the authenticated executor, authority-free engineering identities, and bounded live-source radix reuse on top of the existing R33 and bootstrap path.",
      current:
        "Intermediate 23f326a contains the audited seven-file exact kernel/host ABI delta plus authenticated prefill, readback, and radix work. It is not final or public product integration and remains pending the final fe2o3 repin and RMSNorm fix.",
      blockedBy:
        "No team-local blocker. M1 depends on the exact kernel artifact, an exclusive GPU run, and deployed protected artifact admission.",
      next:
        "Complete the fe2o3 6492c8f repin and RMSNorm repair, settle the final integration head, then run the staged engineering smoke and combined evidence gates.",
      validation:
        "23f326a is audited intermediate integration only; its seven-file ABI delta and authenticated prefill, readback, and radix composition are not yet final public product integration.",
    },
    {
      name: "Kernels",
      scope: "Ferric-owned Qwen kernels compiled with fe2o3",
      state: "integration",
      status: "Blocked on RMSNorm barrier convergence",
      completed:
        "Exact compilation cleared paged-decode and Rope/KV arithmetic; v37 cleared ranked-CFG capacity and v38 cleared the ranked argument limit after 27/27 focused equivalence and source checks.",
      current:
        "The fe2o3 private-slot fix clears both retained-borrow failures, locals 178 and 40, and exact compilation now reaches AMDGPU LLVM lowering.",
      blockedBy:
        "Current exact blocker: qwen3_rmsnorm_v1 bb6 op0 reports UnprovenBarrierConvergence because subgroup-uniform control flow is required but a varying value was found.",
      next:
        "Repair RMSNorm subgroup barrier convergence, finish the fe2o3 6492c8f repin, then rerun focused and exact validation.",
      validation:
        "The patched exact compile reaches AMDGPU LLVM lowering, then exits status 1 at the RMSNorm convergence diagnostic. No artifact or HSACO exists; log SHA 8bb0e6cb1e87f3c22a4328fd3aeebcb6285197d33cab830bf6afedad91653be0.",
    },
    {
      name: "Inference engine",
      scope: "Scheduling, KV, queue custody, R33 lifecycle, and bootstrap",
      state: "integration",
      status: "Progressing, no team blocker",
      completed:
        "Authenticated rollover, paired-prefill execution, authority-free engineering identities, and live-source logical radix reuse are integrated.",
      current:
        "Holding the staged smoke executable and verified Qwen snapshot ready for the first exact aggregate and exclusive GPU slot.",
      blockedBy:
        "No team-local blocker. End-to-end execution depends on the exact HSACO and protected runtime admission.",
      next:
        "Run the authority-free engineering smoke after HSACO production, while retaining fail-closed production admission.",
      validation:
        "Integrated engine: 597 passed, 5 hardware-dependent ignored, 164 doctests; strict workspace Clippy clean.",
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
      "Integration candidate 1dc659beda81d37d746cb05a16d35fd7788e29ef; no aggregate HSACO or Qwen result",
      "Intermediate integration 23f326a3133ef4b5e0da19b9a170bf05f8cb7a6b contains the audited seven-file exact kernel/host ABI delta plus authenticated prefill, readback, and radix; it is not final or public product integration",
    ],
    fe2o3: [
      "Reusable Rust-to-KIR-to-LLVM compiler infrastructure",
      "Generic artifact, descriptor, and compiler-lineage types",
      "Direct-KFD runtime, allocations, AQL queues, completion, and bounded waits",
      "Generic protected verification transport",
      "Public main 6492c8fa85a00d93aa6ca2a4a77675fefad2fee6, tree 68573bf31789625ecc2489491711ad9153eb1cac, contains the reusable private-slot lowering fix selected for Ferric's final repin",
      "No Ferric model kernel or inference policy is moved upstream",
    ],
  },
  latestObservation: {
    title: "Private-slot failures cleared; RMSNorm convergence is next",
    state: "open",
    sourceStatus: "Ferric candidate 72c78c6fb3766ae3de1a4a393f6a8fa358498ca4, tree 6b3a5ab219216fe2d7adf797d7ea890f94c76a60",
    environment: "mi300x patched exact compile using the private-slot fix now public in fe2o3 6492c8f; compiler-only, no hardware execution",
    result:
      "Retained-borrow locals 178 and 40 are cleared. Exact compilation reaches AMDGPU LLVM lowering, then qwen3_rmsnorm_v1 bb6 op0 reports UnprovenBarrierConvergence: Subgroup uniform required, Varying found. Status is 1 and no HSACO exists; log SHA 8bb0e6cb1e87f3c22a4328fd3aeebcb6285197d33cab830bf6afedad91653be0.",
    buildId: "None: no HSACO produced",
    generatedTokenIds: [],
    authority:
      "This is a fail-closed compiler diagnostic, not a compiler artifact or hardware observation. No Qwen token, TTFT, TPOT, numerical result, serving endpoint, or vLLM/SGLang baseline exists.",
  },
  recentProgress: [
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
      "Ferric separates implemented source, intermediate integration, pure proofs, compiler-rooted coverage, artifact acceptance, GPU observation, performance measurement, and M1 qualification. The private-slot compiler failures are cleared and exact compilation reaches RMSNorm barrier convergence in AMDGPU LLVM lowering, but no HSACO or token exists, production protected admission is undeployed, the combined body inventory is pending regeneration, and none of the 33 M1 exit gates is closed.",
    legend: [
      ["implemented", "The named source path exists and passes scoped checks."],
      ["integration", "Reviewed components are joined, but end-to-end authority remains open."],
      ["verified", "Pinned Verus proves only the stated pure model and postconditions."],
      ["open", "Required artifact, runtime, hardware, performance, or receipt evidence is absent."],
    ],
    gates: [
      ["M1 exit gates", "33 / 33", "open"],
      ["Current aggregate HSACO", "0", "open"],
      ["Authenticated Qwen tokens", "0", "open"],
      ["TTFT / TPOT measurements", "0", "open"],
      ["vLLM / SGLang baselines", "0", "open"],
      ["Serving endpoints", "0", "open"],
    ],
  },
});
