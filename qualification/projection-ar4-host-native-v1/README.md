# Projection AR4 Host-Cost Observation

The opt-in observer ran successfully on MI350 through `ssh mi350` on
2026-10-05 UTC. Four own-output autoregressive forwards completed in one
attempt without retries. All 152 tensor slices, 576 terminal states,
control chains and Close/EOF checks passed structural validation. All seven
owned process leaves exited naturally and were reaped; all six surrounding
process/topology audits passed. This is an engineering observation, not
independent full-model numerical acceptance or a throughput benchmark.

The [qualified observer executables](../projection-ar4-host-observation-v1/README.md)
and [fresh runtime audits and assembled request](../projection-ar4-host-runtime-v1/README.md)
were used unchanged. Kernel images, weights, device IDs, request deadlines
and the full-currentness policy match the preceding plain AR4 observation.
Each next input is the preceding output, not a forced reference token:
`9112 -> 67 -> 25 -> 576 -> 2701`.

All four 606,976-byte tensor payloads are byte-identical to the preceding
[plain same-image AR4 run](../rope-indexed-ar4-native-v1/README.md), covering
all 152 captured slices. This is native repeatability with observation
enabled, not agreement with an independent numerical reference.

## Measured Host Costs

The worker records seven snapshots and six checked deltas. Forward wall
time brackets `Owner::run`; it includes host checks, dispatch, waiting,
readback and capture work. It is not GPU execution time. Per-rank columns
below sum the two ranks' counters within the corresponding snapshot interval.

| Position | Forward Wall (s) | Full Checks | Full-Check Scope (s) | Dispatch Prepare (s) | Dispatch Publish (s) | Dispatch Wait (s) | Read Scope (s) | Serialization (ms) |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 10.592764 | 5,524 | 9.386108 | 0.521868 | 0.499856 | 2.891782 | 0.284529 | 2.628702 |
| 1 | 10.599229 | 5,524 | 9.394482 | 0.521897 | 0.500562 | 2.888136 | 0.285185 | 2.490431 |
| 2 | 12.531191 | 6,676 | 11.324401 | 0.521497 | 0.499555 | 2.887729 | 0.284929 | 2.485421 |
| 3 | 12.582495 | 6,676 | 11.378797 | 0.522848 | 0.502215 | 2.891788 | 0.286614 | 2.513232 |

These timers are **inclusive and nested**. Do not add the columns, subtract
them to infer GPU time, or treat dispatch wait as device duration. For
example, dispatch preparation, waiting and reads can themselves perform
full-currentness checks. Serialization occurs after the forward snapshot;
its time is included in the following snapshot interval, not that forward's
wall timer. The raw integer nanosecond values and counter names are in
[host-observation.json](capture/host-observation.json).

The individual full-currentness calls occupy 88.6087%, 88.6336%, 90.3697%
and 90.4336% of the respective forward wall times. Each ratio is
`100 * sum(rank.full_currentness_ns) / forward_host_ns`. These individual
calls execute sequentially on the worker thread, so their rank sum does not
double-count simultaneous calls. This is host wall time within those calls,
not CPU utilization or a prediction of achievable speedup. Shared publication
checks have their own counter and are not included in that numerator.

Every forward submitted 148 rank-zero and 145 rank-one dispatches. The
existing publication path already used 288 shared publication checks per
forward, taking about 0.500 seconds in its separate inclusive scope.
`group_full_checks` stayed zero: the optional shared group-fence policy was
off. Kernel-admission caching, operational currentness and raw timestamps
also stayed off. Zero `commands` counters do not mean zero host work; those
counters belong to the separate wire-command loop, not these direct peer APIs.

| Other Host Scope | Seconds | Boundary |
| --- | ---: | --- |
| Observer-enabled setup | 168.475017 | Fresh observer through sealed setup; excludes earlier model staging and group open |
| Close | 27.149631 | Worker-owned close operation, after the last counter snapshot |
| Owned parent process | 385.664897 | Includes setup, transfers, decode, control and shutdown |
| Supervisor | 394.051782 | Includes the parent and surrounding audits |

These scopes are not a partition of total time. Setup includes 76,076
per-rank full checks and 19,783,930,382 uploaded bytes. The final snapshot is
before Close, so the close timer must not be presented as close counter
deltas. This run has no calibrated device clock or cross-device alignment.

## Next Optimization

The measured full-check costs justify testing the runtime's existing
`configure_performance_v2(false, false, true)` option in a separately selected
and qualified observer route. It shares a fresh topology observation within
each group fence while retaining each rank's before/after device checks and
queue validation. It does not cache a topology snapshot across operations.
Publication already shares a fresh observation, so that is not a new benefit.

Source inspection predicts four-to-one topology discoveries per TP2 group
fence and ten-to-four per group read/write. Those are static call counts,
not measured speedups. No optimized native run or paired ablation is claimed
here. The old observer continues requiring the original policy; it must not
silently accept the new configuration.

## Evidence And Limits

The native completion is 1,156,839-byte SHA-256
`8341d1012f8d57512e7895a1b809ac9298bf2142943b89545c729c5d09af6393`.
The host sidecar is 33,626-byte SHA-256
`14a492cf4baabcab8f3333859ee288df0a24bb4bf7499d5375a188d26512d953`.

[run-observation.json](run-observation.json) records the primary SSH command
and terminal result. `capture/` preserves the original completion, sidecar,
validated counter summary, native JSON frames, process leaves and audits.
All 59 retained case files were checked locally: the completion against the
terminal receipt, and the other 58 bodies against its pins. The nine binary
control/payload/stderr files remain outside Git; their hashes remain in the
published receipts. No model weights or executable bodies are published.

The earlier independent numerical comparison still has unresolved tensor
differences. This observation does not replace that comparison. Four forwards
from position zero do not execute the 2,048-token prompt even though its input
file contains that many tokens. Sustained BF16 2,048/256 decoding, the 700
tokens/s target and all issue #42 M0-M7 acceptance milestones remain open.
