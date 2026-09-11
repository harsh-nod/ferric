# Large KV Pool Plan V1

Status: **planned only; not implemented, emitted, GPU-qualified, or proved**.
This document records a read-only audit for the competitiveness sprint. It does
not expand any current constructor, kernel, artifact, benchmark claim, or M1
authority. Implementation remains deferred while the current 32-row and live
ingress profiles are qualified.

The audit observed integration source through
`58ca738426cf525fb68f67b72bdef4166d76632a` and read fe2o3 Git objects at
`3e3a77284a61654134211f8145dd0ddeebb2ff91`. No builds, tests, GPU jobs, or
protected-worktree edits were performed for the audit. Existing native/model
receipts retain their original compiler, runtime, image, and workload pins.

## Objective And Envelopes

Increase aggregate resident KV capacity for target Qwen3-8B concurrency 16/32
without changing per-request context semantics or inference arithmetic. Start
with TP1, capacity32, baseline paged attention, and existing baseline/MFMA
projection plus the separately selected FP32 head. Keep all legacy profiles
and defaults unchanged. TP2/8 can be a later explicitly qualified extension.

Keep these independent limits fixed: 16 tokens per page, at most 8,192 logical
positions per request, 32 live requests, and 32 physical rows per dispatch.
Larger physical pools do not imply longer logical contexts or larger batches.

For input length `I` and maximum generated length `O`, live ingress reserves
`ceil((I + O - 1) / 16)` private pages per request. The final generated token
does not need another forward pass. Prefix sharing must not be used to
overcommit this worst-case reservation.

| Concurrent requests | Input / output tokens | Total reserved pages | Minimum proposed envelope |
| --- | --- | ---: | --- |
| 16 | 2,048 / 256 | 2,304 | 8,192-page intermediate |
| 32 | 2,048 / 256 | 4,608 | 8,192-page intermediate |
| 16 | 4,096 / 256 | 4,352 | 8,192-page intermediate |
| 32 | 4,096 / 256 | 8,704 | 16,384-page full |
| 32 | 8,192 / 1 | 16,384 | 16,384-page full |

The primary long-context workload is **32 requests with 4,096 input and 256
output tokens each**. The intermediate envelope cannot admit all of it at
once. The proposed full 16,384-page envelope provides 262,144 aggregate slots
and also covers all 32 requests at the existing 8,192-position logical limit.
The exact minimum for the primary workload is 8,704 pages; 16,384 is the
proposed bounded full-profile maximum, not a claim that 8,704 is insufficient.

## Current Coupled Caps

All paths below are relative to the repository root.

| Location | Current contract | Required treatment |
| --- | --- | --- |
| `adapters/m1-engineering-execution-v1/src/tp_paged.rs`, `EngineeringTpPagedLimitsV1::new` | Physical pages 1..512, context 1..8192, sequences 1..32 | Add an explicit larger-pool selector/constructor; do not widen the legacy constructor silently. |
| `adapters/m1-engineering-execution-v1/src/tp_execution/batched.rs`, `new_bounded` | Passes `physical_pages * 16` as inner capacity | Separate physical allocation capacity from logical sequence capacity, with checked multiplication. |
| `adapters/m1-engineering-execution-v1/src/tp_execution.rs`, `new_with_storage` | Uses the same capacity for verified sequence construction and KV allocation | Preserve the single-sequence public API; give batched construction explicit logical and physical geometry. |
| `crates/ferric-engine/src/tensor_parallel_execution.rs` | Verified sequence invariant and constructor require capacity <=8192 | Keep this logical-context invariant unchanged. Do not weaken a proof to accommodate aggregate storage. |
| `device/qwen3-tp-batch32-kernels-v5/src/rope_kv.rs` | Physical pages <=512; physical slot <8192 | New append root must widen physical bounds only and retain all duplicate-slot/exclusive-write checks. |
| `device/qwen3-tp-batch32-kernels-v5/src/baseline_attention.rs` | Physical pages and each loaded physical page ID are bounded by 512 | New attention root must retain exact cache lengths, causal masking, logical bounds, finite checks, and numeric order. |
| `device/qwen3-tp-batch32-kernels-v5/src/attention.rs` | Wave attention has separate physical-page guards | Leave unsupported in the first larger-pool profile; it cannot inherit the extension accidentally. |

The 512-page pool is only 8,192 **aggregate** resident slots, not 8,192 slots
for each request. Changing only the host limit would fail the inner verified
sequence constructor before allocation; bypassing that alone would still
violate frozen device guards.

## Minimal Implementation Boundary

1. Introduce a distinct larger-pool profile with an explicit physical maximum.
   The profile must be bound to the pool, driver, and admitted artifact before
   allocation. Legacy drivers/images must reject it, even when a caller asks
   for fewer than 513 pages. Record actual pages and selected maximum in setup
   identity so performance validators can distinguish envelopes.
2. Decouple logical and physical capacity in internal storage construction.
   Preserve `TensorParallelSequenceV1` and ordinary single-sequence behavior.
   Batched execution already receives logical positions, table stride, and
   physical pages independently; avoid repurposing the single-sequence
   `capacity` field or exposing the batched allocation through its methods.
3. Emit a fresh additive image with closed append and baseline-attention roots,
   preserving their ABIs, workgroup geometry, and arithmetic. Reuse unchanged
   v5 model roots and v8 head roots through explicit routing. Do not relabel or
   alter frozen v2/v3/v5 images. The first profile can support rows1..32 with
   both row-budget16 and row-budget32 experiments using the same images.
4. Widen only physical-page and physical-slot guards in the new source.
   Per-request page-table stride remains at most 512; logical positions and
   attention iteration bounds remain 8192. Update every inlined helper copy and
   preserve source/body-equality fixtures. RoPE need not change.
5. Add exact CLI/artifact/profile/checker admission for the new combination.
   Reject unsupported wave, sequence, capture, peer, or replica combinations
   until separately supported and qualified. Existing checker bytes and old
   profile meanings remain frozen.

## Memory And Integer Bounds

Target Qwen3-8B has 36 layers, 8 KV heads, and head dimension128. Aggregate KV
bytes per resident token are `36 * 2(K,V) * 1024 * 2(BF16) = 147456` bytes.
Each 16-token physical page therefore occupies 2,359,296 bytes, or 2.25 MiB,
across the TP ranks. These are exact array payloads, not measured RSS or total
device allocation.

| Physical pages | Aggregate slots | Aggregate KV bytes | Aggregate KV GiB | TP1 bytes for one layer's K or V |
| ---: | ---: | ---: | ---: | ---: |
| 512, current maximum | 8,192 | 1,207,959,552 | 1.125 | 16,777,216 |
| 8,192, intermediate | 131,072 | 19,327,352,832 | 18 | 268,435,456 |
| 16,384, full | 262,144 | 38,654,705,664 | 36 | 536,870,912 |

For later TP2/8 support, these KV payloads divide exactly by the world size.
Weight replicas, rank-zero tensors, transposed projection weights, activation
workspaces, FP32 head workspace, code/queue allocations, and retained host
weights must be accounted for separately. A shared-machine preflight must
require actual available headroom, not merely compare against nominal HBM.

At the full envelope, each TP1 K/V tensor has 268,435,456 BF16 elements. Its
largest element index is `2^28 - 1`; the largest physical slot is 262143.
Both fit u32 and 64-bit host usize. Checked host products must establish these
bounds before allocating. Attention's logical positions remain below 8192;
physical page IDs remain below 16384. A later wave-attention extension must
recheck its metadata broadcast contract explicitly rather than retain the old
comment that all metadata is below 8192.

The inspected engineering KFD profile bounds one buffer at 32 GiB, total
requested allocations at 128 GiB per worker, and allocation count at 2048.
Neither proposed geometry inherently requires changing those generic limits;
it adds no allocation objects. Those limits are not a memory-availability
guarantee and do not replace complete model allocation accounting.

## Admission And Ownership

`tp_live_ingress.rs::admit` already sums the private worst-case page demand of
active requests and leaves excess arrivals queued. Preserve this policy and
its cancellation/retirement accounting. Use checked arithmetic or explicit
bounded-sum reasoning for both envelopes. The non-live batch runtime admits
metadata separately and can return page backpressure; do not claim that its
smaller-row-budget retry guarantees completion for overcommitted requests.

Keep complete cached pages immutable and partial pages exclusively referenced.
Prefix lookup leaves the final prompt token to execute. Reservation must stay
fail-atomic, submission must precede device effects, and only successful
all-rank/all-layer completion may commit. Submitted uncertainty must continue
to quarantine the entire pool. No new page-copy or partial-page-sharing
mechanism is required for this capacity-only plan.

The engineering pool clones state and validates the full page/node arrays on
transactions. Allocation and radix child lookup scan arrays; expiration and
leaf eviction may repeatedly scan cached nodes. Larger maxima may therefore
increase CPU overhead even at unchanged active context. Measure this with
cache off and on separately; do not remove invariants or claim a throughput
improvement from capacity alone.

## Required Qualification

- Constructor/profile tests must preserve legacy rejection above 512 pages, bind the new
  image before allocation, reject over-cap/overflow and stale pool identity,
  and assert exact logical/physical geometry and per-rank allocation bytes.
- Typed emission must re-establish arithmetic, access, race, and launch bounds
  for the actual new roots. Retain full compiler/source/artifact provenance.
- Native append/attention fixtures must exercise physical IDs511/512 and the
  final page, final physical slot, logical position 8191, nontrivial table
  permutation, rows1/16/17/32, active/tail guards, read-only operands, and
  unchanged causal/numeric references. Invalid page IDs, context bounds,
  duplicate slots, and nonfinite behavior need host rejection/trap fixtures;
  do not deliberately fault a shared GPU.
- Pool/runtime fixtures must cover the full page range, immutable prefix
  sharing, exclusive partial pages, OOM rollback, commit/abort/poison,
  eviction/retirement/cancellation, and 32-request reservation/queue behavior.
  Include the primary 8,704-page demand to demonstrate why the intermediate
  envelope cannot satisfy the primary concurrency target.
- Use frozen exact-token model canaries before long-context runs. Benchmark
  the actual 16/32 concurrent workloads only after memory preflight, strict
  output checks, causal timestamps, close/reap, and global-idle receipts pass.
  Record actual concurrency and observed row schedules, not just capacities.

The executable engineering paged allocator is explicitly **Contracted**, not
same-source Verus-proved; see
`adapters/m1-engineering-execution-v1/src/tp_paged/README.md`. Its tests and
invariant checks cannot establish protected Engine custody, hardware
completion, or M1 authority. This plan does not create a new Verus claim.
Protected same-source refinement of reference conservation, exclusive writes,
radix identity, atomic transactions, and permanent quarantine remains separate
work. Integration must regenerate only the affected source/metadata inventory
after an implementation is accepted.
