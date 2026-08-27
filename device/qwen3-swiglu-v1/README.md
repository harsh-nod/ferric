# Ferric Qwen3 SwiGLU Device V1

This standalone crate is the first Ferric Qwen kernel source migrated from the
retired caller-created Worker V2 handoff to fe2o3's rustc-produced Worker V3
pipeline. It preserves the existing `qwen3_swiglu_bf16_f32_v1` symbol, three
pointer-plus-length slice ABI, exact admitted extents, 256-workitem workgroup,
and eight contiguous output elements per workitem.

The crate is intentionally outside Ferric's stable host workspace and pins the
latest reviewed fe2o3 source used for this migration, including the generic
external-device dependency disambiguation at `2347052f67cf`. Passing its
host-side source and reference tests establishes only the attributed Rust
source contract. It does not establish production compilation, HSACO
inspection, dispatch authority, numerical qualification, or M1 evidence.

The current upstream prerequisites are production BF16 conversion/carrier
lowering, the compiler-owned gfx942 OCML exponential provider envelope, and the
generic Worker V3 verifier/runtime authorization join. After those land, build
this crate only through `cargo fe2o3 authority release build` with an admitted
`FE2O3_PRODUCTION_BUILD_CONFIG_V1`.
