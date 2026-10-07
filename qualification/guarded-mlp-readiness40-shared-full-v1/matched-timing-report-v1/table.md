# Matched Readiness40 Parent Wall Spans

One ordered same-ELF pair with all 40 records and four complete payloads equal. These are disjoint parent wall spans, not GPU time, overlap, throughput or a repeated benchmark.

| Parent interval | Default seconds | Shared seconds | Shared / Default | Change |
| --- | ---: | ---: | ---: | ---: |
| Source preparation | 91.804484 | 92.323468 | 1.005653 | 0.565315% |
| Spawn through setup | 227.087301 | 143.431064 | 0.631612 | -36.838800% |
| Prepare/write | 0.001694 | 0.001235 | 0.729154 | -27.084615% |
| Wait for frame | 593.931118 | 237.647229 | 0.400126 | -59.987409% |
| Validate/retain/commit | 0.022963 | 0.018693 | 0.814050 | -18.595047% |
| Close and retirement | 29.865943 | 14.078233 | 0.471381 | -52.861917% |
| Postcheck and publication | 0.613294 | 0.697339 | 1.137039 | 13.703928% |
| Total | 943.326798 | 488.197262 | 0.517527 | -48.247282% |

The frame-wait category includes worker execution, pipe waiting and framing. Nested control timers and sidecar publication are excluded. Zero-denominator ratios are n/a. No numerical acceptance or Full2303 feasibility is inferred.
