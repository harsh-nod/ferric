# Shared-Full Host Observation

Host-only diagnostic. Wall categories are disjoint snapshot intervals; counters below are inclusive and may nest.
No GPU duration, overlap, throughput, numerical acceptance or performance claim follows.

| Forward | Prefix (s) | Paired (s) | Hidden Read (s) | Other (s) | Bracket (s) | Owner Run (s) |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 1.522309693 | 3.412278854 | 0.510518278 | 0.152886985 | 5.597993810 | 5.597992950 |
| 1 | 1.531516028 | 3.428780926 | 0.516901581 | 0.148459403 | 5.625657938 | 5.625656218 |
| 2 | 1.536764850 | 3.444071616 | 0.515773141 | 0.774321370 | 6.270930977 | 6.270930047 |
| 3 | 1.524605324 | 3.427972136 | 0.514185759 | 0.777886702 | 6.244649921 | 6.244649061 |

## Inclusive Rank Counters

Do not sum these nested scopes into wall time. Generic paired-dispatch timers are incomplete.

| Forward | Rank | Full Checks | Full Check (s) | Admissions | Admission (s) | Commands | Command (s) |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 0 | 496 | 0.844704914 | 184 | 0.015411296 | 0 | 0.000000000 |
| 0 | 1 | 476 | 0.809726847 | 181 | 0.012324499 | 0 | 0.000000000 |
| 1 | 0 | 496 | 0.852665642 | 184 | 0.015433279 | 0 | 0.000000000 |
| 1 | 1 | 476 | 0.818538310 | 181 | 0.012369477 | 0 | 0.000000000 |
| 2 | 0 | 496 | 0.855369324 | 184 | 0.015533112 | 0 | 0.000000000 |
| 2 | 1 | 476 | 0.820816146 | 181 | 0.012427342 | 0 | 0.000000000 |
| 3 | 0 | 496 | 0.849076822 | 184 | 0.015464461 | 0 | 0.000000000 |
| 3 | 1 | 476 | 0.814764597 | 181 | 0.012370986 | 0 | 0.000000000 |

## Inclusive Shared Counters

| Forward | Group Checks | Group Check (s) | Publication Checks | Publication Check (s) |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 1438 | 2.481579231 | 683 | 1.176789038 |
| 1 | 1438 | 2.508196693 | 672 | 1.172186241 |
| 2 | 1799 | 3.134790524 | 676 | 1.181041706 |
| 3 | 1799 | 3.114386212 | 682 | 1.183909100 |

Nonforward snapshot intervals: 92.278931943 s.
All snapshot intervals: 116.018164589 s.
Close host wall, outside those intervals: 15.171818334 s.
Other includes layer-entry/forward-boundary host work. Snapshot wall also includes waits and process gaps.
