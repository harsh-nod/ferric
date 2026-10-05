# Core U32 Widening Qualification

This checkpoint implements the exact core widening recognizer based on the
[captured target body](../guarded-mlp-core-u32-widening-diagnostic-v1/README.md).
Source revision: `d0253acecb63e8884e8727c84204cf2da1d0f00e`.
**The matrix extraction control passes; full qualification remains incomplete
because attention extraction fails.** Ferric's production dependency and
execution path remain unchanged.

## Implementation

The recognizer requires genuine core and `From::from` identities, a safe
nonvariadic Rust `fn(u32) -> u64` signature, and the complete observed body:
one integer-widening Copy cast followed by Return. Exact local types, places,
scope metadata and counts are checked. Calls, extra operations, other integer
conversions, identity `From<T>` and local lookalikes are not admitted.

The rule only discharges the external-HIR source-origin check. Ordinary MIR
import, callee traversal and independent inline-origin auditing remain intact.
The previous Result matcher and its 15 tests are byte-identical to V7.
Production diagnostic hooks are absent.

## Actual MI350 Run

The [retained receipt](attempt-v1/evidence/failed.json) records 422 passing tests
and a failed final attention control. Both compiler builds succeeded. All 36
phases exited naturally and were reaped with no remaining process groups;
the first 35 returned zero. Sources were unchanged and integrity postchecks
were clean. A transient SSH disconnect did not stop the original controller:
its finished receipt was retrieved after access recovered, without a restart.

| Cohort | Actual Outcome |
| --- | --- |
| Previous device, UI, provider and scalar controls | 390 passed |
| Result controls | 15 passed, including 172 existing MIR mutation assertions |
| New widening controls | 6 passed, including 49 real-MIR mutations and 8 identity/signature refusals |
| Atomic extraction controls | 8 passed |
| Unsafe/inlined-origin rejection controls | 2 passed |
| Matrix extraction | 1 passed |
| Attention extraction | 1 failed |

The [new cohort](attempt-v1/evidence/core-u32-widening.stdout) uses genuine
AMD-target core and compiler_builtins metadata, with a cached fixture result
and owned temporary storage. Its six tests passed in 7.86 seconds. Mutation
assertions are checks within those tests, not additional libtest executions.

The unchanged [matrix control](attempt-v1/evidence/matrix-extraction-0.stdout)
passed in 21.980 seconds of controller time. It checks the production Rust to
semantic MIR, ranked PLIRON, Kernel IR and gfx942 LLVM path, including the
kernel symbol, MFMA and workgroup address-space storage in emitted LLVM.
The control explicitly retains `artifact/launch authority false`: this is not
gfx950 HSACO emission or GPU execution.

## Remaining Failure

The unchanged [attention control](attempt-v1/evidence/matrix-extraction-1.stdout)
returned 101 after 15.441 seconds. Its first source-origin refusal is the
genuine core `usize::checked_add` helper. Cross-crate HIR is unavailable, and
optimized MIR does not retain unsafe-block syntax. This failure was previously
hidden behind the matrix failure; it has not been bypassed or counted as a pass.

Next is a bounded, non-admitting capture of the relevant checked-arithmetic
helpers and related Option wrappers reached by this attention kernel. Any
production recognition must retain their arithmetic, overflow, control-flow
and source-origin checks, then pass the same unchanged controls.

The retention manifest pins 193 original files, including six compiler source
bodies, the controller, input manifest and all raw evidence. Full provider
qualification, the Ferric compiler-generation port, guarded gfx950 HSACO,
GPU/model correctness, sustained single-request BF16 2,048/256 decode and
700 tokens/s remain open.
