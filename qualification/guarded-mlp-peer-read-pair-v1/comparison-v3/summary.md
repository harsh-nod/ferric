# Paired-Read Host Interval Diagnostic

Four independent serial runs, two per mode. Each row contains four correlated forward observations, not four independent samples.

| Run | Forward 0 (ms) | Forward 1 (ms) | Forward 2 (ms) | Forward 3 (ms) | Total (ms) |
| --- | ---: | ---: | ---: | ---: | ---: |
| control-0 | 509.181 | 512.940 | 509.660 | 519.989 | 2051.770 |
| paired-0 | 144.079 | 142.170 | 143.744 | 141.362 | 571.355 |
| paired-1 | 145.674 | 145.230 | 145.199 | 144.868 | 580.971 |
| control-1 | 514.980 | 516.076 | 512.632 | 511.860 | 2055.548 |

Each forward sums 36 disjoint outer hidden-read wall intervals. Inclusive currentness/read timer counters are not added to elapsed time.

| Mode | Hidden intervals | Group full checks / interval | Full checks / rank / interval | Reads / rank / interval | Bytes / rank / interval |
| --- | ---: | ---: | ---: | ---: | ---: |
| control | 288 | 4 | 2 | 1 | 8192 |
| paired | 288 | 2 | 0 | 1 | 8192 |

[Host wall plot](hidden-read-host-wall.svg). Exact integer observations are retained in `forward-intervals.csv` and all 576 per-layer rows in `layer-counts.csv`.

These are host timings, including blocking and completion waits, not GPU execution time, overlap, tokens per second, or end-to-end speedup. Two runs per mode are a small diagnostic, not a controlled repeated benchmark. Payload/history equality is a prerequisite, not independent model accuracy or numerical acceptance.

The renderer authenticates the analysis, four terminal receipts and four checked host bodies. It reconciles counts/totals but does not rerun the model, rehash payload bodies, revalidate the entire capsule, or infer a framework reference.

Analysis SHA-256: `a62d50e641ba5be633000042ec5f69886169e65dea305b414f11bb798be7ff1b`.
