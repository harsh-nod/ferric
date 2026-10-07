# Matched Readiness40 Parent Wall Spans

One ordered same-ELF pair with all 40 records and four complete payloads equal. These are disjoint parent wall spans, not GPU time, overlap, throughput or a repeated benchmark.

| Parent interval | Layer V1 seconds | Bank+layer V2 seconds | Candidate / Control | Change |
| --- | ---: | ---: | ---: | ---: |
| Source preparation | 92.230978 | 92.514758 | 1.003077 | 0.307685% |
| Spawn through setup | 223.247939 | 225.375130 | 1.009528 | 0.952838% |
| Prepare/write | 0.001282 | 0.001189 | 0.926881 | -7.311861% |
| Wait for frame | 228.791544 | 136.277128 | 0.595639 | -40.436117% |
| Validate/retain/commit | 0.062859 | 0.020648 | 0.328487 | -67.151272% |
| Close and retirement | 30.063869 | 30.328990 | 1.008819 | 0.881858% |
| Postcheck and publication | 0.637068 | 0.737124 | 1.157057 | 15.705749% |
| Total | 575.035538 | 485.254966 | 0.843870 | -15.613047% |

## Forward Groups

These subsets are not extra time to add to the parent total. All forwards = first use + warm.

| Forward interval | Layer V1 seconds | Bank+layer V2 seconds | Candidate / Control | Change |
| --- | ---: | ---: | ---: | ---: |
| First use (0-1) | 25.965153 | 26.372786 | 1.015699 | 1.569923% |
| Warm (2-39) | 202.890532 | 109.926179 | 0.541800 | -45.819956% |
| All forwards | 228.855685 | 136.298965 | 0.595567 | -40.443269% |

The frame-wait category includes worker execution, pipe waiting and framing. Nested control timers and sidecar publication are excluded. Zero-denominator ratios are n/a. Both cases use scoped layers; the candidate additionally scopes bank rearm. Allocation preflights are unchanged. Neither policy claims temporal equivalence to repeated full currentness. No numerical acceptance or Full2303 feasibility is inferred.
