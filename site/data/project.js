window.FERRIC_PROJECT = Object.freeze({
  updated: "2026-08-26",
  repository: "https://github.com/harsh-nod/ferric",
  milestone: {
    name: "M1",
    label: "Qwen3 speculative inference on one gfx942",
    state: "integration",
    summary:
      "The target-only physical path now generates tokens on MI300X. The speculative serving path and the qualification evidence set remain in progress.",
  },
  envelope: [
    ["Target", "Qwen3-8B"],
    ["Draft", "Qwen3-0.6B"],
    ["Device", "1 x gfx942"],
    ["Precision", "BF16 / FP32 accumulate"],
    ["Context", "up to 8K tokens"],
    ["Concurrency", "up to 32 sequences"],
    ["Runtime", "direct HSA command batches"],
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
      detail: "Fail-closed physical adapter exists; multi-round and rollover closure are active work.",
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
          "Direct HSA publication, completion readback, queue rearm, KV ownership, and token decode on MI300X.",
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
          "A physical lifecycle adapter owns fresh publication and same-shape rearm with independent choice readback.",
      },
      {
        name: "Paired prefill and shape rollover",
        detail:
          "Typed paths exist in pieces; end-to-end production rollover and custody closure are not finished.",
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
    {
      commit: "cf14c93",
      title: "Quarantine terminal serving failures",
      state: "verified",
      detail:
        "Hostile bridge tests cover terminal readback and settlement failures plus retryable custody recovery.",
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
