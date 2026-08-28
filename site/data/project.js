window.FERRIC_PROJECT = Object.freeze({
  updated: "2026-08-27",
  repository: "https://github.com/harsh-nod/ferric",
  milestone: {
    name: "M1",
    label: "Qwen3 speculative inference on one gfx942",
    state: "integration",
    summary:
      "Four finite speculative shapes and four exact production rollover transitions pass the authenticated host gate at 58fd37e. The scoped release proof also passes for that exact source. Ferric source 57f6cfdf completed the protected build for the first Rust Qwen3 kernel family, and qualification source 1b77cb5 matched all 3,072 outputs exactly through HIP. Ferric PR #21 at 41dfb3b2 repins its tested recovered-V2 custody adapter to fe2o3 PR #236 at 5362f3cb on main 5706e3a1, which provides the fixed peer-authenticated connector. Cargo session construction and invocation remain unwired. No live V2 kernel occurrence, repository-label or host-descriptor lineage, deployed authority service and backend FD195/196 exchange, accepting verifier, production KFD load/dispatch, whole-Qwen execution, performance result, or M1 qualification follows.",
  },
  envelope: [
    ["Target", "Qwen3-8B"],
    ["Draft", "Qwen3-0.6B"],
    ["Device", "1 x gfx942"],
    ["Precision", "BF16 / FP32 accumulate"],
    ["Context", "up to 8K tokens"],
    ["Concurrency", "up to 32 sequences"],
    [
      "Build pins",
      "Ferric source 57f6cfdf / fe2o3 compiler 21e4c106 with main e39fdc14 at the protected two-phase build",
    ],
    [
      "Protected artifact",
      "semantic fe2ce532...e9569e8f / HSACO 57ecb86b...fc6afa7 / load-envelope lineage 058b9498...b17014a8",
    ],
    [
      "Evidence record",
      "a1b96acf0b9f32f5f02f0a5c92920df4f24502af674a4d05129eb0c902960572 at Ferric 1ec0d240; custody only",
    ],
    [
      "Compiler head",
      "fe2o3 PR #236 5362f3cb on main 5706e3a1; fixed authenticated supervisor connector",
    ],
    [
      "Pending verifier request",
      "Ferric PR #21 c7362d93 binds the exact build and 22 profiles; authority-free",
    ],
    [
      "V2 custody adapter",
      "Ferric PR #21 41dfb3b2 / feature 7b3a05f1; 8 of 8 adapter tests pass",
    ],
  ],
  readiness: [
    {
      label: "Target-only prompt to text",
      state: "observed",
      detail: "Completed on MI300X with exact assets and the default 8 MiB host stack.",
    },
    {
      label: "Finite-shape speculative serving",
      state: "implemented",
      detail:
        "Exact S1/K4, S8/K4, S1/K8, and S1/K16 admission and same-shape rearm pass the exact-source workspace gate at 58fd37e. This establishes source behavior, not GPU or Qwen correctness.",
    },
    {
      label: "Production shape rollover",
      state: "implemented",
      detail:
        "Paired-prefill rollover covers four exact transitions with roster-indexed KV, output, provider, and queue custody. The exact-source workspace gate passes at 58fd37e; MI300X execution remains open.",
    },
    {
      label: "Authenticated proof release",
      state: "qualified",
      detail:
        "The strict wrapper passes for source closure b922a6cd...2c933: 645 admitted proof bodies, 1,490 verification queries, 37 rejected exact-body mutations, and 10 same-source quality gates.",
    },
    {
      label: "M1 qualification",
      state: "open",
      detail:
        "All 33 roadmap requirements and 17 assurance properties remain open, along with the required external evidence. No complete M1 evidence index or M1 qualification receipt exists. The scoped proof-release receipt does not close M1.",
    },
    {
      label: "Worker V3 kernel migration",
      state: "observed",
      detail:
        "Ferric source 57f6cfdf4b3f5177a556159d1e548b25b63a1541 completed the protected qwen3_swiglu_bf16_f32_v1 build on MI300X/gfx942:xnack-. Qualification source 1b77cb5b82e370ca9a46c04d4465d2ba61737d01 matched all 3,072 elements bit-for-bit through HIP, source 1ec0d240 records exact protected-build evidence SHA-256 a1b96acf0b9f32f5f02f0a5c92920df4f24502af674a4d05129eb0c902960572, and source c7362d93 binds an authority-free pending verifier request. PR #21 at 41dfb3b2d432f91fe2a755cbae2058e1f820c4ef adds tested V2 receipt custody without granting live occurrence, accepting-verifier, load, or launch authority.",
    },
    {
      label: "Mapped KFD adapter binding",
      state: "observed",
      detail:
        "Direct-to-main fe2o3 PR #236 is at 5362f3cba0fccf1c75c6b34d94240b29f17d7b9b on main 5706e3a1f3849180f6f3fca38a9e7d92840633e5 and supersedes intermediate PRs #219-#235. The production client internally creates a CLOEXEC|NONBLOCK UNIX SEQPACKET socket, connects only to /run/fe2o3/compiler-execution-supervisor.sock, authenticates the exact remote and unnamed local addresses plus SO_PEERCRED positive PID and configured nonroot UID/GID, and bounds connect plus handoff to 120 seconds. Its child-session public API cannot inject an FD or path.",
    },
    {
      label: "Recovered V2 custody adapter",
      state: "implemented",
      detail:
        "Ferric feature 7b3a05f129a72f32c907388e1e176bfea1384689 consumes and retains a recovered V2 owner, returns it on failure, projects every receipt identity from that owner, checks the exact build and artifact, and permits raw strict decode only as inert data. All 8 adapter tests plus formatting and clippy pass at repinned PR #21 head 41dfb3b2d432f91fe2a755cbae2058e1f820c4ef. No live V2 kernel occurrence or production authority is claimed.",
    },
    {
      label: "Qualification-only SwiGLU numerics",
      state: "observed",
      detail:
        "On MI300X, the HIP qualification harness reported architecture=gfx942:sramecc+:xnack-, elements=3072, exact=3072, max_ulp=0, and mismatches_gt_1ulp=0 for HSACO 57ecb86b...fc6afa7. This is qualification-only load, dispatch, and numerical evidence. It does not exercise or authorize the production Worker V3 verifier, KFD load/dispatch, or Qwen inference path.",
    },
  ],
  capabilities: {
    runnable: [
      {
        name: "Authenticated Qwen3 inputs",
        detail:
          "Strict pinned config, shared tokenizer metadata, safetensors schema, and prepacked-weight admission.",
      },
      {
        name: "Generated gfx942 execution plans",
        detail:
          "Exact target/draft plans, typed operations, workspace layouts, and Ferric-owned kernel specifications. SwiGLU now has a protected mapped-adapter Worker V3 artifact; the other six kernel families, production verifier join, generated runner, model bundle, and GPU execution remain open.",
      },
      {
        name: "Physical target-only generation",
        detail:
          "Direct HSA publication, completion readback, independent K6 choice capture, generation-safe queue rearm, KV ownership, and token decode on MI300X.",
      },
      {
        name: "Bounded engine state",
        detail:
          "Generational scheduling, paged KV custody, cancellation, completion epochs, and quiescent retirement.",
      },
    ],
    experimental: [
      {
        name: "Finite-shape speculation",
        detail:
          "A physical lifecycle adapter owns fresh publication, same-shape rearm, independent choice readback, and typed terminal cleanup for exact S1/K4, S8/K4, S1/K8, and S1/K16 shapes.",
      },
      {
        name: "Finite production rollover",
        detail:
          "Host-validated production paths cover Prefill S1/T128 to S1/K4, S1/K8, and S1/K16, plus Prefill S8/T128 to S8/K4. Unsupported transitions fail closed.",
      },
      {
        name: "Proof-policy source gate",
        detail:
          "The authenticated source gate matches 143 verified modules to 6,121 executable bodies and binds 645 admitted proof bodies to 1,490 successful verification queries.",
      },
      {
        name: "Rust Qwen3 SwiGLU BF16 production artifact",
        detail:
          "The protected qwen3_swiglu_bf16_f32_v1 build emitted semantic identity fe2o3::semantic::fe2ce53206f36841ea363da0db20869f38e21b881241581f455eebf8e9569e8f and a 14,192-byte HSACO with SHA-256 57ecb86b40db136237e65a5fae04c955f2c92fe3347c085ec5c806984fc6afa7. Its exact load-envelope lineage filename identity is 058b9498ba96b0d6969ed60bb6599da860d0c0e4528e48b36bb8ef14b17014a8. Ferric PR #21 records the build under evidence SHA-256 a1b96acf0b9f32f5f02f0a5c92920df4f24502af674a4d05129eb0c902960572. The record grants no verifier, production load/dispatch, numerical, performance, Qwen, or M1 authority.",
      },
      {
        name: "Host-exposed mapped KFD adapter",
        detail:
          "Ferric PR #21 exposes the compiler-generated mapped arguments and is now at evidence-record source 1ec0d2407f84733f17d24f7049d640b7ba4c71c7. Its binding-only gate passed 1 library, 3 numerical reference, and 7 source/adapter contract tests; source 57f6cfdf completed the protected build, 1b77cb5 added the qualification-only HIP harness, and 1ec0d240 recorded the exact build evidence. The root legacy V2 workspace remains frozen.",
      },
      {
        name: "Qualification-only HIP numerical run",
        detail:
          "The harness loaded and dispatched exact HSACO 57ecb86b40db136237e65a5fae04c955f2c92fe3347c085ec5c806984fc6afa7 on gfx942:sramecc+:xnack-. All 3,072 elements matched exactly, max ULP was 0, and no value exceeded 1 ULP. This evidence is intentionally separate from production Worker V3 verification and KFD execution.",
      },
      {
        name: "Authority-free pending verifier request",
        detail:
          "Ferric PR #21 at c7362d93d031f735ade33bf8bfa25ff8250e359b binds the exact protected build, 22 target/draft profiles, ABI and launch semantics, and untrusted projected V2 carriage axes into a pending SwiGLU verifier request. It rejects current V1 evidence, grants no compiler, load, or launch authority, and is not the future fe2o3 adapter.",
      },
      {
        name: "Recovered V2 receipt custody",
        detail:
          "Ferric feature 7b3a05f129a72f32c907388e1e176bfea1384689 retains the recovered V2 owner, returns it on failure, and derives receipt identity from the owner under exact build and artifact checks. Raw strict decode remains inert. The 8-test adapter gate does not establish a live V2 kernel occurrence, repository-label or host-descriptor lineage, or production authority.",
      },
    ],
    roadmap: [
      {
        name: "Current-source MI300X lifecycle run",
        detail:
          "A one-token Qwen diagnostic on exact source 58fd37e ran for 20 minutes without output and timed out after the host lost exclusivity. It is non-evidence. The generic rollover path has not run on MI300X.",
      },
      {
        name: "Worker V3 artifact migration",
        detail:
          "Ferric PR #21 at 41dfb3b2d432f91fe2a755cbae2058e1f820c4ef carries the mapped artifact, evidence, qualification-only HIP numerics, pending verifier request, and recovered-V2 custody adapter. fe2o3 PR #236 at 5362f3cba0fccf1c75c6b34d94240b29f17d7b9b provides the bounded issuer service, receipt session, and fixed authenticated connector source. A live V2 kernel occurrence, repository-label and host-descriptor lineage, deployed service and backend FD exchange, accepting-verifier authority, six kernel families, generated runner and model bundle, production KFD load/dispatch, performance validation, independent evidence, and formal M1 closure remain open.",
      },
      {
        name: "Production verifier refinement chain",
        detail:
          "The production path remains fail-closed. cargo-fe2o3 does not yet construct PendingCompilerExecutionChildSessionV1 or invoke the fixed connector. No live V2 kernel occurrence, repository labels or host-descriptor lineage, deployed authority service, backend FD195/196 exchange, or accepting policy/rollback/verifier authority exists. The adapter, exact build evidence, and qualification-only HIP results do not grant production Worker V3 load, launch, or KFD dispatch authority.",
      },
      {
        name: "Full M1 evidence closure",
        detail:
          "The scoped release proof is complete. Current-source hardware, Qwen numerical and performance evidence, independent validation, full M1 TCB, and the M1 receipt remain open.",
      },
      {
        name: "Serving breadth",
        detail:
          "Sampling, prefix caching, chunked prefill, quantized KV, and an HTTP boundary follow in M2.",
      },
    ],
  },
  latestObservation: {
    title: "V2 custody is repinned to the fixed connector",
    date: "2026-08-27",
    state: "implemented",
    commit: "41dfb3b2d432f91fe2a755cbae2058e1f820c4ef",
    buildId: "Ferric feature 7b3a05f1 / fe2o3 pin 5362f3cb",
    environment: "Ferric standalone V2 custody adapter; source validation only",
    result: "PASS: 8 of 8 adapter tests; formatting and clippy green",
    generatedTokenIds: [],
    authority:
      "Source custody only. Cargo session construction and invocation, live V2 kernel occurrence, repository-label or host-descriptor lineage, deployed authority service and backend FD195/196 exchange, accepting policy/rollback/verifier authority, production load/dispatch, Qwen, performance, independent-validation, and M1 authority remain open.",
  },
  validation: {
    host: {
      title: "Mapped Worker V3 adapter binding",
      state: "implemented",
      source: "a8df223a6c4d319e998dfa45f674cfd6c5ab5afc",
      result: "PASS: 1 library + 3 reference + 7 adapter contract tests",
      detail:
        "Ferric PR #21 exposes fe2o3's exact mapped KFD arguments on the host. These checks validate bindings only. The root legacy V2 workspace remains frozen because latest fe2o3 removed orphan routes. The later source at 57f6cfdf completed the protected build, while production verifier refinement evidence and GPU execution remain open.",
    },
    proof: {
      title: "Authenticated release proof",
      state: "qualified",
      source: "58fd37e",
      closureSha256:
        "b922a6cd2881bd38403afce0c14dc898cf13da770616875489069a2701f2c933",
      result: "PASS: 645 admitted proof bodies; 1,490 verification queries",
      detail:
        "The source gate covers 143 modules and 6,121 executable bodies. This qualifies only the scoped release proof; it does not establish MI300X execution, Qwen numerical correctness, performance, independent validation, or formal M1 qualification.",
    },
    hardware: {
      title: "MI300X qualification-only HIP numerical run",
      state: "observed",
      source: "1b77cb5b82e370ca9a46c04d4465d2ba61737d01",
      result: "PASS: 3,072 / 3,072 exact; max ULP 0",
      detail:
        "The qualification harness loaded and dispatched exact HSACO 57ecb86b40db136237e65a5fae04c955f2c92fe3347c085ec5c806984fc6afa7 on architecture gfx942:sramecc+:xnack-. It checked 3,072 elements: all 3,072 were exact, max_ulp=0, and mismatches_gt_1ulp=0. This does not exercise or authorize the production Worker V3 verifier, KFD load/dispatch, or Qwen inference path.",
    },
    transitions: [
      ["Prefill S1/T128", "Speculative S1/K4", "implemented"],
      ["Prefill S1/T128", "Speculative S1/K8", "implemented"],
      ["Prefill S1/T128", "Speculative S1/K16", "implemented"],
      ["Prefill S8/T128", "Speculative S8/K4", "implemented"],
    ],
    limitation:
      "The transition catalog is exact. Target-only decode transitions and every unlisted cross-plan transition require explicit queue retirement and fresh launch; they do not inherit native rollover support.",
  },
  recentProgress: [
    {
      commit: "41dfb3b2d432f91fe2a755cbae2058e1f820c4ef",
      title: "Repin V2 custody to the fixed supervisor connector",
      state: "implemented",
      detail:
        "Ferric PR #21 repins its standalone recovered-V2 custody adapter to exact fe2o3 head 5362f3cba0fccf1c75c6b34d94240b29f17d7b9b. All 8 adapter tests plus formatting and clippy pass. The new policy regression preserves the connector-versus-Cargo boundary: cargo-fe2o3 still does not construct the child session or invoke the connector.",
    },
    {
      commit: "5362f3cba0fccf1c75c6b34d94240b29f17d7b9b",
      title: "Connect only to the fixed protected supervisor",
      state: "implemented",
      detail:
        "fe2o3 PR #236 pins main 5706e3a1f3849180f6f3fca38a9e7d92840633e5. Its production client creates its own CLOEXEC|NONBLOCK UNIX SEQPACKET socket, connects only to the fixed supervisor path, authenticates exact addresses and SO_PEERCRED identity, and bounds connect plus handoff to 120 seconds. The public child-session API accepts no injected FD or path; Cargo construction and invocation remain open.",
    },
    {
      commit: "7b3a05f129a72f32c907388e1e176bfea1384689",
      title: "Retain recovered V2 receipt custody in Ferric",
      state: "implemented",
      detail:
        "Ferric feature 7b3a05f129a72f32c907388e1e176bfea1384689 consumes and retains the recovered V2 owner, returns it on failure, and projects all receipt identities from the owner under exact build and artifact checks. Raw strict decode is inert only. Repinned head 41dfb3b2 extends the adapter gate to 8 tests; formatting, clippy, and explicit standalone-adapter CI steps remain green.",
    },
    {
      commit: "c6b53d97745ceb918dd4972b2154363ce370a75b",
      title: "Complete the bounded issuer service source path",
      state: "implemented",
      detail:
        "fe2o3 PR #236 integrated readiness, per-connection service sessions, the fixed SEQPACKET listener, bounded workers, and exact V2 receipt return. Supervisor tests reported 36 pass and 2 explicit ignores; client gates reported 10 unit, 5 channel, 12 client, and 7 receipt tests; semantic-query commands passed 8 of 8. This checkpoint was superseded by fixed-connector head 5362f3cb.",
    },
    {
      commit: "7eba36c2b578c12717f20de15140bbd423993195",
      title: "Return the exact PID-bound V2 compiler receipt",
      state: "implemented",
      detail:
        "fe2o3 PR #236 added rustc-owned FD195 client intake and exact PID-bound FD196 inert carriage return with a policy/subject join. This checkpoint was superseded by c6b53d97, which incorporates the bounded service path on current main.",
    },
    {
      commit: "002029af2e78e62d23df82a6fc4c74b71d9d2b9e",
      title: "Admit client readiness and document the Cargo bridge",
      state: "implemented",
      detail:
        "fe2o3 main added client-readiness admission, retained ServingProtectedIssuer pidfd custody, and documented the Cargo readiness bridge. Current PR #236 head 5362f3cb incorporates this upstream compiler/runtime checkpoint.",
    },
    {
      commit: "c7362d93d031f735ade33bf8bfa25ff8250e359b",
      title: "Bind the pending SwiGLU verifier request",
      state: "implemented",
      detail:
        "Ferric PR #21 binds the exact protected build, 22 target/draft profiles, ABI and launch semantics, and untrusted projected V2 carriage axes. The binder rejects current V1 evidence and grants no compiler, load, or launch authority. Ferric feature 7b3a05f1 later added source-level recovered-V2 custody without changing that authority boundary.",
    },
    {
      commit: "bc237da5af8dc1871f4cd0b963ad948a3e89d52f",
      title: "Advance receipt-bearing admission to current fe2o3 main",
      state: "implemented",
      detail:
        "fe2o3 PR #236 incorporated main 699803675d65ce8b93f2c80472ef86fd01fd4c08, where upstream took ownership of issuer pidfd lifecycle. This checkpoint was superseded by current integrated head 5362f3cb.",
    },
    {
      commit: "1ec0d2407f84733f17d24f7049d640b7ba4c71c7",
      title: "Record protected Worker V3 build evidence",
      state: "implemented",
      detail:
        "Ferric PR #21 records the exact protected-build evidence under SHA-256 a1b96acf0b9f32f5f02f0a5c92920df4f24502af674a4d05129eb0c902960572. The record grants no production verifier, load/dispatch, or numerical authority. The exact gfx942 numerical result remains separate qualification-only evidence at 1b77cb5.",
    },
    {
      commit: "2cb6e4390251d56f8c92cefc6873126043c5efeb",
      title: "Require receipt-bearing Worker V3 admission",
      state: "implemented",
      detail:
        "Direct-to-main fe2o3 PR #236 made Cargo, application, and host recovery V2-only and retained borrowed compiler-execution receipt carriage through inert admission. This early fail-closed checkpoint was superseded by current issuer-service, receipt-session, and connector source at 5362f3cb; deployment and authority joins remain open.",
    },
    {
      commit: "1b77cb5b82e370ca9a46c04d4465d2ba61737d01",
      title: "Qualify the Worker V3 SwiGLU numerics on gfx942",
      state: "observed",
      detail:
        "The qualification-only HIP harness loaded and dispatched HSACO 57ecb86b...fc6afa7 on gfx942:sramecc+:xnack-. It reported elements=3072, exact=3072, max_ulp=0, and mismatches_gt_1ulp=0. Production Worker V3 verification, KFD load/dispatch, and Qwen inference remain unexercised and unauthorized.",
    },
    {
      commit: "57f6cfdf4b3f5177a556159d1e548b25b63a1541",
      title: "Complete the mapped SwiGLU protected build",
      state: "observed",
      detail:
        "The Ferric source completed the protected two-phase build on MI300X/gfx942:xnack-. It produced semantic identity fe2o3::semantic::fe2ce53206f36841ea363da0db20869f38e21b881241581f455eebf8e9569e8f, exact 14,192-byte HSACO 57ecb86b...fc6afa7, and load-envelope lineage 058b9498...b17014a8. Production verification, GPU load/dispatch, numerical validation, and Qwen execution remain blocked.",
    },
    {
      commit: "21e4c10609a7b44687153fc3484d1156b4eb4def",
      title: "Advance the mapped compiler branch with protected issuer custody",
      state: "implemented",
      detail:
        "fe2o3 PR #236 incorporated origin/main e39fdc140cb5af7560084e34890da74d2c172163, retaining the mapped KFD adapter and protected issuer custody. cargo test -p cargo-fe2o3 --locked passes. This compiler result does not grant Ferric GPU execution authority.",
    },
    {
      commit: "a8df223a6c4d319e998dfa45f674cfd6c5ab5afc",
      title: "Expose the exact mapped KFD adapter on the host",
      state: "implemented",
      detail:
        "Ferric PR #21 exposed the exact generated Worker V3 KFD arguments. The binding-only gate passed 1 library, 3 numerical reference, and 7 source/adapter contract tests. The later protected build completed at 57f6cfdf; production verifier and KFD execution authority remain open, while the separate qualification-only HIP result is recorded at 1b77cb5.",
    },
    {
      commit: "4ad348404a57a2823b199a97bea13baae0f3de18",
      title: "Advance and authorize the mapped adapter source",
      state: "implemented",
      detail:
        "fe2o3 PR #236 incorporates the latest upstream compiler/runtime changes and authorizes the exact mapped Worker V3 device source for external builds. Compiler/runtime ownership remains in fe2o3; Ferric retains the Qwen kernel and inference integration.",
    },
    {
      commit: "06c74c64506f15883d64c5ab2ca476561909181d",
      title: "Generate exact mapped Worker V3 KFD arguments",
      state: "implemented",
      detail:
        "fe2o3 generates the exact KFD-only adapter for mapped disjoint slices and retains the compiler-proven index-space identity through host packing and writeback. The adapter alone grants no production artifact or dispatch authority.",
    },
    {
      commit: "5c963cba",
      title: "Advance the compiler integration to current main",
      state: "implemented",
      detail:
        "The Ferric compiler branch merged fe2o3 main b94f30eb, including canonical compiler issuer-key custody. The complete HSACO finalizer suite and all 376 backend library tests remained green. This intermediate integration was superseded by compiler 21e4c106 and the protected build at Ferric 57f6cfdf.",
    },
    {
      commit: "7e1c36aa35d743478772ce4bff14c4f4bbff85c0",
      title: "Produce the first Worker V3 production artifact",
      state: "observed",
      detail:
        "The identity-bound production build for qwen3_swiglu_bf16_f32_v1 succeeded on MI300X/gfx942:xnack- with fe2o3 compiler 4cd2af64645e57bdb3902ac2618baefeb3cb8722. It published the readiness claim, envelope, receipt, semantic KIR identity, and 14,192-byte HSACO identity. Dynamic symbols include the protected kernel, .kd, and defined weak __ocml_exp_f32. Load, dispatch, numerical and performance results, whole-Qwen execution, and M1 completion remain open.",
    },
    {
      commit: "4cd2af64645e57bdb3902ac2618baefeb3cb8722",
      title: "Bind production LLVM to the measured worker layout",
      state: "implemented",
      detail:
        "The fe2o3 compiler branch at 4cd2af64645e57bdb3902ac2618baefeb3cb8722, atop main 8078fa1d at the run, binds rustc's reviewed target layout to the exact measured ROCm LLVM worker spelling. The subsequent Ferric production run succeeded and produced the first replacement artifact.",
    },
    {
      commit: "2c7668d2",
      title: "Preserve the blocked access terminal in release builds",
      state: "implemented",
      detail:
        "Diagnostics proved that release inlining erased the trusted ThreadIndex::checked_block and DisjointSlice<Blocked>::get_block_mut terminals before checked-reference projection. fe2o3 PR #234 preserves both terminals and passes the full fe2o3-device suite, 372 compiler library tests, an integration-test build locally, and its pinned-nightly optimized extraction regression on MI300X. That scoped regression established compiler behavior; the production artifact followed at Ferric commit 7e1c36aa.",
    },
    {
      commit: "24c078e",
      title: "Start the Worker V3 SwiGLU port",
      state: "implemented",
      detail:
        "The compact standalone device target now at Ferric commit 7e1c36aa35d743478772ce4bff14c4f4bbff85c0 preserves the 48-byte BF16-carrier ABI, 256-workitem launch, eight contiguous elements per workitem, and all 15 admitted extents. The first family has a production HSACO; six families and all load, dispatch, numerical, performance, Qwen, evidence, and M1 gates remain open.",
    },
    {
      commit: "58fd37e",
      title: "Authenticate production lifecycle release",
      state: "qualified",
      detail:
        "The strict same-source wrapper passes 645 admitted proof bodies, 1,490 verification queries, 37 exact-body mutations, and 10 default/all-feature quality gates. Its SHA-256-bound receipt qualifies the release proof only; hardware, Qwen correctness, performance, and M1 closure remain open.",
    },
    {
      commit: "8940b28",
      title: "Generalize production queue rollover",
      state: "implemented",
      detail:
        "Four exact paired-prefill transitions retain roster-indexed caches, KV reservations, outputs, provider inputs, shape-specific queue submission, retry custody, and fail-closed catalog admission. Hostile output failures retain earlier move-only reserves. The full workspace gate passes; GPU execution remains open.",
    },
    {
      commit: "c593009",
      title: "Admit finite speculative serving shapes",
      state: "implemented",
      detail:
        "Production serving and same-shape rearm cover four exact shapes, including nonempty S8 live prefixes through 8 and history bound to plan, epoch, and ordered live roster. The 428-pass host result is structural only: no new GPU run, Qwen correctness, performance proof, or formal M1 qualification claim.",
    },
    {
      commit: "9aa7e60",
      title: "Generalize speculative evidence capture",
      state: "implemented",
      detail:
        "Allocation, binding, readback, and rearm preserve exact active-lane authority across four supported shapes while inactive fixed-width rows remain non-authoritative padding. The host result grants no hardware or qualification authority.",
    },
    {
      commit: "a9d1d7e",
      title: "Compose the two-round bridge lifecycle",
      state: "implemented",
      detail:
        "The structural fixture compiles through the real generic bridge from paired prefill through a second native round and atomic teardown with two-round history. It has not run on GPU and makes no Qwen correctness or qualification claim.",
    },
    {
      commit: "e6ccd4d",
      title: "Bind repeated speculative rearm",
      state: "implemented",
      detail:
        "An opaque bridge authority binds physical quiescent custody with the coordinator outcome and feeds repeated S1/K4 rearm from the committed result. Repeated hardware execution remains open.",
    },
    {
      commit: "e29112e",
      title: "Feed rollover from prefill readback",
      state: "implemented",
      detail:
        "The lifecycle path consumes bridge-bound readback rather than caller-authored rollover inputs. Its ignored MI300X fixture compiles but has not run on hardware.",
    },
    {
      commit: "c4404c8",
      title: "Add queued physical input provider",
      state: "verified",
      detail:
        "A concrete move-only provider binds every generation to its exact plan, epoch, and roster; prepares first publication, same-shape rearm, and S1/K4 rollover; and retains typed custody on every rejection.",
    },
    {
      commit: "5051113",
      title: "Publish native S1/K4 queue rollover",
      state: "verified",
      detail:
        "A completed S1 paired-prefill queue can transition into exact S1/K4 through fresh KV reservations and workspaces, preallocated independent output, native predecessor destruction, and a generation-bound replacement observation.",
    },
    {
      commit: "2d071fb",
      title: "Update the fe2o3 compiler revision",
      state: "verified",
      detail:
        "Pinned Ferric to fe2o3 2317f300, which adds verified rooted tensor-layout composition while preserving the previously qualified KFD and service-host runtime trees byte-for-byte.",
    },
    {
      commit: "6fd41b9",
      title: "Bind direct serving evidence",
      state: "verified",
      detail:
        "Direct and speculative paths retain exact serving plans and independent diagnostic choices, reject generic evidence bypass, and replace host-visible choice buffers with fresh sentinel images on every rearm generation.",
    },
    {
      commit: "1c685e4",
      title: "Type terminal serving custody",
      state: "verified",
      detail:
        "Terminal adapter failures now retain the provider and exact lower owner, derive stages structurally, and expose typed diagnostic queue teardown without reopening serving authority.",
    },
    {
      commit: "375f23a",
      title: "Bound rearm submission stack frames",
      state: "observed",
      detail:
        "Moved shape-specific fixed batches behind bounded stack frames. The exact commit subsequently completed target-only generation on the normal host stack.",
    },
    {
      commit: "cc84dcb",
      title: "Retain caches on publication failure",
      state: "verified",
      detail:
        "Publication rejection now returns selected cache custody with the lower failure instead of losing ownership context.",
    },
    {
      commit: "e09baf0",
      title: "Add fail-closed physical serving adapter",
      state: "verified",
      detail:
        "Added identity-branded lifecycle custody, terminal quarantine, exact S1/K4 dispatch, same-shape rearm, and diagnostic history.",
    },
  ],
  evidence: {
    summary:
      "Ferric treats implementation, authenticated proof release, hardware, Qwen correctness, performance, independent validation, and M1 qualification as separate authorities.",
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
      ["qualified", "All required identity-bound gates accepted the exact artifact."],
    ],
  },
  boundaries: {
    ferric: [
      "Model and tokenizer admission",
      "Qwen graph and execution planning",
      "Ferric-owned inference kernels",
      "Mapped build 57f6cfdf, qualification 1b77cb5, evidence record 1ec0d240, request binder c7362d93, and recovered-V2 custody feature 7b3a05f1 in repinned PR #21 head 41dfb3b2",
      "Scheduling, paged KV, and speculation",
      "Generated runner and qualification policy",
    ],
    fe2o3: [
      "Reusable compiler APIs",
      "Artifact and target identities",
      "Typed allocations and host transfers",
      "Long-lived HSA queue and KFD runtime",
      "Generic bounded command publication",
      "Direct-to-main fe2o3 PR #236 at 5362f3cb pins main 5706e3a1 and supersedes intermediate PRs #219-#235",
      "Bounded issuer service, V2 receipt return, and the fixed authenticated connector are integrated in source; Cargo invocation, deployed authority service, and backend FD195/196 exchange remain open",
      "Current protected build used Ferric source 57f6cfdf; compiler/runtime ownership remains in fe2o3",
    ],
  },
});
