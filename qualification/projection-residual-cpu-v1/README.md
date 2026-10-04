# BF16 Projection Residual: CPU Qualification

This 2026-10-04 engineering checkpoint adds a separately named Rust kernel,
[`qwen3-tp-projection-residual-kernels-v1`](../../device/qwen3-tp-projection-residual-kernels-v1/src/collective.rs).
It does not change V18, the copy kernel, or the current runtime route.

The [captured-data replay](../output-residual-boundary-v1/README.md) identified
a missing BF16 materialization between the TP2 projection sum and residual
addition. The new candidate implements:

```text
sum0       = FP32_RNE(+0 + partial_rank0)
sum        = FP32_RNE(sum0 + partial_rank1)
projection = BF16_RNE(sum)
output     = BF16_RNE(FP32_RNE(widen(projection) + widen(residual)))
```

Neither rank's partial is narrowed independently. Every active input and
intermediate must be finite. Projection overflow traps before the residual
is read. The source retains the closed rows=1/world=2 contract, 4,096 active
elements, 64 workgroups of 64 threads, and ten slice pairs followed by two
32-bit scalars. This is a 168-byte explicit source ABI; actual emitted ABI
validation remains a separate gate.

## Actual Tests

The bounded CPU run completed on SSH host `mi350-2` (ASROCK), with GPU
visibility disabled, two CPU cores, nice 10, and a fresh build target.

| Suite | Passed | Ignored |
| --- | ---: | ---: |
| New candidate arithmetic | 12 | 0 |
| New candidate source and ABI contracts | 5 | 0 |
| Unchanged V18 arithmetic | 8 | 0 |
| Unchanged V18 source and ABI contracts | 6 | 0 |
| Total | 31 | 0 |

The tests cover staged rounding, projection ties, cancellation, finite BF16
residuals, subnormals, nonfinite inputs, overflow, load order, inactive ranks,
and unique output ownership. The independent arithmetic reference uses
staged FP64-to-FP32 operations and integer BF16 rounding, not the device
`Bf16` conversion implementation.

All 18 commands exited naturally with empty owned process groups. Formatting,
the exact test inventories, source/tool/input postchecks, and preservation of
the four prior build targets passed. Candidate and control use identical
dependency versions; the candidate lock changes only the root package name.

## Evidence and Limits

- [Publication summary](result.json) binds the tested sources and retained logs.
- [Actual remote completion](complete.json) records all 31 named tests.
- [Qualification runner](controller/run.py) records the exact command recipes.
- [Candidate arithmetic output](raw/candidate-lib-tests-stdout) and
  [contract output](raw/candidate-contract-tests-stdout) retain actual outcomes.

The four large dependency/target inventory maps remain in the local evidence
archive, with hashes in the publication summary. Sources, command records,
test logs, and completion records are retained here; test executables are
pinned in the receipt, not checked into Git.

This is CPU qualification only. Checked HSACO emission, actual GPU execution,
independent model acceptance, and performance remain unqualified at this
checkpoint. The preceding replay still differs from the framework at two
residual values; passing these tests does not resolve those differences or
establish the 700 tokens/s target.
