# Guarded MLP Segment: CPU Qualification

Engineering checkpoint for [issue #42](https://github.com/harsh-nod/ferric/issues/42),
run on MI350 on 2026-10-05. This is CPU qualification of two new device-kernel
sources, not checked HSACO emission, GPU execution, model acceptance or a
performance result. All M0-M7, sustained 2,048/256 and 700 tokens/s gates remain open.

## Result

All 27 Rust tests passed with no failures or ignores. All ten bounded phases
exited naturally with status zero, their process groups were absent afterward,
and source, dependency, tool and configuration postchecks passed. Seven separate
harness regression tests also passed on MI350.

| Phase | Result |
| --- | --- |
| Pinned nightly identity | Pass |
| Format and format check | Pass |
| Offline dependency metadata and lock validation | Pass |
| Non-test host check, including typed Marker roster | Pass |
| Test build, exact inventory and empty ignored inventory | Pass |
| Library tests | 27 passed |
| Non-test host library build | Pass |
| Temporary-directory inventory fixtures | 7 passed |

The [complete receipt](retained/evidence/complete.json), raw commands and logs,
formatted sources, dependency inventories and [summary](result.json) are retained.
The kernel library tests took 0.02 seconds; the 74.19-second controller duration
includes compilation and validation and is not a kernel-performance measurement.

## Kernel Contracts

The [device crate](../../device/qwen3-tp-guarded-mlp-segment-kernels-v1/README.md)
adds a validator that checks all 548 MLP state words and publishes a four-word
generation guard. The guarded TP2 projection-residual kernel requires both rank
guards before touching payloads. Invalid, pending or stale guards leave the
payload untouched. The existing staged FP32/BF16 residual arithmetic is preserved.

Twelve arithmetic tests cover rounding, subnormals, overflow and rank order.
Fifteen guard tests cover every state-word mutation against an independent
range-based oracle, owner values 1 through 64, all-word load coverage, publication
order, both generation halves and no payload access for either rejected guard.
These are host tests of the shared kernel macros, not a hardware memory-order proof.

## Reproduction Boundary

The canonical crate pins fe2o3 `097b4f796a283f554339cc0c0ef5c2c8c3858d2a`.
Qualification changes only the two dependency declarations to local paths into
that exact source snapshot. Original and local manifests are retained; lock
pruning is checked against actual Cargo metadata without changing retained
package versions, sources or checksums. All builds and tests ran on MI350.

Plain Cargo requires a typed-kernel crate binding. This CPU harness supplies
the input-manifest digest as an explicitly synthetic, host-only fixture, and
compiles the non-test Marker roster. It grants no compiler authority. Managed
device lowering must derive its own binding and qualify its actual outputs.

Limits remained two CPUs, two Cargo jobs, nice 10, 12 GiB address space, a 6 GiB
scratch cap, 40/38 GiB initial/live free-space floors and bounded process cleanup.
No GPU was opened. See the retained controller for the complete limits.

## Retained Failures

The [first attempt](attempt-v1/evidence/failed.json) stopped at host check because
the harness omitted the typed-kernel crate binding. No Rust tests ran.
The [second attempt](attempt-v2/evidence/failed.json) passed host check, then its
scratch inventory raced with Cargo removing a temporary directory. The harness
terminated and reaped its owned build processes; this is not a successful build.

The final controller tolerates only vanished descendants of its owned scratch
trees. Permission errors, other I/O errors, root/outside paths and file-count
limits remain failures, covered by the seven retained regression fixtures.
Neither correction changed the six canonical source bodies or relaxed limits.

## Next Gates

Qualify checked gfx950 lowering and actual ABI/ISA, then execute valid and invalid
guard controls on the GPU. Reusable arena lifetime, signal reset, full-worker
comparison and independent numerical validation remain separate requirements.
This crate is not yet connected to Ferric's inference path.
