# Conservative Host Observation

Host-only diagnostic. Wall categories are disjoint snapshot intervals; counters below are inclusive and may nest.
No GPU duration, overlap, throughput, numerical acceptance or performance claim follows.

| Forward | Prefix (s) | Paired (s) | Hidden Read (s) | Other (s) | Bracket (s) | Owner Run (s) |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 3.323525422 | 7.924345342 | 1.232530213 | 0.327920133 | 12.808321110 | 12.808319930 |
| 1 | 3.344133863 | 7.960072926 | 1.239440596 | 0.325958392 | 12.869605777 | 12.869603967 |
| 2 | 3.334090914 | 7.935428804 | 1.238999146 | 2.744235101 | 15.252753965 | 15.252752815 |
| 3 | 3.364836568 | 8.003815631 | 1.248487593 | 2.766447924 | 15.383587716 | 15.383586816 |

## Inclusive Rank Counters

Do not sum these nested scopes into wall time. Generic paired-dispatch timers are incomplete.

| Forward | Rank | Full Checks | Full Check (s) | Admissions | Admission (s) | Commands | Command (s) |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 0 | 3372 | 5.695581265 | 184 | 0.015791041 | 0 | 0.000000000 |
| 0 | 1 | 3352 | 5.658154876 | 181 | 0.012685104 | 0 | 0.000000000 |
| 1 | 0 | 3372 | 5.721371956 | 184 | 0.015794090 | 0 | 0.000000000 |
| 1 | 1 | 3352 | 5.684609768 | 181 | 0.012776255 | 0 | 0.000000000 |
| 2 | 0 | 4094 | 6.915604261 | 184 | 0.015793639 | 0 | 0.000000000 |
| 2 | 1 | 4074 | 6.880775024 | 181 | 0.012670371 | 0 | 0.000000000 |
| 3 | 0 | 4094 | 6.979613671 | 184 | 0.015746079 | 0 | 0.000000000 |
| 3 | 1 | 4074 | 6.945816714 | 181 | 0.012696401 | 0 | 0.000000000 |

## Inclusive Shared Counters

| Forward | Group Checks | Group Check (s) | Publication Checks | Publication Check (s) |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 0 | 0.000000000 | 680 | 1.170396844 |
| 1 | 0 | 0.000000000 | 682 | 1.177307386 |
| 2 | 0 | 0.000000000 | 680 | 1.168979421 |
| 3 | 0 | 0.000000000 | 676 | 1.177458233 |

Nonforward snapshot intervals: 171.271890840 s.
All snapshot intervals: 227.586159408 s.
Close host wall, outside those intervals: 32.619351730 s.
Other includes layer-entry/forward-boundary host work. Snapshot wall also includes waits and process gaps.
