| Category | Packets/forward | All mean sum (ms) | Prompt (ms) | Decode (ms) |
| --- | ---: | ---: | ---: | ---: |
| normalization | 145 | 57.2613 | 57.2682 | 57.2601 |
| MLP down projection | 36 | 14.7343 | 14.7313 | 14.7348 |
| QKV projections | 108 | 14.5455 | 14.5633 | 14.5426 |
| KV cache write | 36 | 11.9466 | 11.9467 | 11.9466 |
| MLP gate/up projections | 72 | 10.5141 | 10.5294 | 10.5116 |
| attention output projection | 36 | 5.1016 | 5.1029 | 5.1014 |
| attention | 36 | 1.7894 | 0.5379 | 1.9913 |
| rotary position | 36 | 1.4085 | 1.4093 | 1.4084 |
| argmax | 1 | 0.9630 | 0.9653 | 0.9626 |
| vocabulary projection | 1 | 0.8692 | 0.8704 | 0.8691 |
| residual add | 72 | 0.5360 | 0.5384 | 0.5356 |
| activation | 36 | 0.3800 | 0.3849 | 0.3792 |
| embedding | 1 | 0.0088 | 0.0105 | 0.0085 |
