# Ferric Qwen3 All Kernels Device V1

This standalone package places all 12 attributed Ferric M1 Qwen3 kernel roots
in one selected Rust compilation unit. This package owns the seven canonical
source modules. The historical family crates are explicitly non-authoritative
test wrappers around those same files, so their GEMM, RMSNorm, RoPE/KV,
prefill, paged-decode, SwiGLU, and logits checks cannot drift from the selected
aggregate source closure.

The package is an integration boundary only. Source presence, a compiler
binding check, or a generated marker roster grants no protected-verifier,
artifact, load, launch, hardware, numerical, performance, Qwen, or M1
authority. Production use still requires one current receipt-bound Worker V3
publication, protected authentication, retained runtime custody, and the full
M1 qualification evidence.
