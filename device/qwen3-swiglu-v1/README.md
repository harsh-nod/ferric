# Ferric Qwen3 SwiGLU Device V1

This standalone crate is the first Ferric Qwen kernel source migrated from the
retired caller-created Worker V2 handoff to fe2o3's rustc-produced Worker V3
pipeline. It preserves the existing `qwen3_swiglu_bf16_f32_v1` symbol, three
pointer-plus-length slice ABI, exact admitted extents, 256-workitem workgroup,
and eight contiguous output elements per workitem.

The crate is intentionally outside Ferric's stable host workspace and pins the
exact combined fe2o3 device-provider revision `06c74c64506f`. That revision
includes external-device dependency disambiguation, authenticated BF16
conversion terminals that remain calls in optimized external MIR, and strict
gfx942 OCML exp admission with the reviewed ROCm 7.2.4 provider hashes. It also
keeps the blocked-index producer and output accessor visible as authenticated
semantic terminals in release builds. The kernel spells its eight owned output
components as eight constant blocked-access calls so M1 does not depend on a
new loop-carried race/progress proof in fe2o3. Its repeated pure element math is
expanded locally before MIR construction, leaving no ordinary helper-function
call for the production semantic projector. Passing its host-side source and
reference tests establishes only the attributed Rust source contract.

The pinned fe2o3 stack contains the reviewed BF16, OCML, provenance, and blocked
terminal support. Ferric `57f6cfdf` completed a protected Worker V3 build with
the current fe2o3 compiler at `21e4c106`, producing the exact `gfx942:xnack-`
HSACO identified in M1 evidence. That artifact and its durable load envelope
remain inert until the production verifier/runtime authorization join closes.

`../../qualification/qwen3-swiglu-v1/hip_numeric.cpp` is a qualification-only
harness for the exact published HSACO. Loading through HIP does not exercise
or grant Ferric's production Worker V3 verifier, KFD load, dispatch, or Qwen
inference authority.
