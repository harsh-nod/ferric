window.FERRIC_PROJECT = Object.freeze({
  updated: "2026-08-26",
  repository: "https://github.com/harsh-nod/ferric",
  milestone: {
    name: "M1",
    label: "Qwen3 speculative inference on one gfx942",
    state: "integration",
    summary:
      "The target-only path has an MI300X smoke observation. Output-fed paired-prefill rollover and committed-outcome-fed repeated S1/K4 rearm are implemented with green host validation; the repeated path has not run on hardware and is not evidence or M1 qualification.",
  },
  envelope: [
    ["Target", "Qwen3-8B"],
    ["Draft", "Qwen3-0.6B"],
    ["Device", "1 x gfx942"],
    ["Precision", "BF16 / FP32 accumulate"],
    ["Context", "up to 8K tokens"],
    ["Concurrency", "up to 32 sequences"],
    ["Runtime", "fe2o3 2317f300 / direct HSA command batches"],
  ],
  readiness: [
    {
      label: "Target-only prompt to text",
      state: "observed",
      detail: "Completed on MI300X with exact assets and the default 8 MiB host stack.",
    },
    {
      label: "S1/K4 speculative serving",
      state: "integration",
      detail:
        "Committed speculative outcomes can feed repeated same-shape rearm through an opaque bridge authority that inseparably binds physical quiescent custody and coordinator outcome. The full engine host suite passes 417 tests with 5 hardware cases ignored; strict workspace Clippy and docs are green.",
    },
    {
      label: "M1 qualification",
      state: "open",
      detail: "No evidence index or qualification receipt exists. M1 is not complete.",
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
        name: "Continuous S1/K4 speculation",
        detail:
          "Same-shape rearm validates the exact adapter, phase, epoch, plan, next roster, per-role committed KV cursors, shared next anchor, canonical target placeholders, empty provider queue, and pre-boxed failure custody.",
      },
      {
        name: "Paired prefill and shape rollover",
        detail:
          "Exact S1 paired-prefill to S1/K4 scheduling, output-fed successor enqueue, KV reselection, workspace replacement, output activation, native queue rollover, and terminal custody closure are implemented.",
      },
      {
        name: "Qualification capture tooling",
        detail:
          "Identity-bound diagnostic and evidence producers exist, but partial artifacts grant no qualification authority.",
      },
    ],
    roadmap: [
      {
        name: "Wider speculative shapes",
        detail:
          "S8/K4, K8, and K16 stay disabled until independent choice evidence and exact lifecycle coverage exist.",
      },
      {
        name: "Full M1 evidence closure",
        detail:
          "Proof, independent validation, hardware, performance, TCB, and receipt gates remain open.",
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
  recentProgress: [
    {
      commit: "e6ccd4d",
      title: "Bind repeated speculative rearm",
      state: "implemented",
      detail:
        "An opaque bridge authority inseparably binds physical quiescent custody with the coordinator outcome and feeds repeated S1/K4 rearm from the committed speculative result. No repeated hardware run has occurred; this makes no Qwen correctness, evidence, performance, or M1 qualification claim.",
    },
    {
      commit: "e29112e",
      title: "Feed native rollover from prefill output",
      state: "implemented",
      detail:
        "The opt-in lifecycle path is implemented and compiles, and host validation is green. Its ignored MI300X fixture also compiles but has not run on hardware. This is non-evidentiary and does not qualify M1 or claim Qwen correctness.",
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
      "Ferric treats implementation, observation, validation, proof, performance, and qualification as separate authorities.",
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
    ],
  },
});
