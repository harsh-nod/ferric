# Four-Step Decode Framework Comparison

The independent CPU comparison completed on ASROCK after the
[MI350 GPU capture](../projection-residual-decode-native-v1/README.md).
All four output tokens match the genuine framework reference. All 152 full
tensor slices contain at least one differing BF16 word, so token agreement
does not establish numerical acceptance.

## Measured Results

| Position | Output Token (Both) | Logit Max Absolute Error | Logit Relative L2 | Exact Logit Words / 151,936 |
| --- | ---: | ---: | ---: | ---: |
| 0 | 67 | 0.0625 | 0.332344% | 85,393 |
| 1 | 198 | 0.1875 | 0.757311% | 34,850 |
| 2 | 25 | 0.125 | 0.410781% | 79,655 |
| 3 | 16 | 0.25 | 0.859355% | 22,013 |

Relative L2 is `||candidate - reference||_2 / ||reference||_2`.
The table reports the measured values, not an acceptance threshold.
[table.json](table.json) retains all 152 comparisons, exact-word counts,
first differing elements, maximum errors, RMSE and layer-by-layer error
trajectories. Error changes between layers do not establish their cause.

At position zero, layer-zero hidden agrees at 2,728/4,096 words, with maximum
absolute error 0.0078125 and relative L2 0.0017142513. This reproduces the
earlier dedicated layer-zero observation. Remaining activation-path differences
are documented in the [SiLU diagnostic](../silu-materialization-diagnostic-v1/README.md);
this full-forward comparison does not isolate them as the only cause.

## Reference and Checks

The unchanged teacher-forced input sequence is 9112, 2190, 3772, 220. The
framework reference contains two independently initialized KV-cache passes.
The comparison rehashed all eight 606,976-byte framework payloads, checked the
recorded reference process completion and repeat equality, and compared the
36 hidden vectors, final norm and logits at each of four positions.

The adapter replayed the new native record format, fourteen capture files,
seven naturally closed process leaves and the recorded six-audit file joins.
It did not launch GPUs or reinterpret MI350 topology as current ASROCK state.
Twelve synthetic tests passed separately in 14.231 seconds, including token
divergence, signed zero, nonfinite values, malformed captures and ownership
failures. Both runs retained unchanged source snapshots.

Actual comparison SHA-256:
`2a5b98ae6a3a2ba4a034dbea0662504988d388b12a5447eaa395f82e3a41dd0c`.
[result.json](result.json) records its inputs, source identities and limitations.
Publication validates authenticated metadata and copies the measured table;
it does not rerun tensor arithmetic locally.

## Remaining Work

No tolerance was introduced or loosened. Numerical acceptance, a wider input
corpus, the 2,048-token prompt / 256-token generated workload, calibrated timing
and the 700 tokens/s target remain open. The next arithmetic candidate is
explicit BF16 SiLU materialization before the up product; it still needs its
own checked lowering and hardware validation.
