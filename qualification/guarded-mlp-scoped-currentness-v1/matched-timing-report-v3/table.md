# Matched Readiness40 Parent Wall Spans

One ordered same-ELF pair with all 40 records and four complete payloads equal. These are disjoint parent wall spans, not GPU time, overlap, throughput or a repeated benchmark.

| Parent interval | Default seconds | Scoped seconds | Scoped / Default | Change |
| --- | ---: | ---: | ---: | ---: |
| Source preparation | 92.468043 | 92.394256 | 0.999202 | -0.079798% |
| Spawn through setup | 223.341399 | 222.558895 | 0.996496 | -0.350362% |
| Prepare/write | 0.001605 | 0.001394 | 0.868580 | -13.142030% |
| Wait for frame | 591.557698 | 230.370293 | 0.389430 | -61.057004% |
| Validate/retain/commit | 0.030827 | 0.022916 | 0.743360 | -25.664018% |
| Close and retirement | 29.896180 | 30.645653 | 1.025069 | 2.506919% |
| Postcheck and publication | 0.757600 | 0.783908 | 1.034726 | 3.472619% |
| Total | 938.053351 | 576.777314 | 0.614866 | -38.513378% |

The frame-wait category includes worker execution, pipe waiting and framing. Nested control timers and sidecar publication are excluded. Zero-denominator ratios are n/a. Scoped warm changes temporal validation sampling and is not SharedFull or full-currentness equivalence. No numerical acceptance or Full2303 feasibility is inferred.
