# Tensor-Parallel Collective Performance V3

These are separate, opt-in Contracted engineering profiles, not protected M1
authority or new Verus proofs. Neither profile changes the baseline v2 image.

## Implemented Profiles

| Profile | Worlds | Arithmetic | Host transport | Extra dispatches per layer |
| --- | --- | --- | --- | --- |
| `host-staged-v1` | 1, 2, 8 | Original ascending-rank FP32 sum, residual once, BF16 once | Original reads and broadcasts | 0 |
| `host-staged-reuse-v3` | 1, 2, 8 | Same arithmetic/order and finite checks | Same reads and broadcasts; staging allocations retained | 0 |
| `device-tp1-v3` | 1 only | GPU FP32 `0 + partial`, residual once, BF16 once | No hidden or partial reads/uploads | 2 |

The host-reuse profile allocates rank partials, one transfer buffer, rounded
output, and broadcast bytes once, sized to the admitted storage capacity. All
72 reductions reuse their active prefixes. It does not reduce bytes transferred
or IPC round trips, and must not be labeled a device collective. There is still
a small embedding staging allocation once per batch.

The TP1 device profile uses two disjoint BF16 hidden allocations and swaps them
only after successful completion. GPU nonfinite checks trap; a failed or
uncertain completion poisons the outer execution and cannot publish a batch
completion token. All active slice lengths and the launch extent are exactly
`rows * 4096`, with 1 through 16 rows and 64 workitems per group. In the unpruned
baseline schedule this means 616 rather than 544 rank-zero dispatches per
forward, while eliminating the embedding host round trip and all 72 residual
partial reads/hidden writes. The packet-budget guard accounts for those extra
dispatches. Measuring total latency is necessary: fewer transfers do not
automatically outweigh the additional dispatch overhead.

Configuration is allowed only before execution. TP2/TP8 device requests fail
before allocating scratch or dispatching anything; no fallback is selected
silently. The separate v3 kernel roster must be admitted by the transport.

## Device TP2/TP8 Boundary

The inspected fe2o3 baseline is
`3546d54d2c4a913f5d079701aed557d0a378bba8`. Its gfx950 backend explicitly rejects
XGMI publication authority in `crates/fe2o3-kfd/src/memory_linux.rs`. The
engineering `Context` in `engineering_gfx950.rs` privately owns one device,
its process-bound VM, queue, buffer map, and backing allocations. Ferric has
one such disposable child per rank. A numeric buffer ID in another child is
not a pointer or an imported allocation capability.

The low-level memory backend has a multi-GPU map wire operation, but that is
not a public, lifetime-checked gfx950 peer-allocation API. Reusing the gfx942
checked topology route, exporting a raw virtual address, or simply enabling
that ioctl is not a valid implementation.

### Preferred Single-Process Multi-Device Owner

A new, explicit fe2o3 engineering group owner could hold every checked device,
queue, and allocation in one disposable process. This removes cross-process
export/import and allows queue completion and allocation lifetime to be
governed by one owner. It does not remove the need to qualify gfx950 peer
routes and mapping/currentness checks. The existing `Context` and target-bound
handles are not sufficient public APIs for Ferric to assemble this today.

The smallest useful group protocol would include:

1. Admit a complete unique physical-device roster and fresh group incarnation;
   retain every device/VM/queue owner. Reject duplicate or foreign devices.
2. Allocate an immutable group-owned reduction workspace with owner rank,
   exact byte extent, element type, and a monotonic generation. Establish
   checked peer-read mappings to a closed peer roster and retain exact partial
   map progress. No pointers or native handles escape the worker.
3. After every source projection completes, issue one collective epoch tied
   to group, layer, operation, rows, and source allocation generations. No
   source buffer may be written, unmapped, or freed until all readers finish.
4. Each destination rank reads source partials in logical rank order, checks
   every FP32 intermediate, adds its resident residual once, and rounds once
   to BF16. This avoids changing numerical order or requiring a reduction
   tree. Only tiny completion records cross the controller boundary.
5. Publish the collective result only after every destination completion.
   On any uncertain map, queue exception, stale process/device incarnation,
   completion timeout, or cleanup result, quarantine the complete group until
   disposable-process teardown. No reuse after uncertainty.
6. Drain all queues, retire reader leases, unmap each peer, unmap the owner,
   then free the allocation. Verify teardown rather than inferring it from a
   successfully written close command.

### Cross-Process Alternative

Keeping one child per rank needs a more complicated export/import protocol.
An export must be an unforgeable group/session-scoped owned capability, not a
serialized GPU address. Imports must bind exporter incarnation, allocation
generation and extent, target device/VM incarnation, allowed access, and the
complete reader lease roster. A two-phase retirement must revoke new uses,
drain all readers, remove every mapping, acknowledge removal to the exporter,
and only then free backing. A crashed reader/exporter quarantines the group;
reusing an old ID in a restarted child cannot revalidate an old capability.

Neither design requires a general symmetric-memory API or computation /
communication overlap as a prerequisite. Both require new generic runtime
work in fe2o3; the ordered reduction kernels and inference scheduling remain
in Ferric.

## Qualification Before Performance Claims

Required negative cases include duplicate devices, wrong group/generation,
partial mapping progress, shape mismatch, source reuse while readers are
active, stale leases, reordered collectives, one-rank queue failure,
nonfinite/overflow intermediates, and failed teardown. Hardware qualification
must separately cover TP2 and TP8 on the exact gfx950 image and worker.

The current host recording tests establish driver order, active extents,
profile gating, transport elimination/reuse, identical synthetic results, and
poison behavior. Scalar kernel tests exercise the actual arithmetic macro;
they do not replace emitted-image or GPU numerical validation. Report TTFT,
TPOT, throughput, copies/bytes, dispatches, and exact source/artifact identities
for each ablation independently, using repeated unprofiled measurements.
