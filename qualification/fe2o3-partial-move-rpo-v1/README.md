# Partial-Move Reverse-Postorder Compiler Qualification

The revised fe2o3 compiler passed 2,700 Rust tests on ASROCK through
`ssh mi350-2`. The 25 pre-existing ignored tests remain ignored. All twelve
build/test phases and the owned process completed naturally, with unchanged
source, dependency, input and configuration postchecks.

| Suite | Passed | Ignored | New Passing Tests |
| --- | ---: | ---: | ---: |
| fe2o3-pliron | 1,504 | 1 | 17 |
| rustc-codegen-fe2o3 | 1,196 | 24 | 0 |
| Total | 2,700 | 25 | 17 |

## Change

Partial-move analysis now uses the existing reverse-postorder block ordering
to prioritize pending work. It deduplicates queued blocks and revisits earlier
blocks when a backedge changes their incoming state. The transfer functions,
joins, reachability and call/unwind behavior are unchanged.

The scheduler reserves its additional storage before allocation and charges
initialization, enqueue operations and scanned slots to the existing work
budget. The storage limit remains 2,097,152 logical words; the work limit remains
67,108,864 units. There are no refunds or relaxed limits. Differential tests
compare the new scheduler with the prior FIFO algorithm, including loops,
irreducible control flow, invalid moves and resource-boundary refusals.

The four formatted, tested Rust files are under [source](source). The
[patch](compiler.patch) applies to the qualified ordinary-induction compiler
generation recorded in [source-pins.json](source-pins.json), not an arbitrary
fe2o3 checkout. This directory does not replace Ferric's production compiler.

## Evidence

[result.json](result.json) records the exact source transition, named outcomes,
Cargo product identities and publication file hashes. [complete.json](complete.json)
and [owner-complete.json](owner-complete.json) retain the actual remote results.
Per-phase commands, output and process records are under [raw](raw).

The publisher rehashes the retained source snapshots and evidence archive. It
does not rerun tests, transport the full compiler tree, or rehash the five
compiler executable/library bodies. The separate 16-test controller-policy
result is retained as a primary-agent SSH observation, not a supervised remote
test receipt. These distinctions are also explicit in the result's limitations.

## Remaining Gates

The earlier RoPE candidates exceeded the partial-move storage budget. This
compiler change is intended to reduce repeated state propagation; its CPU test
success alone does not establish that the actual RoPE kernel now lowers.

Fresh finalizer qualification, both actual-capture replay checks, checked
gfx950 HSACO emission and GPU numerical comparison remain separate steps.
There is no new HSACO, model-accuracy acceptance, sustained 2,048/256 decode
result, speedup measurement or 700-token/s claim in this checkpoint.
