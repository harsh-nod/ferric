# Host-Only Runtime Diagnostics

One retained TF4 run per worker. Inclusive nested host timings, not GPU time or a qualified speedup.

| Forward | CPU475 host ms | CPU522 host ms | Group checks old/new | Removed | Publication old/new |
|---:|---:|---:|---:|---:|---:|
| 0 | 7220.002 | 6280.256 | 2196/1620 | 576 | 288/288 |
| 1 | 7244.989 | 6342.408 | 2196/1620 | 576 | 288/288 |
| 2 | 7816.436 | 6760.717 | 2484/1908 | 576 | 288/288 |
| 3 | 7748.952 | 6829.020 | 2484/1908 | 576 | 288/288 |

Expected removal is 576 group checks per forward; it is checked against actual counters.
Nested timing scopes must not be summed. No framework-reference, model-acceptance or throughput claim.
