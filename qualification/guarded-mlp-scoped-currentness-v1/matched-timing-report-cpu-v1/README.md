# Scoped Timing Report CPU Qualification

All seven synthetic report tests passed on `mi350` with the exact retained
report and controller. The child exited naturally, was reaped, and left no
process group. Source and tool postchecks passed. GPU and model execution
were disabled for this qualification.

The tests check integer-nanosecond arithmetic, decimal rounding, disjoint
timing categories, timeline consistency, rejection of unsupported claims,
zero denominators, and faithful CSV, Markdown and SVG rendering.

The archive contains 16 originals: six sources, seven raw evidence files,
the original successful terminal, collector and manifest. `retention.json`
is the separate local retention record. This README is explanatory metadata.

| Artifact | SHA-256 |
| --- | --- |
| Original terminal | `b5ce2413513f977bd3c29e1a72e0cdd37998eb9a97aa2ef60c2101ddfe293b49` |
| Original archive | `702e3cb7251c130c2681214d93c41c150ac74b5d36d22722976e6b96107bdca8` |
| Tested report | `1bc8cf810a70add1f485bad831e005c30af69739856ab72f65a265ae416f8226` |
| Tested controller | `5f1ff87faeedd7eade3869c5c71219a1d2d2160e0454f0c80a6f3e571133d683` |
| Native collector bound by the report | `951b44ceac7e7207b26b3cbb87a2752decf9b614541759395efbb0c91bbdeb9a` |

The tests do not invoke the native collector or admit a native pair. Actual
rendering separately requires the original authenticated Default/scoped
capsule. Parent-wall timing is not GPU duration, overlap, or tokens/second.
This result does not close numerical acceptance, Full2303, or the 700 tok/s
target.
