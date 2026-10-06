# Deferred Guarded-Pair Binding

This private runtime change separates genuine paired-state allocation from
kernel/buffer binding. It follows the [native exact-residual qualification](../guarded-mlp-exact-residual-native-v1/README.md)
and prepares the ownership interface needed by Ferric's catalog. It does not
enable a model route or public production API.

## Why Two Stages

Ferric loads existing resident artifacts and source buffers before catalog
binding. Catalog binding then reserves layer states, but the complete guarded
input roles are not available yet: layer-root resolution requires CatalogBound,
and the ordered projection image and distinct Down scratch arrive later.
Allocating a fully bound pair at the old state-reservation point is therefore
not possible without changing that ordering.

The [qualified retained implementation](cpu-attempt-v1/fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_combined_mlp_paired_retained_v1.rs)
introduces an opaque, non-Clone `UnboundPair`:

1. `allocate` reserves two genuine Combined552 allocations, checks Group/rank
   identity and generation 1, and reads back initial atomic contents. It needs
   no kernels or model payload roles and grants no dispatch or rearm operation.
2. Consuming `bind` or `bind_exact_own_residual` rechecks the Group, currentness,
   initial physical state and the complete selected role policy. Only then does
   it construct the existing immutable `RetainedPair` binding.
3. `RetainedPair::run` retains all existing binding, ownership, completion and
   private retirement checks. No owner, buffer pointer or caller proof escapes.

Allocation and binding use terminal-error custody guards. Partial allocation,
failed binding or unwind poisons the Group; binding does not return storage for
retry. Dropping an unused unbound handle does not free its backing: Group custody
retains it until teardown. The original one-shot constructors share the same
implementation helpers and preserve their single absolute deadline.

The [native fixture](cpu-attempt-v1/fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_combined_mlp_paired_interleaved_native_v1_tests.rs)
now reserves four unbound pairs before loading kernels or allocating payload
roots, then binds each pair to its actual roles. This is a stronger deferred
sequence than Ferric's current ordering, not an exact reproduction of it. It
checks that binding adds **no Group/GPU allocations**; host `Vec` allocations are
not excluded. The disjoint-output native case keeps its one-shot construction.

## Actual CPU Qualification

The [MI350 CPU receipt](cpu-attempt-v1/evidence/complete.json) records 1,071 passed,
zero failed and eight intentionally ignored tests, with 1,079 names across five
targets and 1,059 library tests. All 1,075 predecessor outcomes are preserved.
Four new tests cover unbound identity/initial-state metadata, invalid allocation
custody, consuming strict/exact bind refusal, and partial-allocation drop/unwind.
Successful physical binding is a native-test obligation, not a CPU-only result.

All 16 phases exited naturally with no timeouts, forced cleanup or postcheck
errors. Qualification took 65.65 seconds on `ssh mi350`, using the pinned nightly
2026-04-03 toolchain, offline locked dependencies, two jobs, CPUs 8/9 and nice 10.
Exactly four of 802 compiler/runtime source bodies change; 798 remain identical.

The [CPU retainer](retain_cpu.py) authenticates 111 members / 5,993,521 expanded
bytes, source ancestry, actual test outcomes and format records as data.

| Artifact | SHA-256 |
| --- | --- |
| CPU receipt | `76c7d14beaecec2f63fff67828ff40cbe5ac439c77f27b0e621da7659ab42ca1` |
| 11,352,416-byte host ELF | `8d5142cfef1eb66c6543a4cba21965c1587ec76c0b39bad63598caee72b71573` |
| 944,653-byte CPU archive | `5528a34505c1d7d2bb41a92e4cc1bad390132a6cebe33668678e7c365ddfabfc` |

## Actual Native Qualification

The [bare GPU attempt](gpu-attempt-v1/evidence/complete.json) passes on `ssh mi350`
using that exact CPU-qualified executable, with one invocation and zero retries.
No debugger or rebuild is involved. The [integer reference](gpu-attempt-v1/verify_native.py)
changes only its expected libtest filtered count, from 1,054 to 1,058; its
numerical contract and mutation cases are unchanged.

| Check | Actual Result |
| --- | --- |
| Deferred setup | Four unbound pairs reserved before kernels/payloads; every consuming bind preserves Group/GPU allocation counts |
| [Computed values](gpu-attempt-v1/evidence/verify.stdout) | 557,056 bit-exact, including 65,536 final BF16 elements |
| Residual reuse | 16 alias observations, eight distinct private allocations, every final element overwrites its finite input |
| Matrices | 48 hashes checked; 20 distinct bodies |
| State/isolation | 104 owner readbacks, 16 terminal observations, 336 inactive payload hashes |
| Schedule | Eight paired segments and two delayed bank rearms |
| [Reference self-tests](gpu-attempt-v1/evidence/reference-tests.stdout) | 918 mutation refusals plus fixed anchors and malformed-input refusals |
| Lifecycle | Eight natural zero-exit phases; no timeout, forced cleanup or postcheck error |
| Shutdown | Healthy queue-first Close; all eight GPUs idle before and afterward |

The entire diagnostic took 118.95 seconds, including fixture setup, transfers
and verification. This is not a kernel-latency or throughput measurement, and
one passing attempt does not establish repeatability or full-model correctness.

The [GPU retainer](retain_gpu.py) authenticates 59 members / 13,806,038 expanded
bytes and the complete CPU/executable/observation/reference joins. CPU and GPU
capsules preserve executable metadata, not host executable bodies.

| Artifact | SHA-256 |
| --- | --- |
| GPU receipt | `2dd9ce9f28442ef12576f705f73462e26ffd1a48fbfee3a145fc2a9a3672f592` |
| 575,855-byte GPU archive | `ca47060877d7352fadea82ed5f07d39388028d6ce38d66b5fc69a0411500f17f` |

## Remaining Integration

Expose the opaque unbound/bound types through an engineering-only interface,
with external-crate signature and compile-fail opacity tests. Pending should
reserve genuine unbound Combined552 pairs and use a distinct guarded kind, not
pretend they are legacy MLP548 owners. Publish an executable roster only after
all real bindings succeed.

The following mixed-bank transaction is still unimplemented. It must hold one
short exclusive Group borrow over up to 36 entries, each with two Prefix284
owners and one retained pair. Require completed Prefix states and private pair
retirement proofs, validate the entire bank before any reset, then reset and
read back every initial state. Publish Ready and the next bank generation only
after the final fence. Errors or unwinds quarantine the whole selected set;
there is no rollback claim. Initial banks need a distinct no-reset validation.

Prefix snapshots do not prove retirement of old commands; Ferric's external
ordering obligations remain. Full-model correctness, attention/KV integration,
arena reclamation, sustained Qwen3-8B BF16 target-only 2,048/256, 700 tokens/s and
M0-M7 remain open. No performance result is claimed here.
