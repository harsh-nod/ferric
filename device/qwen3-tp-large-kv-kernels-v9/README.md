# TP1 Large Physical KV Profile V9

This additive gfx950 engineering image contains exactly two roots. It keeps
the frozen v5 profile and all existing arithmetic unchanged. It is not a new
default, protected proof, native/model result, or performance qualification.

- `ferric_qwen3_tp_batch32_large_kv_append_v9`: unchanged 112-byte explicit
  append ABI, Wave64, exactly one workgroup, grid-leader exclusive writes.
- `ferric_qwen3_tp_batch32_large_kv_paged_gqa_bf16_f32_v9`: unchanged 116-byte
  baseline attention ABI, Wave64, `rows * 32` workgroups, at most 1024.
- Both require TP1, rows 1..32, physical pages 1..16384. Logical positions
  remain below 8192 and per-row page-table stride remains at most 512.
- A full K or V cache has 268435456 BF16 elements (512 MiB); their largest
  element index is 268435455. All 36 layer pairs total 36 GiB. Weights,
  workspaces, queues, host payloads and availability headroom are additional.
- Append retains complete duplicate-slot validation before any write.
  Attention retains causal bounds, serial QK reduction, online softmax order,
  finite checks and BF16 rounding. No wave attention or RoPE root is added.

The host must explicitly bind the separate v9 image before allocation and
reject unsupported wave/peer/sequence/capture/replica combinations. An actual
pool size at or below 512 pages does not make v9 a legacy profile. The host
integration owns profile identity, logical/physical allocation separation and
page ownership; this crate must not silently weaken them.

The kernel bodies and every inline helper are checked against the existing
v5 source after only the declared symbol, TP1-admission and physical-bound
substitutions. Fresh typed emission and root-owned native/model gates remain
required. All builds, tests and emission run on mi300x; the integration lead
alone schedules GPU probes on mi350.
