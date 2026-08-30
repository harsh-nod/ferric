window.FERRIC_PROJECT = Object.freeze({
  updated: "2026-08-30",
  repository: "https://github.com/harsh-nod/ferric",
  fe2o3Repository: "https://github.com/harsh-nod/fe2o3",
  milestone: {
    name: "M1",
    label: "Qwen3 speculative inference on one gfx942",
    state: "integration",
    summary:
      "Ferric retains authenticated Qwen inputs, exact execution plans, bounded engine state, and protected Worker V3 source adapters for all seven M1 kernel families, but cannot run Qwen through the production path. Its qualified compiler/runtime integration baseline remains fe2o3 42639ecc; Ferric main also carries narrower checked-in pins for the SwiGLU device fixture and M0 property binder. Current fe2o3 main is c77d4e72 and is not pinned or qualified by Ferric. Workspace-policy PR #248 is open at 1094cd72 with spawn and coordinator classification plus a test-only bounded readiness-EOF stabilization; exact mi300x focused policy, 100-repeat readiness, supervisor, and affected matrices pass while duplicated Generic core checks remain in progress. The O_PATH normalizer is sound, but same-UID tests do not prove production Stage C: the distinct capability-empty service UID cannot traverse the client-owned 0700 artifact tree. Checked-arithmetic head ae188e0 addresses the three reported blockers in source but remains frozen, unreviewed, and without remote qualification. The production seccomp supervisor remains replayable by a same-UID caller and still lacks a root-held, server-consumed one-use launch authorization. Aggregate verification, Stage C and D, multi-root emission, authenticated KFD execution, end-to-end Qwen, and all 33 M1 requirements remain open.",
  },
  envelope: [
    ["Target", "Qwen3-8B"],
    ["Draft", "Qwen3-0.6B"],
    ["Device", "1 x gfx942"],
    ["Precision", "BF16 / FP32 accumulate"],
    ["Context", "up to 8K tokens"],
    ["Concurrency", "up to 32 sequences"],
    ["Ferric integration", "main be94f69e; Pages PR #29 at ee9e6f34 is open and green; engine integration through f5b11d6; source closure qualified at 3898bf40"],
    ["Ferric fe2o3 pins", "qualified integration baseline 42639ecc; checked-in SwiGLU fixture 2c7668d2; M0 property binder a6fa86b5"],
    ["Current fe2o3 upstream", "main c77d4e72e0b95e0511d6b71cf2a3e0c52b80e2ea; Ferric has not pinned or qualified it"],
    ["Unmerged integration", "workspace-policy and readiness-test PR #248 at 1094cd72 with Generic core in progress; O_PATH PR #247 at 1ea87a3f with a distinct-UID namespace blocker; frozen checked arithmetic ae188e0 awaiting review and remote qualification; aggregate PR #244 at db096436; Stage C and seccomp supervisor authorization; Stage D owner rework; and lower-MIR 2c3140d7"],
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
        "fe2o3 main c77d4e72e0b95e0511d6b71cf2a3e0c52b80e2ea retains shared root-to-distinct-UID protected-service spawn, pidfd custody, provisioned supervisor inputs, deployment-readiness publication, and the coordinator path that launches the protected compiler supervisor; the tip adds deployment-wiring documentation. This is reusable deployment infrastructure. It does not authorize the Cargo application supervisor, admit Ferric or Qwen artifacts, or grant GPU execution authority.",
    },
    {
      label: "Checked-arithmetic repair candidate",
      state: "integration",
      detail:
        "Frozen head ae188e0 addresses the reported recursive range-reconstruction budget, recursive constant-pass budget, and induction-alias escape-authority blockers in source. That source has not completed independent review or remote qualification, so it remains unmerged and carries no compiler or M1 authority. Aggregate PR #244 also remains open on an older head.",
    },
    {
      label: "Ordered Stage C artifact-set handoff",
      state: "integration",
      detail:
        "Stage C remains unmerged. The O_PATH normalizer itself is sound, but its current same-UID tests do not exercise the intended production identity: a distinct capability-empty service UID cannot traverse the client-owned 0700 artifact tree. Production needs an identity-bound, service-authorized namespace view and a distinct-UID vertical test. Separately, seccomp candidate 6827646a still lets a direct same-UID caller enter the hidden application supervisor with caller-created sockets, slots, and sealed images because the supervisor consumes no root-held one-use launch authorization. Error-path kill/reap and parent-death custody also remain incomplete.",
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
        "Ferric cannot run Qwen through the production path. The qualified integration baseline remains fe2o3 42639ecc, while current main is unpinned. Closure needs a qualified workspace-policy line, an identity-bound service-authorized artifact namespace with a distinct-UID O_PATH vertical, reviewed and remotely qualified checked arithmetic, current-main aggregate verification, nonforgeable root-backed supervisor launch authorization with complete lifecycle custody, qualified Stage C and Stage D ownership, whole-module current KIR replay for lower-MIR, generic multi-root emission, authenticated KFD, current artifacts and rosters, a runner, and hardware and performance qualification.",
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
          "Independently review frozen checked-arithmetic head ae188e0, which addresses bounded recursive range reconstruction, the recursive constant pass, and induction-alias escape authority in source, then run its complete remote qualification before any promotion. Aggregate Worker V3 PR #244 is green at db096436 but remains based on an older mainline; restack and requalify it on current main before merge.",
      },
      {
        name: "Ordered Stage C and joint Stage D",
        detail:
          "Complete hosted qualification for PR #248, then restack O_PATH PR #247 on current main. Preserve its sound descriptor normalization while replacing the client-owned 0700 artifact-tree assumption with an identity-bound, service-authorized namespace view and prove the full distinct-UID vertical. Bind the production application supervisor to a root-managed, server-consumed one-use authorization derived from the admitted protected release; same-UID process checks and caller-created sockets are not authority. Add complete PDEATHSIG and kill/reap custody before qualifying Stage C, and replace Stage D's rejected raw tuple with an opaque provenance-carrying owner.",
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
      commit: "c77d4e72e0b95e0511d6b71cf2a3e0c52b80e2ea",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Wire the root-launched protected compiler supervisor",
      state: "implemented",
      detail:
        "Current fe2o3 main is exactly c77d4e72e0b95e0511d6b71cf2a3e0c52b80e2ea. Commit ed393473 wires the coordinator to launch the protected compiler supervisor with retained deployment inputs; the tip records the deployment wiring. Ferric has not pinned or qualified this line. Compiler-supervisor deployment readiness is not a root-held application-supervisor launch permit and does not create a Qwen execution path.",
    },
    {
      commit: "1094cd72a7eb31471858a8c57d07f1691e528c65",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Classify protected service runtimes and stabilize readiness EOF test",
      state: "integration",
      detail:
        "PR #248 is open on current main with exactly two commits: a80bb475 classifies both fe2o3-protected-service-spawn and fe2o3-compiler-execution-coordinator as host runtimes, and 1094cd72 makes readiness-EOF observation bounded and test-only. Exact mi300x focused policy, 100-repeat readiness, supervisor, and affected matrices pass. Hosted parity, Verus, source, and rustc shards pass while both duplicated Generic core jobs remain in progress; the line is not yet qualified or merged.",
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
        "PR #247 exact head 1ea87a3f8f35e71f1f0d9836fab55a5fa056c7ae soundly normalizes imported directory descriptors but diverges from current main and still carries the earlier workspace-classification CI failure. Production audit found the deeper Stage C blocker: its same-UID tests use a client-owned 0700 artifact tree that the intended distinct capability-empty service UID cannot traverse. It must take #248, gain an identity-bound service-authorized namespace view, and pass a distinct-UID vertical before restacking and requalification.",
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
      "Ferric main remains be94f69e and Pages PR #29 remains open at green head ee9e6f34",
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
      "Current fe2o3 main c77d4e72e0b95e0511d6b71cf2a3e0c52b80e2ea carries coordinator launch of the protected compiler supervisor plus deployment-wiring documentation; it does not close application-supervisor authorization, Stage C artifact visibility, multi-root, runtime admission, Ferric, or Qwen gates",
      "Ferric has not pinned or qualified current fe2o3 main",
      "The gfx942 EXEC-control layer assigns no opcode semantics and proves neither machine reconvergence, empty masks, termination, nor launch authority",
      "Checked arithmetic fixes selected production kernel compiles to one -Coverflow-checks=on; its induction certificates do not authorize removal of LLVM overflow guards",
      "Exact single-use u32 guard snapshots may be retained in the induction certificate, but the certificate remains inert without authenticated source-to-KIR-to-LLVM refinement",
      "The landed KIR-to-LLVM replay independently reconstructs target KIR and byte-identical LLVM from canonical neutral KIR; it is exact deterministic derivation evidence, not formal semantic preservation or machine-code authority",
      "Workspace-policy PR #248 exact head 1094cd72a7eb31471858a8c57d07f1691e528c65 classifies protected-service-spawn and compiler-execution-coordinator and adds only bounded readiness-EOF test stabilization; exact mi300x focused policy, 100-repeat readiness, supervisor, and affected matrices pass while Generic core remains in progress",
      "O_PATH PR #247 exact head 1ea87a3f8f35e71f1f0d9836fab55a5fa056c7ae has a sound normalizer, but same-UID tests do not prove production access through the client-owned 0700 artifact tree; Stage C needs an identity-bound service-authorized namespace view and a distinct-UID vertical",
      "Checked-arithmetic head ae188e0 addresses recursive range reconstruction, the recursive constant pass, and induction-alias escape authority in source but remains frozen, unreviewed, remotely untested, and unqualified",
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
