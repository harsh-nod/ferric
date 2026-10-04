# Raw Completion Tick Report

One four-forward teacher-forced observation; not the 2,048/256 workload.
Raw tick deltas are not nanoseconds, calibrated durations, cross-device coordinates or a throughput result.

| Rank | Stage | Count | Min ticks | Median ticks | Max ticks | Zero deltas |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 0 | embedding | 4 | 912 | 956 | 1184 | 0 |
| 0 | prefix | 144 | 1229604 | 1287468 | 1351972 | 0 |
| 0 | post-attention-residual | 144 | 800 | 1072 | 1556 | 0 |
| 0 | mlp | 144 | 2186880 | 2270702 | 2372576 | 0 |
| 0 | post-mlp-residual | 144 | 1008 | 1060 | 1456 | 0 |
| 0 | final-norm | 4 | 74044 | 74644 | 75144 | 0 |
| 0 | head | 4 | 86392 | 87008 | 88168 | 0 |
| 0 | argmax | 4 | 2071220 | 2115788 | 2249884 | 0 |
| 1 | copy | 4 | 912 | 954 | 1196 | 0 |
| 1 | prefix | 144 | 1221476 | 1281532 | 1354332 | 0 |
| 1 | post-attention-residual | 144 | 1000 | 1048 | 2588 | 0 |
| 1 | mlp | 144 | 2163216 | 2262504 | 2361512 | 0 |
| 1 | post-mlp-residual | 144 | 1004 | 1044 | 1904 | 0 |

![Per-device ranges and medians](raw-tick-ranges.svg)

![Layer and position raw ticks](raw-tick-heatmaps.svg)

## Separate Host Intervals

These inclusive host intervals are recorded in nanoseconds and joined to the native Controls.
They are not GPU durations and must not be subtracted from the tick values.

| Rank | Stage | Min host ns | Median host ns | Max host ns |
| --- | --- | ---: | ---: | ---: |
| 0 | embedding | 8455476 | 8482525.5 | 8536825 |
| 1 | copy | 8450465 | 8456715 | 8518425 |
| 0 | prefix | 15744300 | 16280960.5 | 16945461 |
| 1 | prefix | 15600140 | 16215090 | 16965351 |
| 0 | post-attention-residual | 6650824 | 6700709 | 7315655 |
| 1 | post-attention-residual | 4958453 | 4995973 | 5525954 |
| 0 | mlp | 25298036 | 26109441 | 27120107 |
| 1 | mlp | 24991346 | 26047466 | 27066957 |
| 0 | post-mlp-residual | 6654084 | 6698059.5 | 8524345 |
| 1 | post-mlp-residual | 4962453 | 4993593 | 6394024 |
| 0 | final-norm | 8427745 | 8455680 | 8553125 |
| 0 | head | 8413365 | 8455255 | 8483445 |
| 0 | argmax | 24158515 | 24603915 | 25918326 |
