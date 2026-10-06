# Exact own-residual lifetime policy

This is a CPU-qualified private runtime policy needed by Ferric's existing
buffer layout. It is not yet a native reuse result or an enabled model route.
The preceding [checked-root GPU success](../guarded-mlp-shared-root-v1/README.md)
uses a different executable and disjoint output buffers.

## Implementation

Ferric's model layout uses each rank's old R1 residual allocation as the final
R2 output. R1 first reads that residual and writes a distinct post-attention
root. The unchanged guarded packet graph completes R1, MLP and both validators
before R2. R2 reads the post-attention root, not the old residual.

The [private profile](cpu-attempt-v1/fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_combined_mlp_paired_profiles_v1.rs)
adds two closed policies:

| Policy | Accepted Output Relationship |
| --- | --- |
| Strict | Output is disjoint from all retained inputs and state, as before |
| ExactOwnResidual | Both outputs exactly equal their own rank's complete residual allocation |

Exact equality includes allocation identity, base address, requested extent and
backing extent. The new policy always rejects output overlap with all 22 MLP
roots, both partials, the peer residual and peer output. This includes read-only
weights: allowing a residual to share a read-only input must never authorize an
R2 write into that weight.

The [retained pair](cpu-attempt-v1/fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_combined_mlp_paired_retained_v1.rs)
captures the policy in its immutable binding. Its explicitly named unsafe,
private constructor requires the cross-stage lifetime. Each run repeats
identity, metadata and physical-region validation with that same stored policy.
There is no caller-supplied policy argument on run, setter, public skip flag or
change to the input ABI. Quiescent between-run updates remain subject to the
existing exclusive custody and completed-producer requirements.

The existing strict constructors, direct dispatch and borrowing Session stay
strict. Owner, retirement, barrier, deadline, poisoning and generic atomic
admission rules are unchanged. Six existing source files change; no public
engineering facade or Ferric route is added in this checkpoint.

## Actual MI350 Tests

The actual [CPU receipt](cpu-attempt-v1/evidence/complete.json) records:

| Check | Result |
| --- | --- |
| Full suite | 1,065 passed, 0 failed, 7 intentionally ignored |
| Inventory | 1,072 tests across five targets; 1,052 library tests |
| Previous outcomes | All 1,067 preserved |
| New tests | Three profile tests and two retained-policy tests |
| Focused cohorts | 18 paired tests and 11 retained-pair tests passed |
| Lifecycle | 16 natural zero-exit phases; no timeout, forced cleanup or postcheck error |
| Source map | 802 source bodies plus two harness files; unchanged during qualification |

Tests exercise all four same-rank alias masks: only both exact identities pass
the new policy; only fully disjoint outputs pass the original policy. They
reject swapped ranks, changed identity/base/extent/backing, every root/partial/
peer collision, transitive read-only-root aliases, invalid or overflowing
regions, padded overlaps and legacy 2,192-byte state substitutions. Adjacent
nonoverlapping allocations still pass. Policy bindings distinguish the modes,
and invalid constructor/run custody still poisons the group and owners.

The build and tests ran on `ssh mi350` with the pinned nightly 2026-04-03
toolchain, offline locked dependencies, two jobs, CPUs 8/9 and nice 10.
The 66.54-second CPU qualification executed no ignored GPU diagnostic.

## Evidence

The [data-only retainer](retain_cpu.py) authenticates 113 members / 112 pinned
bodies, 5,952,485 expanded bytes, the exact six-file delta, and all old and new
named outcomes. It does not execute the retained source or retain executable
bodies.

| Artifact | SHA-256 |
| --- | --- |
| CPU receipt | `0d54a7f2027199ab3ec23f9794e296163caa55560280267768a8e12a8138f994` |
| 11,336,648-byte library ELF | `cc704d11dfae882c857946c15e4b230036abea514103058fed4f3d71402199bd` |
| 943,117-byte archive | `94f1962ec8d281a4ebf589103e5737a56ea62446444a48a725cd5151bf36c9d9` |

## Next Gate

Run a new native fixture with one private residual/output allocation per rank
per pair. Restore and observe finite R1 inputs before publication; do not apply
the old output-poisoning step to a buffer that R1 must read. Preserve all four
pairs, two-bank isolation checks, eight segments, delayed resets and independent
numerical verification. Record exact residual/output identities and pre-run
input bytes, not merely a claimed reuse annotation.

Then integrate an opaque engineering interface and one mixed Prefix284/Combined
bank reset. Every selected owner and private retirement proof must validate
before any reset; a separate Prefix reset followed by paired reset is not
sufficient. The guarded layer route must not dispatch the old extra R2.

Native exact-residual reuse, the model route, full-model correctness, sustained
Qwen3-8B BF16 target-only 2,048/256, 700 tokens/s and M0-M7 remain open.
