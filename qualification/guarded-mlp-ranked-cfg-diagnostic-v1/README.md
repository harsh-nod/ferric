# Ranked CFG Diagnostic

This diagnostic is intended to identify the function and declared-block count
behind the [guarded lowering rejection](../guarded-mlp-lowering-attempt-v3/README.md).
It preserves the 1,024-block limit, both rejection predicates, work charges
and atomic rules. It does not qualify a GPU executable or model performance.

## Qualified Retry

The [second receipt](attempt-v2/evidence/complete.json) records all 33 phases
passing naturally on MI350 in 651.464 seconds. All processes were reaped and
their groups were absent. Sources and dependencies remained unchanged, with
no timeouts, forced cleanup or integrity postcheck errors.

| Scope | Passing Executions | Historical Ignores |
| --- | ---: | ---: |
| Full compiler library | 1,247 | 24 |
| Full pliron library | 1,504 | 1 |
| Five inherited focused cohorts, repeated | 43 | 0 |
| New CFG diagnostic cohort, repeated | 8 | 0 |
| Atomic extraction controls | 8 | 0 |
| Unsafe-source rejection controls | 2 | 0 |
| Matrix and attention extraction | 2 | 0 |
| Total | 2,814 | 25 |

These are executions, not unique tests; the 51 focused checks repeat library
tests. Historical names and ignored identities are preserved. The eight new
tests pass in both the full suite and their explicit focused invocation.
Matrix/attention controls target gfx942 LLVM; selected atomic controls check
gfx950 LLVM. This is not a guarded gfx950 HSACO or GPU launch result.

The corrected source explicitly handles the diagnostic in `Error::source()`
and checks its `None` result at both rejection sites. No cap was raised.
The [retention manifest](attempt-v2/retention-manifest.json) pins 191 original
files, including all 169 raw records and the four source postimages. Receipt
SHA-256: `21fa53c3bd73e13dbb8f14aa85d1f8f60237becf230722b4046060f8462838f8`.
Seven final products are selected from the actual Cargo records and posthashed;
earlier compiler-build observations are not substituted for final products.
The new backend differs; the extractor body is byte-identical to its qualified
predecessor. Binary bodies are not included in the archive.

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
failed attempt did not reach these tests; the corrected retry passes them.

The qualified pre-diagnostic compiler remains untouched. The diagnostic
producer passes full CPU qualification. The subsequent
[guarded lowering retry](../guarded-mlp-ranked-cfg-lowering-v1/README.md)
identifies 1,121 blocks in `ferric_qwen3_mlp_state_guard_v1`, exceeding the
unchanged 1,024-block limit. All issue #42 milestones, independent model
numerics and the 700 tokens/s benchmark remain open.
