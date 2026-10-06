# Bounded Indexed-Atomic Dead-Cast Census

This compiler experiment addresses the [measured dead-cast analysis rejection](../guarded-mlp-indexed-atomic-membership-lowering-v1/README.md).
It does not change GPU arithmetic or launch geometry.

## Change And Bound

The previous analysis rescanned an immutable function for every candidate
unused pointer-cast temporary. The patch builds one lazy local-mention census
inside that function's escape check. A local is either never mentioned,
mentioned only in one statement, or mentioned in multiple statements or a
terminator. Multiple mentions in the defining statement still qualify, because
the old scan excluded that entire statement. Unreachable blocks, projection
index locals, and every old operand position remain covered. Storage lifetime
markers remain ignored. All original pointer, type, role, membership, layout,
escape and alias checks remain in place.

For N locals, B blocks, S statements, T terminators, O operand visits and P
place/projection visits, construction charges N+B+S+T+O+P, then one unit per
query. The old approach repeated traversal for each candidate. These are
analytical compiler work charges, not measured GPU speedups. A negative query
that formerly stopped early can now require more work.

The new scratch allocation is explicit: one vector, capped at the existing
262,144 capability-state-entry limit, allocated fallibly. Initialization and
traversal are charged. No partial census is published after a failure; no cache
can cross function boundaries. The 3,145,728 graph-work ceiling and structural
limits are unchanged. Resource admission may change; the memory footprint is
not claimed to be unchanged.

## Tests And Status

Nine authored tests include 1,652 differential queries against the previous
scanner, exact defining-statement exclusion, terminators, unreachable blocks,
invalid local IDs, unchanged cast rejection checks, scratch and work bounds,
failure/retry behavior, and 1,656 distinct candidates sharing one scan.

The [proposal](proposal-v1) changes three existing source files and adds one
test file over the qualified membership generation. The source manifest is
`2b6050714c19c9661bb2fc6a7fff022833136be617c6cad0f90dcfc8cb4ff8fa`;
the patch is
`c9b25b89933bf5651fdea6d898eb07a2f553c76155148ee15350afd4b581577a`.

Full qualification on `mi350` passed in 663.612525 seconds. All 38 parent
recipes remain, with one focused census repeat added.

| Check | Result |
| --- | ---: |
| Compiler suite | 1,284 passed, 24 ignored |
| Pliron suite | 1,507 passed, 1 ignored |
| Total, including repeats and extraction controls | 2,894 passed, 25 ignored |
| Ordered phases / test scopes | 39 / 26 |

Every child exited naturally with status zero, was reaped, and left no process
group. Source and dependency postchecks were clean. Historical test and ignore
identities were preserved. These are CPU compiler results, not GPU results.

[attempt-v1](attempt-v1) retains the actual receipt, 199 raw files, controller,
helper, direct baseline evidence and four source bodies. Its archive contains
222 members and 221 pinned bodies, without exporting compiled binaries.
The receipt is
`3a5ae5b18b497282a05895b8beca033f316a261c5632f4fbecf02832cd552e3f`;
the archive is
`bc367c1cc464a7f3f87357dcacff5814309589367ce7f8be186130832bd710b6`.

The separate [loader audit](../guarded-mlp-indexed-atomic-dead-cast-census-tool-audit-v1/README.md)
passed all 14 binary/library checks. Its corrected synthetic controller
fixtures passed 26 tests; the [lowering fixtures](../guarded-mlp-indexed-atomic-dead-cast-census-lowering-v1/README.md)
passed 19. The original failed loader fixture is retained, not replaced.

The [actual guarded compile](../guarded-mlp-indexed-atomic-dead-cast-census-lowering-v1/README.md)
using those audited tools still failed at the unchanged graph-work ceiling.
The measured charge moved from dead-cast analysis to `validate_statement`'s
552-entry atomic-use lookup. No HSACO, GPU execution, independent model
numerical result, or performance result is established here. All issue #42
milestones and the 700 tokens/s target remain open.
