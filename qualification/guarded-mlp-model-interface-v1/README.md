# Guarded MLP Model Interface

This checkpoint exposes the opaque guarded-pair runtime through an
engineering-only interface and adds a mixed Prefix284/Combined552 bank
transaction. It builds on [deferred binding](../guarded-mlp-deferred-binding-v1/README.md).
It does not grant production authority or establish full-model correctness.

## Interface

The [facade](cpu-attempt-v1/fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_combined_mlp_paired_facade_v1.rs)
exports opaque unbound and retained pair handles. Allocate storage before all
model roles exist, then consume it exactly once when binding the kernels and
buffers. A retained pair dispatches R1, MLP, validator and R2 with the existing
private completion/retirement checks. The exact-own-residual binding requires
both ranks' output to alias their own complete residual input, while remaining
disjoint from other live roles.

Observations contain state words, guards, observed queue frontiers and host
duration. They are data, not completion authority: callers cannot construct a
retained pair from an observation, expose its owners, clone it, or reuse a
consumed unbound handle. Two adapter tests, three external signature tests and
nine compile-fail doctests exercise these boundaries.

## Whole-Bank Transaction

The [mixed-bank implementation](cpu-attempt-v1/fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_combined_mlp_paired_mixed_bank_v1.rs)
accepts 1-36 entries, each holding two Prefix284 owners and one retained
guarded pair under exclusive Group custody:

1. Authenticate all owner/arena identities and rank, Group and phase metadata.
2. Check every Prefix terminal state and each pair's private retirement proof,
   physical state, signals and common nonzero generation.
3. Complete the trailing validation fence before the first reset.
4. Reset and read back all selected states; fence again before publishing Ready
   and the next bank generation.

A separate initial-bank operation validates generation-one Ready state without
resetting it. All errors, expired deadlines and unwinds quarantine the selected
owners and Group. Partial reset is fail-stop, not rolled back. Twelve tests
cover ordering, maximum size, duplicates, generations, initial validation,
error/unwind/deadline injection, and native metadata refusal.

Prefix snapshots still do not prove permanent retirement of old commands.
The unsafe rearm caller must satisfy that independent ordering obligation.
The mixed-bank transaction has CPU qualification here, not native qualification.

## Actual CPU Qualification

The [MI350 receipt](cpu-attempt-v1/evidence/complete.json) records:

| Check | Result |
| --- | --- |
| Ordinary tests | 1,088 passed, zero failed, eight intentionally ignored |
| Inventory | 1,096 names across six targets; 1,073 library names |
| Compile-fail doctests | Nine passed |
| Predecessor outcomes | All 1,079 retained unchanged |
| Lifecycle | 20 natural zero-exit phases; no timeout or forced cleanup |
| Source changes | Six changed bodies and five additions; 796 unchanged |
| Duration | 69.04 seconds, not GPU timing |

Tests ran on `ssh mi350` with pinned nightly 2026-04-03, offline locked
dependencies, two jobs, CPUs 8/9 and nice 10. GPU tests were not run by the CPU
controller. [Data-only retention](retain_cpu.py) authenticates 138 members and
6,082,566 expanded bytes without importing or executing capsule sources.

| Artifact | SHA-256 |
| --- | --- |
| CPU receipt | `4a682798a23ac4c8accb0721f332a7b4e7484729bd692209d20f2883b501cee1` |
| 11,435,960-byte host ELF | `c607a1fb102d05cd56fa97e2e781c2e1718d90f24c57e7fb016b12f47138c772` |
| CPU archive | `f5435e67d498508cda293b3d5214f536062a49406621be1f2dcb97f635a4e246` |

## Actual GPU Qualification

The [single native attempt](gpu-attempt-v1/evidence/complete.json) passed on
MI350 using the exact CPU-qualified host executable. The fixture now calls the
public allocation, consuming exact-residual binding and dispatch methods. Its
data-only observation adapter cannot manufacture private retirement authority.
The old private pair rearm is still used in this fixture: this is not a native
mixed Prefix/Combined bank test.

| Check | Result |
| --- | --- |
| Computed values | 557,056 bit-exact, including 65,536 final BF16 elements |
| Residual reuse | Every final element overwrites its finite input; eight private allocations |
| Matrices | 48 hashes checked, 20 distinct bodies |
| State and isolation | 104 owner readbacks, 16 terminal observations, 336 inactive payload hashes |
| Schedule | Eight paired segments, two delayed pair-bank rearms |
| Independent verifier | 918 mutation refusals and all fixed/malformed-input controls passed |
| Lifecycle | Eight natural zero-exit phases, zero retries and no postcheck errors |
| Shutdown | Healthy queue-first Close; all eight GPUs idle before and after |

The [integer reference](gpu-attempt-v1/verify_native.py) changes only its
expected filtered library-test count from 1,058 to 1,072; the numerical
contract is unchanged. This synthetic two-layer fixture is not a model-layer
chain. The complete diagnostic took 129.25 seconds, including setup and
verification; that is not kernel latency or decode throughput.

[Data-only GPU retention](retain_gpu.py) authenticates 59 members and
13,871,164 expanded bytes, including the CPU/ELF, raw observation and reference
joins. The archive retains host executable metadata, not executable bodies.

| Artifact | SHA-256 |
| --- | --- |
| GPU receipt | `7c748bf1a170ecf953ffb3ffff35e6e0e677bdc2331bbb9b8c6a9525c418a81d` |
| GPU archive | `e6e83c3044e083b17836d6b1bf41a2d231d5607a899025154a3a87355092d076` |

## Next Demo Gates

The separately built Ferric route now completes guarded TF4/AR4 through all
36 layers and both ranks, using both banks at generations 1, 1, 2, 2 and healthy
Close. A [fresh layer-zero capture](../guarded-mlp-model-stage-capture-v1/README.md)
also passes and retains the current runtime's intermediate tensors. These
bounded bring-up results satisfy the native four-forward gate, not full-model
numerical acceptance. Before a correctness-qualified community demo, the
remaining tensor differences need evaluation against an independent model
reference, not only agreement with an older Ferric route.

Sustained Qwen3-8B BF16 target-only 2,048/256, long-request arena reclamation,
controlled vLLM comparison, device-overlap plots, 700 tokens/s and M0-M7 remain
open. No speedup or production-readiness claim follows from this checkpoint.
