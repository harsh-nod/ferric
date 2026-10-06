# Ranked CFG Linear Fusion

Status: the first MI350 qualification failed in a new test fixture before
fusion. The [fixture-only correction](proposal-v2/README.md) is independently
reviewed and a fresh full qualification is running. No new compiler
qualification, artifact, GPU execution, numerical acceptance or performance
result is established.
All issue #42 milestones and the 700 tokens/s target remain open.

## First Qualification

The [original attempt](attempt-v1/evidence/failed.json) is retained unchanged.
Its 15 phases completed naturally, with the compiler suite exiting 101;
all child processes were reaped and their groups were absent. Sources,
dependencies and postchecks are clean. The whole attempt took 255.104317 seconds.

| Check | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Pliron full suite | 1507 | 0 | 1 |
| Compiler full suite, raw result | 1322 | 1 | 24 |
| Loader controller fixtures | 25 | 1 | 0 |
| Lowering controller fixtures | 19 | 0 | 0 |

All 1,314 previous compiler passes and 24 ignored identities are preserved.
Eight new tests passed. The ninth, `cfg_linear_fusion_production_projection_route`,
failed while admitting the synthetic semantic owner, before running fusion:
`NonDeterministicOrder { entity: Block }`. The fixture encoded ascending block
ordinals as little-endian byte prefixes, which cease to sort at 255 to 256.
The correction uses big-endian encoding. Production code and all nine test
names are unchanged; the real-owner test is neither removed nor bypassed.

The CPU receipt's 1,507 passes and one ignore describe only the admitted Pliron
suite. The compiler's 1,322 passes, one failure and 24 ignores come from its
[raw stdout](attempt-v1/evidence/compiler-tests.stdout). Its failed exit stopped
qualification before focused repeats and the later extraction controls.

The separate [loader fixture failure](../guarded-mlp-ranked-cfg-linear-fusion-tool-audit-v1/controller-tests-v1/attempt-v1/evidence/failed.json)
patched the previous generation's completion constant, so the new pending
binding check correctly refused before the intended pending-driver check.
The correction changes only that test-local patch target. The original failed
receipt retains a null child observation; its raw stdout records 25 passes
and one failure. The [lowering fixtures](../guarded-mlp-ranked-cfg-linear-fusion-lowering-v1/controller-tests-v1/attempt-v1/evidence/complete.json)
passed all 19 checks. Both fixture cohorts will be rerun against the corrected
compiler generation. No actual new binary-loader audit or guarded compile
has been admitted from the failed CPU attempt.

The failed CPU capsule has 104 members, 103 pinned bodies and 79 raw files:
5,571,470 compressed bytes, SHA-256
`318b80b83505843c1034604fc3e21e756d7ddeb9c74c1529400ac2b74b48a70a`.
The failed loader and passing lowering capsules have 17 and 18 members,
respectively. These are compiler/controller results, not GPU measurements.

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
