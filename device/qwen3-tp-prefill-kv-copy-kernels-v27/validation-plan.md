# V27 Validation and Integration Sketch

This is a plan, not an executable authorization, build receipt or accepted
artifact identity. All builds/tests belong on the parent-selected remote CPU
host under its existing10GiB/512MiB-reserve/1CPU/4GiB-RSS guard. No local build,
GPU use, CI build, push or change to the active repository is part of this
proposal. Do not modify or erase any historical failure/result.

## CPU and Emission

1. Retain this original source manifest. Stage a new standalone source under
   C, generate only its Cargo.lock offline, retain metadata and require the
   c4c fe2o3 /7eb Pliron graph; no new adapter link unit is involved. Use the
   existing stable candidate environment and release cache. Run rustfmt twice
   with retained before/after inventories and require only formatting changes.
2. Separately guarded commands, using the actual staged manifest path:

   ```sh
   cargo test --manifest-path "$V27/Cargo.toml" --offline --locked --release --test contract -- --list
   cargo test --manifest-path "$V27/Cargo.toml" --offline --locked --release --test host -- --list
   cargo test --manifest-path "$V27/Cargo.toml" --offline --locked --release --all-targets -- --test-threads=1
   cargo clippy --manifest-path "$V27/Cargo.toml" --offline --locked --release --all-targets -- -D warnings
   ```

   Require exactly5 contract and6 host tests, nonzero actual passes, and retain
   every failed attempt. These host models are not native parity or proof.
3. Adapt the existing `E/v26-emission-r1` engineering continuation only after
   actual CPU success. Use fresh V27 source/review/output names and the distinct
   crate `ferric_qwen3_tp_prefill_kv_copy_kernels_device_v27`; keep the reviewed
   limited producer/source/vendor/tool identities and original strict-lint
   failure explicit. Its dependency list is unchanged from the c4c kernels.
   The temporarily removed C nightly toolchain must first be restored and
   verified byte-identical, as must the existing private gfx950 OCML provider
   tree and external-provider accounting guard. No host libraries change.
4. Actual metadata review: one exact root,336-byte kernarg segment, four slice
   pairs/four u32 scalars, Wave64 and256x64 geometry. Retain actual VGPR/SGPR,
   scratch/LDS/spills and O2/replay receipts. Inspect ISA for parallel raw16-bit
   loads/stores, no conversion/FP arithmetic, no accidental serial row loop,
   no hidden per-lane invalid bounds fallback. No artifact hash is predeclared.

## Isolated Native Diagnostic

Reuse the frozen owned-process/resource/admission/ABI helpers; do not invent
another lease/lifecycle or broaden launch authority. Use a fresh private stage
and historicalf68 worker; preload original V5 and actual admitted V27 images
in both arms before allocating anything. Keep baseline source/inputs untouched.

Correctness precedes performance. For both source permutations, cover first
positions0,16,112,8176 and remapped physical pages including0/511. Supply exact
positions/tables to V5, the same validated full-page views to V27, and exercise
all65536 u16 encodings over bounded cases. Read back both complete outputs,
entire surrounding caches including an immutable prefix, allocation guards,
all inputs/positions/tables and untouched padding. Require bit-exact equality
to the independent logical-slot oracle and V5, with the expected one-packet
increment and no unexplained counters/commands. Use NaN/raw-bit fixtures only
for isolated copy; do not feed synthetic NaNs through the full model.

Only after all correctness cases pass, measure a whole16-row copy packet per
operation with fresh owned workers per arm/group, identical buffers and both
images loaded. Existing operation-harness groups1/4/16 fit the16-packet bound;
warmup16 and measured256 operations may be reused with unchanged full readback
before the first timing snapshot and after the last. Report worker aggregate
and controller wall time as synthetic cache-hot whole-copy timings, never as
GPU timestamps or inferred model TTFT. Retain exact output guards before timing
and after, immutable source hashes, pre/post resource/KFD evidence and clean
owned cleanup. An error invalidates timing; do not signal discovered jobs.

## Later Adapter Integration

After source/CPU/ISA/native review, a distinct opt-in V27 controller may load
the old five V17 images plus V27 in both arms, retaining historicalf68. Use the
reviewed build-dependency/name-string isolation pattern, not normal c4c linking
into the5a adapter. Add a separate artifact validator, never relabel V19 or V5.

Minimal routing point: after `batched.rs` creates the actual execution order,
derive a sealed16-row-page descriptor from the prepared batch. Reuse all
existing page-table uniqueness, cross-sequence ownership, slot/COW and scalar
checks before deriving the descriptor. Require TP1 Target8B, prefill,16 active
rows from one sequence, exact aligned positions, exact one physical page and
fresh/exclusive suffix-page authority. Check actual post-pruning source order,
not the original scheduler order. Bind K/V input views at offset0 with32768
bytes each and destination page views at `physical_page*32768`, each32768
bytes, distinct allocations with all original pointer/extent/access validation.

For this selected path only, replace the one V5 append command with one V27
command inside the unchanged11-command ordered attention group. No added
barrier/frontier, host commit, scratch allocation or collective mutation. The
group must still complete before cache-dependent attention/output publication;
failures use the existing poison/quarantine and never commit scheduler/pool
state. Baseline and every nonselected shape keep their exact old commands.
No V19/V20/V21/V22 hybrid in the first model experiment.

CPU route tests must compare full old/new command streams, verify the sole
append replacement and exact sliced byte extents, exercise both final-row
orders, immutable prefix/COW handling and all fallback cases, reject malformed
images before allocation, and preserve failure poisoning. Model A/B remains
128in/128out,1warm+3measured/arm, exact1024 generated IDs/text,135 batches and
83139 packets per request. V27 changes288 prefill append packets/request, not
the127 decode append packets. Require exact native model parity before a new
contamination-monitored matched HTTP comparison; no synthetic timing result
alone can establish end-to-end benefit.
