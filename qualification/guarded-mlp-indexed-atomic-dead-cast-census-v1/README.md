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

MI350 qualification is pending. Its planned 39 phases retain all 38 parent
recipes and add one focused repeat. With nine new full-suite tests and nine
repeated executions, success would require 2,894 passing executions and the
same 25 ignored tests. These are planned counts, not results.

The actual guarded compile must then be retried using separately audited
tools. No HSACO, GPU execution, independent model numerical result, or
performance result is established here. All issue #42 milestones and the
700 tokens/s target remain open.
