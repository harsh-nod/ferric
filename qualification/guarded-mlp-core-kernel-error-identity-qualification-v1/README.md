# KernelError Identity Qualification

The scoped compiler controls now pass on MI350 at fe2o3 revision
`5a500d63c29b78f8788356b20dfcbb5c41ec20c9`. This closes the attention extraction
failure observed in the [438-test checkpoint](../guarded-mlp-core-checked-attention-qualification-v1/README.md).
It is not the full runtime compiler qualification, a gfx950 HSACO result,
GPU execution, independent model numerics or a performance claim. Ferric's
production dependency and execution path are unchanged.

## Actual Results

The [complete receipt](attempt-v1/evidence/complete.json) records 445 named
passing tests across 22 validated scopes. All 39 phases exited naturally with
status zero, were reaped and left no process groups. No timeout or forced
cleanup occurred. Source and dependency maps were unchanged; integrity
postchecks were clean. Every previously passing named test is preserved.

| Scope | Passing Tests |
| --- | ---: |
| Device library and two UI targets | 358 |
| Trusted provider and scalar pipeline | 32 |
| Core Result wrappers | 15 |
| Core u32 widening | 6 |
| Checked integer wrappers | 6 |
| Attention Option wrappers | 10 |
| KernelError identity conversion | 6 |
| Atomic extraction controls | 8 |
| Unsafe-source rejection controls | 2 |
| Matrix and attention extraction | 2 |
| Total | 445 |

The compiler builds took 40.601 and 47.274 seconds. The new
[six identity tests](attempt-v1/evidence/core-kernel-error-identity.stdout)
passed in 14.831 seconds, including a genuine positive control, eight
identity/signature refusals and 38 real-MIR mutations. Mutation assertions
are not additional named tests.

The unchanged [matrix control](attempt-v1/evidence/matrix-extraction-0.stdout)
passed in 21.807 seconds; the previously failing
[attention control](attempt-v1/evidence/matrix-extraction-1.stdout)
passed in 44.775 seconds. These controls lower the dynamic matrix and flash
attention examples to gfx942 LLVM on the MI350 host. They do not emit an
accepted gfx950 executable or launch a kernel. The passing atomic controls
include gfx950 LLVM extraction, also without GPU execution.

## Narrow Compiler Change

The earlier [complete MIR observation](../guarded-mlp-core-kernel-error-identity-diagnostic-v1/README.md)
is now enforced by an exact production check. It requires genuine core
`From`/`from_fn` identity, the authenticated provider's concrete `KernelError`,
and a normalized safe Rust signature `KernelError -> KernelError`. The body
must have two matching locals, one uninlined root scope and one non-cleanup
block containing only `_0 = move _1` followed by return.

This is not general permission for identity conversions. Extra operations,
projections, altered locals, scopes, operands, control flow and origins are
rejected. Existing provider authentication, reachable unsafe-source checks
and other callee checks remain in force. The diagnostic overlay is absent
from this production qualification.

## Reproduction And Limits

The [retention manifest](attempt-v1/retention-manifest.json) pins 214 original
files, including 199 raw evidence records, twelve compiler source bodies,
the controller and its full input manifest. The complete receipt SHA-256 is
`9328690781c00c68c745433b5ff8e1e23dc92e915568876f62d6622d6424e7a2`.
Commands and environments are retained per phase. Builds were offline and
locked, used two CPU cores/jobs with GPUs hidden, and had bounded process,
memory, output and scratch lifetimes.

Next is the fresh full runtime compiler/pliron qualification with the
ordinary-induction and RPO changes, followed by the loader audit and guarded
gfx950 lowering. This scoped 445-test result does not replace those gates.
The GPU worker, independent model validation and sustained single-request
Qwen3-8B BF16 2,048/256 workload remain open; 700 tokens/s is not reached.
