# BF16 RoPE Materialization: CPU Qualification

The candidate passed 33 Rust tests on ASROCK through `mi350-2`. All eleven
build/test phases completed naturally with their processes reaped. Source,
dependency, configuration and provider snapshots remained unchanged. GPUs were
hidden throughout this CPU arithmetic qualification.

This is not a GPU result or model numerical acceptance. The tested macros are
retained separately from the default device crate and production routes.

## Arithmetic Change

The [candidate macros](fixture/src/prefix_rope_materialized_numerics_v1.rs)
materialize the BF16 boundaries used by the framework's rotary operation:

```text
c = BF16_RNE(cosine)
s = BF16_RNE(sine)
negative_b = negate_sign_bit(b)
low = BF16_RNE(widen(BF16_RNE(a * c)) + widen(BF16_RNE(negative_b * s)))
high = BF16_RNE(widen(BF16_RNE(b * c)) + widen(BF16_RNE(a * s)))
```

Products and sums use FP32 arithmetic between explicit BF16 conversions.
Negation occurs before the product, preserving `rotate_half` signed-zero
semantics. Every input and intermediate must be finite; rejection remains
sticky. The existing reciprocal, head normalization, value-copy behavior and
scheduling geometry are unchanged. No intermediate GPU buffer is introduced.

The [independent framework diagnostic](../rope-framework-reference-v1/README.md)
separately matched all 38,528 conditional BF16 words with these materialization
boundaries. That diagnostic neither executes these Rust macros nor establishes
equality of native and framework rotary-table generation.

## Tests

| Suite | Passed | Ignored in This Invocation |
| --- | ---: | ---: |
| New RoPE materialization cases | 20 | 0 |
| Ordinary reciprocal regression invocation | 12 | 1 |
| Explicit exhaustive reciprocal invocation | 1 | 0 |

The ignored reciprocal test in the ordinary invocation is the same exhaustive
test explicitly selected and passed in the final invocation, not an untested
additional case.

The [new tests](fixture/tests/rope_materialized_v1.rs) cover coefficient,
product and final rounding ties, signed zero, cancellation, subnormals,
nonfinite inputs and overflow, all 64 lanes and 20 heads, unchanged value bits,
and sticky rejection. A source-contract test restricts the kernel-entry change
to the new include and post-operation macro selection against the retained V7
entry. The kernel entry itself was not host-compiled by this numerical harness.

The build used opt2 with debug assertions and overflow checks, offline locked
dependencies, two CPU cores and low priority. The original 15-root kernel ABI,
provider source and 284-state/256-round schedule are preserved at source level;
actual checked lowering remains a separate gate.

## Reproduction and Evidence

[complete.json](complete.json) retains the remote result;
[result.json](result.json) records publication checks and file identities.
The [raw records](raw) include all commands, test outcomes and source snapshots.
The publisher rehashed all 61 retained raw records and copied 59; the two larger
dependency snapshots remain outside Git. Test executable identities are
recorded from the remote qualification, but their bodies were not downloaded
or locally rehashed. Publication did not rerun tests locally.

The complete 15-file [tested fixture](fixture) uses a numerical-harness Cargo
manifest with the qualified ASROCK provider path. It is not the device-entry
build manifest. The original checked-entry Cargo pair is retained separately
in [lowering-fixture](lowering-fixture). The [controller recipe](controller/README.md)
documents the frozen inputs, toolchain and fresh-output requirements.

Actual CPU completion SHA-256:
`fae8773996fb7555fd12fe69a0c4dea28f69f0d9834f5022b287cef2036c4fe6`.

## Remaining Gates

Checked gfx950 compilation, LLVM/ISA inspection, GPU execution and independent
model comparison remain pending for this candidate. GPU denormal behavior and
native rotary-table equality are not established by these CPU tests. No
acceptance tolerance, sustained 2,048/256 result or 700 tokens/s claim is added.
