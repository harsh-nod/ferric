# Layer-Zero Native Capture Parent

This engineering-only parent adds a candidate-only capture route for the
current V7 prefix and paired-row MLP images, using the genuine Qwen3-8B input
token `9112` at position zero. It will help locate the first observable
difference from the [independent framework capture](../layer0-framework-capture-v1/README.md).
It does not establish numerical correctness, production readiness, or decode
performance.

## Qualification

The fresh build and tests ran on `ssh mi350-2` (ASROCK). All 46 bounded commands
exited naturally, their process groups were reaped, and the input, source and
artifact postchecks passed. GPU visibility was disabled for this CPU run.

| Actual Check | Result |
| --- | ---: |
| Selected Rust tests | 289 passed, 0 ignored |
| New tests included above | 14 passed |
| Library / binary tests | 275 / 14 passed |
| Parent executable builds | 14 passed |
| Default-feature check | Passed |
| Separate CPU-controller policy tests | 20 passed |
| Source formatting and format check | Passed |
| New native intermediate GPU capture | [Passed in a separate MI350 run](../layer0-native-capture-v1/README.md) |

The seven-file patch changes only the parent. The worker and kernel images
were not rebuilt or modified. The parent still compiles against its locked
Git runtime, not the newer sibling runtime archive retained with the inputs.

## Capture Route

The separate `ferric-qwen3-finite-prefix-layer-capture-engineering` binary
requires `--capture-layer-zero` and the existing explicit machine-code opt-in.
Its closed request schema reuses authenticated model, prompt, upload and image
inputs; it introduces no override for hidden values or token identity.

The route reuses the existing owned-child execution path for one candidate
attempt, including terminal-state checks, `Close`, natural exit and reap.
The capture format has 28 arrays across two ranks, including BF16 stage
outputs and separately typed FP32 projection partials. Untouched KV regions
remain checked. The old paired comparison route and its parity requirement
are unchanged; the new observation omits `bitwise_equal` because it performs
no paired comparison.

## Numerical Gate

The already captured current native layer-zero output matches the independent
framework at 1,834 of 4,096 BF16 words, with maximum absolute difference
`0.0625` and relative L2 difference `0.0035785690`. Those measurements establish
a discrepancy, not its cause. The standalone prefix fixture uses another token,
and the historical internal capture uses older images.

The [subsequent GPU experiment](../layer0-native-capture-v1/README.md) captured
the current images with matching input and joined its final hidden values
exactly to the current native output. Comparing the observed intermediates
against the independent framework remains next. A rank-local
FP32 partial is not a full BF16 projection. No tolerance is fitted to the
observed error, and capturing intermediate values is not numerical acceptance.

## Evidence

- [Publication ledger](result.json) and [actual CPU completion](cpu/complete.json).
- [New capture implementation](../../adapters/m1-engineering-execution-v1/src/tp_finite_client/prefix_layer/capture.rs),
  [controller](controller/run.py), [20-test receipt](pure/complete.json), and [format receipt](format/complete.json).
- [Initial packaging failure](initial-packaging-failure/root-observation.json):
  the first archive lacked its required `ferric/` prefix and was rejected before
  Cargo started. The corrected archive ran in a fresh directory with unchanged
  extraction checks. The failed inputs remain retained; this was not a Rust
  compilation failure or a GPU result.

Raw command records and source pins are published; ELF binaries, model data
and source archives remain outside Git. Publication verifies the selected
parent ELF locally, but does not claim local rehashing of the other thirteen
executables or every transitive CPU dependency. The completion SHA-256 is
`78c12f822e95d50a1239f56411c5b8da9411a3fbda7510fbbf38d5644e1dffbb`.

Full-model correctness, calibrated timing, overlap and sustained BF16
single-request 2,048/256 decoding remain open. No 700 tokens/s claim is made.
All issue #42 M0-M7 milestones remain open.
