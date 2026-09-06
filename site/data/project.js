window.FERRIC_PROJECT = Object.freeze({
  updated: "2026-09-05",
  repository: "https://github.com/harsh-nod/ferric",
  fe2o3Repository: "https://github.com/harsh-nod/fe2o3",
  current: {
    siteRefreshBase: "0e683a68037138d0d92cd88e10be03b405d3402d",
    integrationCommit: "78c720551c4b9809ae7e75ba639959ae0d968b1d",
    integrationTree: "685fb93ef9c7ddcb27aed761be0b757cef130c6f",
    r33LifecycleCommit: "eebdb38dab764143b023b33311363a452a1238ee",
    prefillProofSourceCommit: "240bb3d1ce394436cc62244f51888d7737ea6b9c",
    prefillProofIntegratedCommit: "f709a5da2f0130087bcfc0605a4c84ebd966ea0c",
    directReadbackSourceCommit: "18eed253d30b40a23f3984d7249d24b4db7318d2",
    fe2o3LatestMain: "dd802ce4fc5f759a49cb655ed530af664fe4bc61",
    formalVerified: 81,
    formalErrors: 0,
    proofTestsPassed: 26,
    sourceGateTestsPassed: 28,
    scopedUnadmittedRuntimeBodies: 23,
    combinedInventoryCurrent: false,
    r33AdapterTestsPassed: 55,
    r33AdapterHardwareIgnored: 2,
    r33AdapterDoctestsPassed: 3,
    openM1Gates: 33,
    currentAggregateHsaco: false,
    qwenTokenObserved: false,
    servingEndpointAvailable: false,
  },
  milestone: {
    name: "M1",
    label: "Qwen3 speculative inference on one gfx942",
    state: "integration",
    summary:
      "Integration candidate 78c7205 joins the authenticated R33 lifecycle, S1/T128 prefill bootstrap, bounded waits, formal bootstrap model, and authenticated direct readback. Kernel v27 focused source checks are green, but exact compilation has no HSACO and the production paired-prefill executor is still in progress. Every M1 exit gate remains open.",
  },
  readiness: [
    {
      label: "Authenticated engine lifecycle",
      state: "integration",
      detail:
        "78c7205 retains the authenticated multi-window queue path and adds S1/T128 bootstrap, bounded prefill waits, and direct completed-choice readback without exposing caller-supplied token authority.",
    },
    {
      label: "R33 lifecycle",
      state: "integration",
      detail:
        "eebdb38 adds the authenticated R33 ownership and measurement lifecycle. The standalone adapter passes 55 tests with 2 hardware-dependent ignores and 3 compile-fail doctests. Those adapter bodies are outside the root-workspace formal inventory.",
    },
    {
      label: "Authenticated direct readback",
      state: "integration",
      detail:
        "Source 18eed253, integrated as 78c7205, derives the paired-prefill anchor from ordered completed device copies, performs a private semantic join, and retains fail-closed teardown custody. Full remote gates and independent review are clean.",
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
        "Kernel v27 focused source-contract tests are green. The public compiler admits the exact source closure, then exhausts its explicit deterministic scalar-reachability work bound. Diagnostic-only measurement is running; no HSACO exists.",
    },
    {
      label: "Qwen execution and serving",
      state: "open",
      detail:
        "The production paired-prefill executor is in progress. No current aggregate HSACO, derived artifact identity, hardware execution, Qwen token, endpoint, timing measurement, or baseline comparison exists.",
    },
  ],
  envelope: [
    ["Target", "Qwen3-8B"],
    ["Draft", "Qwen3-0.6B"],
    ["Device", "1 x gfx942"],
    ["Precision", "BF16 with FP32 accumulation"],
    ["Context", "up to 8K tokens"],
    ["Concurrency", "up to 32 sequences"],
    ["Ferric integration candidate", "78c720551c4b9809ae7e75ba639959ae0d968b1d; reviewed integration evidence, not public main, a release, or a serving result"],
    ["fe2o3 main", "dd802ce4fc5f759a49cb655ed530af664fe4bc61; consuming WaveLane accessor and generic exact carrier, plus reusable compiler, runtime, and KFD"],
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
        name: "Authenticated prefill bootstrap and readback",
        detail:
          "The exact S1/T128 bootstrap prepares retained prepublication custody, and the direct readback path derives target choices from completed device copies before semantic completion. This is implemented source, not a completed GPU run.",
      },
    ],
    experimental: [
      {
        name: "Ferric-owned Qwen kernel set",
        detail:
          "The model kernels remain in Ferric and are written through fe2o3 compiler APIs. Focused v27 source-contract checks pass, while exact aggregate compilation exhausts the deterministic scalar-reachability work bound before HSACO production.",
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
        name: "Finish the production paired-prefill executor",
        detail:
          "Join the integrated bootstrap and direct readback through physical completion, KV settlement, release, and successor scheduling with exact failure custody.",
      },
      {
        name: "Produce the exact aggregate HSACO",
        detail:
          "Use the diagnostic-only reachability measurement to resolve the explicit work-bound exhaustion without weakening source closure, then complete exact compilation and artifact inspection.",
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
        name: "Close M1 evidence",
        detail:
          "All 33 M1 exit gates remain open pending their exact proof, artifact, runtime, hardware, performance, independent-review, and receipt evidence.",
      },
    ],
  },
  validation: {
    host: {
      title: "Integrated authenticated bootstrap and readback source",
      state: "integration",
      source: "78c720551c4b9809ae7e75ba639959ae0d968b1d",
      result: "PASS: direct-readback full remote gates and review clean; standalone R33 adapter 55 passed, 2 hardware ignored, 3 doctests",
      detail:
        "78c7205 is an integration candidate, not public main. The R33 adapter is a standalone workspace outside the root compiler-rooted inventory and passes its separate gates. None of these source checks is a Qwen or serving result.",
    },
    proof: {
      title: "Authenticated bootstrap and finite-window models",
      state: "verified",
      source: "f709a5da2f0130087bcfc0605a4c84ebd966ea0c",
      result: "PASS: strict Verus 81 verified / 0 errors; proof tests 26/26; source gate passes 28/28",
      detail:
        "Source 240bb3d, integrated as f709a5d, adds the bootstrap model. The latest scoped inventory diagnostic found 23 unadmitted bootstrap runtime bodies and no proof-module admission error. Combined regeneration remains pending after direct-readback and kernel/executor integration, so no current whole-tree verified/unverified total is claimed. The pure proofs do not prove runtime refinement or effects.",
    },
    hardware: {
      title: "No current aggregate hardware result",
      state: "open",
      sourceStatus: "No exact aggregate artifact or authenticated Qwen run",
      result: "OPEN: focused v27 green; deterministic scalar-reachability work bound exhausted; no HSACO",
      detail:
        "Public fe2o3 main admits the exact source closure and then exhausts its explicit deterministic scalar-reachability work bound. Diagnostic-only measurement is running. There is no production fix, derived artifact identity, hardware execution, or timing authority.",
    },
    transitions: [
      ["Speculative S1/K4 (all terminal)", "Paired prefill new window", "implemented"],
      ["Speculative S1/K8 (all terminal)", "Paired prefill new window", "implemented"],
      ["Speculative S1/K16 (all terminal)", "Paired prefill new window", "implemented"],
      ["Authenticated paired-prefill readback", "Fresh speculative coordinator", "implemented"],
      ["Authenticated S1/T128 bootstrap", "Queue-ready prepublication custody", "implemented"],
      ["Queue-ready paired prefill", "Production physical executor", "integration"],
    ],
    limitation:
      "These are source-level transitions in integration candidate 78c7205. The physical executor is not complete, no aggregate artifact has been produced or accepted, and no row is evidence of a Qwen run, serving endpoint, or performance result.",
  },
  teams: [
    {
      name: "Integration",
      scope: "Ferric integration, review, publication, and end-to-end evidence",
      state: "integration",
      status: "Progressing, no team blocker",
      completed:
        "Candidate 78c7205 joins R33 lifecycle, bootstrap, bounded waits, formal bootstrap proof, exact fe2 repin, and authenticated direct readback.",
      current:
        "Sequencing the paired-prefill executor, kernel diagnostic, and final combined evidence refresh.",
      blockedBy:
        "No team-local blocker. M1 depends on the exact kernel artifact, completed physical executor, and protected artifact admission.",
      next:
        "Integrate the next reviewed slices, regenerate the combined formal inventory, and rerun exact remote gates.",
      validation:
        "78c7205 is the current reviewed integration candidate; it is not public main, a release, Qwen run, or serving result.",
    },
    {
      name: "Kernels",
      scope: "Ferric-owned Qwen kernels compiled with fe2o3",
      state: "integration",
      status: "Progressing, no team blocker",
      completed:
        "Kernel v27 source-contract focused tests pass, and public fe2o3 admits the exact source closure.",
      current:
        "Running diagnostic-only measurement of deterministic scalar reachability after explicit work-bound exhaustion.",
      blockedBy:
        "No team-local blocker. Artifact production depends on a sound work-bound resolution in generic compiler infrastructure.",
      next:
        "Classify the measurement, implement a production-safe resolution, rerun exact compilation, and inspect any resulting HSACO.",
      validation:
        "Focused v27 passes; exact compilation produced no derived identities, artifact, or HSACO.",
    },
    {
      name: "Inference engine",
      scope: "Scheduling, KV, queue custody, R33 lifecycle, and bootstrap",
      state: "integration",
      status: "Progressing, no team blocker",
      completed:
        "Authenticated all-terminal rollover, R33 bootstrap, bounded waits, direct choice observation, private semantic join, and exact retry/close paths.",
      current:
        "Implementing the production paired-prefill executor that connects prepublication custody to physical completion and successor scheduling.",
      blockedBy:
        "No team-local blocker. End-to-end execution depends on the exact HSACO and protected runtime admission.",
      next:
        "Complete and independently review the paired-prefill executor, then exercise it with an accepted artifact.",
      validation:
        "Standalone R33 adapter: 55 passed, 2 hardware-dependent ignored, 3 compile-fail doctests; strict Clippy clean.",
    },
    {
      name: "Formal verification",
      scope: "Verus models, source coverage, and hostile mutation policy",
      state: "verified",
      status: "Progressing, no team blocker",
      completed:
        "The combined proof crate passes strict Verus 81/0, proof tests 26/26, and source-gate tests 28/28.",
      current:
        "Keeping runtime effects outside the pure proof claim and preparing final combined inventory regeneration after active source integration.",
      blockedBy:
        "No team-local blocker. Final inventory identity depends on the combined kernel and executor head; runtime refinement remains explicitly unproved.",
      next:
        "Regenerate and audit exact body identities, then rerun the full formal and hostile-policy matrix on the settled combined head.",
      validation:
        "Latest scoped diagnostic: 23 bootstrap runtime bodies unadmitted pending combined regeneration; no current whole-tree closure count is claimed.",
    },
  ],
  boundaries: {
    ferric: [
      "Qwen model, tokenizer, weight, graph, plan, and artifact admission policy",
      "All model-specific kernels and inference semantics",
      "Scheduling, speculative coordination, paged KV ownership, and queue lifecycle composition",
      "Authenticated new-window custody and the R33 lifecycle",
      "Ferric-specific Verus models, source policy, hostile mutations, and M1 evidence",
      "Integration candidate 78c720551c4b9809ae7e75ba639959ae0d968b1d; no aggregate HSACO or Qwen result",
    ],
    fe2o3: [
      "Reusable Rust-to-KIR-to-LLVM compiler infrastructure",
      "Generic artifact, descriptor, and compiler-lineage types",
      "Direct-KFD runtime, allocations, AQL queues, completion, and bounded waits",
      "Generic protected verification transport",
      "Public main dd802ce4fc5f759a49cb655ed530af664fe4bc61 with consuming WaveLane accessor and generic exact carrier",
      "No Ferric model kernel or inference policy is moved upstream",
    ],
  },
  latestObservation: {
    title: "Exact aggregate compilation exhausts its explicit work bound",
    state: "open",
    sourceStatus: "Kernel v27 development source; diagnostic-only measurement in progress",
    environment: "mi300x exact source and compiler checks; no hardware execution",
    result:
      "Focused kernel v27 green; source closure admitted; deterministic scalar-reachability work bound exhausted",
    buildId: "None: no HSACO produced",
    generatedTokenIds: [],
    authority:
      "This is a fail-closed compiler diagnostic, not a hardware observation. No Qwen token, TTFT, TPOT, numerical result, serving endpoint, or vLLM/SGLang baseline exists.",
  },
  recentProgress: [
    {
      commit: "78c720551c4b9809ae7e75ba639959ae0d968b1d",
      title: "Integrated authenticated direct diagnostic readback",
      state: "integration",
      detail:
        "Source 18eed253 derives direct target choices from completed copies, admits no caller token oracle, and retains exact teardown custody. Full remote gates and independent review are clean.",
    },
    {
      commit: "f709a5da2f0130087bcfc0605a4c84ebd966ea0c",
      title: "Integrated the authenticated prefill bootstrap proof",
      state: "verified",
      detail:
        "Source 240bb3d adds the pure S1/T128 lifecycle model. Strict Verus reports 81 verified and 0 errors; proof tests pass 26/26 and source-gate tests pass 28/28.",
    },
    {
      commit: "eebdb38dab764143b023b33311363a452a1238ee",
      title: "Integrated authenticated R33 lifecycle ownership",
      state: "integration",
      detail:
        "The lifecycle owns admitted capabilities and bounded start/ready/measure/stop ordering. Its standalone workspace is validated separately from the root formal inventory.",
    },
    {
      commit: "30af5c2012850afa525539a7e50f8a3b92497f50",
      title: "Integrated authenticated speculative new windows",
      state: "implemented",
      detail:
        "The engine retains deadlines, page admission, program and model-memory authority, terminal lineage, flat predecessor history, and opaque publication/readback custody.",
    },
    {
      commit: "dd802ce4fc5f759a49cb655ed530af664fe4bc61",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Advanced the reusable fe2o3 compiler and runtime",
      state: "implemented",
      detail:
        "Public main adds the consuming WaveLane accessor and generic exact carrier. Compiler, runtime, and KFD remain in fe2o3; Ferric retains model kernels and inference policy.",
    },
  ],
  evidence: {
    summary:
      "Ferric separates source implementation, pure proofs, compiler-rooted coverage, artifact acceptance, GPU observation, performance measurement, and M1 qualification. The current candidate has strong scoped source evidence, but its combined body inventory is pending regeneration and none of the 33 M1 exit gates is closed.",
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
