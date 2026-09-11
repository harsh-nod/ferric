# Competitiveness Sprint V1

Status: in progress. No matched vLLM/SGLang result or competitive claim yet.
The prior measurements remain frozen in `M1_PERFORMANCE_SPRINT_V2.md`.

## Active Teams

| Track | Deliverable | State |
| --- | --- | --- |
| Core runtime | Opt-in shared fresh full-topology observation per peer boundary, preserving all per-rank checks | Reviewed and published3e3a77284;475 library tests+31 doctests and scoped Clippy pass; native gate pending |
| Kernels | Additive 32-row FP32 LM head/argmax and fast-path integration | Integrated; exact3e3 emission,9 kernel tests,191 adapter tests+2ignored,22 policy tests and scoped Clippy pass;15 native fixtures running |
| Serving | Bounded sustained JSONL ingress, wall-clock arrival, token output, cancellation/backpressure | Foundation integrated7a84682; preliminary host tests pass including65 admissions and prefix reuse; loopback HTTP adapter in progress |
| Integration/measurement | Shared streaming benchmark client, baseline identities, GPU scheduling, review and numerical gates |20 client tests pass;6 candidate-checker tests pass, pin-drift followup in test; baseline launch approval requested |
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

At intake, both hosts had approximately 85GiB free. Cached baseline images:

- vLLM 0.28.0+rocm723: `sha256:c5f9efa2623d9c8b2e483d2a139202e496baf5a04900a4f32907149f41a42dba`.
- SGLang 0.5.15.post1.dev20260715+g495ae9aaa6: `sha256:cb8089ca16bd9182698b1bb5a915e6982bf9eeff63d2f6d027a9c31d8d6279d3`.

Fresh fe2o3 origin/main at intake was `310ce7b8c`; its delta from the previous
Ferric pin `6f6a67bb2` was test-only cleanup. Root repinned to310 at7c6edeb,
then to the reviewed published runtime3e3a77284 at1ba3e01. Combined validation
and structural dependency-inventory regeneration are pending. Preliminary
host gates and frozen511 GPU ablations retain their actual provenance.

## First Target-Path Ablation

The existing immutable kernel-admission cache was tested with the now-working
TP1 MFMA plus FP32-head profile. The earlier cache experiments used TP8 and
did not establish this combination's effect. These runs keep the identical
controller, images, worker, four-request/eight-output canary, scheduling and
physical GPU. Only `--runtime-cache-admission` changes. Run order was control1,
cache1, cache2, control2. All four pass unchanged full token/byte references,
worker exit and identity-bound all-eight-GPU idle checks.

| Mode | Run | Output tokens/s | Workload s | Reuse TTFT ms | Reuse TPOT ms | Whole process s |
| --- | --- | --- | --- | --- | --- | --- |
| Control |1|2.005281|3.989466|639.762|493.493|131.580225|
| Cache |1|2.481748|3.223534|485.988|316.198|135.195576|
| Cache |2|2.635293|3.035715|462.558|303.700|128.176270|
| Control |2|2.001220|3.997561|643.552|492.986|128.812970|

Across the two observations per mode, mean output rate improves27.72%, reuse
TTFT falls26.09% and reuse TPOT falls37.16%. Mean whole-process time is1.14%
slower because setup varies. These are short engineering observations, not
steady-state serving, confidence intervals, a new default or a vLLM/SGLang win.
The frozen controller is3d039c49 (Ferric00fa58d/public511), worker aaa0216a,
base image8c81d3fe and separate head image d6086650. They are not relabeled as
the active3e3 source. Raw receipts are retained in the root-owned
`ferric-compete-evidence-v1/tp1-mfma-{control,cache}-r{1,2}` evidence directories.
