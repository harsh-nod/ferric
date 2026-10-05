# TP2 Ordered Projection Residual and MLP Runtime Proposal

Source-only proposal against clean `fe2o3-p228-runtime` commit
`27b53d2b74c1f239988a891a4aed39e089b05663`. Nothing here has been compiled,
imported, tested, deployed or executed by the author. Root owns qualification
and integration. `baseline/` contains exact replaced source bodies;
`candidate/` contains the additive overlay. `source-manifest.json` binds every
preimage and replacement and names all authored tests.

## Public Contract

`Gfx950EngineeringPeerProjectionResidualMlpTilesDispatchV1<'a>` contains
`projection_kernel`, `projection_object_sha256`, `partials: [Buffer; 2]`,
`residual_input`, and the existing typed `mlp` command. The new unsafe Group
method `dispatch_projection_residual_mlp_tiles_round_unchecked_v1` takes
exactly two rank-ordered commands and returns `final_states: [[u32; 548]; 2]`
and `segment_host_ns`. Both embedded MLP timeouts must be identical and in
`1..=10_000`; they denote one aggregate deadline, not two budgets. Existing
types, methods, wire routes and old tests remain available and unchanged in
meaning. No new generic dispatch or arbitrary packet interface is public.

The projection image is restricted to
`ferric_qwen3_tp_peer_tp2_projection_residual_bf16_v1`, the actual 424-byte
kernarg, 168-byte explicit/22-argument ABI, Wave64, zero private bytes and
zero LDS. Its ten pointer fixups and scalar/length bytes match Ferric's existing
`consumer_bytes`: two live ordered 16 KiB FP32 partials, six zero-extent inactive
slots, rank-local 8192-byte residual input, and output at MLP root zero. The
existing typed MLP loader checks, eleven-root geometry and genuine 548-word
initial/terminal validators are reused without weakening.

## Ordering and Storage

This explicitly removes the host fence between residual and MLP. It is not a
policy-neutral optimization. Each queue receives residual then MLP using the
existing `WaitForPrior` packet builder, separate kernargs and signal slots 0/1,
and one final doorbell. Both rank doorbells precede any completion poll. Fresh
all-rank publication currentness checks do not require the already-published
peer queue to be idle. All four signals, retained queue identities/frontiers,
faults and the common deadline are checked. Fresh full Group fences surround
the paired terminal-state reads; only then are states marked Completed and
scratch returned to Idle. Any error poisons both Group and Context ordered
paths, including a deadline overrun detected after retirement. No rollback,
retry or uncertain scratch/state reuse is provided.

Both Prefix284 producers must already have completed and passed genuine state
checks before this call: that is an explicit unsafe caller obligation. Local
queue ordering cannot protect an input partial from the peer's MLP writer.
Consequently both projection partials and residual inputs are rejected if they
overlap any MLP writable allocation, or either projection output. The intended
output-to-MLP-root-zero bridge is retained. Read-only sharing is permitted.
Ferric must supply a distinct fixed 16 KiB Down partial per rank and have both
subsequent final residuals consume that pair directly, with no copyback. The
old `MLP[9] == Prefix[13]` alias is intentionally rejected by this API.

First use retains the existing Context ordered arena at `internal[6]`, exactly
1 MiB per rank, and existing signal-page storage. No GPU allocation is added
on subsequent segments. Total incremental retained storage is 2 MiB for the
arenas plus the caller's 32 KiB TP2 Down pair. Group holds a fixed Option with
incarnation, rank/device/queue epoch, arena/signal VA/handle/extent identities
and Idle/Busy/Poisoned state. Before any stage/reset, a fresh full idle fence,
`completed_write == ring.write`, capacity and identity checks are required.
The current frontier may exceed the last retirement, allowing intervening
legacy Prefix/final-residual operations. Identity or frontier regression is a
terminal refusal, not implicit scratch reinitialization. Existing Context
close/rollover owns arena destruction; no new free or teardown path is added.

Default fresh-full and explicit shared-full Group routes are supported.
Kernel admission caching, operational currentness, legacy performance profiling
and raw-timestamp queues are refused. Host observation counters may remain
enabled. Per-call bounded host Vecs used by existing prepared-dispatch helpers
are not a growing retained segment history.

## Qualification

Twenty-five authored, unexecuted test methods are additive: 23 in the new
coordinator module and two in the existing ordered-batch tests. Fifteen old
fixture files gain only Group scratch `None` initializers. Tests cover paired
publication, prepublication refusal, uncertain stages, missing earlier signals,
one inclusive deadline (including post-retirement), terminal state checks,
strict image/ABI/fixups, cross-rank write hazards, identity drift, legacy
frontier advances and 82,908 simulated fixed-storage retirements. The existing
AQL mock checks two distinct kernarg/signal slots, `WaitForPrior` headers and one
final doorbell. The actual NativeOrdered stage census accepts two preprepared
dispatches with empty transport payload. These mocks do not execute native
mapping/writes or prove GPU memory ordering, liveness or arena reuse.

Root should use its bounded paired-source qualifier, preserve the entire old
KFD inventory and add the manifest's exact names. Representative commands in
the fresh qualified source tree (not run by the author):

```text
cargo test --offline --locked -p fe2o3-kfd --features engineering-gfx950 --lib --no-run
cargo test --offline --locked -p fe2o3-kfd --features engineering-gfx950 --lib -- --list
cargo test --offline --locked -p fe2o3-kfd --features engineering-gfx950 --lib
```

The runtime full suite is CPU-only under its existing review, not necessarily
syscall-free. Root owns toolchain, dependency/source snapshots, resource bounds,
format checks, exact ignored inventory and production executable qualification.
New Ferric routing, allocations, lifecycle/poison tests, fresh executable audits
and a genuine bounded GPU run remain separate gates. No compiler/kernel image
change, numerical equivalence, 2048/256 completion, GPU timing, speedup or
full-model acceptance follows from this source proposal. `segment_host_ns` is
one inclusive host interval, never fabricated residual and MLP sub-durations.
