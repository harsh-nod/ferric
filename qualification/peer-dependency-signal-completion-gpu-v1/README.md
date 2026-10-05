# Peer Signal Completion: Native GPU Qualification

This checkpoint supports [issue #42](https://github.com/harsh-nod/ferric/issues/42).
The finite two-rank copy graph completed on MI350 with correct data, intact
guards and healthy queue-first Close. This qualifies the narrow V3 peer-dependency
probe, not a full-model megakernel, reusable arena, numerical model acceptance or
throughput result. All M0-M7, sustained 2,048/256 and 700 tokens/s gates remain open.

## Actual MI350 Run

Runtime [097b4f796](https://github.com/harsh-nod/fe2o3/commit/097b4f796a283f554339cc0c0ef5c2c8c3858d2a)
was selected from the [1,064-pass CPU checkpoint](../peer-dependency-signal-completion-cpu-v1/README.md).
All 45 controller tests passed on MI350 before the single native attempt.
The probe ran over `ssh mi350` on `smci350-rck-g03-b19-03`, selecting physical
GPUs 1 and 2 by exact KFD unique IDs and independently joined inventory/sysfs records.

Each rank runs the same finite packet sequence:

```text
P0 -> wait(both P0) -> C0 -> wait(both C0)
   -> P1 -> wait(both P1) -> C1 -> wait(both C1)
```

P copies the rank's seed into its intermediate; C copies the peer's intermediate
into its own output. The two iterations use different seeds and output buffers,
with one reused intermediate per rank. A delayed-publication witness confirms
rank 0's first producer completes while its consumer remains blocked until rank 1
is published. All sixteen completion slots are distinct and initialized once.

| Observation | Actual result |
| --- | --- |
| GPU attempts / retries | 1 / 0 |
| Kernel packets / barrier packets | 8 / 8 |
| Completed signals | 16 |
| Actual completion-sample (write, read) | (8, 3) on both queues |
| Delayed-publication witness | Passed |
| Verified guarded buffers | All 10, each 8,320 bytes |
| Source preservation and expected copy patterns | Passed |
| Buffer boundary guards | Passed |
| Completion-slot resets between device iterations | None |
| Healthy queue-first Close | Passed |
| Native aggregate host interval | 94,082,928 ns |
| Native process wall interval | 1.215536164 s |
| Owned subprocesses | All 7 naturally exited 0, reaped, groups absent |
| Selected-pair pre/post audits and input postchecks | Passed |

The reported interval includes host preparation and synchronization; it is not
GPU duration or model throughput. Queue counters are sequential host observations.
The actual read value remains 3, not an invented 8: completion permits checked
payload readback, while only actual reads grant ring capacity. The cause of the
particular read value is not established.

## Shared-Host Audit

The [first controller refused before native execution](../peer-dependency-signal-completion-preflight-v1/README.md)
because unrelated work occupied GPU 0. This successor explicitly requires only
GPUs 1 and 2 idle, retains all eight inventory/process rows, and binds their
AMD-SMI index, BDF, UUID and node to KFD unique IDs before and after execution.
It never manages foreign processes. In this later run, the unrelated work had
finished independently and all eight recorded process rows were idle.

The original 900-second outer and 600-second native caps remain unchanged.
Extra identity audits have an explicit 260-second post-native reserve. There is
no automatic retry, reset, admission fallback or permission escalation.

## Evidence and Next Step

[`result.json`](result.json) binds all 51 retained files. The complete receipt is
[`complete.json`](retained/evidence/complete.json), SHA256
`23bd08205575aa5999c6447399227c88a02db1e54bdf3eebd694911b62a616ef`.
It retains all native buffer hashes, the request, CPU/ELF/source/tool custody,
raw subprocess logs and both device identity audits.

This is one finite host call containing two device iterations. It does not
exercise resetting signals across repeated calls. Next work is a fixed reusable
arena, full MLP-state validators and guarded final residual kernels, followed by
a matched full-worker comparison. The old V1/V2 strict-retirement failures remain
separate evidence and are not retroactively marked passed.
