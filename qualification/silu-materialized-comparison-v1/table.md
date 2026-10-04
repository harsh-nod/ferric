# Retained SiLU Comparison

Diagnostic metrics only. No numerical acceptance or performance claim.

| Stage | Rank | Baseline Exact | Candidate Exact | Words | Baseline Max Abs | Candidate Max Abs | Baseline Rel L2 | Candidate Rel L2 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| norm | 0 | 4096 | 4096 | 4096 | 0 | 0 | 0 | 0 |
| norm | 1 | 4096 | 4096 | 4096 | 0 | 0 | 0 | 0 |
| qkv | 0 | 3071 | 3071 | 3072 | 4.7683716e-07 | 4.7683716e-07 | 1.9595968e-07 | 1.9595968e-07 |
| qkv | 1 | 3072 | 3072 | 3072 | 0 | 0 | 0 | 0 |
| query | 0 | 2047 | 2047 | 2048 | 1.5258789e-05 | 1.5258789e-05 | 2.1737903e-07 | 2.1737903e-07 |
| query | 1 | 2048 | 2048 | 2048 | 0 | 0 | 0 | 0 |
| key-current | 0 | 512 | 512 | 512 | 0 | 0 | 0 | 0 |
| key-current | 1 | 512 | 512 | 512 | 0 | 0 | 0 | 0 |
| value-current | 0 | 512 | 512 | 512 | 0 | 0 | 0 | 0 |
| value-current | 1 | 512 | 512 | 512 | 0 | 0 | 0 | 0 |
| attention | 0 | 2048 | 2048 | 2048 | 0 | 0 | 0 | 0 |
| attention | 1 | 2048 | 2048 | 2048 | 0 | 0 | 0 | 0 |
| first-residual | 0 | 4094 | 4094 | 4096 | 0.00024414062 | 0.00024414062 | 5.438813e-05 | 5.438813e-05 |
| first-residual | 1 | 4094 | 4094 | 4096 | 0.00024414062 | 0.00024414062 | 5.438813e-05 | 5.438813e-05 |
| mlp-norm | 0 | 4094 | 4094 | 4096 | 0.00048828125 | 0.00048828125 | 7.5701516e-05 | 7.5701516e-05 |
| mlp-norm | 1 | 4094 | 4094 | 4096 | 0.00048828125 | 0.00048828125 | 7.5701516e-05 | 7.5701516e-05 |
| gate | 0 | 5911 | 5911 | 6144 | 0.001953125 | 0.001953125 | 0.00040103214 | 0.00040103214 |
| gate | 1 | 5958 | 5958 | 6144 | 0.001953125 | 0.001953125 | 0.0003641741 | 0.0003641741 |
| up | 0 | 5927 | 5927 | 6144 | 0.001953125 | 0.001953125 | 0.00039606014 | 0.00039606014 |
| up | 1 | 5904 | 5904 | 6144 | 0.00390625 | 0.00390625 | 0.00045001022 | 0.00045001022 |
| activation-product | 0 | 4235 | 5766 | 6144 | 0.00390625 | 0.0009765625 | 0.0022693397 | 0.00043476921 |
| activation-product | 1 | 4318 | 5777 | 6144 | 0.00390625 | 0.00048828125 | 0.0017507864 | 0.00022103495 |
| final-hidden | 0 | 2728 | 3807 | 4096 | 0.0078125 | 0.00390625 | 0.0017142513 | 0.00059154807 |
| final-hidden | 1 | 2728 | 3807 | 4096 | 0.0078125 | 0.00390625 | 0.0017142513 | 0.00059154807 |

## Conditional Residuals

| Stage | Rank | Exact Words | Words |
| --- | ---: | ---: | ---: |
| output | 0 | 4096 | 4096 |
| output | 1 | 4096 | 4096 |
| down | 0 | 4096 | 4096 |
| down | 1 | 4096 | 4096 |

## Same-Gate SiLU Controls

Framework product control: 12,288 exact words. Predictions use genuine captured BF16 SiLU, not a reconstruction of native OCML exp.

| Native Capture | Rank | Same-Gate Words | Equal Materialized | Different Materialized |
| --- | ---: | ---: | ---: | ---: |
| baseline | 0 | 5911 | 4317 | 1594 |
| baseline | 1 | 5958 | 4416 | 1542 |
| candidate | 0 | 5911 | 5911 | 0 |
| candidate | 1 | 5958 | 5958 | 0 |

## Native Down Partials

FP32 partial-to-partial changes only; neither side is a full framework projection oracle.

| Rank | Exact FP32 Words | Words | Max Abs Change | Relative L2 Change |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 1 | 4096 | 0.0010414235 | 0.001434407 |
| 1 | 0 | 4096 | 0.0030417442 | 0.0012395602 |
