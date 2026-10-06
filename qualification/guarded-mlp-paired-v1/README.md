# Paired Guarded MLP Coordinator

Status: **private coordinator compiles and CPU qualification passes on MI350.**
The follow-up [nonzero native diagnostic](../guarded-mlp-paired-native-v1/README.md)
now also passes the actual paired GPU chain. Worker integration, reusable state
banks, full-model numerics and performance qualification remain open. This does not establish
the 700 tokens/s target or close any issue #42 milestone.

## Implementation

The coordinator builds this five-packet sequence on each of two ranks:

```text
R1 residual -> tiled MLP -> state validator -> both-validator barrier -> R2
```

All eight dispatches and both signal/kernarg arenas are prepared first. Both
state owners are consumed before ring reservation because owner submission
requires idle queues. Both complete batches are published before the host
begins waiting for completion. The barrier uses the two actual validator
completion handles, not the earlier MLP completions.

R1 consumes the attention output partials and writes the MLP input. R2 consumes
the two distinct MLP Down partials and the post-R1 residual, writing separate
final outputs. Full allocation backings are checked for incompatible aliases.
The 2208-byte combined-state allocation remains genuine: the MLP receives its
2192-byte prefix, the validator the complete allocation, and R2 only its
16-byte guard suffix. No old state token is forged.

Completion requires all ten signals, current queue counters and no queue
exception. Logical work retirement does not fabricate hardware read credit:
only observed GPU read counters may free ring capacity. Both terminal guards
are checked under full fences before either owner becomes Completed. Each
owner completion checks its snapshot again.

Any error poisons both owners, both contexts and the staged coordinator.
Publication is one-shot, with all invalid packet bodies written before release
headers. Signal storage remains retained until queue-first group close.
There is no retry, rollback, arena reuse or paired rearm in this version.

The implementation is a private, unsafe engineering entry. Caller obligations
include reviewed R1/MLP images and model-role bindings, completed attention
producers and established payload coherence. The guarded image has an exact
digest/ABI check. Generic SharedAtomic admission and production authority are
unchanged.

## Actual MI350 Qualification

The [receipt](cpu-attempt-v1/evidence/complete.json) records ten naturally
successful phases in 60.370605 seconds, with all children reaped and process
groups absent. No timeout, forced cleanup or postcheck error occurred.
The build uses a fresh bounded target directory on
`ssh mi350`, offline locked dependencies, two CPU cores and no GPU execution.

| Scope | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Full KFD suite, five targets | 1026 | 0 | 4 |
| Paired coordinator | 14 | 0 | 0 |
| Existing combined owner | 11 | 0 | 1 |
| Existing atomic memory | 7 | 0 | 0 |

All 1016 prior test names and outcomes are preserved. Fourteen passing tests
are added, yielding 1030 inventory entries. The source map contains 793 source
files and two harness files. Only two existing Rust files change: one module
registration and two helper visibility modifiers. Four new Rust files contain
the coordinator, profiles, signal arenas and tests.

The [retention manifest](cpu-attempt-v1/retention-manifest.json) binds 82 bodies
in an 83-member capsule, including the exact formatted source postimages,
54 raw records, harnesses, formatting records and source lineage. No ELF body
is exported. The 903,203-byte archive has SHA-256
`cc4c2557708c55d84bf0e925afe1967dc3468ae80c74e214d9c785030b3016d1`.
The 1,144,022-byte receipt has SHA-256
`880e9391b778189921ecc07578eaac979819fd9dcc3d7a18ecb5df2e670434c6`.
The supervised host interval is not a GPU latency or throughput measurement.

The [focused output](cpu-attempt-v1/evidence/paired-tests.stdout) covers:

- Actual coordinator ordering, before/after-effect failure injection at every
  fallible invocation, and aggregate deadline expiry.
- Validation of both guards before either completion, including peer failure.
- All ten completion signals, asymmetric early rank-zero producer completion,
  actual read-counter regression and ring-capacity limits.
- Actual AQL packet bytes, ten distinct completion addresses, eight distinct
  kernarg addresses and both-validator barrier handles.
- Exhaustive two-rank sequential packet interleavings, with a deliberately
  wrong MLP-only dependency that demonstrates an unsafe counterexample.
- Exact guarded metadata and argument bytes, genuine allocation extents,
  padded backing overlap, cross-stage writer aliases and output isolation.
- The real native entry's wrong-world refusal before any GPU operation.

CPU graph enumeration is not a GPU memory-coherence proof. The fake backend
exercises the actual coordinator control flow, not native publication. The
earlier [serial native component test](../guarded-mlp-stable-r2-native-v1/README.md)
remains separate evidence; its numerical result does not qualify this new
paired execution path.

## Next Gates

The follow-up native diagnostic exercises the actual paired entry on both
MI350 GPUs using the checked images, independent output checks and healthy
close. Next integrate a distinct Ferric completion path that skips the old
second residual call, qualify quiescent reuse across state banks, and run the
2048-prompt/256-generated-token BF16 model workload. Only after correctness
qualification should equal-work vLLM comparisons and overlap/ablation graphs
be treated as performance evidence.
