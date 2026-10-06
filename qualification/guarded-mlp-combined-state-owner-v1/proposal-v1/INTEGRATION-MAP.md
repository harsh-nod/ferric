# Guarded Candidate And Host Integration Map

Paths below are relative to Ferric (`F`) or the pinned RT5a runtime (`RT`).
This is source analysis, not an implemented worker migration.

## Candidate And Measured Boundary

- `F/device/qwen3-tp-guarded-mlp-segment-kernels-v1/src/kernels.rs:17` has two
  shared atomic slice origins, state548 and guard4. Lines 23-39 enforce exact
  extents, nonzero generation, Wave64 and lane0 validation/publication.
- `src/guard.rs:20` loads every literal state index 0..547, even if an earlier
  predicate failed. The range/header/owner/count predicate is independent from
  publication. `:79` writes low/high/reserved Relaxed then verdict Release.
- `kernels.rs:44` R2 reads p0/p1 f32, residual u16 and two four-word AtomicU32
  guards; its output is an exclusive `WriteOnlyDisjointSlice<u16>`. `guard.rs:96`
  acquires verdict then tags/reserved; `:118` evaluates both guards before any
  volatile payload read. Preserve FP32 rank sum, intermediate BF16 conversion,
  then residual addition from `arithmetic.rs`.
- The actual current refusal is alias-origin incompleteness, not a proved race
  or R2 verdict. The new one-slice candidate is separately authored at
  `PL/guarded-mlp-combined-state-candidate-v228-v1`; its state/guard accesses must
  all index the same original 552-word slice directly, not introduce a second
  derived slice whose offset provenance may be lost.

## Legacy State And New Owner Boundary

- `RT/crates/fe2o3-kfd/src/engineering_gfx950_peer_wave_mlp_tiles_state_v2.rs:6`
  fixes STATE_BYTES=2192 and WORDS=548. The opaque owner is at :39; :49 fixes
  the old MLP prefix pointer; :54-86 validates exact request/kind and forbids
  peer mappings. Keep that entire contract unchanged.
- `memory_linux_wave_mlp_tiles_v2.rs:7` checks the retained requested extent,
  not backing-page size. :29 rearm preserves atomic lifetimes. The new owner
  uses a new exact2208 helper, not a relaxed version of this check.
- `engineering_gfx950_wave_mlp_tiles_v2.rs:10` INITIAL_STATE and :236 full
  terminal predicate are reused directly. The new private prefix view is data
  provenance for a future adapter, not authority to construct old StateV2.
- `engineering_gfx950_peer_wave_mlp_tiles_v2.rs:11` command has
  `state: &mut Gfx950EngineeringPeerWaveMlpTilesStateV2`; :29 identity and :72
  paired regions, :105 state fixup, :130 coordinator and :212 state validation
  cannot accept the combined owner without an additive typed command path.

## Peer R2 And Metadata

- `engineering_gfx950_peer.rs:504` admits a peer argument only with Read access
  and an existing mapping for that physical GPU. No peer Write/ReadWrite is
  added. The new owner maps one peer and exposes only guard offset2192/16bytes
  as Read to it. Local R2 uses the same Read suffix policy, not full-owner RW.
- `engineering_gfx950.rs:1441` pointer preparation compares a present metadata
  `.access` with the requested fixup, validates owned extent/alignment and
  refuses overlapping mutable argument ranges. It does not use `.actual_access`
  to downgrade a ReadWrite annotation. An absent access annotation is not the
  same as a proved atomic-read category. Inspect the actual v2 image before
  choosing any new exact metadata policy.
- Distinct R2 guard origins are read-only in the source. That removes the
  validator's mixed shared writes from this particular R2 class, but it is not
  a promise that all later compiler checks pass. Output remains exclusive and
  must be physically disjoint from inputs and both state owners.

## Coordinator And Lifecycle Work Still Required

- `PL/guarded-mlp-segment-v228-v1/runtime_arena.rs:18` prepared rank has R1,
  MLP, validator and guarded R2. :85 guard_pointer currently targets a separate
  guard at arena byte512; it must instead use the combined owner's suffix.
  Keep the arena's signals/kernargs; do not silently turn its page into an
  old StateV2 or shared replacement for the per-layer state bank.
- Its `runtime_lifecycle.rs:173/224/247/286` models begin/reset/completion/
  terminal validation, but remains an unintegrated proposal. Bind combined
  allocation identity, generation and both suffix views into the real owner
  identity. Before reset: healthy Idle, prior paired completion and actual
  queue capacity; before publication: all preparations and Ready consumption.
- Preserve five packets per rank (R1, MLP548, validator, both-validator barrier,
  guarded R2), ten actual signals, both closed publications before polling,
  full548 final readbacks, current Valid tags and deadline/fault fences. A
  completion signal does not manufacture hardware read-index credit.
- New kind registration and queue-first Close are included here. The older
  guarded-arena kind/hook proposal is not applied; aggregate integration must
  include both without reverting any other private resource kind.

## Ferric Route And State Banks

- `F/adapters/tp-peer-finite-engineering-worker-v1/src/state_roster/`
  `prefix_tiles_decode_v6.rs:11` has TWO banks, each with36 layers, each with
  paired Prefix284 and paired MLP548 owners. It is 144 MLP owners, not just one
  pair or one bank. :137 returns mutable old StateV2 references; :148 performs
  actual re-observation/full548 equality before FinalResidualReady. A guarded
  route requires a distinct typed roster/capability path; do not replace all
  old banks or infer its native allocation census from byte counts.
- `resident_layer/prefix_tiles_decode_v6/ordered.rs:44` currently adds two
  Down scratch buffers and binds exact census [715,711]. :129 prepares the old
  compound R1/MLP command with states from the roster; :173 dispatches that old
  route. :80 then performs a separate R2. A new guarded route must not call the
  old separate R2 again after its guarded R2 already ran.
- `resident_layer/mlp_tiles_v2/artifacts.rs` and
  `resident_layer/prefix_tiles_v6/projection_residual.rs` load the old MLP/R1
  artifacts. New validator/R2 symbols require a distinct exact-hash/metadata
  loader and bootstrap identity, not acceptance by old descriptor predicates.

## Acceptance Gates, In Order

1. Candidate CPU source predicate/548-load coverage/publication/generation and
   arithmetic tests, then actual checked lowering of both v2 exports.
2. Native-image hash, two exact exports, observed ABI/resources, and generated
   system atomic ordering and payload dominance. No emitted image exists by
   virtue of CPU tests or a source-only owner.
3. Runtime CPU ownership/mapping/extent/phase/rearm/counter/failure tests, then
   actual paired native positive and negative runs. Cover both local and peer
   guards, stale halves, Pending/Invalid, exact extents, untouched output on
   rejected generations, partial publication, faults, deadlines and queue Close.
4. Actual reusable generation/arena run with fixed observed allocations and
   hardware queue capacity; only then integrate a distinct Ferric route.
5. Preserve Prefix284 plus MLP548 bank/reuse rules, four-forward identity and
   full numerical/state comparisons against the existing ordered route. Prior
   source-only counts or pure lifecycle repetitions are not GPU measurements.

This route is more host work than a kernel signature edit, but it avoids a new
generic compiler exception for shared atomic origins whose alignment and
relative offsets are not retained in ranked IR. No `&mut` alias is fabricated,
and no compiler limit is raised.
