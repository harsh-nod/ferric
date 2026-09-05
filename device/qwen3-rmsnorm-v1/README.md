# Ferric Qwen3 RMSNorm Device V1

This non-authoritative compatibility crate exposes Ferric's exact
`qwen3_rmsnorm_v1` kernel from canonical source owned by
`qwen3-all-kernels-v1`. It is retained for focused tests and is not a
production selected-package or publication root. It retains the five pointer-plus-length slice
records followed by `rows`, `width`, `epsilon`, and `behavior`; 96 explicit
kernarg bytes; a 64-workitem workgroup; one workgroup per row; the exact
132-profile target/draft catalog geometry; and the pure and residual-fused
numerical contracts from `crates/ferric-qwen-kernels`.

The output capabilities are compiler-issued write-only row stripes. Each wave
lane owns columns `lane + component * 64`. Lane zero forms the FP32 sum as the
same ascending serial left fold as Ferric's authoritative direct kernel, then
the convergent wave64 primitive distributes that sum before applying
`sqrt(mean + epsilon)`.
Pure mode requires empty residual and fused-output slices. Fused mode adds BF16
input and residual in FP32, stores the fused value narrowed to BF16, and uses
the full FP32 sum for normalization. The source also checks that the physical
grid has exactly `rows` workgroups before any collective or memory access.
Every observed BF16 value, FP32 intermediate, and round-to-nearest-even BF16
result must remain finite or the kernel traps before publishing that result.

The crate is intentionally outside Ferric's stable host workspace. Both
`fe2o3-device` and `fe2o3-host` are pinned to immutable revision
`d8fa0835c64d6574c8589ac3e69e3c34b0350758`. That closure supplies write-only
generated KFD arguments and accepts empty generated-slice constructors, but its
KFD packer does not produce the required nonnull pointer fixup for empty
slices. Consequently pure-mode KFD packing and dispatch remain unauthorized
until that generic transport requirement is implemented and this source is
compiled and integrated with a later reviewed host/runtime closure.

Passing this crate's host-side source, ABI, profile, adapter-construction, and
reference tests establishes only the reviewed source contract. No compiler
run, KIR or LLVM identity, HSACO, KFD packing, dispatch, hardware result,
performance result, whole-Qwen execution, or M1 completion is claimed.
