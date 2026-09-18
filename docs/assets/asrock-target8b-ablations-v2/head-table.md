| Configuration | TTFT (s) | Mean TPOT (s) | Post-first tokens/s | Observed rate / first | Setup (s) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Scalar + BF16 logits | 5.970907 | 1.280531 | 0.780926 | 1.000x | 169.661754 |
| Scalar + FP32 logits | 6.257646 | 1.281963 | 0.780053 | 0.999x | 167.166619 |
| MFMA + FP32 logits | 1.787298 | 0.462051 | 2.164262 | 2.771x | 211.986581 |
