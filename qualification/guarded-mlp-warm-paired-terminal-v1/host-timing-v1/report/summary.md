# Paired-Terminal Segment Host Diagnostic

One serial control/candidate pair using the same qualified parent and worker ELFs. The four forwards within each run are correlated, not independent repetitions.

| Scope | Segments / mode | Control sum (ms) | Candidate sum (ms) | Candidate - control (ms) | Relative change (%) |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0 | 36 | 6325.010 | 6358.597 | 33.587 | 0.531 |
| 1 | 36 | 6328.507 | 6343.730 | 15.223 | 0.241 |
| 2 | 36 | 5853.234 | 4353.720 | -1499.514 | -25.619 |
| 3 | 36 | 5822.129 | 4346.377 | -1475.752 | -25.347 |
| first-use | 72 | 12653.516 | 12702.327 | 48.811 | 0.386 |
| warm | 72 | 11675.363 | 8700.097 | -2975.266 | -25.483 |

Positions 0/1 use the original first-use terminal path in both cases. At positions 2/3 the candidate uses the paired-terminal path only after genuine retired-arena reuse; the control keeps the original terminal path. All other currentness and hidden-read policies remain unchanged.

The original `segment_host_ns` timer spans coordinator preflight, owner consumption, ring reservation, publication, polling, retirement and terminal validation. Earlier setup/allocation, prefix work, later hidden readback and the complete forward are outside this field. No inclusive counters are added or subtracted.

Integer nanoseconds are preserved in `layers.csv` and `summary.csv`. [Per-layer host-wall plot](segments.svg). Display decimals use round-to-nearest, ties-to-even; a zero control denominator is reported as unavailable.

Both original receipts and every retained body must pass the existing pure retention verifier first. Four-payload byte equality and exact token-history equality are prerequisites, not independent model accuracy. This is one observed pair, not a controlled repeated benchmark or proof of causality. No GPU latency, tokens/second, end-to-end speedup, numerical acceptance or production authority is claimed.
