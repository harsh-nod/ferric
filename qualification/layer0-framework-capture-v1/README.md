# Independent Qwen3 Layer-Zero Capture

The ordinary Qwen3-8B BF16 framework ran on `ssh mi350-2` (ASROCK, gfx950)
and captured 33 genuine layer-zero intermediates twice. Both passes started
with empty KV state and used token `9112` at position zero. Every captured
stage repeated byte for byte. The new layer-zero output also exactly matches
the original independent framework reference.

This is a reproducible numerical diagnostic, not acceptance of Ferric's
arithmetic or a decode benchmark. The current Ferric output still differs.

## Experiment

Each pass executes the ordinary 36-layer `model.model` chain without the
language-model head. Hooks observe layer zero; they neither feed native
intermediates into the framework nor replace framework results with recomputed
expressions. The rotary observer returns the original result objects and is
restored, with all hooks, in `finally`.

The captured stages include the embedding, RMSNorm inputs/outputs, Q/K/V
projections, Q/K head normalization, rotary inputs/outputs, attention output,
O projection, first residual, post-attention norm, gate/up projections, SiLU,
the product consumed by the down projection, down projection, final residual,
and stored K/V. Each tensor must already be BF16, finite, and correctly shaped;
the capture does not silently cast a mismatched dtype.

The installed framework is Torch `2.12.1+rocm7.2` with Transformers `4.51.0`.
The capture retains deterministic math-SDPA settings, disables low-precision
SDPA reduction, and preserves the original FP32 rotary buffer. Loaded Python
implementation files are pinned to their actual installed bytes. This does
not authenticate every native library or machine-code dependency.

| Actual Check | Result |
| --- | ---: |
| Independent fresh-KV passes | 2 |
| Captured BF16 stages per pass | 33 |
| Repeated stages with identical bytes | 33 / 33 |
| Native attempts / retries | 1 / 0 |
| Before / immediate / after device-process audits | 3 / 1 / 3 |
| Natural, reaped owned process leaves | 25 / 25 |
| Capture-policy tests | 16 passed |
| Launcher-policy tests | 18 passed |
| Comparison tests | 14 passed |

These are separate test runs, not one combined suite. The 16 capture tests
also ran inside the owned launcher. The framework child exited naturally;
51.504 seconds includes loading, observation and synchronization, and is not
an inference latency or throughput measurement.

## Numerical Comparison

The [actual comparison](comparison/complete.json) uses authentic position-zero
payloads and verifies the shared model/input identity. All 4,096 BF16 words
of the new layer-zero result are compared, not just a checksum or selected
elements. The original framework payloads and current native payload are
checked at their full byte extent before selecting this slice.

| Candidate Compared With New Framework | Exact BF16 Words | Maximum Absolute Error | RMSE | Relative L2 |
| --- | ---: | ---: | ---: | ---: |
| Original independent framework | 4,096 / 4,096 | 0 | 0 | 0 |
| Current V7 prefix / paired-row MLP Ferric capture | 1,834 / 4,096 | 0.0625 | 0.0013783591 | 0.0035785690 |

The current native input comes from the [paired-row GPU experiment](../paired-row-mlp-native-v1/README.md)
on `mi350`. It contains all layer outputs, but not the internal layer-zero
operations. Thus this comparison cannot yet identify the first differing
operation or attribute the discrepancy to an individual kernel. No tolerance
was fitted to these observations and no numerical acceptance is claimed.

## What This Enables

The installed framework materializes BF16 SiLU before multiplication by the
up projection, and BF16 O/down projections before adding each residual.
Ferric's current fused expressions retain FP32 intermediates through those
boundaries. RMSNorm also has reduction/reciprocal-square-root differences.
These source-level differences are hypotheses, not measured explanations of
the error in the table.

The next experiment must capture the current native chain's internal stages
for the same token `9112`. The standalone prefix fixture uses a different
token, and the historical V227 internal capture uses older images; neither
can substitute for current-image evidence. Once the earliest differing stage
is known, a separately labeled conditional replay can compare arithmetic
using identical immediate inputs, before changing the real kernel.

## Evidence

- [Publication ledger](result.json), [actual outer completion](owner/complete.json),
  [capture receipt](capture/capture.json), and [66-tensor typed hash ledger](tensor-ledger.json).
- [Capture source](source/capture/run.py), [launcher source](source/launcher/launch.py),
  and separate [16-test](tests/capture/complete.json) / [18-test](tests/launcher/complete.json) receipts.
- [Comparison supplement](comparison-manifest.json), [comparison source](comparison-source/compare.py),
  and [14-test receipt](comparison-tests/complete.json).

Source-package READMEs preserve their pre-execution authoring status; the
actual receipts linked above record subsequent execution. Raw tensor bodies,
model weights and copied third-party implementation bodies are retained
outside Git. The publication checks the retained tensor bytes but does not
claim that every transitive input was rehashed or that the outer SSH client's
lifetime was independently audited.

The framework capture's outer completion SHA-256 is
`46fd9acbca798f05bc737a651e6fba54e65c78688e2d425a8287eef402065edc`;
the CPU comparison completion SHA-256 is
`62d3781d670a11d0d2f7fd3f72dc2022909b1929f4d050795f48c72fabffdb9f`.
Neither the CPU comparison nor publication launches another GPU workload.

Full-model and longer-context numerical acceptance, calibrated timing,
overlap, and sustained single-request Qwen3-8B BF16 target-only 2,048/256
decoding remain open. There is no production admission or 700 tokens/s claim.
All issue #42 M0-M7 milestones remain open.
