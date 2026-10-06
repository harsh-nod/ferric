# Guarded MLP Memory-Bounds Preflight Analysis

Source/data-only investigation, 2026-10-06. No compiler, project import, test,
SSH, GPU execution, or canonical edit was performed by this reviewer. Standard
library parsing, hashes, graph inventory, and integer arithmetic were used.
This report proposes no source change and does not qualify a future algorithm.

## Authenticated Evidence

Current files are under
`/mnt/c/Users/harmenon/ferric-session-evidence/20261004/cfg-linear-fusion-lowering-failure-v3/`:

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| `failed.json` | 17737 | `8d4cc1a0cfe0c9104282c3d20349638a616bd98e7e28f92598dcaf0110057ca2` |
| `compile.stderr` | 123245 | `c60dc2edc62bfe11ba7a8f79ce1465322d2c887aeb3fa5661dae8002a33c5095` |

The matched pre-fusion history is under
`/home/harsh/ferric-p227-integration/qualification/guarded-mlp-ranked-cfg-compaction-lowering-v1/attempt-v1/evidence/`:

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| `failed.json` | 16503 | `1bf26b28162414dfb4afbf6dc5ba5a70a60d1c1fe1fd55624c251733840c560b` |
| `compile.stderr` | 164209 | `9a309b8f23c66ad29e98a464bcb3ed8480766e2c7fd807415616dc99d6121a05` |

Current stderr line 65 reports exactly:

```text
fe2o3 rustc extraction: production compilation general kernel verification failed: production analysis resource limit exceeded [memory-bounds]: memory-bounds work hard limit
```

The following rendered function is
`ferric_qwen3_mlp_state_guard_v1`. The receipt remains failed, with artifact
`null`, `source_unchanged=true`, and no postcheck errors. This is not an alias
verdict, numerical result, device run, or successful artifact emission.

## Closed Rendered Graph Inventory

The parser selected the text from the four-space-indented `func` line through
its matching four-space-indented closing brace, excluding the final newline.
It required contiguous block labels, a supported final terminator in every
block, and every successor to resolve inside that function.

| Property | Prior compaction | Actual linear fusion |
| --- | ---: | ---: |
| Blocks | 1675 | 567 |
| Raw successor occurrences | 2230 | 1122 |
| Distinct `(source,target)` edges | 2230 | 1122 |
| Operations including terminators | 3348 | 2240 |
| Empty unconditional-branch blocks | 561 | 4 |
| Reachable blocks from entry | 1675 | 567 |
| Whole graph acyclic | yes | yes |
| `kernel.br` | 1114 | 6 |
| `kernel.cond_br` | 552 | 552 |
| `kernel.index_eq_br` | 5 | 5 |
| `kernel.analysis_split` | 1 | 1 |
| `kernel.trap` | 2 | 2 |
| `kernel.return` | 1 | 1 |

No block header in either rendered graph has arguments. Raw duplicate
successor occurrences are absent in these two particular graphs; this is not
permission for an implementation to deduplicate edges on other inputs.

The extracted prior region is 155836 bytes, SHA-256
`84c48a633f78884d1bd90a3d3b1ed9e4cc76aae91b4e0991ca19c83221dd7ff5`.
The current region is 115154 bytes, SHA-256
`fe899ebe6efa8a8c7619777a7b5ca080f0d5abdb0ee28969c192417025944b0b`.
These region hashes use the exact selection above, not the full stderr hash.

All nonterminator text, including SSA names, has the same multiset before and
after fusion. More strongly, an independent data-only reconstruction found
1108 old unconditional edges whose non-entry target had exactly one raw
predecessor and whose source was non-entry. Concatenating those chains in
ascending original-head order, retaining each final terminator and remapping
all successor labels, reproduced every current block's complete ordered
operation text exactly. This checks the rendered operation order, predicates,
and targets rather than only their counts. It does not reconstruct metadata
that the renderer omits.

Atomic operations are unchanged:

- 548 `AtomicRead <Acquire, System>` operations on `%1118`, with each literal
  index `0..547` present exactly once.
- Three `AtomicWrite <Relaxed, System>` operations and one
  `AtomicWrite <Release, System>` on `%1119`, at indices `0..3` exactly once.
- The exact atomic operation strings and their chain-replayed order are
  unchanged. Fusion removed only 1108 unconditional branches.

## Exact Source Attribution

Inspected source root:
`/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216/guarded-mlp-ranked-graph-work-diagnostic-input-v228-v2/fe2o3`.
Each file below was independently hashed and joined to the qualified compaction
source map at
`/home/harsh/ferric-p227-integration/qualification/guarded-mlp-ranked-cfg-compaction-v1/attempt-v1/evidence/sources-after.json`,
SHA-256 `c00481ef4e62adf81ebce11b6a2a9ecb51823a709c28ccf87194c54c5fa7f420`.
None is among the four changed paths in the subsequent linear-fusion source
manifest `8ac7037f7096c4a053876baef36f94dfffb1c7b0072b02bb199e5121e25f041e`.

All paths in this table are relative to
`crates/fe2o3-pliron/src/production_analysis/`:

| Path | Bytes | SHA-256 |
| --- | ---: | --- |
| `pliron_ranked_bounds.rs` | 23920 | `fb859914f0079bbea14c50e92e2a4eb545f68c65f2009b4cf93348466b6816b9` |
| `pliron_ranked_bounds/execution_v1.rs` | 16101 | `15e41c7d479c29767e8c20310b3cee48ac78279a533d362ee6e46cb2d06bb21f` |
| `pliron_ranked_bounds/facts_v1.rs` | 15635 | `272ef704fb02d3228ef1a8c2820eff8164a5325d6baaab519c8da8fe8d29558e` |
| `pliron_ranked_bounds/access_proofs_v1.rs` | 12399 | `39310163e2bec6e01478536ad52ba846199941f44184626661df8624359343af` |
| `pliron_pipeline.rs` | 48887 | `43ac9c97be8eb09a08b2931e29272ea03526346b366bb3a33d8a159d39855a1f` |
| `pliron_pipeline_protocol.rs` | 46339 | `9268cbbf8fcf963767a4a06c08ccda1844441196a034ab0b76a97bad9ef9af58` |
| `pliron_ir_identity/capture_v1.rs` | 20945 | `f0cf9c1688b26907d2e2415d4c180ff1b06ec7de91993257e57b6cab45b0d215` |
| `pliron_resource_envelope.rs` | 30052 | `eedd9fe9f9eeca93422c3f8ab195aa63c28e31e95b8e5e2218015cc62515ade6` |

The exact error literal has one site in the inspected qualified bounds source:
`pliron_ranked_bounds.rs:174-176`, inside
`preflight_ranked_bounds_resource_upper_bound_v1`. The pipeline invokes this
preflight at `pliron_pipeline.rs:858`, before the memory-bounds execution stage.
It is a static worst-case admission estimate, not a report of an exhausted
runtime work counter or measured iterations.

The census counts both less-than terminator families at
`pliron_ir_identity/capture_v1.rs:314` and
`pliron_pipeline_protocol.rs:596`. The preflight at
`pliron_ranked_bounds.rs:135-172` computes:

```text
F = min(blocks, operations, memory_bounds_guard_candidates, 1024)
W = ceil(F / 64)
intersection_work = (blocks + raw_successors) * (W + 1) * (F + 1)
```

For this rendered graph, `F=552`, `W=9`, and `blocks+raw_successors=1689`.
Therefore the intersection term alone is:

```text
1689 * 10 * 553 = 9,340,170 > 8,388,608
```

The unchanged hard work limit is `65536 * 128 = 8,388,608` at line 66.
Adding only the known `2*operations + 5*(blocks+successors)` terms gives a
lower bound of 9,353,095 before the nonnegative operand/operation-item terms.
This report does not guess the complete charged upper bound, attribute the
failure to actual solver work, or claim that later resource gates would pass.

There are 552 distinct literal-index/extent pairs, not repeated copies of one
fact. The current canonicalizer at `access_proofs_v1.rs:308` makes the literal
LHS constant and retains each unknown RHS value. Thus merely deduplicating
identical facts does not reduce this case's 552 fact candidates.

## Minimal Sound Next Direction

The actual graph is acyclic, but current execution seeds its FIFO in physical
block order (`execution_v1.rs:250-291`). That order is not topological. The
algorithm may revisit a block when predecessor facts change, so deleting the
`F+1` allowance without changing execution would be unjustified.

A candidate optimization is an authenticated, bounded DAG schedule used by
both preflight and execution. On certified DAGs it would intersect a block's
predecessor facts only after all predecessors are final, once per block.
The same intersection function, edge guard facts, entry empty set, access
proofs, and findings semantics must remain. Cyclic inputs must retain the
original bound and FIFO execution. A one-pass bound must never be paired with
an unannounced runtime fallback to FIFO.

Required proof and resource obligations:

- Bind the schedule to the exact immutable function or authenticated identity
  snapshot; validate targets, permutation, and strict predecessor precedence.
- Account for construction, validation, all scanned raw edges, queue/order
  initialization, and fallible scratch before work/allocation. Retain all
  existing hard limits, including work, storage, edges, facts, and findings.
- Include unreachable components in the cycle decision or explicitly retain
  the old behavior; preserve existing unreachable-block findings and entry
  handling. Preserve raw predecessor multiplicity and same-target branches.
- Keep forwarded block-argument transport semantics intact. A conservative
  fallback for those functions is acceptable if its old bound is selected.
- Test DAG/diamond/order permutations, joins, duplicate edges, entry backedges,
  reachable and unreachable cycles, malformed targets, argument transport,
  exact/overflow/denied work-storage boundaries, and unchanged access refusal.
  Differentially compare old and new fact sets/findings on bounded graphs.
- Include a constructed 552-guard fixture and the real production entry point.
  CPU qualification and actual guarded compilation remain required; no
  speedup, artifact, alias, numerical, or GPU result is predicted here.

The rendered ranked views currently use dynamic extent values `%565` and `%741`
defined by `kernel.index_unknown`, not literal 548/4 extents. Consequently this
report does not propose deleting the 552 guard predicates based solely on the
source-level length checks. That would require an additional authenticated
cross-representation proof. The earlier conditional 552-word ABI alternative
is on hold: the actual failure is memory-bounds preflight, not alias analysis.

A diagnostic-only fallback, if desired, can report the already-computed census,
fact/word/wave counts, intersection term and checked total at this unique
refusal. Those integer fields would clarify future cases without changing
admission. It is not necessary to establish the current failing term.
