# Native Peer Dependency Attempt

**Not qualified.** The first native two-GPU test hit its aggregate deadline.
This checkpoint preserves the unsuccessful run; it does not establish peer
output correctness, healthy queue retirement or a performance result.

## Actual MI350 Observation

The root ran the CPU-qualified example from runtime commit
[`998021225`](https://github.com/harsh-nod/fe2o3/commit/998021225db853720b71241ca303d2d43045a99e)
through `ssh mi350`, on `smci350-rck-g03-b19-03`, boot
`2dbbbc5a-d8b7-46ac-bd1f-236e5847e24a`.
[CPU qualification](../peer-dependency-native-cpu-v1/README.md) remains a separate
passing checkpoint.

| Observation | Actual result |
| --- | --- |
| Native invocations | 1; no retry in the controller |
| Native process exit | Natural exit 1; no external timeout or forced cleanup |
| Native process interval | 60.641702701 seconds |
| Configured dispatch deadline | 60 seconds, including host checks |
| Published ranks at failure | Both |
| Last acquired completion values | All seven slots on each rank were zero |
| Rank 0 queue `(write, read)` | `(7, 3)` |
| Rank 1 queue `(write, read)` | `(7, 3)` |
| Before/after GPU process audits | All eight GPUs idle |
| Post-run input/tool/library checks | No errors |
| Data/guard readback and healthy Close | Not reached; not qualified |

The runtime retained its requirement for all fourteen completions **and** both
actual read frontiers to reach seven. It did not replace that requirement with
completion signals alone. The error quarantined the group and required process
teardown; no signal reset, queue reuse or success output followed.

The displayed process interval is not GPU execution time, throughput, or a
valid benchmark. It includes the failed wait and other host work. In particular,
zero completion values are not presented here as proof of correct copied data.
Only the last acquired snapshot was retained; it has no timestamp or transition
history and does not establish how long the observed state persisted.

## Evidence

[`result.json`](result.json) hashes all 50 retained files. The actual native
failure receipt is [`failed.json`](retained/v3-native/evidence/failed.json), SHA256
`35ab6f72032a52680bc20ebf1dd431572e55f7b744ddc4feece666265939c9b4`.
The exact error is in [`native.stderr`](retained/v3-native/evidence/native.stderr).
Commands, process ownership/reap records, ELF/library audit, request, before/after
GPU audits and the runner are retained beside it. No executable is committed.

`retained/v2-preflight/` preserves the earlier controller refusal before any
native invocation. Its library parser failed to recognize the absolute ELF
interpreter as satisfying its matching `DT_NEEDED` entry. V3 requires exact
canonical interpreter-pin equality before admitting that one dependency; other
unresolved dependencies still fail. All twelve parser regression tests passed
on MI350, recorded in `retained/v3-native/library-parser-tests.json`.

## Investigation

The cause of the `(7, 3)` frontiers is unresolved. ROCr's pinned 7.2.4 source
notes that some ASIC EOP paths do not update the read index after each packet,
and its queue interceptor does not assume a protocol-defined latest update
time. These are reasons to investigate cursor publication, not proof that this
particular run can retire safely. See
[AQL queue implementation](https://github.com/ROCm/ROCR-Runtime/blob/97f5574fe2fdc7bef44fb01545347912ee9f1779/runtime/hsa-runtime/core/runtime/amd_aql_queue.cpp#L852)
and [queue interceptor](https://github.com/ROCm/ROCR-Runtime/blob/97f5574fe2fdc7bef44fb01545347912ee9f1779/runtime/hsa-runtime/core/runtime/intercept_queue.cpp#L78).

The next diagnostic must preserve bounded execution, exclusive ownership,
completion checks and process-terminal failure. It must distinguish packet
consumption, signal visibility and queue-frontier publication before enabling
reuse in the Qwen worker. All issue #42 milestones, independent numerical
acceptance, sustained 2,048/256 decoding and 700 tokens/s remain open.
