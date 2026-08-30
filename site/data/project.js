window.FERRIC_PROJECT = Object.freeze({
  updated: "2026-08-30",
  repository: "https://github.com/harsh-nod/ferric",
  fe2o3Repository: "https://github.com/harsh-nod/fe2o3",
  milestone: {
    name: "M1",
    label: "Qwen3 speculative inference on one gfx942",
    state: "integration",
    summary:
      "Ferric retains authenticated Qwen inputs, exact execution plans, bounded engine state, and protected Worker V3 source adapters for all seven M1 kernel families, but cannot run Qwen through the production path. Its qualified compiler/runtime integration baseline remains fe2o3 42639ecc; current fe2o3 main is e5992869 and is not pinned or qualified by Ferric. Mixed-width KIR PR #249 is merged through 35771d28. The checked-compiler candidate now passes the shared-borrow regression and six callable-summary tests, then the real gfx942 matrix fails closed at FE2O3-TENSOR-LAYOUT-002. Stage C inventory mutation evidence passes its four exact tests and the complete artifact-transaction package, while PR #250 remains unmerged and is being restacked. The production seccomp supervisor still lacks a root-held, server-consumed one-use launch authorization. Aggregate verification, Stage C and D, multi-root emission, authenticated KFD execution, end-to-end Qwen, and all 33 M1 requirements remain open.",
  },
  envelope: [
    ["Target", "Qwen3-8B"],
    ["Draft", "Qwen3-0.6B"],
    ["Device", "1 x gfx942"],
    ["Precision", "BF16 / FP32 accumulate"],
    ["Context", "up to 8K tokens"],
    ["Concurrency", "up to 32 sequences"],
    ["Ferric integration", "main 8863177c; Pages PR #29 is merged; engine integration through f5b11d6; source closure qualified at 3898bf40"],
    ["Ferric fe2o3 pins", "qualified integration baseline 42639ecc; checked-in SwiGLU fixture 2c7668d2; M0 property binder a6fa86b5"],
    ["Current fe2o3 upstream", "main e59928697297309678e2c1c2ebc280722dfc8eb2 retains mixed-width KIR merge 35771d28; Ferric has not pinned or qualified it"],
    ["Unmerged integration", "checked compiler 68f17a2d blocked at FE2O3-TENSOR-LAYOUT-002; Stage C inventory PR #250 being restacked; aggregate PR #244; seccomp supervisor authorization; Stage D owner rework; and lower-MIR 2c3140d7"],
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
        "Ferric's pinned fe2o3 baseline 42639ecc carries a durable subject-bound receipt from the compiler backend through inherited FD195 into Cargo, then through recovered V2 admission. Its generic sealed verifier enforces a promotion boundary that requires receipt-complete evidence.",
    },
    {
      label: "KFD runtime foundations",
      state: "implemented",
      detail:
        "Ferric's pinned fe2o3 baseline 42639ecc integrates typed memory, queue ownership, fixed-batch publication, completion readback, and dispatch foundations. These reusable primitives do not by themselves make the Ferric Qwen path runnable.",
    },
    {
      label: "Seven Worker V3 source adapters",
      state: "implemented",
      detail:
        "Qualified source 1d666dbc, landed through be307d52, makes GEMM, RMSNorm, RoPE/KV, prefill, paged decode, SwiGLU, and logits accept only matching compiler-produced, move-only Worker V3 owners before strict structural inspection. Tests do not execute Worker V3 or establish current HSACO existence.",
    },
    {
      label: "Generic Worker V3 roster admission",
      state: "implemented",
      detail:
        "fe2o3 PR #242 merged exact, fail-closed Worker V3 descriptor-roster admission at b3cd6534. This is reusable upstream host infrastructure; Ferric has not pinned it, populated its seven production authorities, or used it to run Qwen.",
    },
    {
      label: "Multi-root ranked semantic rosters",
      state: "implemented",
      detail:
        "fe2o3 PR #243 merged at b167991f after source parity, Generic, and scoped mi300x affected, focused, and ROCm qualification checks passed. It retains verified per-root semantic ownership and ranked projections; aggregate verification, ordered Stage C handoff, and multi-root KIR/LLVM/HSACO remain open.",
    },
    {
      label: "Bounded gfx942 EXEC-control facts",
      state: "implemented",
      detail:
        "fe2o3 main 41e542278 derives bounded structural facts for exact EXECZ/EXECNZ sites in authenticated gfx942 traces: CFG successors, a unique two-half EXEC reaching definition, an immediate post-dominator candidate, scalar mask operands, and a matching saved-mask OR site when structurally present. It assigns no opcode semantics and proves neither an empty mask, hardware reconvergence, termination, nor launch authority.",
    },
    {
      label: "Checked production arithmetic policy",
      state: "implemented",
      detail:
        "fe2o3 main ee93e692 canonicalizes each selected production kernel compile to exactly one -Coverflow-checks=on and rejects disabled or conflicting settings before the in-process rustc driver starts; the driver independently requires that exact canonical form. The exact invocation retains the flag, while semantic induction certificates remain inert and cannot authorize removal of LLVM overflow guards without an independent source-to-KIR-to-LLVM refinement join.",
    },
    {
      label: "Exact rustc induction snapshots",
      state: "implemented",
      detail:
        "fe2o3 main retains d8904ad8's exact single-use u32 header-snapshot admission and the merged test-only repair that distinguishes an accepted header snapshot from a rejected stale preheader alias. The certificate retains both the compared value and snapshot site, rejects extra uses and structural hazards, and preserves the LLVM overflow guard.",
    },
    {
      label: "Canonical scalar correspondence evidence",
      state: "implemented",
      detail:
        "fe2o3 a8cfbb32 retains the complete induction report as canonical authority-free evidence. e16ec53a joins that report to bounded block correspondence plus exact MIR statement, terminator, synthetic-operation, and parameter-binding spans, and 5b232a17 publishes the V4 correspondence through the scalar production semantic-lineage receipt. This does not complete Stage C, Stage D, generic multi-root lowering, LLVM/HSACO emission, or runtime authority.",
    },
    {
      label: "Decoded canonical KIR V8 custody",
      state: "implemented",
      detail:
        "fe2o3 main can return one verified canonical KIR V8 byte owner together with the same decoded, verified Module, avoiding a second full decode for inspecting consumers. This narrow custody API does not establish Stage C, multi-root lowering, LLVM/HSACO emission, runtime authority, or Ferric admission.",
    },
    {
      label: "Exact deterministic KIR-to-LLVM replay",
      state: "implemented",
      detail:
        "fe2o3 main eca3bcaa755b9ad09af5fb93a801b2dd99986a51 includes the workspace and lock closure from merged PR #246. Duplicated 40-minute Generic core runs and every remaining gate passed. The retained scalar replay proves exact deterministic derivation by the reviewed implementation, not formal semantic preservation, LLVM-to-machine refinement, HSACO existence, or launch authority.",
    },
    {
      label: "Exact LLVM-to-HSACO stage custody",
      state: "implemented",
      detail:
        "fe2o3 commit 12c5a161e033ff4cd0adaaea428b3022d8501f65 landed 1af36a2's bounded evidence for the exact linked and optimized LLVM modules, generated object, ordered native-link inputs, LLD invocation, and resulting HSACO; current main retains it. This authority-free custody is scalar infrastructure and does not complete generic multi-root emission, runtime admission, a Ferric pin, or Qwen execution.",
    },
    {
      label: "Root-owned protected service custody",
      state: "implemented",
      detail:
        "fe2o3 main e59928697297309678e2c1c2ebc280722dfc8eb2 retains shared root-to-distinct-UID protected-service spawn, pidfd custody, provisioned supervisor inputs, deployment-readiness publication, and the protected compiler coordinator. Ferric has not pinned or qualified this exact main. This reusable deployment infrastructure does not authorize the Cargo application supervisor, admit Ferric or Qwen artifacts, or grant GPU execution authority.",
    },
    {
      label: "Target-neutral mixed-width KIR shifts",
      state: "implemented",
      detail:
        "fe2o3 PR #249 merged qualified candidate 6c1f7248 through main commit 35771d28059dabc4ac7fe8f80be69cbdc9a43356. Target-neutral KIR accepts integer shift operands of different widths, while target lowering remains responsible for rejecting unsupported combinations. This upstream compiler repair does not qualify Ferric's pin, emit the seven-family roster, run Qwen, or close M1.",
    },
    {
      label: "Checked-arithmetic repair candidate",
      state: "integration",
      detail:
        "Candidate 68f17a2d retains the checked-arithmetic and immutable-custody repairs, passes the shared-borrow regression and six exact callable-summary tests on mi300x, and advances the real gfx942 general matrix beyond those gates. The matrix now fails closed at FE2O3-TENSOR-LAYOUT-002. The candidate remains unmerged and is neither a complete compiler qualification nor Ferric, runtime, Qwen, or M1 authority.",
    },
    {
      label: "Ordered Stage C artifact-set handoff",
      state: "integration",
      detail:
        "Stage C remains unmerged. PR #250's descriptor-only core models a fixed nine-FD inventory of already-open publication objects with fail-closed roster, identity, currentness, descriptor, lock, and teardown checks. Its four exact metadata, ready-record, module-digest, and attempt-registry mutation tests pass on mi300x, as does the complete artifact-transaction package; PR #250 is being restacked. The core is not yet integrated into the authenticated SCM_RIGHTS client/service vertical or qualified under distinct UIDs. Separately, seccomp candidate 6827646a still lets a direct same-UID caller enter the hidden application supervisor because it consumes no root-held one-use launch authorization. Error-path kill/reap and parent-death custody also remain incomplete.",
    },
    {
      label: "Stage D owner and provenance boundary",
      state: "integration",
      detail:
        "Review rejected the old raw-tuple API. Stage D now awaits an opaque-owner and provenance-preserving redesign before joint artifact-set authentication can be implemented or qualified.",
    },
    {
      label: "Lower-MIR refinement candidate",
      state: "integration",
      detail:
        "Candidate 2c3140d7 is audit-only. Its lower-MIR work must be reimplemented under whole-module current KIR replay before it can carry refinement or compiler authority.",
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
        "Ferric cannot run Qwen through the production path. The qualified integration baseline remains fe2o3 42639ecc, while current main e5992869 is unpinned. Mixed-width shifts are merged upstream, but closure still needs a fully green checked-compiler line beyond FE2O3-TENSOR-LAYOUT-002, landed and integrated Stage C descriptor custody with a distinct-UID vertical, current-main aggregate verification, nonforgeable root-backed supervisor launch authorization with complete lifecycle custody, qualified Stage D ownership, whole-module current KIR replay for lower-MIR, generic multi-root emission, authenticated KFD, current artifacts and rosters, a runner, and hardware and performance qualification.",
    },
    {
      label: "M1 qualification",
      state: "open",
      detail:
        "All 33 roadmap requirements remain open. No complete M1 evidence index or qualification receipt exists. The qualified source closure, scoped proof release, historical kernel artifact, and qualification-only numerics do not close the production receipt or M1.",
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
        name: "Seven protected Worker V3 source adapters",
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
      {
        name: "Generic multi-root semantic ownership",
        detail:
          "fe2o3 admits exact Worker V3 descriptor rosters and retains verified per-root semantic owners with deterministic ranked projections. These merged generic stages do not yet aggregate verification or produce multi-root KIR, LLVM, HSACO, or authenticated KFD execution.",
      },
      {
        name: "Bounded gfx942 EXEC-control analysis",
        detail:
          "An inert upstream analysis joins exact EXEC-conditioned branches in authenticated machine traces to structural CFG, reaching-definition, post-dominator, mask-operand, and possible restore-site facts. It establishes no machine semantics, compiler refinement, hardware reconvergence, empty-mask proof, termination, or launch authority.",
      },
      {
        name: "Checked production arithmetic",
        detail:
          "The selected production rustc wrapper inserts or canonicalizes one enabled overflow-check policy, rejects disabled or conflicting settings, and the driver requires exactly one canonical flag. The pipeline reports semantic u32 induction certificates for examined checked additions, but those certificates grant no authority to remove the corresponding LLVM overflow guard.",
      },
      {
        name: "Exact induction-snapshot admission",
        detail:
          "The semantic analysis accepts either the induction local itself or one uniquely defined, single-use, hazard-free u32 temporary copied from it before the guard. Current main retains positive exact-header-snapshot coverage and negative stale-preheader-alias evidence. The resulting certificate remains inert and does not authorize compiler transforms or runtime execution.",
      },
      {
        name: "Lossless scalar MIR-to-KIR evidence custody",
        detail:
          "Canonical evidence retains the exact induction report with bounded MIR-to-KIR block and operation spans, then the scalar production compiler publishes that V4 correspondence through semantic lineage and redecodes it before final handoff. This is authority-free evidence custody, not completed multi-root lowering, source-to-LLVM refinement, code-object emission, or launch authority.",
      },
      {
        name: "Exact scalar KIR-to-LLVM replay",
        detail:
          "Current fe2o3 reconstructs target-bound KIR from exact neutral KIR V8/V9, reruns deterministic gfx942/gfx950 lowering and canonical layout binding, and requires byte equality with retained pre-descriptor LLVM. PR #246 merged and current main is green. Formal semantic preservation, LLVM-to-machine refinement, object authority, runtime authority, and Ferric adoption remain open.",
      },
      {
        name: "Audit-only lower-MIR exploration",
        detail:
          "Candidate 2c3140d7 is useful only as an audit. It must be reimplemented beneath whole-module current KIR replay; it carries no refinement, emission, runtime, or Qwen authority.",
      },
    ],
    roadmap: [
      {
        name: "Checked arithmetic and aggregate verification",
        detail:
          "Continue candidate 68f17a2d beyond the real gfx942 matrix's FE2O3-TENSOR-LAYOUT-002 rejection, then complete broader qualification. Mixed-width shift PR #249 is merged through 35771d28. Aggregate Worker V3 PR #244 remains based on an older mainline and needs current-main restacking and requalification before merge.",
      },
      {
        name: "Ordered Stage C and joint Stage D",
        detail:
          "Restack and land PR #250's remotely tested descriptor-only nine-FD inventory, then integrate it with the authenticated SCM_RIGHTS client/service path and prove the full distinct-UID vertical. Bind the production application supervisor to a root-managed, server-consumed one-use authorization derived from the admitted protected release; same-UID process checks and caller-created sockets are not authority. Add complete PDEATHSIG and kill/reap custody before qualifying Stage C, and replace Stage D's rejected raw tuple with an opaque provenance-carrying owner.",
      },
      {
        name: "Multi-root compiler emission",
        detail:
          "Reimplement lower-MIR candidate 2c3140d7 beneath whole-module current KIR replay, then lower the admitted roster into canonical multi-root KIR, LLVM, and HSACO while preserving per-root identity, limits, and evidence joins.",
      },
      {
        name: "Authenticated fixed-batch KFD",
        detail:
          "Bind the exact emitted roster to fixed-batch publication, completion, and failure handling under authenticated runtime custody.",
      },
      {
        name: "Ferric pin and authority rosters",
        detail:
          "Pin and qualify the completed fe2o3 line, then join all seven live owners to Ferric's protected policy, Worker ledger, lineage, rollback verifier, deployment identities, and keys.",
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
      commit: "68f17a2d",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Advance checked compilation to the tensor-layout gate",
      state: "integration",
      detail:
        "The checked-compiler candidate now admits immutable shared observation without weakening mutable or raw-address escape rejection. Its shared-borrow regression and six exact callable-summary tests pass on mi300x, and the real gfx942 general matrix advances to a fail-closed FE2O3-TENSOR-LAYOUT-002 rejection. The candidate remains unmerged and carries no complete compiler, runtime, Ferric, Qwen, or M1 authority.",
    },
    {
      commit: "fcf0a42261cf8035fe6bc9e83a115ac5775b2aad",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Qualify Stage C inventory mutation evidence",
      state: "integration",
      detail:
        "Four exact metadata, ready-record, module-digest, and attempt-registry mutation tests pass on mi300x; focused log SHA-256 fba363dd1c81da58f71833ec941b481a34a03d9b5b1848aa5695bbe867be4d60 and complete artifact-transaction package log SHA-256 023e0873aea555034b699c06ec84e785bd1030c56e308ec8179c04b80bf75a37 retain the evidence. PR #250 is open and being restacked, so the inventory is neither merged Stage C nor authenticated SCM_RIGHTS or distinct-UID closure.",
    },
    {
      commit: "35771d28059dabc4ac7fe8f80be69cbdc9a43356",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Merge target-neutral mixed-width KIR shifts",
      state: "implemented",
      detail:
        "fe2o3 PR #249 merged candidate 6c1f7248 through main commit 35771d28 after focused and package-wide remote qualification passed. The repair preserves target-neutral mixed-width integer-shift admission while target-specific lowering fails closed on unsupported combinations. Ferric has not pinned or qualified the resulting upstream line, and this compiler repair does not run Qwen or close M1.",
    },
    {
      commit: "6c42a0a2",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Land the superseding runtime policy and EOF fixes upstream",
      state: "implemented",
      detail:
        "Upstream commits 50fdce4e and 890369ed classify fe2o3-protected-service-spawn and fe2o3-compiler-execution-coordinator as host runtimes; 6c42a0a2 adds bounded, test-only readiness-EOF observation. PR #248 was closed as superseded because these upstream changes made its rebased local diff empty. This records source integration only; whole-line Generic qualification remains in progress.",
    },
    {
      commit: "6827646a2a5a541d16b35fb1b937a20a9681ec4c",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Prototype production Cargo application capture",
      state: "integration",
      detail:
        "The unmerged production seccomp branch moves the Cargo-selected application outside the filtered lineage and binds its sealed image, argv, cwd, and one-use invocation admission through a dedicated supervisor. Review found a blocking replay path: the hidden supervisor accepts a same-UID direct caller's independently created channel, slot, and sealed application without a root-held one-use launch permit. Several frontend failure paths also lack terminal kill-and-reap custody.",
    },
    {
      commit: "1ea87a3f8f35e71f1f0d9836fab55a5fa056c7ae",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Refresh the O_PATH prerequisite on current main",
      state: "integration",
      detail:
        "PR #247 exact head 1ea87a3f8f35e71f1f0d9836fab55a5fa056c7ae soundly normalizes imported directory descriptors but diverges from current main. PR #250 now carries the fixed nine-FD publication inventory and has passed its scoped lock and mutation evidence on mi300x. It remains unmerged while being restacked and has not been joined to the authenticated SCM_RIGHTS vertical or qualified under distinct UIDs.",
    },
    {
      commit: "1af36a2b6deb6638e784197791b8aea1d72e8e37",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Retain exact LLVM-to-HSACO stage custody",
      state: "implemented",
      detail:
        "The upstream worker retains bounded identities for the linked and optimized LLVM modules, generated object, ordered native-link inputs, LLD invocation, and output HSACO. The evidence explicitly grants no compiler or runtime authority and does not close generic multi-root emission, Ferric adoption, or Qwen execution.",
    },
    {
      commit: "eca3bcaa755b9ad09af5fb93a801b2dd99986a51",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Merge dependency closure and qualify current main",
      state: "implemented",
      detail:
        "fe2o3 PR #246 merged. Duplicated 40-minute Generic core runs and every remaining gate passed on current main. This closes the workspace dependency and lock failure; it does not authorize Ferric, Qwen, HSACO, or runtime execution.",
    },
    {
      commit: "6ede4ea514153cd417f3c76dda603269e4b44754",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Require replayed KIR-to-LLVM custody",
      state: "implemented",
      detail:
        "fe2o3's scalar production compiler carries an independently validated replay owner through semantic lineage. The verifier reconstructs target-bound KIR, reruns deterministic AMDGPU lowering and exact layout binding, and requires byte equality with retained pre-descriptor LLVM. PR #246 subsequently restored green current-main workspace closure. Formal semantic refinement, LLVM-to-machine proof, HSACO, and runtime authority remain open.",
    },
    {
      commit: "75b226789a06aa8bf884377f49a2974bc755f34a",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Independently replay exact production KIR to LLVM",
      state: "implemented",
      detail:
        "The generic verifier strictly decodes the bounded canonical replay record, validates exact neutral KIR input, reconstructs target KIR and LLVM with the shared production transforms, and returns only validated replay custody after every identity and byte comparison succeeds.",
    },
    {
      commit: "2e25d2761676ee81976e50e2e2bb02a91b893c39",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Add canonical KIR-to-LLVM replay evidence",
      state: "implemented",
      detail:
        "A bounded canonical V1 record binds the KIR V8/V9 version and exact neutral and target KIR identities, target profile, kernel ID, and retained pre-descriptor LLVM. Construction and validation replay the shared deterministic transforms; the record explicitly grants no proof or execution authority.",
    },
    {
      commit: "326cd503c64df5d6a5ef24839115ad37c19bfb50",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Retain a decoded KIR V8 module with custody",
      state: "implemented",
      detail:
        "fe2o3 can now return the owner of exact verified canonical KIR V8 bytes together with the same decoded, verified Module, so an inspecting consumer does not need a second full decode. The API adds no Stage C, multi-root lowering, LLVM/HSACO, runtime, launch, Qwen, or M1 authority.",
    },
    {
      commit: "3e40abf75679bb6646cb7bb50a781a2dfdb584de",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Merge exact-snapshot integration repair",
      state: "implemented",
      detail:
        "fe2o3 merged the test-only repair that treats a unique single-use header copy as the admitted exact induction snapshot and retains a separate stale preheader alias as fail-closed negative evidence. The repair adds no compiler-transform, runtime, launch, Qwen, or M1 authority.",
    },
    {
      commit: "5b232a175ca119de2a376d6cba94acaf6725584d",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Publish lossless correspondence evidence",
      state: "implemented",
      detail:
        "The scalar production semantic-lineage path now publishes the V4 canonical MIR-to-KIR correspondence, including its nested induction evidence, and redecodes it before final handoff while checking the semantic MIR and neutral KIR identities. Multi-root Stage C, Stage D, lowering, LLVM/HSACO, and runtime authority remain open.",
    },
    {
      commit: "e16ec53a8495989f2276d8bb6b20d963529a67a4",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Preserve lossless MIR-to-KIR spans",
      state: "implemented",
      detail:
        "fe2o3 adds bounded canonical V4 custody for block correspondence, MIR statement and terminator spans, synthetic-operation spans, parameter bindings, and the nested canonical induction report. The evidence is explicitly authority-free and does not establish source-to-LLVM or machine-code refinement.",
    },
    {
      commit: "a8cfbb326f8ced1b8370e48f2abc8230b978af9c",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Retain canonical induction reports",
      state: "implemented",
      detail:
        "fe2o3 canonically encodes and independently decodes the complete bounded u32 induction report, including exact identities, sites, work counts, and optional header-snapshot custody. The evidence grants no compiler or runtime authority.",
    },
    {
      commit: "d8904ad8b9ce5ca35b08f0b3bff3dddbecceb6cc",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Admit exact rustc induction snapshots",
      state: "implemented",
      detail:
        "fe2o3 retains an exact single-use header snapshot when rustc compares that copied u32 value against the loop bound. The identity-bound scalar gfx942 certificate records the induction value and snapshot site, rejects extra-use and hostile shapes, and preserves the LLVM overflow guard. It grants no compiler-transform, runtime, launch, Qwen, or M1 authority; aggregate verification remains open in PR #244.",
    },
    {
      commit: "ee93e692ac0e7c2ea69fafadbc07b2f6c5d4a84d",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Enforce checked production arithmetic",
      state: "implemented",
      detail:
        "fe2o3 canonicalizes selected production kernel compiles to exactly one -Coverflow-checks=on, rejects disabled or conflicting forms before rustc enters the in-process driver, requires the canonical flag at the driver boundary, retains it in the exact protected invocation, and observes checked u32 addition lowering through llvm.uadd.with.overflow.i32. Semantic induction certificates remain inert; this commit grants no runtime, guard-removal, launch, Qwen, or M1 authority.",
    },
    {
      commit: "41e5422783e5f45e14e0835c108ec0e51630c8b9",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Derive bounded gfx942 EXEC-control facts",
      state: "implemented",
      detail:
        "fe2o3 adds a fail-closed, bounded structural analysis for exact EXECZ/EXECNZ branch sites in authenticated gfx942 instruction/CFG traces. It records branch successors, unique EXEC reaching definitions, immediate post-dominator candidates, canonical mask operands, and structurally matching saved-mask OR sites; it does not establish compiler or machine semantics, hardware reconvergence, empty masks, termination, launch authority, Qwen execution, or M1.",
    },
    {
      commit: "b167991f03811594ec2a42be745e3b133cb3a6b8",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Retain verified multi-root ranked semantic rosters",
      state: "implemented",
      detail:
        "fe2o3 PR #243 merged qualified head 06bfae35e68c3f7d69a4bf836e3faa6f5e61e97a after source parity, Generic, and scoped mi300x affected, focused, and ROCm checks passed. The merge establishes generic per-root semantic ownership only; aggregate verification, Stage C, multi-root KIR/LLVM/HSACO, authenticated KFD, Ferric adoption, Qwen, and M1 remain open.",
    },
    {
      commit: "b3cd6534f13a3463fc86eb01306aa72aec6b2c75",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Admit exact Worker V3 descriptor rosters",
      state: "implemented",
      detail:
        "fe2o3 PR #242 merged a generic, fail-closed roster admission boundary for exact Worker V3 descriptors. Ferric remains pinned to 42639ecc and has not supplied or qualified its seven production authority rosters.",
    },
    {
      commit: "bb04ceb05d68169a2a54bebd96e7943bdbdda156",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Classify the external host-runtime anchor",
      state: "implemented",
      detail:
        "fe2o3 PR #240 made the host-runtime service boundary explicit for generic integration. It does not deploy Ferric identities, credentials, policy, or runtime authority.",
    },
    {
      commit: "895d5e4b2b3ee584dfa1fc154197670dd610d132",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Repair standalone workspace locks",
      state: "implemented",
      detail:
        "fe2o3 PR #239 refreshed standalone locks after AMD target ownership so the generic workspaces qualify consistently. This is workspace repair, not Qwen or M1 evidence.",
    },
    {
      commit: "027157dd9810cd6acb3a25ff1b613f3514463c33",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Land upstream compiler/runtime convergence",
      state: "implemented",
      detail:
        "fe2o3 PR #236 merged qualified head f11317db7afc6a37b8d03ed4b796cfe13d97a261 (tree 06dd1e5b2e8f65c103952d36bb921ce0baf9ac03) with signed-V4 proof inputs, a sealed protected-verifier adapter, compiler-generated dispatch bindings, and reusable direct-KFD runtime foundations. Ferric has not pinned or qualified this upstream line. Current seven-family artifacts, Ferric's protected policy and backend, Worker ledger and rollback authority, distinct-UID deployment and keys, an authenticated seven-owner collector and roster, generated marker contracts, authority-safe custody fixtures, a runner, end-to-end Qwen, current-source hardware and performance evidence, independent validation, the production receipt, and M1 remain open.",
    },
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
      "Ferric main is 8863177c, including merged Pages PR #29",
      "Ferric's qualified compiler/runtime integration baseline remains fe2o3 42639ecc; main's checked-in SwiGLU fixture pins 2c7668d2 and the M0 binder pins a6fa86b5",
      "Ferric has no qualified pin to current fe2o3 main and cannot run Qwen through the production path",
      "Current artifacts, protected policy and deployment, end-to-end Qwen, hardware and performance evidence, the production receipt, and M1 remain Ferric work",
      "All 33 M1 roadmap requirements remain open",
    ],
    fe2o3: [
      "Reusable compiler APIs, semantic artifact identities, and protected compilation",
      "Durable subject-bound compiler receipt acquisition and recovered V2 carriage",
      "Generic receipt-complete sealed verification and promotion boundary",
      "Typed KFD allocations, USERPTR/AQL queues, fixed-batch publication, completion, and dispatch",
      "Ferric's supported compiler/runtime baseline remains 42639ecc7f2f377ab57e5e884c36133a126f230e until a newer fe2o3 revision is pinned and qualified",
      "fe2o3 PR #246 merged at eca3bcaa after duplicated 40-minute Generic core runs and all gates passed",
      "Current fe2o3 main e59928697297309678e2c1c2ebc280722dfc8eb2 retains the protected-service foundation and mixed-width KIR merge 35771d28; Ferric has not qualified this exact line",
      "Ferric has not pinned or qualified current fe2o3 main",
      "The gfx942 EXEC-control layer assigns no opcode semantics and proves neither machine reconvergence, empty masks, termination, nor launch authority",
      "Checked arithmetic fixes selected production kernel compiles to one -Coverflow-checks=on; its induction certificates do not authorize removal of LLVM overflow guards",
      "Exact single-use u32 guard snapshots may be retained in the induction certificate, but the certificate remains inert without authenticated source-to-KIR-to-LLVM refinement",
      "The landed KIR-to-LLVM replay independently reconstructs target KIR and byte-identical LLVM from canonical neutral KIR; it is exact deterministic derivation evidence, not formal semantic preservation or machine-code authority",
      "PR #248 was closed as superseded after upstream landed the same protected-service classifications and bounded readiness-EOF fix, leaving its rebased local diff empty",
      "PR #250's descriptor-only fixed nine-FD Stage C inventory passes its four exact mutation tests and the complete artifact-transaction package on mi300x, but is being restacked and is not landed, integrated with authenticated SCM_RIGHTS, or distinct-UID qualified",
      "Checked-compiler candidate 68f17a2d passes the shared-borrow regression and six exact callable-summary tests on mi300x, but the real gfx942 general matrix now fails closed at FE2O3-TENSOR-LAYOUT-002",
      "Mixed-width shift PR #249 is merged through fe2o3 main commit 35771d28 after focused and package-wide remote qualification",
      "Aggregate verification PR #244 is green at db096436607539e663450fc5aab317f39d0afd06 but remains open on an older base and needs current-main restacking and requalification",
      "Production seccomp candidate 6827646a implements exact Cargo application capture outside the filtered lineage, but direct same-UID hidden-supervisor replay and missing root-authorizer custody block production admission",
      "The missing supervisor authority must be root-backed, server-retained, one-use, and consumed directly by the supervisor before FD204 and Stage C; parent/current-image equality, a caller-created challenge, or another sealed public memfd is not sufficient",
      "Stage C remains unmerged; feature-bypass evidence and the current seccomp candidate are not production closure",
      "Stage D's raw-tuple API was rejected and awaits opaque-owner provenance rework",
      "Lower-MIR candidate 2c3140d7 is audit-only and must be reimplemented under whole-module current KIR replay",
      "Generic multi-root LLVM/HSACO emission and authenticated fixed-batch KFD remain unmerged upstream integration work",
      "Deployment identities and Ferric-specific inference authority are intentionally not defined upstream",
    ],
  },
});
