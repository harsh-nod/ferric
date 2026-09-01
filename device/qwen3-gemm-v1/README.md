# Ferric Qwen3 GEMM Device V1

This non-authoritative compatibility package exposes three attributed K1
device roots whose canonical source is owned by `qwen3-all-kernels-v1`:
scalar-reference GEMM/GEMV, A4 reduction GEMM/GEMV, and token-embedding
lookup. It is retained for focused source and numerical tests and is not a
production selected-package or publication root. The matrix roots implement BF16 input
and output storage with ascending FP32 accumulation and an exact zero-or-one
residual epilogue. The embedding root copies the admitted vocabulary row
without arithmetic.

The typed matrix output witness and Ferric's host profiles use the same
flattened one-dimensional tile grid. The compiled source artifact, extracted
descriptor ABI, and generated runner still must be reconciled and bound before
this package is an executable M1 path. The A4 root applies four adjacent
reduction terms per loop iteration; hardware inspection and performance
evidence must establish any actual vector load or throughput claim.

The package pins reviewed `fe2o3-device` and generated-host source at exact
revision `2d275684d7a22f8f913114b51b1d1dd524d1ed9b`. That source pin and a
passing `cargo-fe2o3` host-test lane do not identify a production compiler
occurrence or artifact. Production extraction also requires the
compiler-authenticated semantic terminal for safe, bounds-checked
`fe2o3_device::memory::volatile_load`; the dependency API alone does not grant
that production lowering.

The source and host tests establish attributed ordinary-Rust roots, exhaustive
finite-profile admission against both helper and inline root predicates,
ownership-typed output access, generated KFD argument types and effects,
bounded shared-input read custody, exact source-level A4 load/product ordering,
and parity between an independent host numerical oracle and the declared
ascending-FP32/A4 policy. Those tests do not execute the GPU roots or establish
compiled ABI binding, host-plan integration, a Worker V3 run, artifact
identity, hardware numerical results, performance, or M1 closure.
