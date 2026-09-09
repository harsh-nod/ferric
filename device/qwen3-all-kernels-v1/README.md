# Ferric Qwen3 All Kernels Device V1

This standalone package places all 12 attributed Ferric M1 Qwen3 kernel roots
in one selected Rust compilation unit. This package owns the seven canonical
source modules. The historical family crates are explicitly non-authoritative
test wrappers around those same files, so their GEMM, RMSNorm, RoPE/KV,
prefill, paged-decode, SwiGLU, and logits checks cannot drift from the selected
aggregate source closure.

The default `gfx942` feature retains the MI300X target. Select MI350X explicitly
with `--no-default-features --features gfx950`. Missing or simultaneous target
features fail compilation. Both targets compile the same twelve entrypoints
and seven source modules; there is no copied kernel implementation. Device
builds additionally reject a rustc processor or Wave64/xnack feature set that
does not match the selected source contract. RMSNorm target metadata follows
the aggregate selection; its historical host-test wrapper remains gfx942.

Target selection is engineering-only. It does not make an existing gfx942
artifact compatible with gfx950: each target needs its own compiler emission,
inspection, and hardware validation. It adds no tensor parallelism, collective
communication, numerical proof, or protected publication authority.

The target-consistency check is `Contracted`, not a new Verus theorem. It
assumes Cargo reports the actual target architecture and encoded rustc flags;
compiler/runtime custody must still bind those inputs to the emitted object.
Host tests exercise both target selections, the exact twelve-root roster,
metadata agreement, and conflicting or missing target settings. Source tests
also reject target-dependent kernel bodies and explicit negative mutations.

The package is an integration boundary only. Source presence, a compiler
binding check, or a generated marker roster grants no protected-verifier,
artifact, load, launch, hardware, numerical, performance, Qwen, or M1
authority. Production use still requires one current receipt-bound Worker V3
publication, protected authentication, retained runtime custody, and the full
M1 qualification evidence.
