# RoPE BF16 Materialization Reference

This diagnostic ran on ASROCK through `ssh mi350-2`, using the installed
PyTorch 2.12.1/Transformers 4.51.0 implementation on CPU. It loads no model and
launches no GPU work. Fourteen arithmetic and refusal tests passed separately.

The inputs are genuine captured position-zero Q/K tensors and a small synthetic
boundary corpus. Reusing these inputs at other rotary positions isolates the
operator; it does not produce genuine later-position model activations.

## Results

An independent FP32/BF16 round-to-nearest-even calculation matched all
**38,528 words across 28 comparisons**, using the framework's BF16 rotary table.
The calculation rounds each product to BF16 before the final addition, matching
the tensor boundaries in `apply_rotary_pos_emb`. Negation occurs before the
sine product, including signed-zero cases.

The following counts use the same captured Q/K inputs and framework BF16 table.
There are 4,096 Q words and 1,024 K words at each position.

| Position | Different Q words, FP32 products | Different K words, FP32 products | Different words, BF16 products |
| --- | ---: | ---: | ---: |
| 0 | 0 | 0 | 0 |
| 1 | 315 | 71 | 0 |
| 2 | 385 | 98 | 0 |
| 3 | 384 | 95 | 0 |
| 2047 | 1116 | 249 | 0 |
| 2048 | 1094 | 247 | 0 |
| 2303 | 1097 | 280 | 0 |

A separate Python/libm FP64 table-construction model agrees after BF16 rounding
at the first six positions, but differs at position 2303: duplicated sine entries
2 and 66 are `0x3e03` rather than `0x3e02`. Even with product materialization,
this leaves 19 Q and 2 K differences on the captured-input corpus. Actual Rust
table bytes and GPU trigonometry were not replayed. Table-generation equivalence
therefore remains unproven.

## Evidence And Limits

- [Actual comparison](complete.json), [compact table](table.json), and
  [publication ledger](result.json). All 145 output buffers were rehashed before
  publication; numerical payloads remain retained outside Git.
- [Fourteen-test observation](pure-root-observation.json) is the primary agent's
  record of actual SSH output, not a remote process-supervisor receipt.
- [Source and reproduction commands](source/README.md); the installed framework
  source, original captured tensors and independent integer-RNE helper are pinned.

This validates the conditional reference calculation, not a new Rust GPU image.
CPU denormal behavior, GPU table generation, full-model numerical acceptance and
the sustained 2,048/256 workload remain separate checks. There is no performance
or production-admission claim.
