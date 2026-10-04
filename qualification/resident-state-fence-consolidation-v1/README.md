# Paired Resident-State Fence Consolidation

CPU-qualified engineering checkpoint, 2026-10-04. This changes the private
fe2o3 resident-state observation path, not Ferric's production admission.
No GPU execution or performance measurement uses this worker yet. Issue #42
M0-M7 and the BF16 single-request Qwen3-8B 2,048/256 target remain open.

## Change

The prefix V6 and MLP V2 coordinators already place full group fences around
each pair of initial state reads and each pair of terminal state reads. Their
private adapter now reads state within those outer fences instead of calling
the public observer, which repeated two full fences per individual read.

The public observers and coordinator fence ordering are unchanged. The private
unsafe accessor requires the exclusive group borrow, no intervening dispatch,
map, release or rearm, and fresh successful outer fences. It preserves active
group, token, kind, extent, owner-only mapping and local allocation checks,
Acquire loads and quarantine on error. Only Ready and Submitted activations
are allowed. No topology snapshot is cached across calls. CPU tests do not
establish GPU coherence or remove existing external-isolation assumptions.

The eight implementation files are published at
[fe2o3 commit 36734d3bc](https://github.com/harsh-nod/fe2o3/commit/36734d3bc28ed038019d9a16da01c8533cdd83c9),
with byte-identical tested copies under [implementation](implementation/).
This is an engineering branch, not a main-branch runtime release.

## CPU Results

| Check | Actual result |
| --- | ---: |
| Selected runtime tests | 124 passed |
| Full finite-worker tests | 398 passed, 4 ignored |
| Newly added regressions | 6 passed, included above |
| Bounded commands | 17 natural successful exits; owned groups absent |
| Retained raw files rehashed locally | 88 |
| Source files in unchanged before/after inventory | 6,928 |
| Fresh worker | Built from an empty target |

The run executed on `ssh mi350-2` (`asrock-1w300-g2-2b`), with GPU visibility
disabled, two pinned CPUs, nice 10 and bounded resources. It combines the
exact archive/overlay sequence recorded by the controller, rather than
asserting that a Git label alone identifies the build. The parent was not
rebuilt. The new tests exercise invalid activation, identity, ownership and
mapping refusals and the actual adapter's private route. Existing tests cover
the coordinator's failure positions and unchanged public observer fencing.

- [Result summary](result.json)
- [Actual terminal receipt](cpu/complete.json), SHA256
  `33fb539a0823dcaa988d9091d7712f40cfb0ed656c87b6e958df98f8e35ccdf9`
- [Bounded controller and manifest](controller/)
- [Source overlay and exact preimages](controller/overlay.json)
- [Raw logs, test inventories and source postchecks](cpu/)

The rebuilt worker is 4,780,024 bytes, SHA256
`79d2b50a080a39300d02648c2844a398907ac4f89a41abff01c720ac58d42430`.
It has not replaced the deployed CPU475 worker. The superseded V1 build also
passed 522 tests; V2 removes six trailing blank lines and was independently
rebuilt and retested. All results here refer to V2, not transferred V1 evidence.

## Predicted Work Reduction

Four reads per resident launch, two nested fences per read, two resident
stages per layer and 36 layers give `4 * 2 * 2 * 36 = 576` fewer full group
checks per forward. This is static arithmetic, not a measured latency gain.

| TF4 forward | Recorded old group checks | Predicted candidate checks | Publication checks, expected unchanged |
| --- | ---: | ---: | ---: |
| 0 | 2,196 | 1,620 | 288 |
| 1 | 2,196 | 1,620 | 288 |
| 2 | 2,484 | 1,908 | 288 |
| 3 | 2,484 | 1,908 | 288 |

Old observations are in the [V7 TF4 host-policy sidecar](../independent-decode-observation-v1/tf4/host-observation.json).
Those timings are nested host scopes, not disjoint GPU durations. Do not sum
or subtract them to infer GPU throughput. Dispatch and public tensor-read
counts should remain unchanged.

## Remaining Gates

A separately versioned deployment must bind this worker to its CPU receipt,
renew runtime review and preserve the qualified parent, V7 image and workload.
The first GPU comparison must retain all 152 TF4 tensors, check bitwise
identity to the old worker, inspect actual counter changes, pass all six
device audits and complete Close/reaping. Independent framework numerical
diagnostics remain separate; the old full-model discrepancies are unresolved.
Neither a qualified speedup nor sustained 2,048/256 throughput follows from
these CPU results. No tolerance, lifetime bound or production gate was relaxed.
