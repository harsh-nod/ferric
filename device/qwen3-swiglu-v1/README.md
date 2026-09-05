# Ferric Qwen3 SwiGLU Device V1

This non-authoritative compatibility crate exposes the first Ferric Qwen
kernel migrated from the retired caller-created Worker V2 handoff to fe2o3's
rustc-produced Worker V3 pipeline. Its canonical source is owned by
`qwen3-all-kernels-v1`; this crate remains for focused tests and is not a
production selected-package or publication root. It preserves the existing `qwen3_swiglu_bf16_f32_v1` symbol, three
pointer-plus-length slice ABI, exact admitted extents, 256-workitem workgroup,
and eight contiguous output elements per workitem.

The crate is intentionally outside Ferric's stable host workspace and pins the
exact reviewed external fe2o3 revision
`d8fa0835c64d6574c8589ac3e69e3c34b0350758`. That revision provides
the compiler-issued write-only device capability and its generated KFD output
binding. The kernel's output has no readable element, reference, or pointer
surface; it spells its eight owned components as eight constant checked
`write_block` calls so M1 does not depend on a loop-carried race/progress proof.
Its repeated pure element math is expanded locally before MIR construction,
leaving no ordinary helper-function call for the production semantic projector.
Passing its host-side source and reference tests establishes only the
attributed Rust source contract.

Before this write-only migration, the exact Ferric revision
`7e1c36aa35d743478772ce4bff14c4f4bbff85c0`
was compiled on MI300X through `cargo fe2o3 authority release build` using
fe2o3 compiler revision `4cd2af64645e57bdb3902ac2618baefeb3cb8722`.
The protected build admitted KIR identity
`fe2o3::semantic::54361a526f73befabecd65a3a7dc0338ef8653d15209d3b47765356236f34dcc`,
completed reproducible Worker V3 linking, and published a 14,192-byte COV6
HSACO with SHA-256
`0a27ada84a6382331af6a16d4ed0be6fcf1f85333ca5087b908a64618062702a`.
Read-only fe2o3 inspection reports the exact `gfx942:xnack-` target, one kernel,
304-byte kernarg segment, 256-workitem workgroup, 84 SGPRs, 11 VGPRs, zero LDS,
and zero private-segment bytes. Independent ELF inspection finds the protected
kernel, its `.kd` descriptor, and a defined weak `__ocml_exp_f32` symbol.

The same transaction durably published the Worker V3 readiness claim,
canonical envelope, and receipt. Those records remain inert: no production
verifier has promoted them into KFD dispatch authority, and no load, dispatch,
hardware result, numerical comparison, whole-Qwen execution, performance, or
M1 completion is claimed.
