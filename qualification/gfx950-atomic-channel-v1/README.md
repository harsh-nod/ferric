# Indexed Atomic Storage Qualification

This independent reference defines exact bit-preservation checks for the
[256-cell source fixture](../../device/gfx950-atomic-channel-v1/README.md).
It does not launch a GPU or grant execution authority.

Cases cover zero, walking bits, alternating/all-one words and seeded random
u32 values. Both the channel storage and disjoint output must equal the original
input exactly, for 512 word comparisons per dispatch. The checker binds raw
bytes, artifact, probe, worker and reference identities and requires exact
completion, unchanged inputs, allocation guards and successful cleanup.

```sh
python3 -B -m unittest -v test_reference
python3 reference.py generate --case alternating --case-dir NEW_CASE_DIRECTORY
```

Current tests use synthetic reports to test the checker. They are not device
execution evidence. No atomic-slice native artifact or GPU result is recorded
yet. This first fixture reads only each invocation's own write, so even a future
passing GPU result would not prove cross-workgroup tensor publication,
full-model correctness, performance or production authority.

## Reproduction

The compiler, backend and extractor must be a matched immutable build. Preserve
their source identity and all gate output. The native script requires both
compiler and Ferric checkouts to be committed and clean, including no untracked
source files. The device manifest and lockfile pin fe2o3 revision
`3dfa5b3fdac1832bd7d8902e32f591d81300d1e3`; use that exact compiler source
and preserve its matched binary identities.

```sh
FE2O3_COMPILER_BIN=IMMUTABLE_COMPILER_DIRECTORY \
  ROCM_PATH=/opt/rocm-7.2.0 bash build.sh WORK_ROOT FRESH_BUILD_DIRECTORY
bash verify-suite.sh PROBE WORKER HSACO SOURCE PRIVATE_DEVICE_ID NEW_EVIDENCE_DIRECTORY
```

Build output is inert until inspected. Coordinate shared-host device access
before running the suite. The suite freezes four references before dispatch,
binds executable and input hashes, then checks those hashes again afterward.
Both raw `channels.u32le` and `output.u32le` must exactly equal the input. Only
after completion, guard validation, reverse-order frees, Close and worker exit
does the probe write a report; the independent checker writes `numerical.json`.
No timeout permits automatic retry, GPU reset or foreign-process management.
Keep the private selector and raw worker stderr out of published evidence.
