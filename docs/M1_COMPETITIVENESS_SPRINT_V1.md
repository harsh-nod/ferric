# Competitiveness Sprint V1

Status: in progress. No matched vLLM/SGLang result or competitive claim yet.
The prior measurements remain frozen in `M1_PERFORMANCE_SPRINT_V2.md`.

## Active Teams

| Track | Deliverable | State |
| --- | --- | --- |
| Core runtime | Opt-in shared fresh full-topology observation per peer boundary, preserving all per-rank checks | Published `3e3a77284`; 475 library tests, 31 doctests and scoped Clippy pass; native TP2/TP8 producer fixtures pass with the option off and on |
| Kernels | Additive 32-row FP32 LM head/argmax and fast-path integration | Integrated; exact `3e3` emission and 15 native fixtures pass; full model canary in progress |
| Serving | Bounded sustained JSONL ingress, wall-clock arrival, token output, cancellation/backpressure | JSONL and loopback HTTP integrated; combined gate passes 289 Rust tests, 16 HTTP tests and strict Clippy; real HTTP smoke pending |
| Integration/measurement | Shared streaming benchmark client, baseline identities, GPU scheduling, review and numerical gates | 20 client tests and 7 candidate-checker tests pass, including checksum drift rejection; baseline launch approval requested |
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
boundaries and cannot silently share a ranking. The shared Ferric HTTP adapter
is implemented; open-loop queue-inclusive SLO qualification remains separate work.

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
Ferric pin `6f6a67bb2` was test-only cleanup. Root repinned to `310` at `7c6edeb`,
then to the reviewed published runtime `3e3a77284` at `1ba3e01`. The final combined
controller passes 206 library, 55 batch CLI, 6 replica-control and 22 source-policy
tests, with three preexisting ignores, plus strict Clippy and release build.
The separate source/dependency gate at `121609f` passes 38 source-gate tests,
31 verifier policies, 28 metadata configurations, five unchanged generated
inventories and negative/release policies. Two stale verifier raw-file checksums
were independently audited and refreshed at `4182cad`; acceptance logic did not
change. Coverage remains 173 modules and 8,235 executable bodies. This is not a
new Verus proof or model qualification. Preliminary host gates and frozen `511`
GPU ablations retain their actual provenance.

## Native And Serving Gates

The additive v8 image passes scalar head, MFMA head and FP32 argmax fixtures at
1, 16, 17, 31 and 32 rows. Full-vocabulary outputs, immutable inputs, inactive
tails and surrounding guards are checked. Image `5f19b3ba` and worker `933d73d2`
retain exact `3e3` provenance. The root-reviewed result hash is
`5e27c9f5175321c570954b601d7d1cd0d5d14fd330ab347a4f9376ec1cf0a85a`.

Shared-currentness producer tests pass on TP2 and TP8, separately off and on,
using the same frozen base/peer images. These direct-core fixtures establish
GPU producer/peer-consumer ordering and checked teardown, not worker-wire,
concurrent-round, model or speed qualification. All native runs finish with
identity-bound idle checks across all eight GPUs.

The new HTTP path is loopback-only raw-prompt greedy streaming, with bounded
admission, cancellation, backpressure, deadlines and owned worker cleanup.
The common client passes fake-child ordinary and split-UTF8 streams. The real
smoke uses three complete reference cases and repeats the first, nine outputs
total, sequentially with prefix caching off. It is not the static cancellation
workload, a concurrent load test or a physical queue-rollover qualification.

The physical KV pool remains capped at 512 pages. Conservative reservations
allow at most 32, 6 and 1 active requests at ISL/OSL 128/128, 1024/256 and
4096/256 respectively. The [large-pool plan](M1_LARGE_KV_POOL_PLAN_V1.md) describes
the required coordinated host/kernel expansion; it is not implemented.

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

Across the two observations per mode, mean output rate improves 27.72%, reuse
TTFT falls 26.09% and reuse TPOT falls 37.16%. Mean whole-process time is 1.14%
slower because setup varies. These are short engineering observations, not
steady-state serving, confidence intervals, a new default or a vLLM/SGLang win.
The frozen controller is `3d039c49` (Ferric `00fa58d` / public `511`), worker
`aaa0216a`, base image `8c81d3fe` and separate head image `d6086650`. They are not
relabeled as the active `3e3` source. Raw receipts are retained in the root-owned
`ferric-compete-evidence-v1/tp1-mfma-{control,cache}-r{1,2}` evidence directories.
