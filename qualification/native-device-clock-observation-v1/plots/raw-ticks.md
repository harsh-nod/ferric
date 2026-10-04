# Raw Clock And Completion Tick Report

One four-forward teacher-forced observation; not the 2,048/256 workload.
Raw tick deltas are not nanoseconds, calibrated durations, cross-device coordinates or a throughput result.

| Rank | Stage | Count | Min ticks | Median ticks | Max ticks | Zero deltas |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 0 | embedding | 4 | 944 | 956 | 1276 | 0 |
| 0 | prefix | 144 | 1212472 | 1285928 | 1346876 | 0 |
| 0 | post-attention-residual | 144 | 1004 | 1080 | 1296 | 0 |
| 0 | mlp | 144 | 2173720 | 2272832 | 2382228 | 0 |
| 0 | post-mlp-residual | 144 | 1024 | 1068 | 1224 | 0 |
| 0 | final-norm | 4 | 71560 | 74352 | 74492 | 0 |
| 0 | head | 4 | 86656 | 86800 | 87276 | 0 |
| 0 | argmax | 4 | 2066568 | 2159574 | 2218516 | 0 |
| 1 | copy | 4 | 924 | 936 | 1124 | 0 |
| 1 | prefix | 144 | 1201736 | 1281008 | 1341016 | 0 |
| 1 | post-attention-residual | 144 | 980 | 1036 | 1316 | 0 |
| 1 | mlp | 144 | 2154788 | 2264908 | 2345444 | 0 |
| 1 | post-mlp-residual | 144 | 956 | 1028 | 1280 | 0 |

![Per-device ranges and medians](raw-tick-ranges.svg)

![Layer and position raw ticks](raw-tick-heatmaps.svg)

## Separate Host Intervals

These inclusive host intervals are recorded in nanoseconds and joined to the native Controls.
They are not GPU durations and must not be subtracted from the tick values.

| Rank | Stage | Min host ns | Median host ns | Max host ns |
| --- | --- | ---: | ---: | ---: |
| 0 | embedding | 8508076 | 8564455.5 | 8604575 |
| 1 | copy | 8488145 | 8544300.5 | 8622075 |
| 0 | prefix | 15538470 | 16280405 | 16830301 |
| 1 | prefix | 15480490 | 16235485 | 16969441 |
| 0 | post-attention-residual | 6660944 | 6757019 | 7111064 |
| 1 | post-attention-residual | 4969193 | 5038618 | 5343894 |
| 0 | mlp | 25172485 | 26204851 | 27273657 |
| 1 | mlp | 24964866 | 26045676 | 27087227 |
| 0 | post-mlp-residual | 6663874 | 6754664.5 | 7335844 |
| 1 | post-mlp-residual | 4972523 | 5038308 | 5396904 |
| 0 | final-norm | 8499605 | 8516710.5 | 8544405 |
| 0 | head | 8460526 | 8486535 | 8540866 |
| 0 | argmax | 24108145 | 24988255.5 | 25580095 |

## Raw Clock Samples

These are sixteen actual KFD samples, taken sequentially by rank before and after each forward.
GPU, CPU and system counters are distinct raw values. The system frequency is not a GPU tick frequency.
Host brackets use one process-local Instant origin and include device currentness checks.
They do not establish simultaneous sampling or a GPU dispatch clock-domain relationship.

| Position | Rank | Boundary | Recorded rows | GPU counter | CPU counter | System counter | Host bracket ns |
| ---: | ---: | --- | ---: | ---: | ---: | ---: | --- |
| 0 | 0 | pre | 0 | 51739313920894 | 517214411168617 | 517218466136208 | 89776433862..89779843304 |
| 0 | 1 | pre | 0 | 51739314271521 | 517214414549029 | 517218469516720 | 89779843344..89783221236 |
| 0 | 0 | post | 293 | 51739844244156 | 517219714527944 | 517223769495845 | 95079848219..95083185631 |
| 0 | 1 | post | 293 | 51739844590772 | 517219717868206 | 517223772836007 | 95083185671..95086517533 |
| 1 | 0 | pre | 293 | 51739845942064 | 517219731507465 | 517223786475066 | 95096777980..95100192002 |
| 1 | 1 | pre | 293 | 51739846294884 | 517219734909727 | 517223789877558 | 95100192042..95103592314 |
| 1 | 0 | post | 586 | 51740377868860 | 517225050903262 | 517229105870973 | 100416211637..100419577789 |
| 1 | 1 | post | 586 | 51740378218700 | 517225054275734 | 517229109243305 | 100419577829..100422933481 |
| 2 | 0 | pre | 586 | 51740379537720 | 517225067592273 | 517229122559864 | 100432880798..100436258220 |
| 2 | 1 | pre | 586 | 51740379886116 | 517225070950295 | 517229125918066 | 100436258250..100439622132 |
| 2 | 0 | post | 879 | 51740960389316 | 517230876247703 | 517234931215304 | 106241565208..106244907680 |
| 2 | 1 | post | 879 | 51740960737032 | 517230879598985 | 517234934566756 | 106244907730..106248250832 |
| 3 | 0 | pre | 879 | 51740962056928 | 517230892924244 | 517234947891825 | 106258238649..106261582061 |
| 3 | 1 | pre | 879 | 51740962403648 | 517230896265546 | 517234951233117 | 106261582091..106264912463 |
| 3 | 0 | post | 1172 | 51741539847436 | 517236670968025 | 517240725935766 | 112036277630..112039628702 |
| 3 | 1 | post | 1172 | 51741540195556 | 517236674323307 | 517240729290888 | 112039628742..112042981665 |

Rank 0: KFD GPU ID 39903, reported system-counter frequency 1000000000 Hz.

Rank 1: KFD GPU ID 22482, reported system-counter frequency 1000000000 Hz.

## Same-Device Counter Differences

Signed post-minus-pre integer differences only, without wrap correction or unit conversion.
A negative value is retained and flagged in JSON; it is not silently repaired or treated as a duration.
The host span includes both sampling calls and all intervening host/device work, not GPU compute alone.

| Position | Rank | GPU difference (raw) | CPU difference (raw) | System difference (raw) | Host bracket span ns |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 0 | 530323262 | 5303359327 | 5303359637 | 5306751769 |
| 0 | 1 | 530319251 | 5303319177 | 5303319287 | 5306674189 |
| 1 | 0 | 531926796 | 5319395797 | 5319395907 | 5322799809 |
| 1 | 1 | 531923816 | 5319366007 | 5319365747 | 5322741439 |
| 2 | 0 | 580851596 | 5808655430 | 5808655440 | 5812026882 |
| 2 | 1 | 580850916 | 5808648690 | 5808648690 | 5811992582 |
| 3 | 0 | 577790508 | 5778043781 | 5778043941 | 5781390053 |
| 3 | 1 | 577791908 | 5778057761 | 5778057771 | 5781399574 |
