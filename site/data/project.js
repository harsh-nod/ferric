window.FERRIC_PROJECT = Object.freeze({
  updated: "2026-08-27",
  repository: "https://github.com/harsh-nod/ferric",
  milestone: {
    name: "M1",
    label: "Qwen3 speculative inference on one gfx942",
    state: "integration",
    summary:
      "Four finite speculative shapes and four exact production rollover transitions pass the authenticated host gate at 58fd37e. The scoped release proof also passes for that exact source. Ferric now has a protected production HSACO for the first of seven Rust Qwen3 kernel families: qwen3_swiglu_bf16_f32_v1. The identity-bound build succeeded on MI300X for gfx942:xnack-, and its readiness claim, envelope, and receipt were published. The artifact has not been loaded or dispatched. Six kernel families, the generated runner and model bundle, numerical and performance validation, independent evidence, whole-Qwen execution, and formal M1 qualification remain open.",
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
      "Ferric device 7e1c36aa / fe2o3 compiler 4cd2af64 atop main 8078fa1d at the production run",
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
        "The protected production build for qwen3_swiglu_bf16_f32_v1 succeeded on MI300X/gfx942:xnack- at Ferric device commit 7e1c36aa35d743478772ce4bff14c4f4bbff85c0 with fe2o3 compiler commit 4cd2af64645e57bdb3902ac2618baefeb3cb8722 atop main 8078fa1d at the run. The 14,192-byte HSACO, readiness claim, envelope, and receipt were published. This is the first of seven kernel families; it has not been loaded or dispatched, and six families plus every execution and M1 gate remain open.",
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
          "Exact target/draft plans, typed operations, workspace layouts, and Ferric-owned kernel specifications. SwiGLU now has a production Worker V3 artifact; the other six kernel families, generated runner, and model bundle remain in migration.",
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
          "The qwen3_swiglu_bf16_f32_v1 production build emitted KIR fe2o3::semantic::54361a526f73befabecd65a3a7dc0338ef8653d15209d3b47765356236f34dcc and a 14,192-byte HSACO with SHA-256 and filename identity 0a27ada84a6382331af6a16d4ed0be6fcf1f85333ca5087b908a64618062702a. Its ABI has three pointer-plus-length explicit arguments, 304 bytes of kernarg, workgroup size 256, 84 SGPRs, 11 VGPRs, and no spills or dynamic stack. Dynamic symbols include the protected kernel, its .kd descriptor, and defined weak __ocml_exp_f32. Production compilation grants no load, dispatch, numerical, performance, Qwen, or M1 authority.",
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
          "The protected production run succeeded for the SwiGLU kernel at Ferric device commit 7e1c36aa35d743478772ce4bff14c4f4bbff85c0 using fe2o3 compiler commit 4cd2af64645e57bdb3902ac2618baefeb3cb8722 atop main 8078fa1d at the run. The first family now has an identity-bound production artifact. Six kernel families, the generated runner and model bundle, artifact loading and dispatch, numerical and performance validation, independent evidence, and formal M1 closure remain open.",
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
    title: "First Worker V3 production artifact succeeds",
    date: "2026-08-27",
    state: "observed",
    commit: "7e1c36aa35d743478772ce4bff14c4f4bbff85c0",
    buildId: "HSACO SHA/filename 0a27ada84a6382331af6a16d4ed0be6fcf1f85333ca5087b908a64618062702a",
    environment: "MI300X / gfx942:xnack-; fe2o3 compiler 4cd2af64 atop main 8078fa1d",
    result: "PASS: qwen3_swiglu_bf16_f32_v1 production build; 14,192-byte HSACO",
    generatedTokenIds: [],
    authority:
      "Identity-bound production compilation only. The readiness claim, envelope, and receipt were published, but the HSACO has not been loaded or dispatched. No numerical result, performance result, whole-Qwen execution, hardware-execution evidence, or M1 qualification claim follows.",
  },
  validation: {
    host: {
      title: "Finite production rollover",
      state: "implemented",
      source: "58fd37e",
      result: "Default and all-feature debug/release workspace suites passed",
      detail:
        "The authenticated same-source wrapper passed fmt, default and all-feature clippy, both debug suites, both release suites, and the M1 benchmark, reference, and R29 differential policies.",
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
      title: "MI300X production artifact build",
      state: "observed",
      source: "7e1c36aa35d743478772ce4bff14c4f4bbff85c0",
      result: "PASS: qwen3_swiglu_bf16_f32_v1; HSACO 0a27ada8...62702a",
      detail:
        "The protected build emitted KIR fe2o3::semantic::54361a526f73befabecd65a3a7dc0338ef8653d15209d3b47765356236f34dcc and a 14,192-byte HSACO with SHA-256 and filename identity 0a27ada84a6382331af6a16d4ed0be6fcf1f85333ca5087b908a64618062702a. The published ABI records three pointer-plus-length arguments, 304-byte kernarg, workgroup size 256, 84 SGPRs, 11 VGPRs, and no spills or dynamic stack. The artifact has not been loaded or dispatched.",
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
      commit: "5c963cba",
      title: "Advance the compiler integration to current main",
      state: "implemented",
      detail:
        "The Ferric compiler branch merged fe2o3 main b94f30eb, including canonical compiler issuer-key custody. The complete HSACO finalizer suite and all 376 backend library tests remain green. The published SwiGLU artifact retains its exact production-run compiler identity 4cd2af64645e57bdb3902ac2618baefeb3cb8722; it has not been rebuilt or requalified under the newer merge.",
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
      "Scheduling, paged KV, and speculation",
      "Generated runner and qualification policy",
    ],
    fe2o3: [
      "Reusable compiler APIs",
      "Artifact and target identities",
      "Typed allocations and host transfers",
      "Long-lived HSA queue and KFD runtime",
      "Generic bounded command publication",
      "Production run used Ferric device 7e1c36aa and fe2o3 compiler 4cd2af64 atop main 8078fa1d; compiler/runtime ownership remains in fe2o3",
    ],
  },
});
