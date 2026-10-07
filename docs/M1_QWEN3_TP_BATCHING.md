# Qwen3 TP Batching Engineering Path

This additive opt-in path joins continuous request scheduling, real multi-row
prefill/decode dispatch, paged causal GQA, and persistent complete-page radix
prefix retention. It targets Qwen3-8B BF16 on one, two, or eight physical gfx950
GPUs. The existing single-sequence thirteen-root profile is unchanged.

Implementation is in Ferric; compiler and KFD dependencies use public fe2o3
`3546d54d2c4a913f5d079701aed557d0a378bba8`. This is Contracted engineering
execution, not protected M1 authority, a Verus refinement, or an HTTP server.
The combined release host checks and all-targets strict Clippy pass. Both
target images emit, eleven synthetic gfx950 GPU probes pass, and the complete
TP1 and TP8 mixed-request workloads match the frozen reference, including
TP8 with prefix retention both enabled and disabled. Previous single-sequence
TP8 timings must not be attributed to this new path.

## Data Flow

One controller uploads each weight shard once and retains one worker per
physical GPU. Between GPU batches it admits new requests, retires completed
ones, and processes cancellation. The scheduler selects up to sixteen token
rows, reserving progress for both decode and prefill. A prefill chunk may
contain multiple contiguous prompt positions but does not cross into decode.
Only a prompt's final row or a decode row publishes a generated token.

The pool atomically reserves physical pages for the selected rows. Page IDs
are shared across all ranks/layers, while KV contents remain rank-local.
Each attention row uses its own absolute position and logical-to-physical
table, masking future positions before reading page entries or KV. GEMM
executes multiple rows in one GPU launch; this is not a host loop of m=1
transformer passes. Ordered host FP32 reductions combine active rows, add
the residual once, and round once to BF16.

Every rank and layer must finish before completion is accepted. Dispatch
counts, choice cardinality/vocabulary, and controller time are preflighted
before request/page metadata advances. A submitted failure permanently
quarantines the pool and poisons the request stream; closing the workers is
the only recovery. An unsubmitted page shortage may evict unused cache pages
and retry a smaller token budget without advancing requests.

## Prefix Retention

Completed requests can leave immutable sixteen-token pages in a bounded radix
tree keyed by exact tokens and bound to the model and resident GPU session.
The actual physical KV pages remain resident after the request is retired.
Admission retains the longest complete-page prefix, always replaying at
least the last prompt token to obtain logits. Partial pages are never shared.
Cancellation publishes no prefix. Live references prevent eviction/reuse;
unused cache leaves use deterministic TTL and oldest-leaf-first eviction.

This is page-granular prefix caching plus paged causal attention, not SGLang's
complete cache/scheduler implementation or a claim of feature/performance
parity. There is no cross-process, cross-model, or cross-session cache reuse.

## Workload Runner

The binary is `ferric-qwen3-tp-batch-engineering`, behind the separate
`tp-batch-engineering` Cargo feature in the standalone engineering adapter.
It accepts the exact new nine-root engineering artifact and explicit
`--allow-unauthenticated-machine-code`; the old thirteen-root artifact is
rejected. Build only on `mi300x` in a private stage. The host graph uses
Rust 1.97.1 with `RUSTC_BOOTSTRAP=fe2o3_macros,fe2o3_device` restricted to the
two pinned SDK crates requiring nightly features; GPU emission uses the
separately pinned compiler toolchain. Neither changes the protected workspace.

An example workload is
[`tp8-continuous-prefix-v2.json`](../adapters/m1-engineering-execution-v1/workloads/tp8-continuous-prefix-v2.json).
It uses four staggered arrivals, a 17-token prefix seed, a 19-token reuse
request, short decoding, and cancellation between batches. Prompt token IDs
must be checked against the frozen independent reference before interpreting
the output comparison.

```sh
ferric-qwen3-tp-batch-engineering \
  --source MODEL_SOURCE --artifact EXACT_NINE_ROOT_ARTIFACT \
  --worker ENGINEERING_WORKER --devices ID0,ID1,ID2,ID3,ID4,ID5,ID6,ID7 \
  --requests tp8-continuous-prefix-v2.json \
  --allow-unauthenticated-machine-code \
  --batch-tokens 16 --prefill-chunk 16 --context 64 --pages 8 \
  --cache-ttl 1024 --max-batches 16
```

The runner emits JSONL setup identities, admission/cache hits, actual selected
rows and committed outputs, per-request tokens/decoded bytes/latencies, and
confirmed worker teardown. Repeat with `--disable-prefix-cache` for uncached
output comparison. All observations carry `authority: none`.

Limits are 1-16 rows, 1-32 workload requests, context at most 8192 tokens,
1-512 physical pages, and at most 240 batches in the current no-ring-rollover
profile. A smaller user batch budget remains binding. Chunk size cannot
exceed the row budget. Requests generate a fixed 1-256 tokens unless cancelled.
The library supports continued admission after retiring request slots; the
CLI workload itself is bounded to 32 requests. Sustained service beyond the
packet bound is not implemented.

Arrival/cancellation ticks are logical completed-batch boundaries, not an
external load generator's wall-clock schedule. TTFT begins at actual admission
and includes in-runtime waiting; TPOT uses adjacent committed output times.
Setup is separate. Page backpressure that cannot fit even one row is a
controlled CLI error; the library supports explicit cancellation and retry.
These workload latencies are not a controlled comparison with vLLM/SGLang.

## September 10 GPU Results

The same four-request workload passes on TP1 with caching and on TP8 with
caching enabled and disabled. Each run produces eight exact expected output
token IDs and decoded bytes from frozen reference subsequences. These are
short mixed-request checks, not new 32-token runs. Both TP8 traces exercise
16-row prefill, mixed decode/prefill, a page-16 crossing, retirement and
generational slot reuse while another request remains active, and cancellation
after exactly one output. The cached run reuses an actual retained KV page.

| TP8 Observation | Cache On | Cache Off |
| --- | ---: | ---: |
| Committed GPU batches | 5 | 6 |
| Physical token rows | 34 | 50 |
| Reused prefix tokens | 16 | 0 |
| Rank-0 dispatches | 2,720 | 3,264 |
| Dispatches per other rank | 2,700 | 3,240 |
| Setup, seconds | 189.758861 | 192.698332 |
| Whole run, seconds | 466.414186 | 461.166303 |

| Request | Cache-On TTFT (s) | Cache-Off TTFT (s) | Cache-On TPOT (s) | Cache-Off TPOT (s) |
| --- | ---: | ---: | ---: | ---: |
| seed-prefix | 113.270822 | 85.731951 | 53.756931 | 44.195118 |
| arriving-short | 54.762319 | 41.184784 | 50.541258 | 44.769957 |
| cancel-between-batches | 53.756866 | 44.195060 | N/A | N/A |
| reuse-prefix | 47.325505 | 86.270679 | 47.801794 | 37.697435 |

Prefix retention demonstrably avoids sixteen token rows and one full forward
batch. It does not establish a latency speedup: whole-run time is longer with
caching in this pair. These are separate, single, unrepeated logical-tick
workloads. Except for the short request's two intervals, each TPOT describes
only one interval. The cancelled request has no post-first interval.

Raw JSONL, comparator reports, exact source history, host checks, and an
independent paired review are retained in the local
`ferric-tp8-batch-evidence-v1` archive. Both TP8 runs exit zero; all seventeen
worker PIDs across the three model runs were independently confirmed absent,
with all eight GPUs idle afterward. The small runnable bundle remains at
`mi350:/tmp/ferric-tp8-batch.Yn5F9M`, using the retained model source at
`mi350:/tmp/ferric-qwen8.TRNKht/models`.

Exact identities, all SHA-256:

- Controller: `098be2f8425bffcc46545ba66f313a3c63090eb01360eccafb276d887350e151`.
- KFD worker: `77a53d18b56e4ee7a67a434feffa8ac18a4fe60f8c9e5daace351f502c72e1da`.
- gfx950 HSACO: `af5019d3cfc4e860b33ebf0d97a82439f870a9e4e893c730118a97d8735c2d6a`.
- Cache-on JSONL: `562cf9d5fd8dbb737ddc92ef928913f06625f6f6992ddac66796e90b80595981`.
- Cache-off JSONL: `d7d7d646374b7ec07e190d8f17d06772829a30c1c6089ef566f45e9e0e57ca87`.
- Cache-on comparison: `340b2bd61a8b05bbdca45f4ed9151b28acd8a2f1ee4351b7310e76ccfc456460`.
- Cache-off comparison: `a13146920a0ad686b7f477e4605fc5af29c3d1988b31d862f5448e51036b44ac`.

Implementation commits `7224c33` and `65cb435` are local, not published
implementation releases. The old thirteen-root profile is byte unchanged.

## Validation Boundaries

Host checks cover transactional allocation, refcounts, cache isolation,
fairness, stale generations, cancellation, submission failure, precommit
validation, true batched command shape, and negative mutations. Kernel host
tests exercise source-linked arithmetic and page/row boundaries. They do not
replace actual GPU probes or model-token comparison. The bounded synthetic
GPU probe explicitly identifies the roots it exercises and excludes full
model numerical or benchmark claims.

The combined adapter passes 139 library tests (one preexisting real-image test
ignored), seventeen CLI/worker tests, twenty-two source policies, and
all-targets strict Clippy. Old-profile regression and default-feature checks
also pass. Kernel source/target checks pass thirty-two tests per target; the
unchanged imported RMSNorm body retains a baseline `let_and_return` Clippy
warning, so whole-kernel strict Clippy is not claimed clean. No new Verus proof
covers the scheduler, pool, batched driver, or numerical kernels.

Shared-host execution must first confirm the selected physical GPUs are idle,
use a unique private output directory and bounded process lifetime, then
confirm worker teardown and idle devices. Retain source/image hashes and raw
results before removing build stages. Never modify another user's processes,
models, runtime settings, or files.
