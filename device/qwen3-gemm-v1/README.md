# Ferric Qwen3 GEMM Device V1

This standalone package owns three attributed K1 device roots intended to
supply the Ferric M1 Qwen3 envelope: scalar-reference GEMM/GEMV, A4 reduction
GEMM/GEMV, and token-embedding lookup. The matrix roots implement BF16 input
and output storage with ascending FP32 accumulation and an exact zero-or-one
residual epilogue. The embedding root copies the admitted vocabulary row
without arithmetic.

The typed matrix output witness and Ferric's host profiles use the same
flattened one-dimensional tile grid. The compiled source artifact, extracted
descriptor ABI, and generated runner still must be reconciled and bound before
this package is an executable M1 path. The A4 root applies four adjacent
reduction terms per loop iteration; hardware inspection and performance
evidence must establish any actual vector load or throughput claim.

The package pins immutable reviewed `fe2o3-device` and generated-host source at
revision `d955209099c7`. A newer `cargo-fe2o3` compiler may admit and compile
that dependency closure, but the dependency pin is not a claim about the
compiler used to produce an artifact. Production extraction also requires the
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
