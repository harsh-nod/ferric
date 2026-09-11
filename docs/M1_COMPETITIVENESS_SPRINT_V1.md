# Competitiveness Sprint V1

Status: in progress. No matched vLLM/SGLang result or competitive claim yet.
The prior measurements remain frozen in `M1_PERFORMANCE_SPRINT_V2.md`.

## Active Teams

| Track | Deliverable | State |
| --- | --- | --- |
| Core runtime | Opt-in shared fresh full-topology observation per peer boundary, preserving all per-rank checks | Implementing; fe2o3 owns reusable runtime code |
| Kernels | Additive 32-row FP32 LM head/argmax and fast-path integration | Implementing; Ferric owns kernels and inference |
| Serving | Bounded sustained JSONL ingress, wall-clock arrival, token output, cancellation/backpressure | Implementing; engineering interface, not HTTP or production qualification |
| Integration/measurement | Shared streaming benchmark client, baseline identities, GPU scheduling, review and numerical gates | Implementing; baseline launch approval requested |
| Speculation | Draft execution plus target verification and accepted-prefix KV integration into the fast path | Queued after target-path integration; not a completed performance feature |

## Frozen Comparison Contract

Start with Qwen/Qwen3-8B BF16, raw completion prompts, greedy decoding, fixed
output lengths, no chat-template transformation, target-only execution, and
prefix caching disabled. Use the identical canonical checkpoint/tokenizer.
First compare one GPU; separately compare the best allocation of eight GPUs,
including replicas rather than requiring TP8 for every engine. Do not compare
the old tick-driven four-request canary rate to a different serving workload.

The shared `competitive_benchmark.py` client records bounded closed-loop
windows, exact workload/client/identity hashes, per-request failures, final
server token usage and raw SSE arrival times. TTFT is client send to the first
nonempty text chunk; TPOT uses first/last text arrival and the server token
count. Chunk intervals are NOT per-token ITL, particularly under speculation.
An HTTP baseline and an in-process Ferric timer are different measurement
boundaries and cannot silently share a ranking. Open-loop queue-inclusive SLO
qualification and a shared Ferric HTTP adapter remain separate work.

Every run is explicitly non-qualifying. Passing a short canary or collecting
30 windows alone does not satisfy `PERFORMANCE.md`: equal baseline tuning,
held-out workloads, three fresh starts, numerical checks, controlled hardware,
paired confidence intervals and the entire declared primary suite still apply.

## Resource And Publication Rules

Build/test/format only on mi300x, private stages, jobs2, no local builds.
Root serializes all mi350 GPU use after identity-bound idle checks. Never kill
foreign jobs or delete shared models/caches/images. Archive evidence before
removing owned stages/worktrees. Ferric implementation remains local; only
separately reviewed Pages content may be published. fe2o3 updates must rebase
onto freshly fetched main before their authorized non-forced push.

At intake, both hosts had approximately85GiB free. Cached baseline images:

- vLLM0.28.0: `sha256:c5f9efa2623d9c8b2e483d2a139202e496baf5a04900a4f32907149f41a42dba`.
- SGLang0.5.15.post1 ROCm720 MI35x: `sha256:cb8089ca16bd9182698b1bb5a915e6982bf9eeff63d2f6d027a9c31d8d6279d3`.

Fresh fe2o3 origin/main at intake: `310ce7b8c`; its delta from the current
Ferric pin `6f6a67bb2` is test-only cleanup. Aggregate repin and validation are
pending; previous GPU results are not relabeled.
