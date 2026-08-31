# Ferric Qwen3 RoPE and Paged-KV Device V1

This standalone package owns the two attributed K3 device roots intended to
supply Ferric's M1 Qwen3 envelope: split-half rotary embedding and P16 paged
key/value-cache writes. It retains the exact finite target/draft machine
profiles, Wave64 ABI, BF16/FP32 storage types, fixed deployment trigonometric
tables, global 16,384-page cache pool, and immutable source dependency closure.

Both roots flatten Ferric's retained two-dimensional host grids into an exact
scalar-dependent one-dimensional launch. RoPE assigns one wave to each active
row and uses compiler-authenticated row-striped write-only outputs. Paged-KV
cache addresses depend on untrusted page-table contents, for which d955 has no
parallel safe ownership witness. That root therefore uses the unique grid
leader to perform the admitted copies sequentially through `GridExclusive`
write-only capabilities. This is a correctness-first source boundary, not a
parallel schedule or performance claim.

The generated d955 write-only KFD path seeds each device allocation from the
destination and writes it back only after successful dispatch completion. That
preserves cache elements not touched by this source without granting device
read access, but it also stages both fixed 512 MiB cache buffers in each
direction. This package makes no device-resident cache or transfer-performance
claim; eliminating those transfers requires a later authenticated lifecycle.

The package pins immutable reviewed `fe2o3-device` and generated-host source at
revision `d955209099c7`. A newer `cargo-fe2o3` compiler may admit and compile
that dependency closure, but the dependency pin is not a claim about the
compiler used to produce an artifact.

Source and host tests establish the exact two-root roster, finite machine
profiles, scalar-dependent launch equations, ownership mappings, BF16 rotary
source order, paged-cache index arithmetic, and generated KFD argument effects.
They do not establish host-plan integration, page-table provenance or
generation, artifact identity, Worker V3 execution, numerical qualification,
hardware behavior, parallel KV performance, or M1 closure.
