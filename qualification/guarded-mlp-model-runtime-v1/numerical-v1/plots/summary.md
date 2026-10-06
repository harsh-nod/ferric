# Guarded Model Numerical Diagnostic

**Diagnostic only: not numerical acceptance or performance evidence.**

All eight output tokens match the independent reference on matching input histories. None of the 304 complete compared tensors is bit-identical. The original failed TF4 controller receipt remains unchanged; its separate data revalidation authorized comparison.

![Logits relative L2 error by position](logits-relative-l2.svg)

| Mode | Position | Input | Output (Both) | Logits Relative L2 (%) | Max Absolute Error | RMSE | Exact Logits Words |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| TF4 | 0 | 9112 | 67 | 0.304480 | 0.0625 | 0.010776199 | 90,814/151,936 |
| TF4 | 1 | 2190 | 198 | 0.619824 | 0.1875 | 0.04095589 | 47,971/151,936 |
| TF4 | 2 | 3772 | 25 | 0.402571 | 0.15625 | 0.032432615 | 78,377/151,936 |
| TF4 | 3 | 220 | 16 | 0.533481 | 0.171875 | 0.034020986 | 56,126/151,936 |
| AR4 | 0 | 9112 | 67 | 0.304480 | 0.0625 | 0.010776199 | 90,814/151,936 |
| AR4 | 1 | 67 | 25 | 0.941348 | 0.20703125 | 0.049389407 | 31,458/151,936 |
| AR4 | 2 | 25 | 576 | 1.174368 | 0.1875 | 0.082381315 | 546/151,936 |
| AR4 | 3 | 576 | 2701 | 0.507859 | 0.125 | 0.034438248 | 53,565/151,936 |

Relative L2 (%) is exactly `100 * recorded_relative_l2`; the recorded ratio is `||candidate - reference||_2 / ||reference||_2`. This renderer does not recompute tensors or change the comparison policy.

[CSV with full recorded numeric values and tensor hashes](logits.csv). The chart and table round display values only.

Source: the retained `ferric-guarded-mlp-model-numerical-diagnostic-v1` report, 389,586 bytes, SHA-256 `55b550cf19e3deae73384db0895407b7d16cd9c75d6e677aa3159eed922da482`. The renderer verifies that exact report before and after rendering. It does not independently rehash the 250 upstream inputs or repeat the model/reference executions.

No acceptance threshold, full-model correctness, speedup, theoretical performance bound, or sustained 2,048-prompt/256-decode result is asserted.
