# Matched Readiness40 Parent Wall Spans

One ordered same-ELF pair with all 40 records and four complete payloads equal. These are disjoint parent wall spans, not GPU time, overlap, throughput or a repeated benchmark.

| Parent interval | Census V3 seconds | Tail V4 seconds | Candidate / Control | Change |
| --- | ---: | ---: | ---: | ---: |
| Source preparation | 91.572824 | 92.066966 | 1.005396 | 0.539616% |
| Spawn through setup | 222.683234 | 224.455931 | 1.007961 | 0.796062% |
| Prepare/write | 0.001473 | 0.001556 | 1.056339 | 5.633942% |
| Wait for frame | 98.051694 | 94.429460 | 0.963058 | -3.694209% |
| Validate/retain/commit | 0.021092 | 0.021157 | 1.003090 | 0.309033% |
| Close and retirement | 29.858758 | 30.206053 | 1.011631 | 1.163127% |
| Postcheck and publication | 0.903682 | 0.874703 | 0.967932 | -3.206765% |
| Total | 443.092756 | 442.055825 | 0.997660 | -0.234021% |

## Forward Groups

These subsets are not extra time to add to the parent total. All forwards = first use + warm.

| Forward interval | Census V3 seconds | Tail V4 seconds | Candidate / Control | Change |
| --- | ---: | ---: | ---: | ---: |
| First use (0-1) | 25.789911 | 26.078570 | 1.011193 | 1.119270% |
| Warm (2-39) | 72.284347 | 68.373602 | 0.945898 | -5.410224% |
| All forwards | 98.074258 | 94.452172 | 0.963068 | -3.693208% |

The frame-wait category includes worker execution, pipe waiting and framing. Nested control timers and sidecar publication are excluded. Zero-denominator ratios are n/a. Both cases scope bank rearm, layers and the warm-layer allocation censuses. The candidate additionally scopes the warm tail. Neither policy claims temporal equivalence to repeated full currentness. The census counters remain a layer subset. Tail counters are separate diagnostics, not durations or extra spans to add. No numerical acceptance or Full2303 feasibility is inferred.
