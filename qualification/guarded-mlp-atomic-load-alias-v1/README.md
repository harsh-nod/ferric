# Atomic Load Alias Refinement

Status: **reviewed source proposal; compiler qualification pending**. The
proposal addresses the
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

Required next gates are full kernel-IR library/integration qualification,
the existing compiler and Pliron suites, focused descriptor rejection tests,
fresh tool auditing and actual checked lowering. All issue #42 milestones
and the 700 tokens/s target remain open.
