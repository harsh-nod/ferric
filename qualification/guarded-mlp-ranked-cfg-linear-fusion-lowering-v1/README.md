# Chain Fusion Guarded Lowering

Status: the actual MI350 guarded compile fails closed at the memory-bounds
preflight work estimate. It produces no HSACO. This is not GPU execution,
model numerical acceptance, or a performance result. All issue #42 milestones
and the 700 tokens/s target remain open.

The [failed receipt](attempt-v3/evidence/failed.json) follows the
[fully qualified compiler](../guarded-mlp-ranked-cfg-linear-fusion-v1/README.md)
and [14 passing binary-loader checks](../guarded-mlp-ranked-cfg-linear-fusion-tool-audit-v1/README.md).
The child exits naturally with code 1 after 127.021051 seconds; the complete
attempt takes 130.662030 seconds. It is reaped and leaves no process group.
There is no timeout, forced cleanup, source change or postcheck error. All 214
before/after input pins match. The artifact remains `null`.

The [raw diagnostic](attempt-v3/evidence/compile.stderr) is:

```text
production analysis resource limit exceeded [memory-bounds]: memory-bounds work hard limit
```

## Observed Graph

Independent analysis of the rendered `ferric_qwen3_mlp_state_guard_v1`
compares it with the retained pre-fusion compile:

| Property | Prior Compaction | Actual Chain Fusion |
| --- | ---: | ---: |
| Blocks | 1675 | 567 |
| Raw successor occurrences | 2230 | 1122 |
| Operations including terminators | 3348 | 2240 |
| Acquire/System atomic reads | 548 | 548 |
| Relaxed/System atomic writes | 3 | 3 |
| Release/System atomic writes | 1 | 1 |
| Distinct bounds guard candidates | 552 | 552 |

All 567 blocks are reachable and the rendered graph is acyclic. Independently
replaying all 1,108 eligible chain merges reproduces each resulting block's
ordered operations, predicates and targets exactly. This checks rendered IR,
not metadata that the renderer omits. The observed counts are below the
unchanged 1,024-block and 2,048-edge ceilings, but do not by themselves certify
later compiler stages, alias checks or successful artifact generation.

## New Refusal

The [source-authenticated investigation](analysis-v1/README.md) attributes the
unique error literal to `preflight_ranked_bounds_resource_upper_bound_v1`.
This is a static upper-bound refusal, not measured runtime-counter exhaustion.
For `B=567` blocks, `E=1122` successor occurrences and `F=552` candidate facts,
one term alone exceeds the unchanged work cap:

```text
(B + E) * (ceil(F / 64) + 1) * (F + 1)
= 1689 * 10 * 553
= 9,340,170 > 8,388,608
```

Other charged terms are nonnegative. The proposed next step is a bounded,
authenticated topological schedule with matching one-pass execution and a
sound admission bound for DAGs. Merely dropping the `F+1` factor from the
existing iterative solver's estimate would be unjustified. No safety limit,
bounds predicate or atomic ordering is relaxed.

This attribution and graph census are subsequent independent investigation.
The raw receipt's predeclared `actual_failure_caller_identified` and
`actual_failure_block_count_observed` fields remain `false`; the receipt is
retained unchanged, not retroactively promoted.

## Retention

The capsule has 13 members, 12 pinned bodies and 10 raw files, totaling
4,361,649 uncompressed body bytes. Its compressed size is 692,514 bytes,
SHA-256 `5da019dbd42c032a29b5ab0288f6e713225a941c3c53eeb46f53179aeee6f7cf`.
The receipt is 17,737 bytes, SHA-256
`8d4cc1a0cfe0c9104282c3d20349638a616bd98e7e28f92598dcaf0110057ca2`.
The stderr is 123,245 bytes, SHA-256
`c60dc2edc62bfe11ba7a8f79ce1465322d2c887aeb3fa5661dae8002a33c5095`.
All retained bodies and input/source joins were independently checked.
