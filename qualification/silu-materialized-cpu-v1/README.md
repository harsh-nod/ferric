# BF16 SiLU Materialization: CPU Qualification

The candidate passed 38 Rust tests on ASROCK, accessed through `mi350-2`, plus
12 separate controller-policy tests. All sixteen build/test phases completed
naturally with their processes reaped. Source, dependency, configuration and
tool postchecks passed. There were no ignored tests.

This is a CPU arithmetic checkpoint, not GPU numerical acceptance or a
performance result. The candidate is not installed into the default device
crate or selected by any production route.

## Arithmetic Change

The [candidate macro](fixture/src/mlp_silu_materialized_numerics_v1.rs) adds an
explicit BF16 round-to-nearest-even boundary after FP32 SiLU and before its
product with up:

```text
exponential = existing_exp_f32(-abs(gate))
sigmoid = (gate >= 0 ? 1 : exponential) / (1 + exponential)
silu = FP32(gate * sigmoid)
materialized = BF16_RNE(silu)
output = BF16_RNE(FP32(widen(materialized) * up))
```

The exponential, division and FP32 SiLU expression are unchanged. The candidate
also rejects a nonfinite materialized value. It retains all 96 components per
lane, sticky failure handling, the two-row Down implementation and existing
scheduling geometry. It does not allocate an intermediate SiLU buffer.

This follows the earlier [captured-product diagnostic](../silu-materialization-diagnostic-v1/README.md),
which reproduced the framework's products from captured BF16 SiLU and up.
It does not prove that the native and framework exponential implementations
produce identical SiLU values.

## Tests

| Suite | Passed | Ignored |
| --- | ---: | ---: |
| Existing tiled MLP arithmetic | 4 | 0 |
| Existing two-row Down arithmetic | 10 | 0 |
| Existing claimed MLP arithmetic | 8 | 0 |
| New SiLU materialization cases | 16 | 0 |

The [new tests](fixture/tests/mlp_silu_materialized_v1.rs) exercise the actual
macro with controlled exponential return bits and an independent BF16 rounding
implementation. Cases cover both rounding ties, positive and negative gates,
signed zero, subnormal materialization, overflow, missing inputs, write failures
and full lane coverage. A source-contract test checks that the other kernel
functions and attributes match the actual checked Down2 entry.

The test build used opt2 with debug assertions and overflow checks, offline
locked dependencies and two CPU cores at low priority. GPUs were hidden.
The twelve policy tests cover source isolation, test inventories and executable
selection; they are not twelve additional numerical cases.

## Reproduction and Evidence

[result.json](result.json) records the actual receipts and copied-file hashes.
[complete.json](complete.json) and [raw](raw) retain commands, individual test
outcomes and source snapshots. The four compiled test executables and two
dependency snapshots are retained outside Git with their identities in the
publication record. Publication does not rerun tests locally.

The full 34-file tested numerical fixture is retained in [fixture](fixture).
Its Cargo manifest intentionally preserves the qualified ASROCK provider path;
it is not a portable replacement for the production device crate. The bounded
[controller recipe](controller/README.md) documents the provider, toolchain and
fresh-output requirements for reproducing the run.

The original checked AST baseline is available in
[finite_mlp_tiles_v2.rs](fixture/src/finite_mlp_tiles_v2.rs). The source-contract
test needs `FE2O3_SILU_BASELINE_ENTRY` to identify those exact bytes. The live
device crate still uses the older Down helper, so dropping only the three new
files into that crate would be incomplete. This publication keeps the tested
dependency set together and leaves that crate unchanged.

Actual CPU completion SHA-256:
`e139729d8fa95715ef8017679ee21f1175ba302b681785789e3dc84137508a30`.
Actual policy-test completion SHA-256:
`9bd4d7334711ce841810f33b54257203c72f33ff7188c18b19afc848d05f6aa1`.

## Remaining Gates

Checked lowering, emitted LLVM/ISA inspection, a genuine GPU capture and an
independent framework comparison are separate gates. This CPU result does not
close model correctness, sustained 2,048/256 decoding or the 700 tokens/s target.
