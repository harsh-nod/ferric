# Genuine Autoregressive Framework Comparison

The finite Qwen3-8B BF16 runtime's four-step own-output decode was compared
with an independent PyTorch/Transformers execution on ASROCK through
`ssh mi350-2`. Each implementation starts with token `9112` and feeds back
its own output. All four tokens match: `67, 25, 576, 2701`.

The framework ran twice with fresh KV caches. All retained payloads repeat
byte for byte. Both histories remain comparable through all four steps;
conditional replay was unnecessary. There are 152 compared tensor slices:
36 layer outputs, final normalization and full-vocabulary logits per step.
**No complete tensor slice is bitwise equal between native and framework.**

## Measured Differences

These are diagnostics, not an acceptance threshold. Each logit row contains
151,936 BF16 values. Relative L2 is `||native - reference||2 / ||reference||2`.

| Position | Input Token | Native / Reference Output | Logit Relative L2 | Maximum Absolute Error | Exact Logit Words |
| --- | ---: | --- | ---: | ---: | ---: |
| 0 | 9112 | 67 / 67 | 0.0030448043 | 0.0625 | 90,814 |
| 1 | 67 | 25 / 25 | 0.0140315337 | 0.25 | 9,715 |
| 2 | 25 | 576 / 576 | 0.0043564111 | 0.125 | 74,009 |
| 3 | 576 | 2701 / 2701 | 0.0045131941 | 0.1875 | 69,994 |

Position 1 has the largest logit relative error, about 1.40%. Some intermediate
layer errors are larger: position 3, layer 12 reaches about 2.55% relative L2.
Matching four argmax tokens does not establish full-model correctness. The
[complete comparison](comparison.json) retains all layer-level differences.
This is a different input history from the earlier teacher-forced diagnostic;
the two tables are not a controlled optimization ablation.

## Method And Evidence

The reference uses the pinned model and installed implementation recorded in
[reference/reference.json](reference/reference.json). It loads BF16 weights,
uses FP32 RoPE coefficient arithmetic and math SDPA, disables BF16 reduced-
precision reduction, and captures BF16 layer outputs and logits. It does not
consume native intermediate tensors to construct its genuine trajectory.

All 25 owned child/utility leaves completed naturally. Three before audits,
one immediate audit and three after audits passed. Twenty framework policy
tests also ran inside this attempt; the earlier separate
[38-test policy observation](../projection-ar4-supervisor-v1/README.md)
remains a distinct result. The native execution and its lifecycle are recorded
in the [native checkpoint](../projection-ar4-native-v1/README.md), not rerun by
this framework comparison.

[result.json](result.json) records exact artifact identities and publication
checks. The publisher independently rehashes all eight retained framework
payloads, tensor slices, repeat equality, recurrence, lowest-index argmax and
exact-word counts. Relative L2, RMSE and maximum-error metrics are retained
from the authenticated reference report, not recomputed by the publisher.
Binary captures and installed third-party source copies remain in the retained
evidence archive; model weights, private caches and executables are not
published. The inner report was written before process exit; the outer owner
records establish natural child completion without rewriting that report.

This four-step, single-seed diagnostic is not the sustained 2,048-token prompt
and 256-token decode workload. It provides neither a throughput measurement
nor a 700-token/s result. Full numerical and production acceptance remain open.
