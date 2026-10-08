# Instrumented Readiness40 Host Attribution

One admitted diagnostic run; integer nanoseconds below are host elapsed time, not GPU time or speedup.

## Warm Serial Partition

| Component | Nanoseconds | Milliseconds |
| --- | ---: | ---: |
| Bank callbacks | 874840556 | 874.840556 |
| Other bank guarded body | 20999048 | 20.999048 |
| Layer callbacks | 20181996517 | 20181.996517 |
| Tail callbacks | 185663640 | 185.663640 |
| Other parent-forward time | 47308361656 | 47308.361656 |
| Complete warm parent forwards | 68571861417 | 68571.861417 |

Expanded disjoint attribution is B + Clayers + Ctail. Cbank is already inside B;
the displayed split is Cbank + (B - Cbank) + Clayers + Ctail + residual.
Residual is relative to each complete parent forward, never only the flush-to-frame-read span.

## Callback Categories

| Scope | Callback | Calls | Nanoseconds |
| --- | --- | ---: | ---: |
| bank | before | 27588 | 436276685 |
| bank | discover | 76 | 132129119 |
| bank | after | 27588 | 143504343 |
| bank | root_generation | 27550 | 162930409 |
| layers | before | 559642 | 8837488056 |
| layers | discover | 2736 | 4808508418 |
| layers | after | 559642 | 2880315013 |
| layers | root_generation | 610258 | 3655685030 |
| tail | before | 1748 | 28308005 |
| tail | discover | 76 | 133624963 |
| tail | after | 1748 | 9210569 |
| tail | root_generation | 2394 | 14520103 |

Callback categories are serial subtotals, not additions to the partition above.
Positions 0 and 1 have no callback measurements; blank CSV cells mean unmeasured, not zero.

## Parent Disjoint Spans

| Category | Nanoseconds |
| --- | ---: |
| Source preparation | 92227530910 |
| Spawn through setup | 225665259048 |
| Prepare/write | 8532793 |
| Wait for frame | 94936357100 |
| Validate/retain/commit | 24564517 |
| Close and retirement | 30240425696 |
| Postcheck and publication | 560338437 |
| Total (124 disjoint spans) | 443663008501 |

Only these parent categories sum to the total. The following forward groups are subsets:

| Forward subset | Forwards | Nanoseconds |
| --- | ---: | ---: |
| First use (unmeasured callbacks) | 2 | 26397592993 |
| Warm | 38 | 68571861417 |
| All forwards | 40 | 94969454410 |

Instrumentation costs and observer effects are included. This is not an independent numerical reference,
a matched comparison, a GPU overlap measurement, or Full2303 feasibility evidence.
