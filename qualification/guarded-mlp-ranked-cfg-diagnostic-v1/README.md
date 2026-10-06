# Ranked CFG Diagnostic

This diagnostic identifies the function and declared-block count behind the
[guarded lowering rejection](../guarded-mlp-lowering-attempt-v3/README.md).
It preserves the 1,024-block limit, both rejection predicates, work charges
and atomic rules. It does not qualify a GPU executable or model performance.

## First Attempt

The [first receipt](attempt-v1/evidence/failed.json) records a failed compiler
build on MI350. The compiler reported `E0004`: the new `CfgBlockLimit` error
variant was missing from the exhaustive `std::error::Error::source()` match.
The diagnostic module was therefore not qualified, and no tests executed.

Three setup/build phases exited zero; `compiler-products` exited 101
naturally. All four processes were reaped and their groups were absent.
There were no timeouts, forced cleanups or integrity postcheck errors.
Sources and dependencies remained unchanged. The attempt took 67.569 seconds,
which is a CPU qualification duration, not GPU or decode performance.

The [retention manifest](attempt-v1/retention-manifest.json) pins 46 original
files, including all 24 raw records, four source postimages, the controller,
helper, input and historical lineage. The failure receipt SHA-256 is
`b6ddf7a795f6c5199adcde7530b6c2bc26597bad1c94e1e3605a0b1c43f20858`.
Raw logs and receipts are retained unchanged; binary bodies are omitted.

## Diagnostic Design

The error records a bounded function identity, role, source location, actual
block count, limit and rejecting site. Available export/root labels are
bounded and escaped. A failure during callable-summary construction reports
its function index without inventing a root identity. Empty CFGs remain
distinct from oversized CFGs, and unreachable declared blocks still count.

Eight regression tests cover both boundaries, unreachable blocks, unchanged
work-budget precedence, callable/root context and bounded rendering. The
failed attempt did not reach these tests. The follow-up source adds the
missing explicit `None` error-source arm and an assertion for that behavior
at both rejection sites. Its fresh MI350 qualification is still pending.

The qualified pre-diagnostic compiler remains untouched. The next accepted
producer needs full CPU tests, final binary inspection and an actual guarded
lowering retry. No CFG limit has been raised. All issue #42 milestones,
independent model numerics and the 700 tokens/s benchmark remain open.
