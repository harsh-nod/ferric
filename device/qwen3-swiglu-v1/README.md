# Ferric Qwen3 SwiGLU Device V1

This standalone crate is the first Ferric Qwen kernel source migrated from the
retired caller-created Worker V2 handoff to fe2o3's rustc-produced Worker V3
pipeline. It preserves the existing `qwen3_swiglu_bf16_f32_v1` symbol, three
pointer-plus-length slice ABI, exact admitted extents, 256-workitem workgroup,
and eight contiguous output elements per workitem.

The crate is intentionally outside Ferric's stable host workspace and pins the
exact combined fe2o3 device-provider revision `2c7668d23326`. That revision
includes external-device dependency disambiguation, authenticated BF16
conversion terminals that remain calls in optimized external MIR, and strict
gfx942 OCML exp admission with the reviewed ROCm 7.2.4 provider hashes. It also
keeps the blocked-index producer and output accessor visible as authenticated
semantic terminals in release builds. The kernel spells its eight owned output
components as eight constant blocked-access calls so M1 does not depend on a
new loop-carried race/progress proof in fe2o3. Passing its host-side source and
reference tests establishes only the attributed Rust source contract. It does
not establish production compilation, HSACO inspection, dispatch authority,
numerical qualification, or M1 evidence.

The pinned fe2o3 stack contains the reviewed BF16, OCML, provenance, and blocked
terminal support. Production compiler integration remains open; no successful
replacement artifact exists yet. Every production attempt must use `cargo
fe2o3 authority release build` with an admitted
`FE2O3_PRODUCTION_BUILD_CONFIG_V1`, followed by strict Worker V3 artifact
inspection and the verifier/runtime authorization join.
