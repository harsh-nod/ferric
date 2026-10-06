# Ranked CFG Linear Fusion

Status: [source implementation independently reviewed](proposal-v1/README.md),
with nine authored regression tests. No new compiler qualification, artifact,
GPU execution, numerical acceptance or performance result is established.
All issue #42 milestones and the 700 tokens/s target remain open.

## Measured Baseline

The [previous guarded compile](../guarded-mlp-ranked-cfg-compaction-lowering-v1/README.md)
renders 1,675 blocks and 2,230 raw successor occurrences. Its first structural
identity refusal is at 1,025 blocks against a 1,024-block ceiling. The count
1,025 is not the graph total. The independent 2,048-edge admission gate has
not passed either.

The previous attempt's [raw stderr](../guarded-mlp-ranked-cfg-compaction-lowering-v1/attempt-v1/evidence/compile.stderr)
is 164,209 bytes, SHA-256
`9a309b8f23c66ad29e98a464bcb3ed8480766e2c7fd807415616dc99d6121a05`.
Its rendered guard contains 548 Acquire/System reads and four System writes,
three Relaxed and one Release. All 552 bounds predicates remain obligations.

## Source Change

Fuse an unconditional branch source with its target only when the target has
one raw predecessor. Preserve the entry block and do not deduplicate successor
occurrences. The production hook runs only above the existing 1,024-block
identity ceiling; smaller graphs keep their established layout. Independent
data-only analysis of the retained graph identifies
1,108 eligible branches: 556 empty sources and 552 atomic-effect sources.
Removing these branches would produce 567 blocks and 1,122 raw edges. These
are derived counts, not observed outputs from an implemented compiler pass.

Fusion concatenates operations without removing predicates or reordering
atomic effects. It must remap ranked block/operation coordinates in access,
executable-effect and wave records while preserving semantic identities.
Unsupported operations and argument-bearing graphs retain their original layout. New
scratch storage must be fallibly allocated and charged against the existing
canonical resource owner; independent resource limits remain unchanged.

This narrower pass does not need separate empty-block threading. Acceptance
requires differential traces, coordinate and identity checks, malformed-input
and budget refusals, the complete previous test census, and a fresh guarded
compile on MI350. Graph-size success alone will not establish HSACO, alias,
GPU, model correctness or sustained decode throughput.
