# Checked Attention Core Diagnostic

This is a bounded compiler observation after the
[422-test widening checkpoint](../guarded-mlp-core-u32-widening-qualification-v1/README.md),
not a production recognition rule or successful attention compilation.
The diagnostic uses base source revision
`d0253acecb63e8884e8727c84204cf2da1d0f00e` plus an explicitly pinned three-file
overlay. Ferric's production dependency and execution path are unchanged.

## Actual MI350 Run

Both compiler builds succeeded and all six renderer tests passed. The
unchanged `dynamic_attention_kernel_reaches_gfx942_llvm` control then returned
101, preserving the original source-origin refusal at core
`usize::checked_add`. The attention control's recorded leaf wall time was
15.110 seconds.

The [actual failed receipt](attempt-v1/evidence/failed.json) records all 19
phases exiting naturally and being reaped, with no remaining process groups.
The first 18 returned zero; no timeout or forced cleanup occurred. Sources
were unchanged and integrity postchecks were clean. The renderer tests check
output limits and completion reporting, not arithmetic correctness. This run
does not repeat or add to the earlier production qualification count.

## Observed Helpers

The [raw attention output](attempt-v1/evidence/attention-extraction-0.stdout)
contains nine complete helper bodies selected from 29 already-collected
instances reachable from the same attention root. No new callees or other
roots were traversed. The observed cohort is:

| Helper | Concrete Instances | Body Details Relevant To Recognition |
| --- | --- | --- |
| `usize::checked_add` | 1 | Overflow tuple, branch, guarded unchecked addition, and Option result |
| `usize::checked_div` | 1 | Zero-divisor branch and guarded division |
| `u32::checked_mul` | 1 | Overflow tuple, Option result, and an inlined `overflowing_mul` origin |
| `u32::is_multiple_of` | 1 | Explicit zero-divisor case, remainder assertion, and comparison |
| `Option::ok_or` | 3 | Discriminant, payload movement, Result construction, and error drop |
| `Option::and_then` | 2 | Discriminant, closure drop, and concrete `FnOnce` call |

The three `ok_or` payloads are `usize`, `u32`, and the genuine device
`DisjointTile2D<Index1D, 64, 16, 16, 4>`; all use `KernelError`.
The two `and_then` instances map `usize` to `usize` through distinct local
closures capturing a shared `u32` reference. Capturing a wrapper does not
authenticate its callback or drop behavior. Inlined-origin records also do
not constitute a separate capture of every inlined function's own body.

The derived [marker record](attempt-v1/derived-core-checked-attention-diagnostic.json)
joins byte-for-byte to the raw output: offset 5,325, extent 32,706 bytes,
including a 32,620-byte body and the completion trailer. Its SHA-256 is
`185be033df91cd8aac7bf1788830de1e446bf3ba0e9761fba0a05fd57a508165`.
The complete scan reports nine matched, retained and fully rendered bodies,
zero partial bodies and zero omissions. This describes the selected cohort,
not every helper in the source graph.

## Bounds And Remaining Work

The opt-in is enabled only for the attention control. Output is limited to
one 64 KiB aggregate per compiler process, with at most 4,096 scanned instances
and 24 retained bodies. Per-body limits remain 128 locals, blocks and scopes,
and 1,024 statements. The rejected body is first. Reserved trailer space keeps
partial and omitted counts visible if rendering reaches a bound. The hook
cannot approve a helper and leaves the original rejection intact.

The [retention manifest](attempt-v1/retention-manifest.json) pins 105 original
files and one derived observation. It preserves the controller, exact input
manifest, three diagnostic source bodies, original receipt and raw evidence.

Next are separately reviewed structural recognizers for the observed integer
and Option families, genuine-core positive and negative fixtures, and real-MIR
mutation tests. Independent call, drop and inline-origin checks must remain
intact, followed by the unchanged qualification controls. No such production
rule is qualified by this capture. The control targets gfx942 LLVM; guarded
gfx950 HSACO, GPU/model correctness, sustained BF16 single-request 2,048/256
decode and 700 tokens/s remain open.
