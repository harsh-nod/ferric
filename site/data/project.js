window.FERRIC_PROJECT = Object.freeze({
  updated: "2026-09-01",
  repository: "https://github.com/harsh-nod/ferric",
  fe2o3Repository: "https://github.com/harsh-nod/fe2o3",
  current: {
    siteRefreshBase: "e419160a3d21db5e8b25f414fd696982a959a171",
    implementationCommit: "4369786fde888e1ec64fe6b05fbced39bc33090d",
    historicalImplementationBaseline: "5f40e404ba4bc76c16eed15868c63a72e60e716c",
    selectedFe2o3Pin: "9f97985ee0a4a8ef0bc8f0fa0fd33771c8180592",
    historicalFe2o3Baseline: "b5374c6e6a4c1215ad481cefcd294334dcb1cbeb",
    repinState: "integration",
    githubCiRun: "33490985105",
    githubCiState: "qualified",
    authenticatedReleaseRun: "33490985170",
    authenticatedReleaseState: "integration",
    remoteRootAdapterState: "qualified",
    genericCoreState: "qualified",
    fallbackBindingParityState: "open",
    freshFe2o3QualificationState: "integration",
    devicePackages: [
      "gemm",
      "logits",
      "paged-decode",
      "prefill",
      "rmsnorm",
      "rope-kv",
      "swiglu",
    ],
    repinCompilationTestValidatedDevicePackages: [
      "gemm",
      "logits",
      "paged-decode",
      "prefill",
      "rmsnorm",
      "rope-kv",
      "swiglu",
    ],
    generatedExpectations: 12,
    sourceGateModules: 151,
    sourceGateExecutableBodies: 6850,
    plannerSlots: 354,
    openM1Gates: 33,
  },
  milestone: {
    name: "M1",
    label: "Qwen3 speculative inference on one gfx942",
    state: "integration",
    summary:
      "M1 remains incomplete and all 33 roadmap gates remain open. Ferric 4369786f commits and pushes the corrected integration repin to exact fe2o3 9f97985e. GitHub CI, the exact mi300x root and Worker V3 adapter matrix, generic-core qualification, the deterministic 354-slot planner policy, and all seven device compilation/test/rmeta lanes passed. The public binding checker only printed compiler and fallback bindings without comparing them; those bindings differ, so fallback parity remains open and fallback regeneration is required. Fresh exact qualification of upstream fe2o3 62e527c9 and authenticated release run 33490985170 remain in progress. Ferric still lacks the current protected verifier/service, theorem contract, receipt-bound current HSACO artifacts and roster owners, authenticated runtime path, and hardware, formal, numerical, and performance evidence required to run Qwen.",
  },
  envelope: [
    ["Target", "Qwen3-8B"],
    ["Draft", "Qwen3-0.6B"],
    ["Device", "1 x gfx942"],
    ["Precision", "BF16 / FP32 accumulate"],
    ["Context", "up to 8K tokens"],
    ["Concurrency", "up to 32 sequences"],
    ["Pages refresh base", "e419160a3d21db5e8b25f414fd696982a959a171; merge of implementation 5f40e40 with the published Pages history"],
    ["Host adapter surface", "all seven K1-K7 family adapters and all 12 host symbols/ABI inspectors exist"],
    ["Landed attributed device surface", "K6 SwiGLU only: one of seven required packages and one of 12 required device roots; no current artifact or run"],
    ["Current implementation", "4369786fde888e1ec64fe6b05fbced39bc33090d commits and pushes the corrected exact fe2o3 9f97985e repin across the workspace, Worker V3 adapter, and seven standalone device packages"],
    ["Historical implementation baseline", "5f40e404ba4bc76c16eed15868c63a72e60e716c: exact fe2o3 b5374c6e device roots, validation policy, and 12 generated marker/argument expectations; binding parity is not qualified"],
    ["Historical fe2o3 baseline", "b5374c6e6a4c1215ad481cefcd294334dcb1cbeb retains compilation/test evidence, but not compiler-versus-fallback binding parity"],
    ["Active fe2o3 transition", "9f97985ee0a4a8ef0bc8f0fa0fd33771c8180592 is the exact corrected pin committed and pushed by Ferric 4369786fde888e1ec64fe6b05fbced39bc33090d; generic-core, GitHub CI, root/adapter, planner, and all-seven compilation/test/rmeta lanes passed, while fallback parity remains open and authenticated release qualification is in progress"],
    ["GitHub CI", "run 33490985105 passed for exact Ferric implementation 4369786fde888e1ec64fe6b05fbced39bc33090d"],
    ["Exact mi300x root/adapter matrix", "PASS: root fmt, strict clippy, locked workspace, UI, and documentation tests; Worker V3 adapter fmt, strict clippy, and locked tests"],
    ["Corrected device matrix", "all seven exact 4369786f/9f97985e lanes passed formatting, direct locked tests, compiler-derived wrapper check, locked all-target wrapper check/test, and rmeta embedding; this is not fallback binding parity"],
    ["Fallback binding parity", "OPEN: the public checker printed compiler and checked-in fallback bindings but did not compare them; the reported bindings differ, so the fallback must be regenerated and the hardened comparison must pass"],
    ["Authenticated release", "GitHub Actions run 33490985170 remains in progress and grants no release authority until terminal success"],
    ["Upstream roster handoff", "fe2o3 62e527c960b40716290ba8cb82ba5594be4f3706 is newly pushed generic integration infrastructure under fresh exact qualification; Ferric 4369786f remains pinned to 9f97985e and has not selected 62e527c9"],
    ["Opaque device roots", "gemm, logits, paged-decode, prefill, rmsnorm, rope-kv, and swiglu are integrated as runtime source without Verus, theorem, artifact, load, launch, or Qwen authority"],
    ["Qualified development work", "exact 11-file snapshot admission at 8e7fbbd and snapshot-only operational intake at edfaefa, including a source-path-absent 22-plan MI300X proof, are integrated into the current unpublished M1 integration lineage, not main"],
    ["Canonical selector manifest", "format ferric.m1-worker-v3-selector-manifest.v1 is bounded to 64 KiB, canonical pretty ASCII JSON with one trailing newline, exactly seven K1-K7 entries, canonical absolute paths, exact BuildAttempt values, fixed family order, and duplicate exact-publication rejection"],
    ["Exact artifact selectors", "seven named K1-K7 selectors each retain one durable output directory and one exact BuildAttempt; no directory scan or latest-attempt inference is allowed"],
    ["Production compiler extraction", "exact fe2o3 b5374c6e is the historical compilation baseline and corrected fe2o3 9f97985e is the active transition target, but fallback parity and receipt-bound current K1-K7 HSACO roster admission remain open"],
    ["Authenticated retained runtime", "the f76ef8e lineage preserves authenticated Worker V3 witness, operation-plan, program, queue, and Ferric batch custody across first submit and repeated same-native rounds"],
    ["Authenticated completed-step KV release", "ce14b99 returns quiescent retired pages to their exact pools in deterministic draft-then-target order while retaining authenticated retry, success, teardown, and failure owners without raw queue conversion"],
    ["Concrete Worker V3 roster", "a537b70 acquires the seven concrete rosters from exact selectors; implementation 5f40e40 fixes 12 generated expectations in current compiler order, with K3 paged-KV write then RoPE and K7 lowest-ID argmax, compact completion, then speculative token assembly"],
    ["Authority-free host preflight", "ferric-m1-worker-v3-preflight hashes the selector manifest, recovers and host-admits all seven rosters, requires 12 markers, reports authentication/load/launch/GPU authority false, and releases roster custody before printing its canonical result"],
    ["Protected verifier backend", "the production path still requires one backend implementing protected verification for every concrete K1-K7 roster before 12-program composition; Ferric has no production backend or positive real-custody fixture yet"],
    ["Authenticated repeat-round lifecycle", "f76ef8e retains the 62a2ef4 combined parked-roster scheduling, KV reserve, workspace prepare, same-native rebind, resubmission, completion readback, settlement, page release, retry, and teardown path"],
    ["Historical compilation baseline", "on mi300x, all seven device packages passed exact cargo-fe2o3 locked all-target check/test and direct fallback tests at fe2o3 b5374c6e; the prior wrapper/fallback parity claim is withdrawn because the checker did not compare its printed bindings"],
    ["Corrected repin validation", "generic-core, planner policy, GitHub CI run 33490985105, exact mi300x root/adapter matrix, and all-seven compilation/test/rmeta lanes pass; fallback parity is open, while fresh exact fe2o3 62e527c9 qualification and authenticated release run 33490985170 remain in progress"],
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
        "fe2o3 exposes public exact-attempt recovery for durable RecoveredWorkerV3LoadEnvelopeV2 owners, and its generic roster types can retain all seven compiler results. Ferric a537b70 now calls that API for each named K1-K7 selector. Neither the generic recovery capability nor the new call path supplies real current custody or grants load or launch authority.",
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
        "Qualified source 1d666dbc, landed through be307d52, provides all seven K1-K7 family host adapters and all 12 host symbols and ABI inspectors, requiring matching compiler-produced, move-only Worker V3 owners before strict structural inspection. Implementation 5f40e40 qualified the standalone authority-free Worker V3 envelope adapter against fe2o3 b5374c6e; implementation 4369786f commits the corrected 9f97985e repin, whose exact mi300x root and adapter matrix passed. K6 SwiGLU remains the only attributed package and root landed on main; the active integration branch carries all seven opaque device packages without creating a current HSACO or execution authority.",
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
        "Ferric implementation 5f40e40 retains canonical selector-manifest admission for seven explicit output-directory and BuildAttempt pairs in fixed K1-K7 family order, exact V2 recovery, and host admission against compiler-generated rosters. The recovered aggregate is deliberately inert before protected authentication. Production extraction has not populated a receipt-bound current K1-K7 HSACO roster or move-only owners, so this API is not load, dispatch, hardware, or Qwen execution authority.",
    },
    {
      label: "Canonical selector-manifest admission",
      state: "qualified",
      detail:
        "The parser accepts only the exact v1 format in bounded canonical pretty ASCII JSON with one trailing newline. It requires exactly seven entries and exact object keys, fixed K1-K7 family order, canonical absolute non-root paths, canonical BuildAttempt values, and distinct exact publications. Tests reject family or schema drift, compact JSON, noncanonical attempts, parent-directory paths, and a duplicate K1/K3 publication.",
    },
    {
      label: "Authority-free Worker V3 preflight",
      state: "integration",
      detail:
        "The ferric-m1-worker-v3-preflight command reads one bounded selector manifest, binds its SHA-256, recovers and host-admits all seven rosters, requires exactly 12 markers, verifies that host admission created no authentication authority, releases custody, and emits canonical JSON declaring authentication, load, launch, and GPU-submission authority false. Its one command-shape test passed; no real custody was supplied to a positive run.",
    },
    {
      label: "Protected authentication and program composition API",
      state: "integration",
      detail:
        "The public a537b70 API passes all seven host-admitted rosters through one protected verifier adapter whose backend must implement every concrete roster contract, then invokes the existing fail-closed 12-program composition. Typed failures retain the exact family and acquisition, authentication, or composition stage. Ferric still lacks the production protected-roster verifier backend and a positive real-custody fixture.",
    },
    {
      label: "Exact acquisition negative evidence",
      state: "qualified",
      detail:
        "Three targeted tests passed on mi300x. They preserve exact named selector paths and attempts, reject one duplicate output-directory plus BuildAttempt publication before recovery, and prove a missing exact attempt fails at K1 recovery without probing later families. Strict ferric-engine clippy also passed with all targets, all features, and warnings denied. These checks qualify only the source acquisition boundary.",
    },
    {
      label: "Authenticated Ferric kernel custody",
      state: "integration",
      detail:
        "Implementation 4369786f commits the corrected exact fe2o3 9f97985e repin while retaining the path from a canonical manifest through seven durable selectors, V2 recovery, host roster admission, 12 generated marker/argument expectations, and the authority-free preflight. GitHub CI, exact mi300x root/adapter, generic-core, and all-seven compilation/test/rmeta gates passed. Fallback binding parity remains open after the fail-open checker was discovered, and authenticated release qualification remains in progress, alongside receipt-bound current roster owners and the production protected verifier/service. Authenticated runtime behavior, Qwen, performance, and M1 authority remain unqualified.",
    },
    {
      label: "Concrete K3 and refreshed K7 roster order",
      state: "integration",
      detail:
        "All 12 generated marker/argument expectations are integrated. K3 uses exact RoPE and paged-KV-write marker types with canonical paged-KV-write then RoPE order; K7 uses lowest-ID argmax, compact completion, then speculative token assembly. For corrected 9f97985e, all seven packages passed their compilation/test/rmeta lanes. The checker did not compare its printed compiler and fallback bindings, which differ, so this establishes typed source compilation only, not fallback parity, extracted artifacts, or execution.",
    },
    {
      label: "Latest fe2o3 device validation",
      state: "integration",
      detail:
        "Ferric 4369786f commits exact fe2o3 9f97985e across gemm, logits, paged-decode, prefill, rmsnorm, rope-kv, and swiglu. All seven passed formatting, direct locked tests, compiler-derived wrapper check, locked all-target wrapper check/test, and rmeta embedding on mi300x. The public checker failed open: it printed compiler and fallback bindings without comparing them, and the reported bindings differ. Fallback parity remains open pending regeneration and a passing hardened comparison. No current package result grants theorem, artifact, load, launch, runtime, hardware, or Qwen authority.",
    },
    {
      label: "Source gate and deterministic planner",
      state: "integration",
      detail:
        "The corrected deterministic M1 planner policy accepts exactly 354 slots and rejects hostile policy mutations. GitHub CI run 33490985105, the exact mi300x root/adapter matrix, and generic-core exact 9f qualification passed. The prior source-gate inventory remains scoped historical evidence; fresh exact qualification of fe2o3 62e527c9 and authenticated release run 33490985170 remain in progress. These results do not establish fallback parity, the missing theorem contract, production verifier, or Qwen authority.",
    },
    {
      label: "Authenticated retained readback, settlement, and KV release",
      state: "integration",
      detail:
        "The 5f40e40 lineage retains f76ef8e's authenticated completion through the shared Engine/device-KV settlement core, returns every quiescent retired page to its exact pool after complete preflight, and makes the released owner available to the next round. Retry rejection preserves custody; success and teardown retain queue, program, observation, cache, parked-roster, terminal-lineage, and round-history ownership without exposing a raw queue. Authenticated hardware behavior remains open.",
    },
    {
      label: "Authenticated repeat-round same-native lifecycle",
      state: "integration",
      detail:
        "Head 5f40e40 retains f76ef8e's combined active and parked roster scheduling, KV reservation, workspace preparation, effectful rebind, submit, wait, recycle, readback, completion, page release, retry, and teardown path for the same-native queue. It preserves round history and terminal lineage across target-only and speculative K4/K8/K16 rounds while keeping invalid transitions fail-closed. This is host lifecycle source only; it has no authenticated hardware result.",
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
        "The corrected non-null empty-slice transport is retained in exact pinned fe2o3 head b5374c6e and participates in Ferric's qualified host lifecycle stack. No current artifact dispatch or GPU execution result has exercised that transport through the production Qwen path.",
    },
    {
      label: "Generic volatile-load production support",
      state: "integration",
      detail:
        "Volatile-load support is retained in exact pinned fe2o3 head b5374c6e. Ferric's host qualification covers integration of the pinned workspace, not production extraction, current HSACO artifacts, GPU launch, or numerical behavior.",
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
        "K2 source qualification is complete and retained at implementation 5f40e40 through source commit adc019684d415e45a9543c07e66bc3a17d20edad. It caps rows at 65,536 and uses lane-zero ascending serial FP32 accumulation with bounded loops and bounded volatile reads. Its exact-fe2o3 b5374c6e package compilation and tests passed, but compiler-versus-fallback parity is not qualified. Production extraction, current artifacts, authenticated KFD dispatch, hardware results, and performance remain open.",
    },
    {
      label: "K3 RoPE and KV device source",
      state: "integration",
      detail:
        "K3 source and disjoint-ownership proof work is retained at implementation 5f40e40. Its paged-KV root uses a fixed 16,384-physical-page grid, and its exact generated markers form a concrete two-entry Worker V3 roster in canonical paged-KV-write then RoPE order. Its exact-fe2o3 b5374c6e package compilation and tests passed, but compiler-versus-fallback parity is not qualified. Production extraction, runtime artifact population, authenticated KFD dispatch, hardware numerics, and performance remain open.",
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
        "The attributed K7 logits package at 5d821ee5b13aab01fa5b2c553143e0e7de1c20bc is integrated through merge 9a367e7. Implementation 5f40e40 refreshes its concrete Worker V3 roster to canonical lowest-ID-argmax, compact-completion, speculative-token-assembly binding order for fe2o3 b5374c6e. Source integration does not establish production extraction, a current artifact, authenticated KFD dispatch, hardware numerical results, or performance authority.",
    },
    {
      label: "Staged lifecycle integration",
      state: "integration",
      detail:
        "Implementation 5f40e40 retains canonical selector-manifest admission and the authority-free host preflight ahead of a537b70's exact artifact acquisition and f76ef8e's concrete 12-marker intake and retained lifecycle. Pages refresh base e419160 merges that implementation with published site history, but the implementation is not merged to Ferric main. The current protected verifier/service, receipt-bound HSACO roster and owners, authenticated runtime path, and hardware, Qwen correctness, performance, and M1 evidence remain open.",
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
        "Ferric cannot yet run Qwen through the production path. Implementation 4369786f commits all seven device packages against corrected exact fe2o3 9f97985e and retains the canonical manifest, authority-free preflight, and 12 generated marker/argument expectations. GitHub CI, exact mi300x root/adapter, planner, generic-core, and all-seven compilation/test/rmeta gates pass. Fallback binding parity is open because the public checker did not compare differing compiler and fallback bindings; fresh fe2o3 62e527c9 and authenticated release qualification remain in progress. There is no current protected verifier/service, theorem contract, receipt-bound current HSACO roster and owners, or authenticated runtime path. Exact graph execution and hardware, formal, numerical, performance, and independent qualification evidence remain open.",
    },
    {
      label: "M1 qualification",
      state: "open",
      detail:
        "All 33 roadmap requirements remain open. The current protected verifier/service, theorem contract, receipt-bound current HSACO artifacts and roster owners, authenticated runtime path, Qwen numerical, hardware, formal, and performance evidence, independent validation, and the complete M1 qualification receipt remain open. The current source gates and historical evidence do not close M1.",
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
        name: "Canonical Worker V3 selector manifest",
        detail:
          "A bounded canonical JSON document names exactly seven K1-K7 publications in fixed order using canonical absolute paths and exact BuildAttempt values. It rejects representation, schema, family-order, path, attempt, and duplicate-publication drift before recovery begins.",
      },
      {
        name: "Authority-free seven-roster preflight",
        detail:
          "The preflight binds the manifest digest, recovers and host-admits seven exact rosters, requires 12 markers, releases custody, and reports authentication, load, launch, and GPU work authority false. It is an operator-facing host check, not the production verifier or a GPU runner.",
      },
      {
        name: "Exact seven-family Worker V3 acquisition",
        detail:
          "Seven named selectors preserve exact durable output directories and BuildAttempt values, recover exact V2 envelopes, and host-admit each result against its concrete generated roster. A separate public stage requires protected authentication for all seven families before composing the 12-program set. No real current custody or production verifier backend has exercised the positive path.",
      },
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
          "The exact snapshot intake, canonical selector manifest, authority-free preflight, seven-family recovery API, concrete roster admission, retained same-native lifecycle, and corrected fe2o3 9f97985e pins are integrated through 4369786f, not merged to main. GitHub CI, exact mi300x root/adapter, generic-core, and all-seven compilation/test/rmeta gates passed. Regenerate fallbacks and pass the hardened binding comparison, then complete fresh fe2o3 62e527c9 and authenticated release qualification before supplying receipt-bound current owners and qualifying the authenticated runtime lifecycle on hardware.",
      },
      {
        name: "Complete KFD edge handling",
        detail:
          "Ferric's historical compilation baseline pins fe2o3 b5374c6e, and the corrected active integration tree pins 9f97985e through 4369786f. Its GitHub CI, exact mi300x root/adapter matrix, and generic-core qualification passed; fallback parity remains open. Complete fallback regeneration, hardened comparison, authenticated release qualification, and the authenticated runtime path without exposing lower or raw queue ownership.",
      },
      {
        name: "Ordered Stage C and joint Stage D",
        detail:
          "Integrate merged PR #250's descriptor-only nine-FD inventory with the authenticated SCM_RIGHTS client/service path and prove the full distinct-UID vertical. Bind the production application supervisor to a root-managed, server-consumed one-use authorization derived from the admitted protected release; same-UID process checks and caller-created sockets are not authority. Add complete PDEATHSIG and kill/reap custody before qualifying Stage C, and replace Stage D's rejected raw tuple with an opaque provenance-carrying owner.",
      },
      {
        name: "Extract the integrated device packages",
        detail:
          "All seven K1-K7 source packages, exact selectors, and the canonical input manifest are retained through 4369786f. Every corrected 9f97985e package passed formatting, direct locked tests, compiler-derived wrapper check, locked all-target wrapper check/test, and rmeta embedding. The checker did not compare its printed compiler and fallback bindings, which differ, so fallback regeneration and a passing hardened parity comparison remain required. Every family also needs receipt-bound current KIR, LLVM, HSACO, descriptors, roster owners, authenticated launch, and hostile evidence before entering the live Ferric artifact roster.",
      },
      {
        name: "Authenticated fixed-batch KFD",
        detail:
          "Authenticated fixed-batch publication and repeated same-native scheduling, reserve, prepare, rebind, submit, wait, recycle, readback, completion, retired-page return, retry, and typed teardown exist. Bind exact extracted artifacts and finish rollover, diagnostic evidence, hardware qualification, and production serving custody.",
      },
      {
        name: "Ferric compiler integration and authority rosters",
        detail:
          "The historical compilation baseline pins fe2o3 b5374c6e; implementation 4369786f commits and pushes the corrected exact 9f97985e repin. Its planner, GitHub CI, exact mi300x root/adapter, generic-core, and all-seven compilation/test/rmeta gates pass, but fallback parity remains open after discovery of the fail-open checker. Upstream fe2o3 62e527c9 adds generic roster-handoff infrastructure under fresh exact qualification; Ferric has not selected it. Complete fallback regeneration, hardened comparison, fresh qualification, and authenticated release before producing receipt-bound current compiler artifacts and owners.",
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
      title: "Corrected fe2o3 repin gates",
      state: "integration",
      source: "4369786fde888e1ec64fe6b05fbced39bc33090d",
      result:
        "PASS: CI, root/adapter, planner 354, generic-core, all-seven compile/test/rmeta; OPEN: fallback parity; IN PROGRESS: fe2o3 62e qualification, release",
      detail:
        "Ferric 4369786f commits and pushes corrected fe2o3 9f97985e. GitHub CI run 33490985105, exact mi300x root/adapter, planner, and generic-core qualification passed. All seven device packages passed formatting, direct locked tests, compiler-derived wrapper check, locked all-target wrapper check/test, and rmeta embedding. The public checker printed compiler and checked-in fallback bindings without comparing them; those bindings differ, so parity remains open pending fallback regeneration and a passing hardened comparison. Fresh exact fe2o3 62e527c9 qualification and authenticated release run 33490985170 remain in progress. These results grant no theorem, protected-verifier, artifact, runtime, hardware, Qwen, performance, production-receipt, or M1 authority.",
    },
    proof: {
      title: "Authenticated release proof",
      state: "qualified",
      source: "58fd37e",
      closureSha256:
        "b922a6cd2881bd38403afce0c14dc898cf13da770616875489069a2701f2c933",
      result: "PASS: 645 admitted proof bodies; 1,490 verification queries",
      detail:
        "The current source gate accepts 151 modules and 6,850 executable bodies. Newly discovered bodies remain explicitly unverified and pending with no authority grant. The retained 58fd37e release proof remains scoped to its recorded closure; neither result establishes the missing current theorem contract, protected verifier, hardware execution, Qwen correctness, performance, or M1.",
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
      commit: "4369786fde888e1ec64fe6b05fbced39bc33090d",
      title: "Commit the corrected Worker V3 compiler/runtime repin",
      state: "integration",
      detail:
        "This pushed Ferric branch commit pins the root workspace, authority-free Worker V3 adapter, and all seven standalone device packages to corrected exact fe2o3 9f97985e. GitHub CI run 33490985105, exact mi300x root/adapter, planner, generic-core, and all-seven compilation/test/rmeta gates passed. Fallback parity is open because the public checker did not compare differing compiler and fallback bindings; fresh fe2o3 62e527c9 and authenticated release run 33490985170 remain in progress. No Qwen or M1 authority follows.",
    },
    {
      commit: "9f97985ee0a4a8ef0bc8f0fa0fd33771c8180592",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Select the corrected compiler/runtime repin target",
      state: "integration",
      detail:
        "Ferric 4369786f pins this exact upstream source across the root workspace, authority-free Worker V3 adapter, and all seven standalone device packages. Generic-core, GitHub CI, exact mi300x root/adapter, planner, and all-seven compilation/test/rmeta gates pass. Fallback parity remains open pending regeneration and a passing hardened comparison; authenticated release qualification remains in progress.",
    },
    {
      commit: "62e527c960b40716290ba8cb82ba5594be4f3706",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Push generic Worker V3 roster-handoff infrastructure",
      state: "integration",
      detail:
        "This upstream fe2o3 commit adds generic roster-handoff integration infrastructure and is undergoing fresh exact qualification. Ferric implementation 4369786f remains pinned to 9f97985e and has not selected or qualified 62e527c9, so this record grants no current Ferric compiler/runtime, roster, Qwen, or M1 authority.",
    },
    {
      commit: "0c04ab7f94072eb6b763ffdcaa878af6e3c5a2f7",
      title: "Superseded: commit the first Worker V3 repin",
      state: "observed",
      detail:
        "This historical pushed Ferric commit pinned the root workspace, authority-free Worker V3 adapter, and all seven standalone device packages to fe2o3 61967a3c. It and its scoped device results were superseded by corrected implementation 4369786f and grant no current validation, Qwen, or M1 authority.",
    },
    {
      commit: "61967a3cb3958faddcda3a5e7ed6b19fd6e68ebb",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Superseded: validate the first compiler/runtime repin target",
      state: "observed",
      detail:
        "All seven Ferric device workspaces passed a scoped matrix against this historical upstream source on mi300x. Corrected fe2o3 9f97985e supersedes it, so those results are not current generic-core, device, root, adapter, CI, release, Qwen, or M1 evidence.",
    },
    {
      commit: "5f40e404ba4bc76c16eed15868c63a72e60e716c",
      title: "Historical: qualify the b537 device integration",
      state: "observed",
      detail:
        "Ferric selected exact fe2o3 b5374c6e across the root workspace, Worker V3 adapter, and all seven opaque device packages. On mi300x, package and fallback tests plus root and adapter gates passed. The prior independent wrapper/fallback parity claim is withdrawn: the checker printed both binding sets without comparing them. This historical result creates no current parity, theorem, artifact, load, launch, hardware, Qwen, performance, or M1 authority.",
    },
    {
      commit: "b5374c6e6a4c1215ad481cefcd294334dcb1cbeb",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Historical: select the compiler/runtime baseline",
      state: "observed",
      detail:
        "This exact upstream source was selected by Ferric implementation 5f40e40, its authority-free Worker V3 adapter, and all seven standalone device packages. Package compilation and tests passed, but binding compatibility is not qualified because the checker did not compare compiler and fallback bindings. Ferric-specific theorem, protected-verifier, artifact, runtime, and Qwen authority remain downstream.",
    },
    {
      commit: "922dcb621e5bb2acc41eb623cf2894b5ffa21a37",
      title: "Add the canonical Worker V3 roster preflight",
      state: "qualified",
      detail:
        "A canonical seven-family selector manifest now enforces bounded pretty ASCII JSON, exact schema and K1-K7 order, canonical absolute paths and BuildAttempt values, and duplicate-publication rejection. ferric-m1-worker-v3-preflight binds the manifest digest, recovers all seven host rosters, requires 12 markers, releases custody, and reports authentication/load/launch/GPU authority false. All seven device all-target cargo-fe2o3 checks, strict engine clippy, engine 465/5, harness 11, packet 2, qualification 81/1, preflight 1, and doctests 132 passed on mi300x. Real custody, production verification, Qwen, GPU, and performance remain open.",
    },
    {
      commit: "a537b70ce13f6cc61f0a6763a85b42dac86a6875",
      title: "Add exact authenticated roster acquisition",
      state: "qualified",
      detail:
        "Seven named K1-K7 selectors now bind durable output directories to exact BuildAttempt values, recover their V2 envelopes, host-admit them against concrete generated rosters, and expose protected authentication followed by existing 12-program composition. Duplicate exact publication and missing-attempt recovery fail closed. Three targeted tests and strict all-target, all-feature engine clippy passed on mi300x. No real current V2 custody or production protected verifier backend exists, so Qwen remains unrunnable.",
    },
    {
      commit: "f76ef8e5c08a9ecafe1e4ecb56ee26ab16c8c192",
      title: "Integrate the concrete K3 Worker V3 roster",
      state: "qualified",
      detail:
        "All seven K1-K7 families now use concrete compiler-generated marker rosters covering exactly 12 programs. K3 is canonical paged-KV write then RoPE; refreshed K7 is compact completion, lowest-ID argmax, then speculative token assembly. On mi300x, strict engine clippy, engine 458 passed/5 ignored, hardware-harness 11 passed, packet diagnostics 2 passed, qualification capture 81 passed/1 ignored, and 132 doctests passed. K3's standalone tests and compiler-derived all-target wrapper check also passed; this did not establish compiler-versus-fallback parity. No artifact, GPU execution, Qwen, performance, or M1 authority follows.",
    },
    {
      commit: "2f6da870a31b8e430fd0af9c756ca86685e67572",
      repository: "https://github.com/harsh-nod/fe2o3",
      title: "Qualify retained host runtime on Rust 1.97",
      state: "qualified",
      detail:
        "This exact upstream head retains authenticated rosters across service queues and rebind, refreshes retained queue lock dependencies, exposes authenticated roster source identities, and passes strict all-feature host/runtime qualification on Rust 1.97. Ferric pins it through 047f226; the generic runtime work does not populate Ferric artifacts or prove GPU or Qwen execution.",
    },
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
        "K2 source qualification is complete and the exact package is retained through implementation 5f40e40. Its exact-fe2o3 package checks, tests, and fallback tests passed on mi300x. The prior binding-parity claim is withdrawn because the checker printed compiler and fallback bindings without comparing them. Production extraction, artifacts, authenticated KFD dispatch, hardware numerics, performance, Qwen execution, and M1 remain open.",
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
        "Current implementation 5f40e404ba4bc76c16eed15868c63a72e60e716c retains the attributed Qwen3 GEMM and embedding merge and validates it against exact fe2o3 b5374c6e. This is branch integration and source-level evidence, not an origin/main landing, extracted production artifact, authenticated KFD dispatch result, hardware result, performance result, Qwen run, or M1 authority.",
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
        "This commit began the standalone K2 package. Qualified revision adc01968 replaced the XOR tree with authoritative lane-zero ascending serial FP32 accumulation and is retained through implementation 5f40e40; its current exact-fe2o3 device validation passed on mi300x. Production extraction, artifacts, authenticated KFD dispatch, hardware results, and performance remain open.",
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
      "This Pages refresh is based on merge e419160a3d21db5e8b25f414fd696982a959a171, which combines implementation 5f40e40 with the published Pages history",
      "Exact 11-file snapshot admission at 8e7fbbd and snapshot-only operational intake at edfaefa are qualified development work integrated into the current unpublished M1 integration lineage, not main",
      "Implementation 5f40e40 retains canonical selector-manifest admission and an authority-free seven-roster host preflight over a537b70's exact recovery and authentication APIs, but is not merged to main and is not a production inference baseline",
      "All seven K1-K7 family host adapters and all 12 host symbols and ABI inspectors exist",
      "K6 SwiGLU is the only attributed device package and root landed on main, through PR #32; no current artifact or run is claimed",
      "M1 implementation 5f40e404ba4bc76c16eed15868c63a72e60e716c contains all seven K1-K7 source packages, seven concrete rosters over 12 generated marker/argument expectations, the exact acquisition API, canonical manifest parsing, and authority-free preflight",
      "The manifest is bounded canonical pretty ASCII JSON with exactly seven ordered family entries, canonical absolute paths, canonical BuildAttempt values, and duplicate exact-publication rejection",
      "The preflight binds the manifest SHA-256, recovers and host-admits all seven rosters, requires 12 markers, releases custody, and reports authentication, load, launch, and GPU-submission authority false",
      "Each family selector binds one durable output directory to one exact BuildAttempt; a duplicate exact publication is rejected before recovery and a missing exact attempt fails at K1 without probing later families",
      "Recovered V2 custody is only host-admitted and explicitly carries no verification, load, or launch authority until the protected verifier adapter accepts every concrete roster and exact program composition succeeds",
      "K2 source qualification is complete at adc01968; its current artifact extraction, authenticated KFD, hardware, and performance gaps remain",
      "K3 RoPE/KV uses exact generated markers in canonical paged-KV-write then RoPE order; its corrected 9f97985e device validation, extraction, runtime admission, hardware numerics, and performance remain open",
      "K7's concrete roster is refreshed to canonical lowest-ID-argmax, compact-completion, speculative-token-assembly binding order; this is typed source admission, not artifact or execution authority",
      "K4 prefill commit 7e333905, K5 paged-decode commit 863e82ec, and K7 logits commit 5d821ee5 are integrated source packages; none has production artifact, authenticated KFD, hardware, or performance authority",
      "Ferric implementation 5f40e40 and fe2o3 b5374c6e form a historical compilation/test baseline, not a binding-parity qualification; Ferric 4369786f commits and pushes corrected fe2o3 9f97985e, whose GitHub CI, exact mi300x root/adapter matrix, planner, generic-core, and all-seven compilation/test/rmeta lanes pass while fallback parity remains open and authenticated release remains in progress",
      "The corrected deterministic planner accepts exactly 354 slots and rejects hostile policy mutations; GitHub CI run 33490985105 and the exact mi300x root/adapter matrix also pass, but these results grant no runtime, Qwen, or M1 authority",
      "Authenticated combined parked-roster scheduling, reserve, prepare, rebind, submit, wait, recycle, readback, completion, page release, retry, and teardown retain ownership without raw queue conversion; rollover, diagnostic serving, and end-to-end inference remain open",
      "Authenticated hardware behavior is not yet qualified",
      "Ferric cannot yet run Qwen through the production path",
      "A current protected verifier/service, theorem contract, receipt-bound current HSACO artifacts and roster owners, authenticated runtime path, end-to-end Qwen, numerical, hardware, formal, and performance evidence, independent validation, the production receipt, and M1 remain Ferric work",
      "All 33 M1 roadmap requirements remain open",
    ],
    fe2o3: [
      "Reusable compiler APIs, semantic artifact identities, and protected compilation",
      "Durable subject-bound compiler receipt acquisition and recovered V2 carriage",
      "Generic receipt-complete sealed verification and promotion boundary",
      "Typed KFD allocations, USERPTR/AQL queues, fixed-batch publication, completion, and dispatch",
      "Exact head b5374c6e6a4c1215ad481cefcd294334dcb1cbeb is Ferric's historical compiler/runtime compilation baseline; binding parity is not qualified",
      "Exact head 9f97985ee0a4a8ef0bc8f0fa0fd33771c8180592 is the corrected active pin committed and pushed by Ferric 4369786f across the root workspace, Worker V3 adapter, and seven standalone device packages; CI, root/adapter, planner, generic-core, and all-seven compilation/test/rmeta lanes pass while fallback parity and authenticated release remain open or in progress",
      "Upstream head 62e527c960b40716290ba8cb82ba5594be4f3706 adds generic roster-handoff integration infrastructure under fresh exact qualification; Ferric 4369786f remains pinned to 9f97985e and has not selected or qualified it",
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
      "The exact b5374c6e pin retains corrected non-null empty-slice transport and volatile-load support; Ferric has host-qualified integration but no receipt-bound current production artifacts or authenticated GPU result",
      "Deployment identities and Ferric-specific inference authority are intentionally not defined upstream",
    ],
  },
});
