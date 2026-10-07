# Qwen3-8B TP2 Decode: The 700 Tokens/s Planning Budget

**Source-derived planning, not measured performance.** This is a BF16, single-request incremental-decode traffic model, not a TTFT prediction, GPU timing result, numerical-acceptance gate or claim that current kernels sustain 700 tokens/s. It assumes two devices each capable of **8 TB/s peak theoretical HBM bandwidth**. AMD lists that peak for MI350X/MI355X; the retained native topology does not authenticate a marketed SKU or sustained bandwidth. [AMD architecture reference](https://rocmdocs.amd.com/en/latest/reference/gpu-arch/mi350.html#memory-hierarchy), [actual topology](../../guarded-mlp-readiness40-v1/gpu-v1/readiness/initial-topology.json).

## What Must Move

The authenticated [checkpoint index](model.safetensors.index.json) contains 399 tensors and **16,381,470,720 bytes**. The executed [reference loader](../../guarded-mlp-readiness40-v1/reference-v1/source/run.py#L195) checks 36 layers, width 4,096, MLP width 12,288, 32 query heads, eight KV heads, head dimension 128, vocabulary 151,936, untied embedding/head and BF16 GPU parameters. This geometry reconciles the index total exactly: 13,891,534,848 dense-block bytes + 608,256 block-norm bytes + 8,192 final-norm bytes + two 1,244,659,712-byte embedding/head tables.

One decode step reads only one **8,192-byte embedding row**, but needs the **entire untied LM head**. Current [layer bindings](../../../adapters/tp-peer-finite-engineering-worker-v1/src/resident_layer.rs#L26) shard dense weights across TP2 and replicate norm weights. Current [tail bindings](../../../adapters/tp-peer-finite-engineering-worker-v1/src/tail_bindings.rs#L46) put the full LM head on rank 0, not half on each rank. Its transposed representation is counted once per step, not as two head reads.

| One-Pass Weights and Lookup | Rank 0 Bytes | Rank 1 Bytes |
| --- | ---: | ---: |
| 36 dense-block shards | 6,945,767,424 | 6,945,767,424 |
| Replicated block norms | 608,256 | 608,256 |
| Full LM head | 1,244,659,712 | 0 |
| Final norm and one embedding row | 16,384 | 0 |
| **Subtotal** | **8,191,051,776** | **6,946,375,680** |

K and V caches are **BF16**, not FP32: the [buffer contract](../../../adapters/tp-peer-finite-engineering-worker-v1/src/finite_composition_wire.rs#L458) uses two-byte elements and 512 elements/token/cache/rank; [device reads](../../../device/qwen3-tp-wave-rmsnorm-kernels-v15/src/attention_numerics_v4.rs#L33) interpret them as BF16. Across 36 layers and both ranks, each token adds `2 * 36 * 8 * 128 * 2 = 147,456` cache bytes. The allocated 2,304-token capacity is 339,738,624 bytes (324 MiB), not traffic that must be read at every position.

For 2,048 prompt tokens and 256 outputs, the prompt's last forward produces output 1. Outputs 2 through 256 require **255 incremental forwards**, with old-cache lengths 2,048..2,302 and visible lengths `S=2,049..2,303`; mean `S=2,176`. The optimistic model reads old KV once and writes new KV once: `147,456*S` bytes/pair. It assumes reuse across the four query heads sharing a KV head. Writing then rereading the new KV adds 147,456 bytes/pair/step.

## Conditional Streaming Roofline

These calculations assume one HBM transfer per participating weight and the KV traffic above, without retained-cache credit. Decimal TB/s is used. For per-rank traffic `B0,B1`, the aggregate-only floor is `(B0+B1)/16e12`, but the current placement requires at least `max(B0/8e12,B1/8e12)`. [Exact byte table](roofline.csv).

| Visible Context | Pair Bytes/Step | Rank 0 Bytes/Step | Aggregate Floor | Rank Bottleneck Floor | Traffic-Only Ceiling |
| --- | ---: | ---: | ---: | ---: | ---: |
| 2,049 | 15,439,564,800 | 8,342,120,448 | 0.964973 ms | 1.042765 ms | 958.99 tokens/s |
| 2,176 (mean) | 15,458,291,712 | 8,351,483,904 | 0.966143 ms | 1.043935 ms | 957.91 tokens/s |
| 2,303 | 15,477,018,624 | 8,360,847,360 | 0.967314 ms | 1.045106 ms | 956.84 tokens/s |

The rank-0 head alone contributes 0.155582 ms at the conditional peak. Aggregate bandwidth hides this imbalance. These are **conditional streaming floors, not unconditional physical lower bounds**: on-chip cache reuse can reduce HBM bytes, while repeated loads and imperfect GQA reuse can increase them. AMD lists a 256 MB memory-side cache per MI350X/MI355X; neither effective residency nor physical traffic was measured here. Do not subtract that capacity independently from multiple traffic classes. [AMD cache description](https://rocmdocs.amd.com/en/latest/reference/gpu-arch/mi350.html#memory-hierarchy).

## Interpreting the Target

At 700 tokens/s, each inter-token interval is **1.428571 ms**. The 255 intervals span **0.364286 seconds**, excluding TTFT. Dividing all 256 outputs by 700 instead gives 0.365714 seconds, a different counting convention.

| Mean-Context Requirement at 700 Tokens/s | Calculated Budget |
| --- | ---: |
| Pair traffic | 10.820804 TB/s (67.6300% of summed peak) |
| Rank 0 traffic | 5.846039 TB/s (73.0755% of its peak) |
| Rank 1 traffic | 4.974765 TB/s (62.1846% of its peak) |
| Remaining interval beyond ideal rank-0 traffic floor | 0.384636 ms |
| Traffic-only ceiling at 75% effective rank-0 peak | 718.44 tokens/s |
| Traffic-only ceiling at 70% effective rank-0 peak | 670.54 tokens/s |

That remaining interval must accommodate nonoverlapped compute, communications, dispatch, synchronization and host work. The model therefore does not rule out 700, but does not establish that it is attainable. Peer-link bandwidth is not added to HBM bandwidth. Reducing host checks alone does not prove this target.

Prefill has a different traffic/compute model; batched prompt processing can amortize weight loads, while serial one-token forwards cannot claim that benefit. [Readiness40](../../guarded-mlp-readiness40-v1/README.md) covers 40 prompt positions and zero generated tokens, not a measured 2,048/256 workload. Numerical acceptance remains a separate requirement. A performance claim needs the actual workload and output history, identified devices, measured sustained bandwidth/cache behavior, TTFT and all 255 inter-token intervals.

## Evidence Join

The index copy is verbatim, 32,878 bytes, SHA256 `f9fdbcb91c23971c13ec5d5f2573d2349e8f61f2f049371ec699281748fdb1bc`. Its size/hash joins both the [qualified model record](../../guarded-mlp-readiness40-v1/reference-v1/inputs/qualified-model-record.json) and the successful [reference receipt](../../guarded-mlp-readiness40-v1/reference-v1/output/complete.json) (222,668 bytes, SHA256 `0f7de41e19f710752cdb9228bec17d48f240fbc7fc852cc8d34421ff3f0efe9a`). Tensor names and loaded geometry were checked independently; the raw config body was not available locally and is not claimed as additional evidence. Integer counts and rational divisions were independently recomputed. No new model run or benchmark was performed for this table. Official hardware documentation checked 2026-10-06.
