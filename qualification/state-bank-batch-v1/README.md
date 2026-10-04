# Bounded State-Bank Batching

CPU-tested engineering checkpoint, 2026-10-04. Ferric now reads each complete
prefix/MLP state bank through a bounded fe2o3 batch operation. This candidate
has compiled and passed its CPU tests on `ssh mi350-2`
(`asrock-1w300-g2-2b`). It has not yet been deployed or measured on the GPU.
The subsequent [deployment/controller checkpoint](../state-bank-batch-observation-v1/README.md)
records 144 passing policy tests and a completed worker export, not a GPU run.
All issue #42 milestones and the single-request BF16 Qwen3-8B 2,048/256,
700 tokens/s target remain open.

## Implementation

Each of 36 layers has two prefix and two MLP state records: 144 records per
bank. Previously, each state read repeated a full-group entry and exit check.
The new `observe_state_bank_v1` operation validates the complete, distinct
typed roster, performs Acquire loads under an exclusive group borrow, and
returns snapshots only after a fresh exit check. It accepts 1 through 144
entries, never caches topology across calls, and quarantines the group on
error without returning a partial snapshot vector.

Ferric still validates every state before the first rearm store. Both outer
ledger fences, each individual rearm validation, rearm ordering, second
readback scan, and generation commit rules are unchanged. Idle bank reads
can observe Ready, Submitted and Completed allocations; the existing private
resident path still accepts only Ready or Submitted. Snapshots grant no
completion, dispatch or reuse authority. Bounded vectors avoid a whole-bank
stack allocation.

The paired runtime implementation is
[fe2o3 commit 3d217aabc](https://github.com/harsh-nod/fe2o3/commit/3d217aabc3c48e9c767fa28a05bd596987c29f9d).
The [implementation copies](implementation/) match the compiled and integrated
bytes; the [overlay](controller/overlay.json) records seven replacements and
three additions across the two repositories.

## Actual CPU Results

| Check | Result |
| --- | ---: |
| Selected runtime tests | 144 passed |
| Full finite-worker tests | 409 passed, 4 unchanged ignored |
| New runtime/worker regressions | 21 passed, included above |
| Mapped atomic-state/rearm tests | 10 passed, included above |
| Controller policy tests | 14 passed, separate from the Rust total |
| Bounded build/test commands | 21 natural successful exits; owned groups absent |
| Raw records retained and rehashed | 109 |
| Compiled source inventory | 6,931 unchanged files |

The build started with an empty target, reused the offline dependency cache,
and used two pinned CPUs with nice 10 and disabled GPU visibility. The parent
binary was not rebuilt. Tests cover whole-roster validation, duplicate and
foreign identities, invalid activation/mapping, mixed typed order, missing or
extra snapshots, terminal failures, all existing rearm failure positions,
and the unchanged individual observer behavior.

The [actual CPU receipt](cpu/complete.json) has SHA256
`14dab6e776cdc2184457b2865cf7d48b91a71008d49bea040deca68bba75838f`.
The selected worker is 4,797,496 bytes, SHA256
`6901a1319352d0401e452d616efa50d2de08c40943f50cb054d89f3f26d7fd52`.
See the [result summary](result.json), [controller](controller/),
[controller test log](pure/tests.log), and [raw CPU records](cpu/).
Binary bodies remain in the task-owned evidence store, not Git.

## Predicted Work Reduction

Two scans of 144 records with two checks per record previously used
`2 * 144 * 2 = 576` group checks. Two batch operations with an entry and exit
check use `2 * 2 = 4`. The predicted saving is **572 checks per forward**.
The outer ledger checks and all individual rearm checks remain in both totals.

| TF4 forward | Prior measured group checks | Candidate prediction | Expected publication checks |
| --- | ---: | ---: | ---: |
| 0 | 1,620 | 1,048 | 288 |
| 1 | 1,620 | 1,048 | 288 |
| 2 | 1,908 | 1,336 | 288 |
| 3 | 1,908 | 1,336 | 288 |

Prior measurements are in the
[CPU522 GPU checkpoint](../resident-state-decode-observation-v1/tf4/README.md).
These predictions are not measured latency, throughput, or a speedup claim.

## Remaining Validation

The next GPU run must bind the new worker to this CPU result while retaining
the parent, V7 image, numerical prerequisites, six device audits, one-attempt
limit and Close/reaping checks. All 152 tensors and four complete payloads and
tokens must match the prior worker. Forwards three and four must exercise
Completed-bank reuse. Independent full-model numerical acceptance, sustained
2,048/256 timing, and production admission remain separate, unresolved gates.
This checkpoint changes neither `main` nor the tutorial site.
