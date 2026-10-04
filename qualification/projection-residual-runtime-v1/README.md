# Projection-Residual Runtime Route

The additive parent/worker route passed joint CPU qualification on `mi350-2`
on 2026-10-04: **988 passed, four pre-existing worker ignores**. All 84 bounded
commands exited naturally and were reaped. Source, input, dependency, original
build-directory and selected-artifact checks passed. This is not GPU execution.

## Route

The new parent binary is
`ferric-qwen3-finite-projection-residual-layer-capture-engineering`, selected by
`--capture-projection-residual-layer-zero`. It accepts the closed
`FerricFiniteProjectionResidualLayerCaptureRequestV1` wrapper containing the
original layer request and one additional image pin. The worker uses the
separate `--engineering-native-projection-residual-layer-v1` selector.

The extra [checked gfx950 image](../projection-residual-lowering-v1/README.md)
materializes the combined projection in BF16 before adding the residual. It is
selected at both the output-projection and down-projection residual stages.
Its identity is included in the new profile digest and checked on both ranks.
The original image still supplies copy operations; existing selectors, default
arithmetic, requests and image pins are preserved.

The capture retains the existing 28-stage layout, finite-value and untouched-KV
checks, evidence limits, terminal Close and child-reaping requirements. New
observations do not reuse old parity results or grant numerical acceptance.

## Actual Coverage

| Compiled suite | Passed | Ignored | Additional test executions |
| --- | ---: | ---: | ---: |
| Paired runtime regressions | 208 | 0 | 0 |
| Worker | 477 | 4 | 16 |
| Parent | 303 | 0 | 14 |
| Total | 988 | 4 | 30 |

The parent additions include nine direct library tests, one binary test, and
four shared wire tests also compiled in the worker. Tests cover schema and
selector isolation, image/profile/ABI identity, both residual selections,
loading failures, terminal state, Close, evidence bounds, and legacy behavior.
Synthetic loader tokens do not establish successful native image loading.

Both new executable identities and all sixteen build outputs are recorded in
the [result ledger](result.json). The parent retains its locked Git dependencies;
the worker builds against the paired local runtime. Neither dependency route
was silently substituted. The private build output ended at 1,813,233,284 bytes.

The [executed controller](controller.py) and [publication verifier](publish.py)
retain the exact build/test recipes. The verifier replayed every named test
outcome, checked all 427 raw records, checked the two selected executable bodies,
and matched all 22 integrated files to their formatted, tested source hashes.
Large raw logs, source maps, archives and executables stay in retained evidence
outside Git; the ledger records their paths and hashes.

Actual CPU completion SHA-256:
`bf1a12f78981d9ff9b8157e1dec6dca300752b238680e16380b98e9d1260bafb`.

## Next Gate

Fresh runtime checks and a genuine candidate GPU capture must precede the
[independent comparison](../projection-residual-comparison-v1/README.md) of both
residuals and the framework's intermediate values. Full-model correctness,
calibrated performance, sustained 2,048/256 decode and the 700 tokens/s target
remain open. This route is an explicit engineering diagnostic, not a production
default or a performance result.
