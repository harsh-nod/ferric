# Device Diagnostic Runtime Audits

The CPU-qualified device-tick parent and worker were transported to `mi350`
and inspected successfully on `smci350-rck-g03-b19-03`. This checkpoint does
not execute either model binary or establish a GPU timing result.

| Check | Actual result |
| --- | --- |
| Parent identity | 13,691,232 bytes; SHA-256 `cfb38b6c06bf39bd0928e3066204753cd6b5998e9331c5aa77295aa246323a7d` |
| Worker identity | 4,872,432 bytes; SHA-256 `eda6a2450636d5fdc8ceca0f39b84769cdbcf47a74b73bb66310eb5cc8233d03` |
| ELF and dependency inspection | Two commands per binary; four natural zero exits and reaps, empty stderr |
| Shared libraries | All dependencies resolved; no RPATH/RUNPATH |
| Runtime-auditor policy tests | 7 passed on MI350; separate from native CPU qualification |
| Original GPU-controller policy tests | 44 passed on MI350; missed the admission integration defect below |
| Native/GPU attempts in this checkpoint | 0 |

The parent needs the recorded loader, libc, libgcc_s and libm; the worker does
not need libm. The audits bind the current boot and the original ordered GPU
IDs `16366993098680759275` and `10838076764495710945`. Binary hashes join the
separate [257-test parent](../native-device-parent-v1/README.md) and
[609-test worker](../native-device-routing-v1/README.md) qualifications.
These are distinct cohorts, not a combined test run.

## Retained Evidence

[result.json](result.json) inventories 65 byte-preserved files: both complete
audit trees, command/owner records, readelf/ldd output, before/after topology,
software records, controller sources and actual test evidence. The seven-test
auditor result is an actual [transcript](retained/device-runtime-audit-pure-v228-v1.log),
not a fabricated structured receipt. The historical controller has its own
[44-test receipt](retained/device-timing-gpu-pure-v228-v1/complete.json).

The original audit receipts deliberately retain `reviewed: false`: collection
is not review or execution authority. Library bytes were hashed on the host,
but are not copied into Git. Historical source-package READMEs retain their
author-time wording; the measured status is reported here and in the receipts.

## Admission Defect

The first real controller admission exited with `ModuleNotFoundError:
host_comparison`. Historical replay had restored its temporary module aliases
before a later legacy-validator import. This occurred before creating the GPU
case directory or launching the native parent. A follow-up remote directory
check confirmed the case directory was absent.

That failure was observed in the root execution-tool output, not in a retained
remote GPU receipt. It must not be counted as a native attempt or relabeled as
a GPU result. The original 44-test pass does not establish integration success.
A successor must bind all legacy imports explicitly and test restoration on
success and failure before another admission attempt.

The [follow-on GPU checkpoint](../native-device-observation-v1/README.md) now
records that correction, 47 passing tests and a successful raw-tick capture.
Those later results are separate from the runtime-only evidence above.

No calibrated latency, cross-device alignment, overlap, sustained 2,048/256
throughput, independent numerical acceptance or production authority is
established. All issue #42 milestones and the 700 tokens/s target remain open.
