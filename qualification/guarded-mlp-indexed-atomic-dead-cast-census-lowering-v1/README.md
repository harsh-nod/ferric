# Dead-Cast Census Guarded Lowering

The controller consumes the [census tool audit](../guarded-mlp-indexed-atomic-dead-cast-census-tool-audit-v1/README.md)
and its qualified CPU producer. The guarded candidate, vendor, driver, gfx950
target and optimized-inline normalization are unchanged. No work or structural
ceiling is raised for this attempt.

## Controller Fixtures

[controller-tests-v1/attempt-v1](controller-tests-v1/attempt-v1) passed all
19 synthetic tests on `mi350` in 4.972207 seconds: 15 admission checks and four
normalization checks. The child exited naturally with status zero, was reaped,
and left no process group. Sources stayed unchanged; postchecks were clean.
There were no skipped tests, expected failures or unexpected successes.

The receipt is
`6dca3d1e430e463dcc3512d4d680ff143ff19f4784878b0b3bc068a87756173e`;
the retained archive is
`44b261dfa0cb2817d539fcfbba291ab74b9f988e2d5cd4dac23763018de60ebe`.
It contains 16 members, 15 pinned bodies and seven raw files.

## Actual Guarded Compilation

The actual compile using the audited tools failed after 117.357422 seconds
(121.042018 seconds for the controller). It exited naturally with status one,
was reaped, and left no process group. There was no timeout or forced cleanup;
source and input postchecks were clean. No HSACO was emitted.

The diagnostic identifies the next exhausted work charge:

| Field | Measured Value |
| --- | --- |
| Caller | `indexed_atomic_v1.rs:101:14`, `validate_statement` |
| Work before / requested / sum | 3,145,488 / 552 / 3,146,040 |
| Unchanged work ceiling | 3,145,728 |
| Function index / role | 1 / `KernelRoot` |
| Kernel | `ferric_qwen3_mlp_state_guard_v1` |

This lookup linearly searches the authenticated atomic-use inventory for the
current block and statement. The previous attempt exhausted work in dead-cast
analysis; this attempt reaches a different charge. That movement is not a GPU
speedup, semantic acceptance, or evidence of a later alias-check failure.

[attempt-v1](attempt-v1) retains the controller, failed receipt and all ten
raw files, including stderr, command and child lifecycle records. The archive
has 13 members and 12 pinned bodies. Receipt:
`34699054a75a7300bd308d99e22635c25237c3d85ecb8f15a1ca9de00c5831fd`.
Archive:
`469ad45cfaa89772a00cfc9bad29439249329bf735f5b5a5e9fdc5cb2620a069`.

Guarded HSACO emission remains open. No GPU, independent model numerical or
performance acceptance is claimed. All issue #42 milestones and the
700 tokens/s target remain open.
