# Core U32 Widening MIR Observation

This 2026-10-05 MI350 diagnostic captures the genuine core `From<u32> for u64`
implementation rejected after the [415-test Result checkpoint](../guarded-mlp-core-result-qualification-v1/README.md).
**It adds no compiler admission and qualifies no provider, GPU kernel, model
result or performance target.** Ferric's dependency and production path remain
unchanged.

The isolated source tree derives from `b4705801ef39840ad6a408c2669dc38e80182cc0`
with three explicitly pinned diagnostic source bodies. It is not represented
as that exact Git revision. The existing matrix fixture and all production
authenticators are unchanged.

## Actual Outcome

Both compiler builds and all six bounded-renderer tests passed. The
[failed receipt](evidence/failed.json) preserves 19 natural, reaped phases
with no remaining process groups: the first 18 returned zero, and the unchanged
matrix extraction test returned 101 after 14.910 seconds at its original
external-helper source-safety refusal. Sources were unchanged and integrity
postchecks passed. This diagnostic did not rerun the prior 415-test cohort;
attention and GPU execution did not run.

The [derived observation](derived-core-u32-widening-diagnostic.json) is an exact
1,260-byte slice of the [raw output](evidence/matrix-extraction-0.stdout),
starting at byte 5,232. Its complete body is 1,180 bytes, with SHA-256
`e0b9ea98198ff49cb195e3edbf6bffd179241a5be068258a664a623debb63771`
for the full slice including markers.

| Observed Property | Value |
| --- | --- |
| Signature | Safe Rust `fn(u32) -> u64`, no generic arguments |
| Arguments / locals | 1 / 2 |
| Basic blocks / source scopes | 1 / 1 |
| Statements | 1 |
| Calls / cleanup blocks / inlined scopes | 0 / 0 / 0 |
| Assignment | `_0 = copy _1 as u64 (IntToInt)` |
| Terminator | `return` |

Both locals, the assignment and the terminator use scope zero. That scope has
no parent or inline origin. The actual core implementation and genuine
`From::from` trait identities are retained in the observation; the impl's
numeric definition index is evidence, not a proposed stable identity check.
This is numeric widening, not identity `From<T>` or a generic conversion rule.

## Scope And Next Gate

Selection requires the existing genuine-core identity boundary, the real
diagnostic `From` and `from_fn` anchors, and the exact monomorphic signature.
Only the matrix leaf receives the diagnostic opt-in. Output is limited to one
capture per process, 64 KiB, 128 locals/blocks/scopes and 1,024 statements.
The hook is immediately before the original rejection and cannot authorize
the helper. Partial output never counts as a complete observation.

The subsequent [production-recognizer run](../guarded-mlp-core-u32-widening-qualification-v1/README.md)
passed its six new tests and the matrix extraction control. Full qualification
remains incomplete at an attention checked-add refusal; no result is attributed
to this diagnostic itself.

The retention manifest pins 105 original files and one separately derived
observation. The recognizer requires genuine-core positives, wrong-conversion
and lookalike negatives, actual-MIR mutations and the unchanged qualification
controls. No production rule is qualified by this capture alone.
Checked HSACO emission, GPU/model correctness, sustained
single-request BF16 2,048/256 decode and 700 tokens/s remain open.
