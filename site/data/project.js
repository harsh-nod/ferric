window.FERRIC_PROJECT = Object.freeze({
  updated: "2026-08-28",
  repository: "https://github.com/harsh-nod/ferric",
  fe2o3Repository: "https://github.com/harsh-nod/fe2o3",
  milestone: {
    name: "M1",
    label: "Qwen3 speculative inference on one gfx942",
    state: "integration",
    summary:
      "Ferric has authenticated Qwen inputs, exact execution plans, bounded engine state, and source-level protected Worker V3 adapters for all seven M1 kernel families. PR #24 qualified canonical Git-tree source-closure modes at 3898bf40 and landed them in m1/bundle-admission through 49a539f2. No authenticated collector currently acquires the seven live owners; current artifacts, protected authority, end-to-end Qwen, hardware and performance qualification, the production receipt, and M1 remain open.",
  },
  envelope: [
    ["Target", "Qwen3-8B"],
    ["Draft", "Qwen3-0.6B"],
    ["Device", "1 x gfx942"],
    ["Precision", "BF16 / FP32 accumulate"],
    ["Context", "up to 8K tokens"],
    ["Concurrency", "up to 32 sequences"],
    ["Ferric integration", "PR #24 qualified 3898bf40 / landed 49a539f2"],
    ["fe2o3 integration", "42639ecc; compiler receipt, verifier, and KFD runtime line"],
    [
      "Protected artifact",
      "SwiGLU semantic fe2ce532...e9569e8f / HSACO 57ecb86b...fc6afa7",
    ],
    [
      "Current authority",
      "Source and scoped evidence only; whole-Qwen production authority remains open",
    ],
  ],
  readiness: [
    {
      label: "Ferric inference foundations",
      state: "implemented",
      detail:
        "Authenticated model inputs, exact target and draft plans, generational scheduling, paged KV custody, cancellation, completion epochs, and finite speculative lifecycle paths exist in source.",
    },
    {
      label: "Durable compiler receipt path",
      state: "implemented",
      detail:
        "fe2o3 42639ecc carries a durable subject-bound receipt from the compiler backend through inherited FD195 into Cargo, then through recovered V2 admission. Its generic sealed verifier enforces a promotion boundary that requires receipt-complete evidence.",
    },
    {
      label: "KFD runtime foundations",
      state: "implemented",
      detail:
        "fe2o3 42639ecc integrates typed memory, queue ownership, fixed-batch publication, completion readback, and dispatch foundations. These reusable primitives do not by themselves make the Ferric Qwen path runnable.",
    },
    {
      label: "Seven Worker V3 family adapters",
      state: "implemented",
      detail:
        "Qualified source 1d666dbc, landed through be307d52, makes GEMM, RMSNorm, RoPE/KV, prefill, paged decode, SwiGLU, and logits accept only matching compiler-produced, move-only Worker V3 owners before strict structural inspection. Tests do not execute Worker V3 or establish current HSACO existence.",
    },
    {
      label: "Canonical M1 source closure",
      state: "qualified",
      detail:
        "PR #24 qualified exact head 3898bf40 and landed its tree through merge 49a539f2. The 495-file closure hashes to 25bbf05b8cec3d0e7157c4f1b66e6dbe6b31c1cfa647d3cde07a640a2e50699e and derives executable state only from canonical Git tree modes 100644 and 100755. All scoped checks passed.",
    },
    {
      label: "Historical SwiGLU candidate",
      state: "observed",
      detail:
        "The older protected Qwen3 SwiGLU BF16 candidate matched all 3,072 qualification outputs exactly through the HIP harness. It is not a fresh seven-family V3 occurrence and cannot enter production.",
    },
    {
      label: "End-to-end Qwen through Ferric",
      state: "open",
      detail:
        "The production path still needs current seven-family artifacts, Ferric's protected policy, Worker ledger, external rollback verifier, distinct-UID service deployment and keys, an authenticated seven-owner collector and roster, authority-safe custody fixtures, a runner, end-to-end Qwen execution, and current-source hardware and performance qualification.",
    },
    {
      label: "M1 qualification",
      state: "open",
      detail:
        "No complete M1 evidence index or qualification receipt exists. The qualified source closure, scoped proof release, historical kernel artifact, and qualification-only numerics do not close the production receipt or M1.",
    },
  ],
  capabilities: {
    runnable: [
      {
        name: "Authenticated Qwen3 inputs",
        detail:
          "Strict pinned configuration, shared tokenizer metadata, safetensors schema, and prepacked-weight admission are implemented for the declared model envelope.",
      },
      {
        name: "Exact execution planning",
        detail:
          "Ferric generates typed target and draft operations, workspace layouts, queue plans, and Ferric-owned kernel specifications for admitted shapes.",
      },
      {
        name: "Bounded host lifecycle",
        detail:
          "Generational scheduling, paged KV custody, cancellation, completion epochs, finite-shape speculative rearm, and quiescent retirement are implemented with fail-closed ownership transitions.",
      },
      {
        name: "Target-only diagnostic path",
        detail:
          "An earlier exact-source target-only prompt-to-text path completed on MI300X. It is a diagnostic observation, not evidence that the current Worker V3 production path runs Qwen.",
      },
    ],
    experimental: [
      {
        name: "Finite speculative serving",
        detail:
          "Host-validated admission and same-shape rearm cover S1/K4, S8/K4, S1/K8, and S1/K16 with generation-bound evidence and explicit inactive-lane handling.",
      },
      {
        name: "Finite production rollover",
        detail:
          "Four exact paired-prefill transitions preserve queue, KV, output, and provider custody. Every unlisted transition remains rejected.",
      },
      {
        name: "Seven protected Worker V3 adapters",
        detail:
          "Every K1-K7 family now binds its exact compiler handoff to one matching move-only Worker V3 owner and applies post-worker structural inspection. Publication also checks canonical link options, execution limits, and the reviewed Worker measurement.",
      },
      {
        name: "Receipt-complete upstream boundary",
        detail:
          "fe2o3's backend acquires the durable subject-bound receipt, Cargo recovers it under currentness, and the generic sealed verifier enforces a promotion boundary that requires complete compiler receipt evidence.",
      },
      {
        name: "Typed KFD execution substrate",
        detail:
          "Reusable typed allocation, USERPTR/AQL queue ownership, fixed-batch publication, completion, and dispatch components are integrated upstream for Ferric to consume.",
      },
    ],
    roadmap: [
      {
        name: "Ferric production authority",
        detail:
          "Define the concrete protected kernel policy, bind a Worker ledger, join repository and host-descriptor lineage, and require an external rollback verifier before promotion.",
      },
      {
        name: "Authenticated evidence collector",
        detail:
          "Replace the retired Worker V2 artifact command with an in-process collector that acquires all seven real V3 owners and passes them atomically to the fail-closed artifact publisher.",
      },
      {
        name: "Generated contracts and evidence",
        detail:
          "Add receipt-bound generated marker contracts for the six non-SwiGLU families, exercise every public V3 bind path with authority-safe positive and hostile custody fixtures, and complete authenticated proof/executable, Rust layout, Rust effect, safety, and operator-refinement evidence for all seven lanes.",
      },
      {
        name: "Protected service deployment",
        detail:
          "Deploy the compiler execution service and backend under distinct nonroot identities with production keys, permissions, and operational supervision.",
      },
      {
        name: "Complete Qwen execution",
        detail:
          "Join the seven-family artifact set to the full target and draft graph, model bundle, generated runner, and production KFD path.",
      },
      {
        name: "Hardware and evidence closure",
        detail:
          "Run current-source end-to-end Qwen on gfx942, validate numerical behavior and performance, obtain independent evidence, and produce the complete M1 qualification receipt.",
      },
      {
        name: "Serving breadth",
        detail:
          "Sampling, prefix caching, chunked prefill, quantized KV, and an HTTP boundary follow after the exact M1 path closes.",
      },
    ],
  },
  latestObservation: {
    title: "SwiGLU qualification matches exactly on gfx942",
    date: "2026-08-26",
    state: "observed",
    commit: "1b77cb5b82e370ca9a46c04d4465d2ba61737d01",
    buildId: "HSACO 57ecb86b...fc6afa7",
    environment: "MI300X / gfx942:sramecc+:xnack-; qualification-only HIP harness",
    result: "PASS: 3,072 / 3,072 exact; max ULP 0",
    generatedTokenIds: [],
    authority:
      "This run validates one exact SwiGLU artifact through the qualification harness. It does not exercise the production Ferric verifier, KFD execution path, complete Qwen graph, token generation, performance envelope, or M1 qualification.",
  },
  validation: {
    host: {
      title: "Canonical M1 source closure",
      state: "qualified",
      source: "3898bf406e5be7f536ead442b05ba3254abafbf3",
      closureSha256:
        "25bbf05b8cec3d0e7157c4f1b66e6dbe6b31c1cfa647d3cde07a640a2e50699e",
      result: "PASS: 495 files; canonical 100644/100755 Git-tree modes",
      detail:
        "PR #24 hardened source-closure measurement so regular and executable blobs derive only from exact canonical Git tree modes 100644 and 100755. All scoped checks passed at 3898bf40; merge 49a539f2 landed the identical tree in m1/bundle-admission. This qualifies source-closure integrity only, not the seven-owner collector, current artifacts, hardware, performance, end-to-end Qwen, a production receipt, or M1.",
    },
    proof: {
      title: "Authenticated release proof",
      state: "qualified",
      source: "58fd37e",
      closureSha256:
        "b922a6cd2881bd38403afce0c14dc898cf13da770616875489069a2701f2c933",
      result: "PASS: 645 admitted proof bodies; 1,490 verification queries",
      detail:
        "The source gate covers 143 verified modules and 6,121 executable bodies. It qualifies the scoped release proof only, not the evolving Worker V3 integration, hardware execution, Qwen correctness, performance, or M1.",
    },
    hardware: {
      title: "Qualification-only SwiGLU run",
      state: "observed",
      source: "1b77cb5b82e370ca9a46c04d4465d2ba61737d01",
      result: "PASS: 3,072 / 3,072 exact; max ULP 0",
      detail:
        "The HIP harness loaded and dispatched the exact protected HSACO on gfx942. No current-source production Worker V3 verifier, KFD path, complete Qwen graph, or token-generation result was exercised.",
    },
    transitions: [
      ["Prefill S1/T128", "Speculative S1/K4", "implemented"],
      ["Prefill S1/T128", "Speculative S1/K8", "implemented"],
      ["Prefill S1/T128", "Speculative S1/K16", "implemented"],
      ["Prefill S8/T128", "Speculative S8/K4", "implemented"],
    ],
    limitation:
      "The catalog records host-validated source paths. Target-only decode and every unlisted cross-plan transition require explicit queue retirement and a fresh admitted launch; none inherit rollover authority.",
  },
  recentProgress: [
    {
      commit: "49a539f251c8681644e6551c20fcce35e5fd4216",
      title: "Land PR #24 source-closure hardening",
      state: "qualified",
      detail:
        "PR #24 merged qualified head 3898bf406e5be7f536ead442b05ba3254abafbf3 into m1/bundle-admission with the identical source tree. M1, end-to-end Qwen, the seven-owner roster and collector, current artifacts, hardware and performance qualification, and the production receipt remain open.",
    },
    {
      commit: "3898bf406e5be7f536ead442b05ba3254abafbf3",
      title: "Qualify canonical Git-tree source modes",
      state: "qualified",
      detail:
        "The source-closure gate passed all scoped checks over 495 files with SHA-256 25bbf05b8cec3d0e7157c4f1b66e6dbe6b31c1cfa647d3cde07a640a2e50699e. It recognizes exact 100644 regular blobs and 100755 executable blobs and rejects noncanonical mode input; it grants no runtime or qualification authority.",
    },
    {
      commit: "be307d5269b29f8f2c3326af685df01e5c20c3d5",
      title: "Land the seven-family migration in m1/bundle-admission",
      state: "implemented",
      detail:
        "PR #21 merged qualified source 1d666dbce8094fd8e96c40a00e316d6167e17fc2 into m1/bundle-admission. This lands source infrastructure and scoped checks; it does not create seven current artifacts, execute Qwen, or produce an M1 qualification receipt.",
    },
    {
      commit: "1d666dbce8094fd8e96c40a00e316d6167e17fc2",
      title: "Qualify all seven M1 kernels on protected Worker V3 evidence",
      state: "implemented",
      detail:
        "GEMM, RMSNorm, RoPE/KV, prefill, paged decode, SwiGLU, and logits now require matching compiler-produced V3 owners. The artifact publisher requires all seven, exact link and execution policy, and reviewed Worker measurement before any staging directory is created. A live collector and authority-safe positive and hostile custody fixtures remain open.",
    },
    {
      commit: "42639ecc7f2f377ab57e5e884c36133a126f230e",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Integrate current compiler receipt, verifier, and KFD foundations",
      state: "implemented",
      detail:
        "The current fe2o3 line combines durable subject-bound compiler receipts, recovered V2 carriage, receipt-complete sealed verification, Worker ledger acquisition, typed KFD memory and queues, fixed-batch completion, and dispatch. Deployment identity and Ferric inference policy remain outside the upstream boundary.",
    },
    {
      commit: "1b77cb5b82e370ca9a46c04d4465d2ba61737d01",
      title: "Qualify the first Worker V3 kernel numerics",
      state: "observed",
      detail:
        "The qualification-only HIP harness dispatched the exact protected SwiGLU HSACO on gfx942 and matched all 3,072 outputs exactly with max ULP 0.",
    },
    {
      commit: "57f6cfdf4b3f5177a556159d1e548b25b63a1541",
      title: "Complete the protected SwiGLU build",
      state: "observed",
      detail:
        "The protected two-phase build produced the exact semantic KIR identity, 14,192-byte HSACO, and load-envelope lineage for qwen3_swiglu_bf16_f32_v1 on gfx942:xnack-.",
    },
    {
      commit: "c7362d93d031f735ade33bf8bfa25ff8250e359b",
      title: "Bind an authority-free verifier request",
      state: "implemented",
      detail:
        "Ferric binds the protected build, 22 admitted profiles, ABI, launch semantics, and projected receipt axes into a pending request. The request is inert until a concrete accepting policy and external rollback verifier join it.",
    },
    {
      commit: "58fd37e",
      title: "Authenticate the scoped lifecycle proof release",
      state: "qualified",
      detail:
        "The strict same-source wrapper passed 645 admitted proof bodies, 1,490 verification queries, 37 rejected exact-body mutations, and 10 source-quality gates. Its authority remains scoped to that proof release.",
    },
    {
      commit: "8940b28",
      title: "Generalize exact production rollover",
      state: "implemented",
      detail:
        "Four paired-prefill transitions retain roster-indexed caches, KV reservations, outputs, provider inputs, queue submission, retry custody, and fail-closed catalog admission.",
    },
    {
      commit: "c593009",
      title: "Admit finite speculative serving shapes",
      state: "implemented",
      detail:
        "Production serving and same-shape rearm cover S1/K4, S8/K4, S1/K8, and S1/K16 with histories bound to plan, epoch, and ordered live roster.",
    },
  ],
  evidence: {
    summary:
      "Ferric treats implementation, authenticated proof release, hardware observation, Qwen correctness, performance, independent validation, and M1 qualification as separate authorities.",
    gates: [
      ["Roadmap requirements", "33", "open"],
      ["Assurance properties", "17", "open"],
      ["Hardware cases", "58", "open"],
      ["Performance intakes", "36", "open"],
      ["Independent validations", "44", "open"],
      ["Receipt gates", "7", "open"],
    ],
    legend: [
      ["implemented", "Source path exists and passes its scoped checks."],
      ["observed", "A bound run produced the stated result; authority remains limited."],
      ["qualified", "All gates for the named, exact scope accepted the artifact."],
    ],
  },
  boundaries: {
    ferric: [
      "Model, tokenizer, and weight admission",
      "Qwen target and draft graphs and execution plans",
      "Ferric-owned inference kernels and their concrete protected policy",
      "Worker ledger, repository lineage, host-descriptor lineage, and rollback admission",
      "Scheduling, paged KV, speculation, generated runner, and M1 qualification",
      "Qualified source 1d666dbc binds all seven K1-K7 families to protected Worker V3 custody and landed through be307d52 in m1/bundle-admission",
      "PR #24 qualified canonical 100644/100755 source-closure modes at 3898bf40 and landed them through 49a539f2",
      "Authenticated seven-owner collection and roster, current artifacts, generated marker contracts, authority-safe custody fixtures, protected policy, end-to-end Qwen, hardware and performance evidence, the production receipt, and M1 remain Ferric work",
    ],
    fe2o3: [
      "Reusable compiler APIs, semantic artifact identities, and protected compilation",
      "Durable subject-bound compiler receipt acquisition and recovered V2 carriage",
      "Generic receipt-complete sealed verification and promotion boundary",
      "Typed KFD allocations, USERPTR/AQL queues, fixed-batch publication, completion, and dispatch",
      "Current integrated compiler/runtime head 42639ecc7f2f377ab57e5e884c36133a126f230e",
      "PR #236 convergence work is active but unlanded and is not part of Ferric's supported baseline",
      "Deployment identities and Ferric-specific inference authority are intentionally not defined upstream",
    ],
  },
});
