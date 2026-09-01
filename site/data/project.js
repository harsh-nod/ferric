window.FERRIC_PROJECT = Object.freeze({
  updated: "2026-08-31",
  repository: "https://github.com/harsh-nod/ferric",
  fe2o3Repository: "https://github.com/harsh-nod/fe2o3",
  milestone: {
    name: "M1",
    label: "Qwen3 speculative inference on one gfx942",
    state: "integration",
    summary:
      "The Ferric status site is live, but M1 is incomplete and all 33 M1 roadmap gates remain open. Qualified integration head 62a2ef4 retains all seven K1-K7 attributed device-source packages and completes the authenticated repeat-round same-native host lifecycle: combined parked-roster scheduling, reserve, prepare, rebind, submit, wait, recycle, readback, completion, page release, retry, and teardown. Exact fe2o3 head e98f280f remains pinned. Strict ferric-engine clippy, 457 unit tests, and 132 doctests passed on the mi300x qualification host; 5 hardware tests remained ignored. Authenticated hardware behavior is not qualified, production artifacts are not populated, and Ferric cannot yet run Qwen.",
  },
  envelope: [
    ["Target", "Qwen3-8B"],
    ["Draft", "Qwen3-0.6B"],
    ["Device", "1 x gfx942"],
    ["Precision", "BF16 / FP32 accumulate"],
    ["Context", "up to 8K tokens"],
    ["Concurrency", "up to 32 sequences"],
    ["Ferric main", "ec1a2e03a2923e7a6431ebc26aa30d04884f8a69; GitHub Pages status is live"],
    ["Host adapter surface", "all seven K1-K7 family adapters and all 12 host symbols/ABI inspectors exist"],
    ["Landed attributed device surface", "K6 SwiGLU only: one of seven required packages and one of 12 required device roots; no current artifact or run"],
    ["M1 integration head", "62a2ef46af9225a72209248ae2df50cd6ef05595: all seven K1-K7 attributed source packages plus the host-qualified authenticated repeat-round same-native lifecycle"],
    ["Qualified dependency pin", "74bc611e9d29c74756dcdd102ebecd9c29a581fc pins every Ferric fe2o3 dependency and all seven standalone device-package locks to exact head e98f280f"],
    ["Exact fe2o3 head", "e98f280fb9aeb53b7248793b0a759952d8f58106 retains the authenticated service-queue runtime and exposes authenticated roster source identities for Ferric's exact intake checks"],
    ["Qualified development work", "exact 11-file snapshot admission at 8e7fbbd and snapshot-only operational intake at edfaefa, including a source-path-absent 22-plan MI300X proof, are integrated into the current unpublished M1 integration lineage, not main"],
    ["Production compiler extraction", "exact fe2o3 e98f280f is pinned and host-qualified with Ferric, but the current seven-program-family artifact set has not been extracted or admitted"],
    ["Authenticated retained runtime", "the 62a2ef4 lineage preserves authenticated Worker V3 witness, operation-plan, program, queue, and Ferric batch custody across first submit and repeated same-native rounds"],
    ["Authenticated completed-step KV release", "ce14b99 returns quiescent retired pages to their exact pools in deterministic draft-then-target order while retaining authenticated retry, success, teardown, and failure owners without raw queue conversion"],
    ["Authenticated repeat-round lifecycle", "62a2ef4 joins combined parked-roster scheduling to KV reserve, workspace prepare, same-native rebind, resubmission, completion readback, settlement, page release, retry, and teardown"],
    ["Scoped qualification", "on mi300x, strict ferric-engine clippy with warnings denied passed; 457 unit tests passed with 5 hardware tests ignored; 132 doctests passed; authenticated hardware behavior is not qualified"],
    [
      "Historical protected artifact",
      "SwiGLU semantic fe2ce532...e9569e8f / HSACO 57ecb86b...fc6afa7; qualification-only and not supplied by the PR #32 landing",
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
        "fe2o3 exposes public discovery and recovery for durable RecoveredWorkerV3LoadEnvelopeV2 owners, and its generic roster types can retain all seven compiler results. That generic recovery capability does not by itself grant Ferric load or launch authority.",
    },
    {
      label: "KFD runtime foundations",
      state: "implemented",
      detail:
        "Ferric source integrates typed memory, queue ownership, fixed-batch publication, completion readback, and dispatch foundations. These reusable primitives do not by themselves make the Ferric Qwen path runnable.",
    },
    {
      label: "Seven Worker V3 source adapters",
      state: "implemented",
      detail:
        "Qualified source 1d666dbc, landed through be307d52, provides all seven K1-K7 family host adapters and all 12 host symbols and ABI inspectors, requiring matching compiler-produced, move-only Worker V3 owners before strict structural inspection. K6 SwiGLU is the only attributed package and root landed on main, through PR #32 at da25cef3. Its tests establish source and adapter behavior only; they do not execute Worker V3 or establish a current HSACO.",
    },
    {
      label: "Generic Worker V3 roster admission",
      state: "implemented",
      detail:
        "fe2o3 PR #242 merged exact, fail-closed Worker V3 descriptor-roster admission at b3cd6534. This is reusable upstream host infrastructure; Ferric has not populated its production authorities or used it to run Qwen.",
    },
    {
      label: "Multi-root ranked semantic rosters",
      state: "implemented",
      detail:
        "fe2o3 PR #243 merged verified per-root semantic ownership and ranked projections at b167991f, and PR #254 merged exact aggregate Worker V3 roster custody at bf093149. PR #20 later completed canonical exact multi-root KIR, LLVM, and HSACO lowering through merge d32d8a11. Ferric has not supplied or run its production roster.",
    },
    {
      label: "Aggregate Worker V3 roster custody",
      state: "implemented",
      detail:
        "fe2o3 PR #254 merged at bf093149 after exact independent replay reconstructed the move-only finalizer roster. Typed entry borrows remain non-escaping and carry no load or launch authority. Ferric has not populated its current production authorities.",
    },
    {
      label: "Seven-artifact recovery to runtime join",
      state: "integration",
      detail:
        "Ferric head 74bc611 pins the complete workspace to fe2o3 e98f280f, whose authenticated roster exposes exact source identities and whose retained service-queue runtime is consumed by the current integration lineage. Production extraction has not populated the seven current Worker V3 artifact owners, so this qualified host integration is not load, dispatch, hardware, or Qwen execution authority.",
    },
    {
      label: "Authenticated Ferric kernel custody",
      state: "integration",
      detail:
        "Qualified integration head 62a2ef4 requires authenticated Worker V3 owners for the seven-family, 12-program runtime set and retains their witness, source identity, operation plan, program, queue, and Ferric batch custody through repeated same-native rounds. The exact scope passed strict ferric-engine clippy, 457 unit tests with 5 hardware tests ignored, and 132 doctests on mi300x. Authenticated hardware behavior remains unqualified, and no production artifact population, Qwen run, performance result, or M1 authority follows from those host checks.",
    },
    {
      label: "Authenticated retained readback, settlement, and KV release",
      state: "integration",
      detail:
        "The 62a2ef4 lineage carries authenticated completion through the shared Engine/device-KV settlement core, returns every quiescent retired page to its exact pool after complete preflight, and makes the released owner available to the next round. Retry rejection preserves custody; success and teardown retain queue, program, observation, cache, parked-roster, terminal-lineage, and round-history ownership without exposing a raw queue. Authenticated hardware behavior remains open.",
    },
    {
      label: "Authenticated repeat-round same-native lifecycle",
      state: "integration",
      detail:
        "Qualified head 62a2ef4 connects combined active and parked roster scheduling to KV reservation, workspace preparation, effectful rebind, submit, wait, recycle, readback, completion, page release, retry, and teardown for the retained same-native queue. It preserves round history and terminal lineage across target-only and speculative K4/K8/K16 rounds while keeping invalid transitions fail-closed. This is host lifecycle qualification only; it has no authenticated hardware result.",
    },
    {
      label: "Long-lived KFD queue lifecycle",
      state: "implemented",
      detail:
        "fe2o3 PR #255 merged at 90f16d26 with canonical AQL control layout, queue-fault typestate, allocation-slot reclamation, generation-preserving rebind, rollover range reissue, and exact replacement preflight. These generic runtime capabilities are not Ferric artifact admission or Qwen execution authority.",
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
        "fe2o3 a8cfbb32 retains the complete induction report as canonical authority-free evidence. e16ec53a joins that report to bounded block correspondence plus exact MIR statement, terminator, synthetic-operation, and parameter-binding spans, and 5b232a17 publishes the V4 correspondence through the scalar production semantic-lineage receipt. That scalar evidence alone does not establish Stage C, Stage D, runtime authority, or Ferric admission.",
    },
    {
      label: "Decoded canonical KIR V8 custody",
      state: "implemented",
      detail:
        "fe2o3 main can return one verified canonical KIR V8 byte owner together with the same decoded, verified Module, avoiding a second full decode for inspecting consumers. This narrow custody API does not establish Stage C, runtime authority, Ferric artifact admission, or Qwen execution.",
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
        "fe2o3 commit 12c5a161e033ff4cd0adaaea428b3022d8501f65 landed 1af36a2's bounded evidence for the exact linked and optimized LLVM modules, generated object, ordered native-link inputs, LLD invocation, and resulting HSACO; current main retains it. PR #20 extends exact publication across multi-root modules. Neither upstream capability supplies Ferric artifacts, runtime admission, or Qwen execution.",
    },
    {
      label: "Compiler-generated write-only KFD arguments",
      state: "implemented",
      detail:
        "fe2o3 PR #258 merged at d955209099c7b434dfceb69e1152d948dab76b22 with all 20 exact-head checks green. It adds compiler-known store-only disjoint slices, WriteOnly KIR and descriptor custody, generated KFD write arguments, seeded staging, and completion-gated host writeback. This generic core does not integrate a Ferric package, admit Qwen artifacts, or grant Qwen execution authority.",
    },
    {
      label: "Target-neutral mixed-width KIR shifts",
      state: "implemented",
      detail:
        "fe2o3 PR #249 merged qualified candidate 6c1f7248 through main commit 35771d28059dabc4ac7fe8f80be69cbdc9a43356. Target-neutral KIR accepts integer shift operands of different widths, while target lowering remains responsible for rejecting unsupported combinations. This upstream compiler repair does not integrate Ferric, emit the seven-family roster, run Qwen, or close M1.",
    },
    {
      label: "Exact 11-file Qwen snapshot admission",
      state: "integration",
      detail:
        "Development commit 8e7fbbd implements and qualifies a self-contained, exact 11-file Qwen snapshot with authenticated snapshot-owned metadata and strict fail-closed roster validation after source removal. It is integrated into the current unpublished M1 integration lineage, not main, and grants no production Qwen or M1 authority.",
    },
    {
      label: "Snapshot-only operational intake",
      state: "integration",
      detail:
        "Development commit edfaefa implements snapshot-only operational intake and has a source-path-absent 22-plan MI300X proof. It is qualified for that development scope and integrated into the current unpublished M1 integration lineage, not main; it is not a production intake or M1 authority.",
    },
    {
      label: "Exact multi-root production lowering",
      state: "implemented",
      detail:
        "fe2o3 PR #20 merged as d32d8a11715eed8cd64493f345d169ac094370ff after all exact-head checks passed. It lowers ordered two- and three-root production inputs into one canonical KIR, LLVM, and HSACO module while preserving per-root ownership, descriptors, geometry, effects, lineage, and replay. This generic compiler support does not create Ferric artifacts or authorize Qwen execution.",
    },
    {
      label: "Reviewed external source trust",
      state: "implemented",
      detail:
        "fe2o3 PR #21 merged as 1739669000eea19614fdf772127f4d5d705530c0 after all exact-head checks passed. It admits reviewed write-only external source closure without weakening compiler-owned access and lineage checks. Ferric still needs exact package integration and current artifacts.",
    },
    {
      label: "Non-null empty KFD transport",
      state: "integration",
      detail:
        "The corrected non-null empty-slice transport is retained in exact pinned fe2o3 head e98f280f and participates in Ferric's qualified host lifecycle stack. No current artifact dispatch or GPU execution result has exercised that transport through the production Qwen path.",
    },
    {
      label: "Generic volatile-load production support",
      state: "integration",
      detail:
        "Volatile-load support is retained in exact pinned fe2o3 head e98f280f. Ferric's host qualification covers integration of the pinned workspace, not production extraction, current HSACO artifacts, GPU launch, or numerical behavior.",
    },
    {
      label: "K1 GEMM and embedding device source",
      state: "integration",
      detail:
        "K1 is integrated on the M1 integration branch through merge 61f645b10def0e2a3d5069038cc87f7061bf9d26. Its managed source/profile/reference tests do not establish production extraction, KIR, LLVM, HSACO, authenticated KFD dispatch, hardware numerics, or performance authority.",
    },
    {
      label: "K2 RMSNorm device source",
      state: "integration",
      detail:
        "K2 source qualification is complete and the package is retained at integration head 62a2ef4 through qualified source commit adc019684d415e45a9543c07e66bc3a17d20edad. It caps rows at 65,536, uses lane-zero ascending serial FP32 accumulation with bounded loops and bounded volatile reads, and passed managed formatting, check, and 17 debug plus 17 release tests on MI300X. Production extraction, current artifacts, authenticated KFD dispatch, hardware results, and performance remain open.",
    },
    {
      label: "K3 RoPE and KV device source",
      state: "integration",
      detail:
        "K3 source and disjoint-ownership proof work is complete and retained at integration head 62a2ef4. Its paged-KV root uses a fixed 16,384-physical-page grid. Production extraction, runtime artifact population, authenticated KFD dispatch, hardware numerics, and performance remain open.",
    },
    {
      label: "K4 prefill device source",
      state: "integration",
      detail:
        "The attributed K4 prefill package at 7e3339057a08203002f629717a9ba9f06b79c5f8 is integrated through merge b8675d9. Its scoped source checks do not establish a production extraction, current artifact, authenticated KFD dispatch, hardware numerical result, or performance authority.",
    },
    {
      label: "K5 paged-decode device source",
      state: "integration",
      detail:
        "The attributed K5 paged-decode package at 863e82ece35b94f5161866b8a0e3dc4f50d0a816 is integrated through merge a37e0f7. Its scoped source checks do not establish a production extraction, current artifact, authenticated KFD dispatch, hardware numerical result, or performance authority.",
    },
    {
      label: "K7 logits device source",
      state: "integration",
      detail:
        "The attributed K7 logits package at 5d821ee5b13aab01fa5b2c553143e0e7de1c20bc is integrated through merge 9a367e7. Source integration does not establish a production extraction, current artifact, authenticated KFD dispatch, hardware numerical result, or performance authority.",
    },
    {
      label: "Staged lifecycle integration",
      state: "integration",
      detail:
        "Integration head 62a2ef4 is qualified for its exact authenticated repeat-round same-native host lifecycle against fe2o3 e98f280f, including combined parked-roster scheduling, KV reserve, workspace prepare, rebind, submit, wait, recycle, readback, completion, page release, retry, and teardown. It is not merged to Ferric main, and authenticated hardware, Qwen correctness, performance, and M1 evidence remain open.",
    },
    {
      label: "Ordered Stage C artifact-set handoff",
      state: "integration",
      detail:
        "PR #250 merged the descriptor-only fixed nine-FD inventory at ce5de889 after its exact metadata, ready-record, module-digest, and attempt-registry mutation tests passed on mi300x. The core is not yet integrated into the authenticated SCM_RIGHTS client/service vertical or qualified under distinct UIDs, so Stage C remains open. Separately, seccomp candidate 6827646a still lets a direct same-UID caller enter the hidden application supervisor because it consumes no root-held one-use launch authorization. Error-path kill/reap and parent-death custody also remain incomplete.",
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
        "Ferric cannot yet run Qwen through the production path. Head 62a2ef4 has all seven K1-K7 sources and a host-qualified authenticated repeat-round same-native lifecycle, but it has no populated current artifact roster or authenticated hardware result. Production extraction, exact graph execution, numerics, performance, independent qualification, and the remaining serving path remain open.",
    },
    {
      label: "M1 qualification",
      state: "open",
      detail:
        "All 33 roadmap requirements remain open. Qwen numerical, hardware, and performance evidence, independent validation, formal closure, and the complete M1 evidence index and qualification receipt remain open. The qualified development snapshots, source closure, scoped proof release, historical kernel artifact, and qualification-only numerics do not close the production receipt or M1.",
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
        name: "Authenticated retained first-generation runtime",
        detail:
          "Exact Worker V3 source identities, program witnesses, operation plans, loaded programs, queue typestates, and Ferric batch custody remain joined across authenticated creation, submit, bounded wait, recycle, observation, detach, and terminal release. The qualified scope is host lifecycle behavior, not evidence of a populated production roster or GPU execution.",
      },
      {
        name: "Authenticated readback and completed-step settlement",
        detail:
          "Capture-free compact completion can be observed and semantically joined without exposing raw queue custody, settled through the shared Engine/device-KV completion core, and followed by deterministic authenticated retired-page return. Typed outcomes retain retry, success, poison, and teardown ownership for use by the next retained round. This host-qualified path has no authenticated hardware result.",
      },
      {
        name: "Authenticated repeat-round same-native lifecycle",
        detail:
          "Qualified source schedules a combined active and parked roster, reserves KV pages, prepares workspace images, rebinds and resubmits the retained native queue, then waits, recycles, reads back, completes, releases pages, retries rejected preflight, or tears down with ownership preserved. It covers target-only and speculative K4/K8/K16 repeated rounds and has no authenticated hardware result.",
      },
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
        name: "Exact generic multi-root production modules",
        detail:
          "fe2o3 admits exact Worker V3 descriptor rosters, retains verified per-root semantic owners with deterministic ranked projections, reconstructs aggregate roster custody, and now lowers exact ordered rosters into one canonical KIR, LLVM, and HSACO module. PR #20 merged this generic compiler path; it does not supply Ferric artifacts or authenticated Ferric KFD execution.",
      },
      {
        name: "Long-lived KFD queue lifecycle",
        detail:
          "Merged upstream runtime support retains queue and memory generation across rebind, reissues shifted ranges across rollover, reclaims released allocation slots, and fails closed around queue faults and replacement preflight. Ferric has not used it to execute Qwen.",
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
          "Canonical evidence retains the exact induction report with bounded MIR-to-KIR block and operation spans, then the production compiler publishes that V4 correspondence through semantic lineage and redecodes it before final handoff. This remains authority-free evidence custody, not formal source-to-LLVM refinement or launch authority.",
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
        name: "Integrate snapshot and lifecycle branches",
        detail:
          "The exact snapshot intake and authenticated repeat-round same-native lifecycle are integrated and host-qualified at 62a2ef4, not merged to main. Populate current production artifacts and qualify the lifecycle on authenticated hardware before publishing a production continuing-serving claim.",
      },
      {
        name: "Complete KFD edge handling",
        detail:
          "Ferric pins exact fe2o3 head e98f280f and now host-qualifies authenticated retained repeat rounds through retry and teardown. Complete rollover, diagnostic evidence joins, authenticated hardware qualification, and production serving custody without exposing lower or raw queue ownership.",
      },
      {
        name: "Ordered Stage C and joint Stage D",
        detail:
          "Integrate merged PR #250's descriptor-only nine-FD inventory with the authenticated SCM_RIGHTS client/service path and prove the full distinct-UID vertical. Bind the production application supervisor to a root-managed, server-consumed one-use authorization derived from the admitted protected release; same-UID process checks and caller-created sockets are not authority. Add complete PDEATHSIG and kill/reap custody before qualifying Stage C, and replace Stage D's rejected raw tuple with an opaque provenance-carrying owner.",
      },
      {
        name: "Extract the integrated device packages",
        detail:
          "All seven K1-K7 source packages, including complete K3 source with its fixed 16,384-physical-page grid, are retained at 62a2ef4. Every family still needs exact production KIR, LLVM, HSACO, descriptor, authenticated launch, and hostile evidence before entering the Ferric roster.",
      },
      {
        name: "Authenticated fixed-batch KFD",
        detail:
          "Authenticated fixed-batch publication and repeated same-native scheduling, reserve, prepare, rebind, submit, wait, recycle, readback, completion, retired-page return, retry, and typed teardown exist. Bind exact extracted artifacts and finish rollover, diagnostic evidence, hardware qualification, and production serving custody.",
      },
      {
        name: "Ferric compiler integration and authority rosters",
        detail:
          "The workspace is pinned and host-qualified against fe2o3 e98f280f. Produce the current compiler artifacts, populate the authenticated live roster, then join those owners to Ferric's protected policy, Worker ledger, lineage, rollback verifier, deployment identities, and keys.",
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
      title: "Authenticated repeat-round same-native lifecycle",
      state: "qualified",
      source: "62a2ef46af9225a72209248ae2df50cd6ef05595",
      result: "PASS: strict ferric-engine clippy; 457 unit tests; 132 doctests",
      detail:
        "On the mi300x qualification host, the exact 62a2ef4 tree with dependency pin 74bc611 and fe2o3 e98f280f passed strict ferric-engine clippy with warnings denied, 457 unit tests with 5 hardware tests ignored, and 132 doctests. This qualifies the authenticated combined parked-roster scheduling, reserve, prepare, same-native rebind, submit, wait, recycle, readback, completion, page release, retry, and teardown source path as host lifecycle code only. Authenticated hardware behavior remains unqualified; this is not a numerical, Qwen, performance, production-receipt, or M1 result.",
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
      commit: "62a2ef46af9225a72209248ae2df50cd6ef05595",
      title: "Complete authenticated retained queue rounds",
      state: "qualified",
      detail:
        "This head joins combined active and parked roster scheduling to KV reserve, workspace prepare, same-native queue rebind, submit, wait, recycle, readback, completion, page release, retry, and teardown while retaining terminal lineage and round history. Strict ferric-engine clippy, 457 unit tests with 5 hardware tests ignored, and 132 doctests passed on mi300x. This is scoped host lifecycle evidence, not authenticated hardware, Qwen, performance, production-receipt, or M1 authority.",
    },
    {
      commit: "669da4b15a25ff5221a56fbc9ff67567bd7bee4a",
      title: "Stage authenticated retained queue rebind",
      state: "qualified",
      detail:
        "This head adds a crate-private effectful same-native-queue rebind core for all five fixed-batch shapes. It validates retained authenticated program families and operations, fresh workspace images, device and KV identities, unchanged recipes, and diagnostic-capture restrictions before replacement and packet rebuild. Strict clippy, 457 library tests with 5 ignored, and 124 doctests passed on mi300x. The public released-step scheduling/resubmission bridge and authenticated hardware behavior remain unqualified.",
    },
    {
      commit: "ce14b99553c1004e431c78f5b0e8a2074e71e591",
      title: "Release authenticated completed-step KV pages",
      state: "qualified",
      detail:
        "This parent returns each quiescent retired page to its exact KV pool after whole-roster preflight, advances generations once in deterministic draft-then-target order, and retains authenticated retry, success, teardown, and failure custody without raw queue conversion. It does not schedule or resubmit the released step and establishes no authenticated hardware result.",
    },
    {
      commit: "63491e524ae6abb196d1274ef4e6a5ac62711d61",
      title: "Settle authenticated completed steps",
      state: "qualified",
      detail:
        "This integration head carries authenticated Worker V3 custody through retained packet lowering, first-generation queue lifecycle, compact observation, capture-free semantic readback, and the transactional Engine/device-KV completion core. Rejection, success, poison, and teardown retain authenticated owners without raw queue conversion. Strict clippy, 452 library tests with 5 ignored, and 119 doctests passed on mi300x; no GPU execution or Qwen result is claimed.",
    },
    {
      commit: "74bc611e9d29c74756dcdd102ebecd9c29a581fc",
      title: "Pin the retained runtime to exact fe2o3",
      state: "qualified",
      detail:
        "This parent pins all Ferric fe2o3 dependencies and all seven standalone device-package lockfiles to e98f280f. The exact pin participated in the 63491e5 host qualification; it does not populate current artifacts or establish runtime hardware, numerical, Qwen, performance, or M1 authority.",
    },
    {
      commit: "e98f280fb9aeb53b7248793b0a759952d8f58106",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Expose authenticated roster source identities",
      state: "integration",
      detail:
        "This exact fe2o3 head exposes source identities from authenticated Worker V3 roster entries while retaining the authenticated service-queue runtime used by Ferric. Ferric's dependency pin and host tests validate consumption of that API; they do not make the roster contain production artifacts or prove GPU execution.",
    },
    {
      commit: "adc019684d415e45a9543c07e66bc3a17d20edad",
      title: "Qualify and integrate K2 RMSNorm source",
      state: "integration",
      detail:
        "K2 source qualification is complete and the exact package is retained at integration head 62a2ef4. Its scoped MI300X checks establish source behavior only; production extraction, artifacts, authenticated KFD dispatch, hardware numerics, performance, Qwen execution, and M1 remain open.",
    },
    {
      commit: "5d821ee5b13aab01fa5b2c553143e0e7de1c20bc",
      title: "Integrate the K7 attributed logits source",
      state: "integration",
      detail:
        "The K7 attributed logits package is integrated through merge 9a367e7. It has no production extraction, current artifact, authenticated KFD dispatch, hardware numerical result, performance result, Qwen run, or M1 authority.",
    },
    {
      commit: "863e82ece35b94f5161866b8a0e3dc4f50d0a816",
      title: "Retain the K5 attributed paged-decode source",
      state: "integration",
      detail:
        "The K5 attributed source package is integrated through merge a37e0f7. It has no extracted production artifact, authenticated KFD dispatch, hardware numerical result, performance result, Qwen run, or M1 authority.",
    },
    {
      commit: "7e3339057a08203002f629717a9ba9f06b79c5f8",
      title: "Retain the K4 attributed prefill source",
      state: "integration",
      detail:
        "The K4 attributed source package is integrated through merge b8675d9. It has no extracted production artifact, authenticated KFD dispatch, hardware numerical result, performance result, Qwen run, or M1 authority.",
    },
    {
      commit: "61f645b10def0e2a3d5069038cc87f7061bf9d26",
      title: "Integrate K1 on the M1 integration branch",
      state: "integration",
      detail:
        "Current M1 integration head 62a2ef46af9225a72209248ae2df50cd6ef05595 retains the attributed Qwen3 GEMM and embedding merge. This is branch integration and source-level evidence, not an origin/main landing, extracted production artifact, authenticated KFD dispatch result, hardware result, performance result, Qwen run, or M1 authority.",
    },
    {
      commit: "da25cef31032e126cbad3aa21923da07a8f9b900",
      title: "Land the K6 attributed device-source package",
      state: "implemented",
      detail:
        "Ferric PR #32 merged the attributed Qwen3 SwiGLU package and standalone source checks into main. The package retains exact source, write-only output, ABI, launch, and host adapter contracts. It establishes no current compiler extraction, KIR, LLVM, HSACO, KFD dispatch, hardware result, performance result, Qwen run, or M1 authority.",
    },
    {
      commit: "d32d8a11715eed8cd64493f345d169ac094370ff",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Merge exact multi-root production lowering",
      state: "implemented",
      detail:
        "fe2o3 PR #20 merged ordered multi-root production lowering after every exact-head CI check passed. Real two- and three-root fixtures proved one canonical KIR, LLVM, and HSACO module with all expected entrypoints and per-root receipt, descriptor, geometry, effect, replay, and lineage custody. Ferric has not yet produced its roster through this path.",
    },
    {
      commit: "1739669000eea19614fdf772127f4d5d705530c0",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Merge reviewed external source trust",
      state: "implemented",
      detail:
        "fe2o3 PR #21 merged reviewed write-only external source closure after every exact-head CI check passed. This generic trust path preserves compiler-owned access, semantic, and lineage validation; it does not admit a Ferric package or artifact by itself.",
    },
    {
      commit: "75768486109b0d2beac06d04f8767064e32aaa35",
      title: "Begin the K2 RMSNorm attributed source",
      state: "integration",
      detail:
        "This commit began the standalone K2 package. Qualified revision adc01968 replaced the XOR tree with authoritative lane-zero ascending serial FP32 accumulation, added bounded row and volatile-read behavior, passed managed formatting, check, and 17 debug plus 17 release tests on MI300X, and is retained at integration head 62a2ef4. Production extraction, artifacts, authenticated KFD dispatch, hardware results, and performance remain open.",
    },
    {
      commit: "d955209099c7b434dfceb69e1152d948dab76b22",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Merge compiler-generated write-only KFD arguments",
      state: "implemented",
      detail:
        "fe2o3 PR #258 merged generic compiler-known WriteOnly slices, GuardedStore KIR V9 custody, exact descriptor and generated adapter validation, seeded KFD staging, and successful-completion-only host writeback. All 20 exact-head checks passed. Ferric adoption, artifact admission, Qwen execution, and M1 authority remain open.",
    },
    {
      commit: "edfaefa743c6393c224148c3d09fa2e892eb9252",
      title: "Qualify snapshot-only operational intake",
      state: "integration",
      detail:
        "The development branch accepts operational input only from the authenticated snapshot and has a source-path-absent 22-plan MI300X proof. It is implemented and qualified for that branch scope and integrated into the current unpublished M1 integration lineage, not main; it grants no production Qwen or M1 authority.",
    },
    {
      commit: "8e7fbbd8eb53196268b4bfdd6160f9c679dda661",
      title: "Qualify self-contained Qwen snapshot admission",
      state: "integration",
      detail:
        "The development branch authenticates an exact 11-file Qwen snapshot with snapshot-owned metadata and strict roster, length, type, and mutation checks after the source path is removed. It is implemented and qualified for that branch scope and integrated into the current unpublished M1 integration lineage, not main.",
    },
    {
      commit: "90f16d261ce90c4ece6e0e2d57ffaecc58fb1b4f",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Merge long-lived KFD queue lifecycle hardening",
      state: "implemented",
      detail:
        "fe2o3 PR #255 merged generic AQL control layout, queue-fault typestate, allocation-slot reclamation, generation-preserving rebind, shifted-range rollover reissue, and exact lifecycle replacement preflight. Remote runtime, KFD, service, doctest, and Ferric engine checks covered 910 passing tests with 5 intentional ignores. Ferric has not run Qwen with this support.",
    },
    {
      commit: "bf093149c3232171cbe5f03ca5d05aa66b7ca0db",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Merge aggregate Worker V3 roster custody",
      state: "implemented",
      detail:
        "fe2o3 PR #254 merged exact independently replayed aggregate-roster reconstruction after host, vertical, hsaco-finalize, UI, doctest, and focused strict-clippy qualification passed on mi300x. The move-only finalizer custody grants no load or launch authority, and Ferric has not populated it.",
    },
    {
      commit: "ce5de8891973af844c18a2c76362438b9d0779f5",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Merge descriptor-only Stage C inventory custody",
      state: "implemented",
      detail:
        "fe2o3 PR #250 merged the fixed nine-FD inventory for already-open publication objects with fail-closed roster, identity, currentness, descriptor, lock, and teardown checks. This is the descriptor-only core, not the authenticated SCM_RIGHTS distinct-UID vertical or complete Stage C authority.",
    },
    {
      commit: "35771d28059dabc4ac7fe8f80be69cbdc9a43356",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Merge target-neutral mixed-width KIR shifts",
      state: "implemented",
      detail:
        "fe2o3 PR #249 merged candidate 6c1f7248 through main commit 35771d28 after focused and package-wide remote qualification passed. The repair preserves target-neutral mixed-width integer-shift admission while target-specific lowering fails closed on unsupported combinations. This compiler repair does not integrate Ferric, run Qwen, or close M1.",
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
        "PR #247 exact head 1ea87a3f8f35e71f1f0d9836fab55a5fa056c7ae soundly normalizes imported directory descriptors but diverges from current main. PR #250 subsequently merged the fixed nine-FD publication inventory at ce5de889. That inventory has not been joined to the authenticated SCM_RIGHTS vertical or qualified under distinct UIDs.",
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
        "fe2o3 retains an exact single-use header snapshot when rustc compares that copied u32 value against the loop bound. The identity-bound scalar gfx942 certificate records the induction value and snapshot site, rejects extra-use and hostile shapes, and preserves the LLVM overflow guard. It grants no compiler-transform, runtime, launch, Qwen, or M1 authority; PR #254 later merged aggregate roster custody without widening that authority.",
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
        "fe2o3 PR #243 merged qualified head 06bfae35e68c3f7d69a4bf836e3faa6f5e61e97a after source parity, Generic, and scoped mi300x affected, focused, and ROCm checks passed. PR #254 later merged aggregate roster custody, and PR #20 completed exact multi-root KIR/LLVM/HSACO lowering. Stage C vertical integration, authenticated Ferric KFD, Ferric adoption, Qwen, and M1 remain open.",
    },
    {
      commit: "b3cd6534f13a3463fc86eb01306aa72aec6b2c75",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Admit exact Worker V3 descriptor rosters",
      state: "implemented",
      detail:
        "fe2o3 PR #242 merged a generic, fail-closed roster admission boundary for exact Worker V3 descriptors. Ferric has not supplied or qualified its production authority rosters.",
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
        "fe2o3 PR #236 merged qualified head f11317db7afc6a37b8d03ed4b796cfe13d97a261 (tree 06dd1e5b2e8f65c103952d36bb921ce0baf9ac03) with signed-V4 proof inputs, a sealed protected-verifier adapter, compiler-generated dispatch bindings, and reusable direct-KFD runtime foundations. Current artifacts, Ferric's protected policy and backend, Worker ledger and rollback authority, distinct-UID deployment and keys, an authenticated collector and roster, generated marker contracts, authority-safe custody fixtures, a runner, end-to-end Qwen, current-source hardware and performance evidence, independent validation, the production receipt, and M1 remain open.",
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
      title: "Integrate compiler receipt, verifier, and KFD foundations",
      state: "implemented",
      detail:
        "That upstream revision combined durable subject-bound compiler receipts, recovered V2 carriage, receipt-complete sealed verification, Worker ledger acquisition, typed KFD memory and queues, fixed-batch completion, and dispatch. Deployment identity and Ferric inference policy remained outside the upstream boundary.",
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
      "This Pages refresh is based on Ferric main ec1a2e03a2923e7a6431ebc26aa30d04884f8a69; the published GitHub Pages site is live",
      "Exact 11-file snapshot admission at 8e7fbbd and snapshot-only operational intake at edfaefa are qualified development work integrated into the current unpublished M1 integration lineage, not main",
      "Integration head 62a2ef4 is qualified for the authenticated repeat-round same-native host lifecycle through retry and teardown, but is not merged to main and is not a production inference baseline",
      "All seven K1-K7 family host adapters and all 12 host symbols and ABI inspectors exist",
      "K6 SwiGLU is the only attributed device package and root landed on main, through PR #32; no current artifact or run is claimed",
      "M1 integration head 62a2ef46af9225a72209248ae2df50cd6ef05595 contains all seven K1-K7 attributed source packages, including complete K3 source with its fixed 16,384-physical-page grid",
      "K2 source qualification is complete at adc01968; its latest-compiler extraction, artifacts, authenticated KFD, hardware, and performance gaps remain",
      "K3 RoPE/KV source and ownership-proof work is integrated; exact-compiler qualification, extraction, runtime admission, hardware numerics, and performance remain open",
      "K4 prefill commit 7e333905, K5 paged-decode commit 863e82ec, and K7 logits commit 5d821ee5 are integrated source packages; none has production artifact, authenticated KFD, hardware, or performance authority",
      "Ferric head 62a2ef4 retains exact fe2o3 e98f280f and qualifies the authenticated repeat-round same-native host lifecycle; the result grants no populated artifact, authenticated hardware, Qwen, performance, or M1 authority",
      "Authenticated combined parked-roster scheduling, reserve, prepare, rebind, submit, wait, recycle, readback, completion, page release, retry, and teardown retain ownership without raw queue conversion; rollover, diagnostic serving, and end-to-end inference remain open",
      "Authenticated hardware behavior is not yet qualified",
      "Ferric cannot yet run Qwen through the production path",
      "Current artifacts, protected policy and deployment, end-to-end Qwen, numerical, hardware, and performance evidence, independent validation, formal closure, the production receipt, and M1 remain Ferric work",
      "All 33 M1 roadmap requirements remain open",
    ],
    fe2o3: [
      "Reusable compiler APIs, semantic artifact identities, and protected compilation",
      "Durable subject-bound compiler receipt acquisition and recovered V2 carriage",
      "Generic receipt-complete sealed verification and promotion boundary",
      "Typed KFD allocations, USERPTR/AQL queues, fixed-batch publication, completion, and dispatch",
      "Exact pinned head e98f280fb9aeb53b7248793b0a759952d8f58106 retains authenticated service-queue custody and exposes authenticated roster source identities used by Ferric's qualified host integration",
      "fe2o3 PR #246 merged at eca3bcaa after duplicated 40-minute Generic core runs and all gates passed",
      "fe2o3 PR #258 merged compiler-generated write-only KFD arguments at d9552090 with all 20 exact-head checks green",
      "The gfx942 EXEC-control layer assigns no opcode semantics and proves neither machine reconvergence, empty masks, termination, nor launch authority",
      "Checked arithmetic fixes selected production kernel compiles to one -Coverflow-checks=on; its induction certificates do not authorize removal of LLVM overflow guards",
      "Exact single-use u32 guard snapshots may be retained in the induction certificate, but the certificate remains inert without authenticated source-to-KIR-to-LLVM refinement",
      "The landed KIR-to-LLVM replay independently reconstructs target KIR and byte-identical LLVM from canonical neutral KIR; it is exact deterministic derivation evidence, not formal semantic preservation or machine-code authority",
      "PR #248 was closed as superseded after upstream landed the same protected-service classifications and bounded readiness-EOF fix, leaving its rebased local diff empty",
      "PR #250 merged its descriptor-only fixed nine-FD inventory at ce5de889 after exact mutation and package qualification; authenticated SCM_RIGHTS integration and distinct-UID qualification remain open",
      "Mixed-width shift PR #249 is merged through fe2o3 main commit 35771d28 after focused and package-wide remote qualification",
      "Aggregate Worker V3 roster PR #254 merged at bf093149 after current-main restacking and mi300x qualification; its exact finalizer custody grants no load or launch authority",
      "Long-lived KFD queue lifecycle PR #255 merged at 90f16d26; the staged Ferric lifecycle join remains under qualification",
      "Production seccomp candidate 6827646a implements exact Cargo application capture outside the filtered lineage, but direct same-UID hidden-supervisor replay and missing root-authorizer custody block production admission",
      "The missing supervisor authority must be root-backed, server-retained, one-use, and consumed directly by the supervisor before FD204 and Stage C; parent/current-image equality, a caller-created challenge, or another sealed public memfd is not sufficient",
      "The descriptor-only Stage C inventory is merged, but the authenticated distinct-UID Stage C vertical remains open; feature-bypass evidence and the current seccomp candidate are not production closure",
      "Stage D's raw-tuple API was rejected and awaits opaque-owner provenance rework",
      "Lower-MIR candidate 2c3140d7 is audit-only and must be reimplemented under whole-module current KIR replay",
      "Exact multi-root KIR/LLVM/HSACO lowering merged through PR #20 at d32d8a11, and reviewed external source trust merged through PR #21 at 17396690",
      "The exact e98f280f pin retains corrected non-null empty-slice transport and volatile-load support; Ferric has host-qualified integration but no current-source production artifact extraction or GPU result",
      "Deployment identities and Ferric-specific inference authority are intentionally not defined upstream",
    ],
  },
});
