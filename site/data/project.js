window.FERRIC_PROJECT = Object.freeze({
  updated: "2026-08-27",
  repository: "https://github.com/harsh-nod/ferric",
  milestone: {
    name: "M1",
    label: "Qwen3 speculative inference on one gfx942",
    state: "integration",
    summary:
      "Four finite speculative shapes and four exact production rollover transitions pass the authenticated host gate at 58fd37e. The scoped release proof also passes for that exact source. Current-source MI300X validation, Qwen correctness and performance evidence, independent validation, and formal M1 qualification remain open.",
  },
  envelope: [
    ["Target", "Qwen3-8B"],
    ["Draft", "Qwen3-0.6B"],
    ["Device", "1 x gfx942"],
    ["Precision", "BF16 / FP32 accumulate"],
    ["Context", "up to 8K tokens"],
    ["Concurrency", "up to 32 sequences"],
    ["Runtime", "compatible fe2o3 pin 2317f300 / direct HSA command batches"],
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
        "No complete M1 evidence index or M1 qualification receipt exists. The scoped proof-release receipt does not close M1.",
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
          "Exact target/draft plans, typed operations, workspace layouts, and Ferric-owned kernel artifacts.",
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
    ],
    roadmap: [
      {
        name: "Current-source MI300X lifecycle run",
        detail:
          "Source 58fd37e has not run its generic rollover path on MI300X. The latest recheck could not resolve SSH host 300x; the last known device state had unrelated /dev/kfd use. No new hardware authority exists.",
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
    title: "Normal-stack target-only smoke passed",
    date: "2026-08-26",
    state: "observed",
    commit: "375f23a",
    buildId: "fab09edf144588f4c3c82f90d49a789aca4fa762",
    environment: "MI300X / gfx942, default 8,388,608-byte host stack",
    result: "Exit 0; GPU allocations released cleanly",
    generatedTokenIds: [138955, 62696, 41213, 138070],
    authority:
      "Smoke observation only. It is not evidence, numerical qualification, hardware qualification, performance qualification, or M1 closure.",
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
      title: "Generic production rollover",
      state: "open",
      source: null,
      result: "Current-source MI300X execution pending",
      detail:
        "The latest access recheck could not resolve SSH host 300x; the last known device state had unrelated /dev/kfd use. Source 58fd37e has not run on gfx942, so no Qwen correctness, numerical, performance, or qualification claim follows.",
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
      "Compatible pin 2317f300; newer remote main is not API-compatible",
    ],
  },
});
