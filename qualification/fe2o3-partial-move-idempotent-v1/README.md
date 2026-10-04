# Idempotent Partial-Move Compiler Qualification

This is an isolated fe2o3 compiler delta used for Ferric's gfx950 bring-up,
not a Ferric runtime adoption or an HSACO replacement. The
[patch](compiler.patch) contains the implementation and six regression tests;
[source pins](source-pins.json) identify its exact before/after file bodies.

## Change

The partial-move analysis records an empty path when an entire local is moved.
That marker subsumes every field or descendant of the local. Previously, a
repeated whole-local mark cleared and reinserted the same empty path, charging
another cumulative storage entry even though the abstract state was unchanged.

Check for an existing whole-local marker before clearing the set. Repeated
whole-local and descendant marks then return without another insertion.
Genuine new insertions still consume budget. Field-to-whole replacement still
charges once; clearing or reinitializing a local does not refund earlier
charges. Storage/work limits, overflow handling and moved-value rejection are
unchanged. This is a compiler-work optimization, not a measured GPU speedup.

## Regression Coverage

The six added tests cover:

- Repeated whole-local and descendant marks at the exact storage limit.
- Rejection of a genuine new insertion beyond that limit.
- Cumulative charging across actual `StorageLive`/`StorageDead` operations.
- Field-to-whole replacement without refunding prior insertions.
- Rejection of reads from wholly moved locals and their descendants.
- An unchanged fixed-point merge that does not recharge the same marker.

The last test exercises the analysis helpers; existing CFG, loop, join and
streaming-replay tests must also run. The build-host runner inventories and
runs the complete `fe2o3-pliron` and `rustc-codegen-fe2o3` default library
suites, with ignored tests reported separately, before building a new compiler.
The execution result is recorded separately from this source description.

## Measured CPU Result

The fresh-source run on `mi350-2` passed on 2026-10-03. The
[execution summary](cpu-result.json) records the source, receipt and artifact
hashes, named new regressions, ignored tests, and all ten completed commands.

| Check | Passed | Ignored |
| --- | ---: | ---: |
| Complete `fe2o3-pliron` default library suite | 1,487 | 1 |
| Complete `rustc-codegen-fe2o3` default library suite | 1,188 | 24 |
| CPU-runner parser and policy tests | 9 | 0 |

All six new regressions executed. Both default suites were compared against
their complete named test inventories. The compiler backend and extractor
built successfully in the fresh target. Original sources, copied sources,
dependencies and prior targets passed their before/after checks; all owned
processes exited naturally and were reaped. The raw records, full compiler
source copy and five selected compiler/test products were retained and
rehashed locally.

The first run's controller rejected a passing libtest `should panic` annotation
after the Pliron suite passed. That failure is preserved. The parser was fixed
and regression-tested, then both suites and the compiler build ran again under
fresh output paths. No test failure was converted into a pass.

The actual prefix-tile replay is among the 24 ignored compiler tests. CPU
success therefore does not qualify that replay; it must execute against the
next candidate's actual retained captures.

## Source Boundary

The implementation preimage is SHA-256
`2de297a25d2556452f6cebe6bb13ecf0f9e4ef2c8fe6980b878c6337d0347c13`.
The test file must be absent. This is the retained compiler generation used
by the failed reciprocal probes, not the separately published runtime-tree
generation whose file starts with hash `97584ebf`.

Check both conditions against `source-pins.json` before applying:

```sh
git apply --check /path/to/compiler.patch
git apply /path/to/compiler.patch
```

The source pins identify this delta, not a complete publicly reproducible Git
baseline. The full copied compiler source and dependency snapshots remain
part of the build-host evidence. Do not apply the patch to another generation
and treat matching context lines as equivalent provenance.

## Remaining Gates

Both reciprocal candidate probes exceeded the unchanged partial-move storage
budget before producing an HSACO. The first rejected charge does not identify
the responsible insertion site or the total function requirement. This patch
may or may not make the complete function fit.

CPU regression success and fresh compiler binaries are prerequisites, not
evidence that checked lowering, replay, emission or GPU execution works.
Those need a new pinned compiler/tool recipe and separate MI350 numerical
validation. The existing CPU475 worker and CPU633 parent are not rebuilt by
this qualification work. No production, full-model or 700 tokens/s claim is made.
