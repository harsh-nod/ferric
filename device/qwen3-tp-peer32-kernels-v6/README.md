# Qwen3 TP32-Row Peer Kernels V6

This additive gfx950 engineering image contains exactly two roots. It does not
replace the frozen rows1..16 v4 peer image or establish protected M1 authority.

| Root | Explicit ABI | Bounds |
| --- | --- | --- |
| `ferric_qwen3_tp_batch32_peer_ordered_residual_bf16_v6` | Eight immutable FP32 slices, BF16 residual, write-only BF16 output, rows/world u32: 168 bytes | TP2/8, rows1..32, width4096 |
| `ferric_qwen3_tp_batch32_peer_copy_bf16_v6` | Immutable BF16 source, write-only BF16 output, rows u32: 36 bytes | rows1..32, width4096 |

Both roots require workgroup64 and exactly rows*64 workgroups, at most2048.
Every active slice has exactly rows*4096 elements. TP2 p2..p7 slices are empty
and never read. Each output invocation has a unique checked index. Ordered
reduction starts at FP32 zero, adds each rank in ascending order, checks every
input/intermediate, adds the residual once and rounds to BF16 once. Nonfinite
inputs, intermediate overflow and nonfinite narrowed output trap. Copy preserves
every BF16 bit pattern, including nonfinite payloads.

Compiler/SDK sources are pinned to public fe2o3 `3e74a9324`. Managed emission
must bind this exact two-root roster via `compiler_expectation_roster_v6()`.
Host macro tests cover unchanged arithmetic and rows17/31/32; the shared extent
macro rejects zero,33 and u32::MAX before multiplication. Source contracts check
the ABI, exact launch geometry and single-writer ownership for every row count.

`tools/probe.py --self-test` validates 19 bounded arithmetic fixtures without
opening KFD. Explicit `--run` uses a digest-bound disposable single-device worker
for rows1/16/17/31/32, bit-copy coverage and adversarial rounding/order cases.
It is not a peer-mapping or model/performance qualification. A separate native
multi-device owner probe and model comparison remain required before promoting
the new image beyond its engineering scope.
