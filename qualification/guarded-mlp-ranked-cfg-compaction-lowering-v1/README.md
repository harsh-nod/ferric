# CFG Compaction Guarded Lowering

This checkpoint uses the [compaction-qualified compiler](../guarded-mlp-ranked-cfg-compaction-v1/README.md)
and [fresh loader audit](../guarded-mlp-ranked-cfg-compaction-tool-audit-v1/README.md)
for the unchanged guarded MLP candidate. The optimized-inline gfx950 profile,
vendor inputs, numerical operations and resource limits are unchanged.

The [fixed lowering fixtures](controller-tests-v1/attempt-v1) pass all 19 named
tests on `mi350` in 5.228275 seconds: 15 admission cases and four unchanged
normalization cases. Source bytes remain unchanged, all child statuses are
passing, and process cleanup and postchecks are clean.

## Actual Guarded Attempt

The [retained compile](attempt-v1) fails at a different, later check:

```text
error[FE2O3-PRESERVE-028]: structural identity is unavailable
error[FE2O3-PRESERVE-002]: basic blocks count 1025 at function exceeds identity limit 1024
```

Here 1,025 is the first refused count, not the total graph size. The pinned
stderr includes the complete rendered ranked PLIRON for
`ferric_qwen3_mlp_state_guard_v1`, with 1,675 distinct contiguous block labels
`bb0` through `bb1674`. Independent inventories agree on these counts:

| Observed Graph Property | Count |
| --- | ---: |
| Blocks before compaction, from the prior diagnostic | 2,778 |
| Blocks after compaction, from the rendered graph | 1,675 |
| Raw successor edges after compaction | 2,230 |
| Atomic reads / writes | 548 / 4 |
| Empty unconditional-branch blocks | 561 |

Compaction removes 1,103 blocks, a 39.7% reduction in graph size, not a GPU
speedup. The graph now fits the 2,048 projected-block limit, but not the
separate 1,024-block structural-identity limit. Its raw edge inventory also
exceeds the unchanged 2,048-edge ceiling; this run does not reach an edge
admission verdict. Removing the 561 empty branch blocks alone would still
leave 1,114 blocks, so that alone cannot resolve the identity refusal.

The compiler exits naturally with status 1 after 130.542604 seconds; total
controller time is 134.248464 seconds. It is reaped and leaves no process
group, with no timeout, forced cleanup, exception or postcheck error. All
213 input pins and the candidate source map remain unchanged. No target IR,
HSACO, alias verdict, GPU or model result is produced. The rendered graph is
a diagnostic in stderr, not an emitted executable artifact.

Receipt SHA-256:
`1bf26b28162414dfb4afbf6dc5ba5a70a60d1c1fe1fd55624c251733840c560b`.
The 697,972-byte archive SHA-256 is
`152fde144882ad553107abb9a1fa4a9deadf73484bb12eb04e263a2387a47dd8`.
It retains 13 members, 12 pinned bodies and 10 raw files totaling 4,395,529
body bytes. Stderr SHA-256:
`9a309b8f23c66ad29e98a464bcb3ed8480766e2c7fd807415616dc99d6121a05`.
Current-failure identification fields in the receipt remain predeclared false;
the new attribution and inventory come from these raw diagnostic bytes, not
from changing historical evidence fields.

Further control-flow simplification is being investigated with unchanged
proof obligations. GPU execution, independent model numerics, sustained
performance, all issue #42 milestones and the 700 tokens/s target remain open.
