# Gfx950 Clock Sampling API

The fe2o3 runtime now provides checked raw clock sampling for a gfx950 device
and for every rank of an engineering peer group. This is the API needed to
bracket Ferric's [existing raw dispatch capture](../native-device-observation-v1/README.md).
It is not yet connected to that recorder and has not performed a native clock
sampling run on MI350. No GPU timing conversion or speedup is claimed here.

## Validated Results

All compilation and tests ran on `ssh mi350-2` (`asrock-1w300-g2-2b`), with CPU
affinity 8/9, niceness 10 and empty GPU visibility. Sources were reconstructed
in a fresh directory and the Cargo target began empty.

| Check | Actual result |
| --- | --- |
| Runtime tests | 208 passed, including all 169 previous selections |
| New clock tests | 14 passed: six device tests and eight peer-group tests |
| Other added coverage | 25 existing currentness, device-profile and queue tests |
| Ferric worker tests | 440 passed; four existing tests ignored |
| Combined Rust cohort | 648 passed, four ignored |
| Worker compatibility build | Successful, with the new runtime |
| Owned commands | All 33 exited naturally; no process group left behind |
| Controller policy tests | Separate 14-test run passed without skips |
| Formatting | Both remote rustfmt and rustfmt check passed |
| Postchecks | Source, lockfile, input, tool, dependency-manifest and artifact checks passed |

The [CPU receipt](cpu/complete.json), [controller policy receipt](pure/complete.json),
[formatter receipt](format/complete.json) and [publication record](result.json)
retain the measured counts and hashes. The CPU suite compiled the complete
runtime inventory and required exactly the prior inventory plus the 14 named
clock tests. The worker inventory and four ignored names were unchanged.
This is the `engineering-gfx950` worker dependency configuration, not an
all-feature or default-feature test run.

## API And Failure Handling

`CheckedGfx950XnackMinusDevice::observe_clock_correlation()` samples KFD's GPU,
CPU and system counters through the already-owned KFD descriptor. It performs
full device-currentness checks before and after the ioctl. Wrong GPU identity,
reserved padding, zero system frequency, syscall failure or failed currentness
checks permanently poison the token.

`Gfx950EngineeringPeerGroupV1::observe_clock_correlation_v1()` validates the
whole 2- or 8-rank roster before sampling, takes full group entry/exit checks,
and returns no partial vector on failure. The group's existing quarantine
path handles errors. Each observation carries the group incarnation, rank,
device unique ID, KFD GPU ID, queue epoch and raw counters, plus a process-local
host bracket around the checked device call. Ranks are sampled sequentially.
No GPU resource, queue setting or performance policy is changed.

Tests cover successful mock routing, fresh repeated samples, every check/sample
failure, malformed counters, duplicate identities, identity/epoch drift and
terminal-group refusal. These tests cannot establish successful native ioctls
or a relationship between completion-signal and KFD clock domains.

## Source And Evidence

The [ten-file runtime snapshot](runtime-source/crates/fe2o3-kfd/src/engineering_gfx950_peer_clock_correlation.rs)
matches the formatted, compiled and integrated source. The [overlay](controller/overlay.json)
records six exact preimages and four new modules. The [runner](controller/run.py)
reconstructs the previously qualified CPU609 archive/worker-overlay pair and
requires its full source map before applying this runtime-only change. No
Ferric Rust source or dependency manifest changed in this qualification.

Runtime implementation: [fe2o3 commit 27b53d2b7](https://github.com/harsh-nod/fe2o3/commit/27b53d2b74c1f239988a891a4aed39e089b05663),
also published to the powderluv fork's matching engineering branch.

The first formatter stopped at the shared-host free-space floor, before any
formatting or source changes. The successor ran after removing only three
completed task-owned `target/debug/deps` caches. The [cleanup records](cleanup/)
preserve the checks and 1,895,018,496 bytes of allocated cache entries removed;
this is not a measurement of net free-space growth on the shared machine.
All 14 selected executables, source trees, source maps and referenced evidence
were preserved. Unpinned compiled test harnesses were retired with the caches.

## Next Gate

Add pre/post samples around each of the four recorded forwards: two ranks
times two endpoints times four forwards gives exactly 16 samples. Join those
to the existing 1,172 dispatch rows and publish only after consuming Close.
Then validate same-device counter correlation and bounded conversion.

KFD's frequency describes the **system counter**, not the GPU counter. Raw
samples alone do not provide calibrated nanoseconds, cross-GPU alignment or
overlap. This checkpoint does not change the kernel images or model arithmetic,
does not run the sustained 2,048/256 workload, and does not establish numerical
acceptance or the 700 tokens/s target. Issue #42 M0-M7 remain open.
