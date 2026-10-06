# Atomic Load Alias Refinement

Status: **MI350 CPU compiler qualification passes; checked lowering pending**.
The source refinement addresses the
[combined-state descriptor refusal](../guarded-mlp-combined-state-lowering-v1/README.md).
It is not installed in the canonical compiler and has no successful checked
lowering, HSACO, GPU, model-numerical or performance result.

The [proposal](proposal-v1/README.md) distinguishes atomic loads from actual
writes while constructing formal alias requirements. Atomic access kinds,
receipt encoding, bounds and race checks, and descriptor ownership checks
remain unchanged. Stores and all read-modify-write operations still require
separation. This is an explicit semantic admission change, not just a compiler
speed optimization.

The [manifest](proposal-v1/source-manifest.json) pins one production-file
replacement and one added nine-test integration executable against the
qualified DAG compiler source. It is 3,781 bytes, SHA-256
`aedc4e4595c843c87586414da9fe120a5228e3655469e73ad878f0ff7a4d5a6d`.
The [patch](proposal-v1/integration.patch) is 26,440 bytes, SHA-256
`74d3e27f6fadb1fbe1a8bae751da88fc31e2efe78d0a6cc81e140ee64e7925e9`.

The exact source bodies were [formatted and checked on MI350](format-v1/evidence/complete.json)
with the pinned Rust toolchain. All three formatting phases exit naturally
with status zero, are reaped and leave no process group. The closed formatting
capsule has 20 members and 19 content pins. Its receipt is 6,774 bytes, SHA-256
`96a7cfbe0a143871270152e76d60708f648d8c745263b41299220ad2ce3513e0`;
archive SHA-256 is `e147382296e0b412765eb13016ed6de65ae9414fb48131248756567103e4d3c8`.
Formatting does not execute the tests or qualify the compiler.

## CPU Qualification

The [actual CPU receipt](attempt-v1/evidence/complete.json) records all 176
phases and 77 scopes passing on `mi350` in 944.338853 seconds. This is CPU
compiler testing on the GPU host, not a GPU execution result.

| Scope | Passing Executions | Ignored |
| --- | ---: | ---: |
| Kernel-IR library and 41 integration targets | 839 | 1 |
| Compiler library | 1,323 | 24 |
| Pliron library | 1,516 | 1 |
| Focused repeats and extraction controls | 162 | 0 |
| Total | 3,840 | 26 |

The nine new alias cases pass both their full integration suite and focused
repeat. Two existing shared-atomic descriptor rejection tests also pass their
focused repeats. Repeated executions are not unique tests. The kernel-IR
census is measured from this run; the prior compiler qualification did not
retain a raw kernel-IR census. Historical compiler and Pliron test names and
ignore sets are unchanged.

All child processes exit naturally, are reaped and leave no process group.
The 5,811-row source map is unchanged, dependency and artifact postchecks pass,
and no resource limit is raised. The semantic alias-admission and existing
scratch-map lifetime changes remain explicit; receipt encoding and ownership
checks are unchanged.

The [retention manifest](attempt-v1/retention-manifest.json) pins 906 bodies
in a 907-member capsule, including 884 raw evidence files, totaling 34,407,649
expanded bytes. The original receipt is 2,134,795 bytes, SHA-256
`18925ee6216977c561523f84a1899db379fcaaff99a050cc750ef32befebb708`.
The 5,589,785-byte archive has SHA-256
`1981e6368ce0a5ada3bd6c3afc17933303bab9db005a404b7a3df8c0a7b0a652`.
Binary bodies are not in the capsule; the 49 recorded products are metadata,
not permission to load or launch them.

The first data-only export refused before archive creation because its recipe
comparison omitted the recorded `prlimit` wrapper. The retained exporter
checks the exact wrapper and executable recipe. Neither the completed CPU
run nor its raw records was modified or repeated.

Required next gates are fresh compiler-tool auditing and actual checked
lowering, followed by separate runtime, GPU and model-numerical validation.
[34 tool-audit controller fixtures](../guarded-mlp-atomic-load-alias-tool-audit-v1/README.md)
and [31 lowering-controller fixtures](../guarded-mlp-atomic-load-alias-lowering-v1/README.md)
pass on MI350. They do not substitute for the actual binary audit or compile.
All issue #42 milestones and the 700 tokens/s target remain open.
