# Observed Host-Wall Comparison

One observation per mode, not a controlled repeated benchmark. All four payloads and genuine histories are identical.
Ratios divide conservative host wall by shared-full host wall; they are not device speedups or throughput.

| Forward | Conservative Bracket (s) | Shared-Full Bracket (s) | Shared Minus Conservative (s) | Observed Ratio |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 12.808321110 | 5.597993810 | -7.210327300 | 2.288 |
| 1 | 12.869605777 | 5.625657938 | -7.243947839 | 2.288 |
| 2 | 15.252753965 | 6.270930977 | -8.981822988 | 2.432 |
| 3 | 15.383587716 | 6.244649921 | -9.138937795 | 2.463 |

The separate per-mode tables retain inclusive rank/shared counters without adding them to wall time.
The chart uses only disjoint prefix, paired, hidden-read and other snapshot intervals. Close is outside these brackets.
No GPU timing, overlap, tokens/second, numerical acceptance, or production authority is claimed.
