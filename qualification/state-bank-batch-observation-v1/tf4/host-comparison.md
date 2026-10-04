# Host-Only Runtime Diagnostics

One retained TF4 run per worker. Inclusive nested host timings, not GPU time or a qualified speedup.

| Forward | CPU522 host ms | State-bank host ms | Group checks old/new | Removed | Publication old/new |
|---:|---:|---:|---:|---:|---:|
| 0 | 6280.256 | 5258.709 | 1620/1048 | 572 | 288/288 |
| 1 | 6342.408 | 5267.448 | 1620/1048 | 572 | 288/288 |
| 2 | 6760.717 | 5754.206 | 1908/1336 | 572 | 288/288 |
| 3 | 6829.020 | 5760.197 | 1908/1336 | 572 | 288/288 |

Expected removal is 572 group checks per forward; publications must remain exactly 288 per forward.
Nested timing scopes must not be summed. No framework-reference, model-acceptance or throughput claim.
