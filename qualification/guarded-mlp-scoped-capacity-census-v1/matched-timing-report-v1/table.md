# Matched Readiness40 Parent Wall Spans

One ordered same-ELF pair with all 40 records and four complete payloads equal. These are disjoint parent wall spans, not GPU time, overlap, throughput or a repeated benchmark.

| Parent interval | Bank V2 seconds | Census V3 seconds | Candidate / Control | Change |
| --- | ---: | ---: | ---: | ---: |
| Source preparation | 91.218519 | 91.431435 | 1.002334 | 0.233413% |
| Spawn through setup | 223.329916 | 224.441977 | 1.004979 | 0.497946% |
| Prepare/write | 0.001206 | 0.001233 | 1.022192 | 2.219177% |
| Wait for frame | 135.140426 | 98.650156 | 0.729983 | -27.001742% |
| Validate/retain/commit | 0.019878 | 0.019097 | 0.960730 | -3.926987% |
| Close and retirement | 30.064117 | 30.114751 | 1.001684 | 0.168420% |
| Postcheck and publication | 0.551579 | 0.614666 | 1.114375 | 11.437491% |
| Total | 480.325641 | 445.273315 | 0.927024 | -7.297617% |

## Forward Groups

These subsets are not extra time to add to the parent total. All forwards = first use + warm.

| Forward interval | Bank V2 seconds | Census V3 seconds | Candidate / Control | Change |
| --- | ---: | ---: | ---: | ---: |
| First use (0-1) | 25.999121 | 26.194714 | 1.007523 | 0.752305% |
| Warm (2-39) | 109.162388 | 72.475772 | 0.663926 | -33.607378% |
| All forwards | 135.161509 | 98.670486 | 0.730019 | -26.998088% |

The frame-wait category includes worker execution, pipe waiting and framing. Nested control timers and sidecar publication are excluded. Zero-denominator ratios are n/a. Both cases scope bank rearm and layers; the candidate additionally scopes the two warm-layer allocation censuses. Neither policy claims temporal equivalence to repeated full currentness. The census counters are a subset of the candidate layer counters, not extra time or work to add. No numerical acceptance or Full2303 feasibility is inferred.
