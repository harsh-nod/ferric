# gfx950 Clock Parent Diagnostic

This CPU-qualified engineering parent validates the clock recorder V2 report.
It is not a native clock capture, calibration result or production admission.
The single-request BF16 Qwen3-8B 2,048/256, 700 tokens/s target and all issue #42
milestones remain open.

## Interface And Validation

The distinct `ferric-qwen3-finite-prefix-decode-device-clock-engineering` binary
requires `--observe-device-clocks` and the existing explicit machine-code opt-in.
It accepts only `FerricFinitePrefixDecodeDeviceClockRequestV2`, selects the
worker's clock V2 mode, and consumes `<directory>-device-clock-v2.json`.
There is no fallback to a V1 request, flag or sidecar.

The parent first requires native Close, four complete forwards, successful
worker exit and process-group reap. It validates the complete V2 schema with
all sixteen rank/boundary/identity samples, then applies the existing V1
worker, image, transcript, Control and capture joins to the embedded raw report.
The complete V2 file counts toward the unchanged 2 MiB sidecar and 8 MiB
aggregate limits. Only then is the observation summary published.

The existing raw-device parent retains its strict V1 schema and selector.
Its report validation is shared, not weakened to accept either wrapper.

## Actual Qualification

The isolated `mi350-2` build used fresh Ferric `924703865d9cfcb1dc791af211b7558ff7c58894`
archives, eight parent source changes and the twelve matching worker-source
changes. Worker schema modules are imported by path; the native worker itself
is not built by this parent qualification.

| Check | Actual result |
| --- | --- |
| Selected library tests | 262 passed |
| Parent executable interface tests | 13 passed |
| Total Rust tests | 275 passed, 0 ignored |
| New tests included above | 10 parent validation, 7 shared schema, 1 CLI |
| Separate controller-policy tests | 20 passed |
| Parent executables built | 13 |
| Default-feature library check | Passed |
| Bounded commands | 44 naturally completed |
| Source, input, tool and artifact postchecks | Passed |

The previous 257-test parent cohort is preserved. Full library inventory must
equal the prior inventory plus seventeen new names; all twelve existing binary
inventories remain unchanged. Tests cover malformed samples, source/Control/
capture mismatch, incomplete shutdown, strict mode separation and file budgets.
Compilation used Rust 1.97.1, offline/locked Cargo, two jobs, CPU affinity 8/9,
nice 10, an empty target and checked optimized profiles with no visible GPU.

The parent retains its existing locked fe2o3 Git dependency at
`faaaf15d68eff996b22951758b1a9fa83317d6d2`. The paired `27b53d2b` runtime archive
is authenticated source data, not this parent's compiled runtime. The separately
[qualified worker](../gfx950-clock-recorder-v1/README.md) does compile that newer
sibling runtime. These are distinct build results, not one combined suite.

The CPU receipt SHA-256 is
`d2a118dd2a3bfac749b18de26883a661a1078a22ebf374853a11b081f43b1484`.
See the [evidence ledger](result.json), [raw receipt](cpu/complete.json), and
[bounded controller](controller/run.py). Local publication rehashes retained
evidence, source archives and the selected new parent executable; it does not
rerun the remote compiler or claim every external dependency body was retained.

## Next Gate

Both new executables require matching runtime audits and an owned MI350 run
that validates sixteen native clock samples, all 1,172 dispatch records,
unchanged tensor outputs and normal Close/reap. No GPU result, clock-domain
proof, nanosecond conversion, cross-device alignment, overlap, independent
full-model numerical acceptance or sustained throughput follows from this CPU
checkpoint. No kernel image, arithmetic or model workload changed.
