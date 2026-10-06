# Membership Guarded Lowering

## Actual Compilation

The qualified membership build still rejects the guarded gfx950 compilation.
The diagnostic now identifies `indexed_atomic_temporary_unused_v1`, at
`indexed_atomic_dead_cast_v1.rs:99:23`, instead of the previous linear
membership query. It is the statement charge inside a whole-function scan
that checks whether a candidate temporary is mentioned outside its definition.

| Diagnostic field | Measured value |
| --- | --- |
| Kernel | `ferric_qwen3_mlp_state_guard_v1` |
| Function index / role | 1 / `KernelRoot` |
| Phase | `root-projection` |
| Work before / requested charge | 3,145,728 / 1 |
| Attempted sum / unchanged limit | 3,145,729 / 3,145,728 |
| Compile child | Natural exit 1 after 117.360708 seconds |

The failed charge does not commit the attempted sum. The controller completes
in 120.926201 seconds with the child reaped, its process group absent, unchanged
candidate source and clean input/source postchecks. There is no timeout or
forced cleanup, and no HSACO or GPU execution. This is not an alias-check
verdict, a numerical result or a performance measurement.

[attempt-v1](attempt-v1) retains the actual failed receipt, deployed controller
and ten raw files, including the complete command and stderr. The receipt is
`914ba56097474530279206963c2a08268026cfc3ea389cce30a81157e653dfde`;
stderr is `7dbc29c02010d6d2a34a20992381de9faccff678d5540aa0c1a2f5609e88cb6e`.
The 13-member archive is
`73e4608221221b7c0d53d359c09a435f131dc84d4583602b7a6802d3b1aee622`.
The retained controller's `actual_failure_caller_identified=false` is unchanged;
the attribution here is derived from that pinned stderr and qualified source.

The next candidate is a bounded, function-local census of local mentions to
avoid rescanning the whole function for each dead sibling cast. It must preserve
the exact statement, projection and terminator checks and the unchanged work
ceiling. No implementation or successful result for that candidate is claimed.

## Controller Fixtures

The membership generation's lowering controller passes all 19 synthetic
fixtures on `mi350` in 3.962865 seconds: 15 admission/output-contract tests
and four normalization tests. The one bounded child exits naturally with
status zero, is reaped, and leaves no process group. Sources stay unchanged,
with clean postchecks and no timeout or forced cleanup.

The tests require the current qualified producer and loader receipts, retain
the original driver and vendor contracts, and reject stale generations,
changed limits, tool substitutions and missing bindings before effects.
A source-AST test separately checks the actual emitted result flags and
membership test roster. Baseline failure attribution is not promoted to a
claim about a new compilation.

[controller-tests-v1/attempt-v1](controller-tests-v1/attempt-v1) retains the
tested sources, fixed runner, input, actual receipt and seven raw files.
The receipt is `775742900d6bb0b7b0fb41b0d091939515467be37de0aebd478874cca9df60ac`.
The 16-member archive is
`13cc05ede8bc75da7451163d273ab6e2f88b41d74bdf8ffbd3aef06cfe890d91`.

These tests are separate from the actual failed compilation above, which used
the [qualified membership CPU build](../guarded-mlp-indexed-atomic-membership-v1/README.md)
after all [14 fresh loader inspections](../guarded-mlp-indexed-atomic-membership-tool-audit-v1/README.md)
passed. All issue #42 milestones and the 700 tokens/s target remain open.
