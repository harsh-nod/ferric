# Bounded Ranked CFG Capacity

The [measured state-guard rejection](../guarded-mlp-ranked-cfg-lowering-v1/README.md)
contains 1,121 declared blocks. This isolated compiler experiment permits up
to 2,048 blocks while preserving independent limits on other resources.
It is a structural admission change, not a diagnostics-only patch. It has
not been integrated into Ferric's production execution path.

| Resource | Previous | Experiment |
| --- | ---: | ---: |
| Blocks | 1,024 | 2,048 |
| Edges | 2,048 | 2,048 |
| Facts | 1,024 | 1,024 |
| Projector graph work | 3,145,728 | 3,145,728 |
| Bounds work | 8,388,608 | 8,388,608 |
| Bounds storage items | 131,072 | 131,072 |
| Operations | 65,536 | 65,536 |
| Findings | 4,096 | 4,096 |

Previously coupled limits are made explicit at their old values. Atomic,
alias, source-origin and guard-protocol checks remain unchanged. Dense graphs
can still fail the existing work/storage checks below the new block limit.
The kernel sources, ABI and runtime packet graph are unchanged.

## First Attempt

The MI350 run **failed qualification** after 15 phases in 230.013570 seconds.
All leaves exited naturally and were reaped with absent process groups.
The compiler test process returned 101; the other fourteen returned zero.
Source/dependency postchecks were clean, with no timeout or forced cleanup.

| Executed Suite | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Pliron library, accepted receipt scope | 1,507 | 0 | 1 |
| Compiler library, raw failed test output | 1,247 | 1 | 24 |

The receipt records only the completed Pliron scope. The compiler's partial
passing count is from its [raw test output](attempt-v1/evidence/compiler-tests.stdout),
not an accepted compiler result. Focused repeats and extraction controls were
not reached. Compiled products from this failed generation cannot authorize
the next loader/lowering attempt.

The sole failing test was
`raw_parallel_switch_edges_are_bounded_separately_from_unique_successors`.
Its positive fixture used `MAX_RANKED_BOUNDS_BLOCKS + 1` switch arms, plus a
fallback edge. After separating the limits, this exceeded the unchanged
2,048-edge cap, and the compiler correctly rejected it. The corrected retry
uses `MAX_RANKED_BOUNDS_EDGES - 1` arms plus the fallback for exactly 2,048
edges. The existing negative still has 2,049 edges. No production edge gate
is changed to make this test pass. The retry is not yet qualified.

## Evidence

[attempt-v1](attempt-v1) retains the executed controller, helper, input,
seven source postimages, baseline lineage, failed receipt and all 79 raw
records. Its retention manifest pins 104 bodies; all 105 archive members
and 33,142,457 body bytes were verified after transfer. Receipt SHA-256:
`7f075680fbae052dccf01daac563c615079cc148355f0c91e7e1290521dc3dc9`.
The 5,477,306-byte archive SHA-256 is
`766413016beb0e9c1b46be2c0c40a6baf699da87bcaeed7e16e759bc52afc1d1`.

Full compiler qualification, fresh binary-loader inspection, guarded gfx950
HSACO/GPU execution, independent model numerics and sustained BF16 2,048/256
decode remain open. No issue #42 milestone or 700 tokens/s result is claimed.
