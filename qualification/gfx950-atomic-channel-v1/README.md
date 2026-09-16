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

The four checker unit tests use synthetic reports; they are not device evidence.
The separate [native GPU evidence](evidence-native-v1.json) records four passing
dispatches on `mi350-2`. Each invocation reads its own write, so this establishes
only the tested indexed atomic storage path, not cross-workgroup tensor
publication, full-model correctness, performance or production authority.

## Native Result

| Case | Channel words | Output words | Result |
| --- | ---: | ---: | --- |
| Zero | 256 | 256 | Exact |
| Walking bits | 256 | 256 | Exact |
| Alternating/all-one words | 256 | 256 | Exact |
| Seeded random | 256 | 256 | Exact |
| Total | 1024 | 1024 | 2048 exact comparisons |

All input bytes and allocation guards remained unchanged. Each dispatch
completed, then all three buffers were freed in reverse order, the worker
closed, and its process exited successfully. Source, executable, artifact and
reference identities were frozen before dispatch and rechecked afterward.

Ferric source checkpoint `d6fa1241344cb10d212455376950f709d23a3147` and clean
fe2o3 `3dfa5b3fdac1832bd7d8902e32f591d81300d1e3` produced HSACO
`549717e479cdabfcb4e803d1edd5ee5f06d900b03c551cc38637c021db67dbfe`.
The ordinary Rust pipeline retained nominal atomic storage, actual slice
guards, ranked/formal memory checks and target validation before emitting the
compiler handoff. The unmodified LLVM contains an aligned four-byte Release
store and Acquire load at System scope. ROCm 7.2.0 LLVM 22 linked that checked
output and its standard providers; no handwritten kernel IR or HIP was used.

The launch uses two 128-thread workgroups, two wave64 waves per workgroup,
48 explicit argument bytes, no LDS/private segment, 22 SGPRs and 5 VGPRs with
no spills. These are artifact/resource observations, not a speed measurement.
The engineering worker uses `VRAM|WRITABLE|PUBLIC`, without the COHERENT flag;
neither HBM residency nor a protected coherence contract is inferred from this
passing test. Protected generated KFD and HSA preparation still reject the
unjoined shared-atomic runtime contract.

Compiler library tests passed 682/682. Separate source controls passed both
indexed positives and the shared-input alias negative. The probe passed 34 host tests, strict
Clippy and formatting. The compiler itself retains 38 verified pre-existing
Clippy diagnostics; it is not reported as Clippy-clean.

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
