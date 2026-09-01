# Ferric Qwen3 logits K7 attributed source V1

This standalone Ferric-owned package defines the three attributed device roots
required by the M1 K7 boundary:

- `ferric_qwen3_lowest_id_argmax_bf16_v1` selects the lowest token ID attaining
  the maximum finite BF16 logit for each flattened target or draft row.
- `ferric_qwen3_compact_completion_v1` publishes the canonical 120-byte target
  completion record, including request authority, completion epoch, 32-byte
  plan identity, accepted-prefix count, emitted count, zero reservation, and
  up to 17 token IDs.
- `ferric_qwen3_speculative_token_assembly_v1` converts anchor TokenIds plus
  iteration-major draft choices into sequence-major target verification input.

All roots use exact Wave64 workgroups. Argmax launches one group per logits row;
compact completion and token assembly launch one group per sequence. Immutable
device reads use the bounded `fe2o3_device::memory::volatile_load` provider.
Every output uses compiler-known `WriteOnlyDisjointSlice` ownership with a
64-lane row-striped map. No ordinary mutable slice or readable output surface is
present.

The compact output is the canonical `[S,120]` U8 record expected by the Ferric
host contract. Bytes 0 through 7 contain slot and generation, bytes 8 through
15 contain the little-endian epoch, bytes 16 through 47 contain the plan
identity, bytes 48 and 49 contain accepted and emitted counts, bytes 50 and 51
are reserved zero, and bytes 52 through 119 contain the fixed token array. The
`RowStriped2D<Index1D,64,2>` map gives every lane byte `lane` and, for lanes
zero through 55, byte `64 + lane`; this covers all 120 bytes exactly once. An
inactive sequence emits 120 zero bytes without reading request authority.

Direct target profiles pass a zero-length draft slice. Its data pointer must
remain non-null even though the kernel never reads it. Ferric deliberately does
not substitute a dummy buffer: production KFD packing depends on the pending
generic fe2o3 non-null-empty-slice support.

The package pins exact reviewed fe2o3 revision
`b5374c6e6a4c1215ad481cefcd294334dcb1cbeb`. This package grants no artifact,
source-to-machine, load, launch, completion, hardware, performance, or M1 gate
authority. Final integration still requires extracting all three roots,
reconciling generated ABI and launch descriptors with `ferric-qwen-kernels`,
and qualifying them on MI300X.
