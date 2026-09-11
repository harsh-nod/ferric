# TP1 Large Physical KV Profile V9

This is an explicit, Contracted engineering profile. It does not extend the
verified logical sequence invariant, confer protected Engine custody, or qualify
long-context model numerics, GPU allocation headroom, or serving performance.
Native image and model receipts must be qualified separately.

## Selection

Select both `--kv-pool-profile large-kv-v9` and `--large-kv-artifact DIR`, with a
strictly admitted `v5-wave32` or `v5-mfma32` base image and an explicit
`fp32-v8` or `bf16-v8-control` head image. Only TP1, baseline paged attention,
and baseline/MFMA projection are admitted. The wave-only base image supports
baseline projection only. Wave arithmetic, Auto projection, peer transports,
dispatch sequences, numerical capture, and replica control remain rejected.
Live stdin still requires checked queue rollover; no legacy default changes.

The v9 image is structurally admitted before pool construction and loaded before
device buffers. The pool retains a sealed image binding. Driver preflight checks
the exact two successfully loaded v9 roots against that HSACO identity before
allocation. Legacy pool and driver constructors reject the v9 profile even with
only one actual page. A large-pool driver rejects a legacy pool or missing,
partial, foreign, stale, or unsupported image/transport binding.

## Independent Geometry

- Logical context: at most 8192 positions per request, unchanged.
- Page size: 16 tokens; logical page-table stride: at most 512, unchanged.
- Active requests: at most 32; physical dispatch rows: at most 32.
- Physical pages: 1..16384, only under the explicit v9 profile.
- Maximum aggregate physical slots: 262144, never passed as logical capacity.
- Maximum target TP1 K/V array payload: 38654705664 bytes across 36 layers.
- One maximum layer K or V array: 536870912 bytes, 268435456 BF16 elements.

All allocation products use checked arithmetic. The private execution storage
geometry separates physical tokens from the logical capacity supplied to the
unchanged `TensorParallelSequenceV1` constructor. Single-sequence and legacy
pool APIs retain their prior envelopes. Only append and baseline-attention
dispatches route to v9; the other v5 model and v8 head roots remain unchanged.

Setup adds exactly `kv_pool_profile`, `kv_pool_max_physical_pages`,
`kv_pool_payload_bytes`, and `kv_pool_artifact`. The artifact object records
`artifact_hsaco_id`, `artifact_manifest_id`, and `artifact_handoff_id`. Existing
`physical_pages` is the actual allocation count. Payload excludes weights,
transposes, workspaces, code, queue storage, and allocation rounding; the shared
machine still requires a separate actual-memory preflight.

## Ownership And Verification

Existing full pool invariants, transaction ordering, immutable cached pages,
exclusive partial pages, cancellation, refcounts, and permanent quarantine stay
unchanged. A prepared batch also carries the explicit physical profile, in
addition to the monotone pool identity and logical/physical geometry.

Focused CPU tests exercise real reservations beyond page512, an invariant-checked
full-pool boundary state reaching slot16383:15, immutable sharing of a high
physical cached page, stale-pool rejection, rollback and quarantine, exact sparse
allocation sizes, and actual dispatch routing with recording transports. Sparse
allocation and recording transports do not allocate 36GiB or execute a GPU.
An image-bound fake-worker test uses the emitted v9 descriptors and faulted
load/binding states; it likewise provides no native execution evidence.

Live-admission tests reserve worst-case private pages without prefix-cache
credit. The primary 32-request workload with 4096 input and 256 output tokens
requires 8704 pages: an 8192-page pool admits 30 requests and queues two, while
8704 or 16384 pages admit all 32. Cancellation releases reservation before another
request is admitted. These are host admission tests, not a completed model run.

The full metadata arrays are still scanned and cloned on transactions. Increasing
physical capacity may increase CPU cost; no speed improvement follows from this
capacity change alone. Existing frozen comparison tools intentionally do not
accept the new setup identity without separate explicit qualification support.
