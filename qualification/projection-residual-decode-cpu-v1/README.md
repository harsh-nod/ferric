# Projection-Residual Four-Step Decode CPU Qualification

The opt-in Qwen3-8B BF16 TP2 route builds and passes its joint CPU regression
suite on ASROCK through `ssh mi350-2`. This is not a GPU numerical result,
a production admission, or a performance measurement.

## Implementation

The new parent executable is
`ferric-qwen3-finite-projection-residual-decode-engineering`. Its request wraps
the existing four-step decode configuration and an additional hash-pinned
projection-residual image. It accepts teacher-forced inputs only.

The worker's `--engineering-native-projection-residual-decode-v1` selector
loads that image for both O and Down residual stages across all 36 layers.
The original residual image remains the copy image. The selected image is
bound to a separate profile hash through bootstrap, requests, completions,
transcript and consuming Close. Existing selectors and serialized routes
are preserved. No kernel arithmetic or default execution path changes here.

This extends the [layer-zero candidate](../projection-residual-native-capture-v1/README.md)
to four complete forwards. The candidate is not required to reproduce the
old native outputs: its purpose is to correct the projection materialization
boundary. Independent framework comparison remains required.

## Actual Results

| Selected tests | Passed | Ignored |
| --- | ---: | ---: |
| fe2o3 runtime regressions | 208 | 0 |
| Ferric worker | 495 | 4 |
| Ferric parent library and executable tests | 319 | 0 |
| Total | 1,022 | 4 |

The four ignores are unchanged historical worker tests. All 34 additional
executed tests passed: 18 worker tests, 15 parent-library tests including the
shared wire's five tests, and one executable test.

All 87 bounded commands completed with exit code zero and their process groups
absent. All 17 executable artifacts built, the default-feature check passed,
and source, input, dependency, artifact and old-target postchecks passed.
The build used fresh paired sources and targets, two CPU cores, low priority,
hidden GPUs, and the existing disk/cache limits.

The actual completion is `projection-residual-decode-cpu-v228-v1/complete.json`,
SHA-256 `1f4365064da1a0884385035d1f2d280e6bba66afc2b5bb4265bf7d60b1418c83`.
The parent ELF is `549c7b4379c12f7cf0bcafc1461627cc88357e0ad91be38789dfd1f5864d876b`;
the worker ELF is `57959a8c771c08ab9b7184d88cc3f2760802f8965f88d918a3d684bf264aa5e1`.

## Evidence and Limits

[result.json](result.json) records the actual test selections, executable pins,
commands, source maps and integration checks. [controller.py](controller.py)
is the executed remote controller. [publish.py](publish.py) replayed the
retained test outcomes and source/archive checks before installing all 21
tested source bodies. All 442 current raw records, 427 prior raw records and
17 ELF bodies were rehashed locally; the raw files and executables remain in
the retained evidence directory rather than Git.

The worker uses the paired local fe2o3 source; the parent retains its locked
Git dependency. This checkpoint does not rebuild HSACO, run either executable
on GPUs, establish full-model numerical acceptance, calibrate clocks, or
measure the 2,048-prompt/256-decode workload. Those gates and the 700 tokens/s
target remain open.
