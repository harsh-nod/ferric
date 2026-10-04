# Parent Clock CPU Qualification Draft

This unexecuted successor uses the actual parent257 baseline, not an aggregate
worker qualification. It is based on `p228-device-parent-cpu-v2/run.py`; the
same pinned host-policy helpers, stable 1.97.1 toolchain, bounded ownership and
resource limits are reused. Twenty policy tests are authored, not executed.
No source, manifest, controller or test is imported by the proposal author.

## Frozen Inputs Required

`overlay.json` binds all twenty bodies from the completed remote formatter V3.
Root freezes all four package files and runs the pure suite before executing the
CPU controller. Formatting is not a successful compilation or test result.

- Fresh source manifest `clock-recorder-source-inputs-v228-v1.json` authenticates
  Ferric `924703865d9cfcb1dc791af211b7558ff7c58894` and runtime
  `27b53d2b74c1f239988a891a4aed39e089b05663`, with exact archive/tree pins.
- The overlay contains exactly eight parent and twelve worker destinations,
  ten new files total. Existing bytes are checked before any writes. Only the
  new parent binary may change the structured Cargo manifest.
- Parent formatted bodies are expected under
  `p228-gfx950-clock-parent-source-v2/draft`; worker formatted bodies under
  `p228-gfx950-clock-recorder-source-v2/draft`.
- Historical `device-parent-cpu-v228-v2/complete.json` is bound to its actual
  298774-byte SHA `3b51d61268bee492dd6ded7623742153513c46cd3d246ebc60a6bc9a2555077b`.
  All208 raw records and41 command/environment/start/result/stream joins are
  replayed, along with actual245 selected library and12 binary tests.

## Source And Test Scope

Before overlay, the new tree must match the old qualified local Ferric package
subtrees, the complete shared-worker subtree, and root Cargo/toolchain/config
files. Only the two explicitly named parent/worker README differences are
excluded from this historical compatibility comparison. Full new archive and
post-overlay source maps are still retained and checked before/after execution.

The parent does not compile the sibling runtime source or the native worker
crate. Its existing locked Git runtime dependencies and all external manifests
must equal the prior metadata. It imports the worker wire/data/test modules via
source aliases, including the new V2 clock schema. The whole worker proposal is
present for source identity, but this run does not qualify its native recorder.
The result explicitly records `worker_rebuilt=false` and
`sibling_runtime_rebuilt=false`.

All41 old parent phases remain, including every old binary test/build and the
default-feature library check. Three phases are added: the clock shared-data
selector and the new clock binary's list/test. The build phase also emits that
new binary. The resulting44 phases retain223 raw records including three source
maps. Full compiled library inventory must equal all prior names plus the frozen
17 new library names; old binary inventories must remain exact. The new binary
has one declared test. Nested ten direct tests run once under `parent-client`;
the seven shared-schema tests run once under their disjoint selector. Expected
275 passes and zero ignores are accepted only after all actual named outcomes
and inventory-derived totals agree; no worker/runtime test count is added.

## Root-Owned Execution

```text
python3 p228-gfx950-clock-parent-cpu-v1/run.py MANIFEST_SHA gfx950-clock-parent-cpu-v228-v1
```

Use the existing bounded outer invocation after all other Cargo jobs are
terminal. The script creates a fresh source tree and empty target, with no
automatic cleanup or target reuse. Cargo remains offline/locked/jobs2, host
opt2 with debug assertions/overflow checks, zero debug info and incremental
disabled. Stable `RUSTC_BOOTSTRAP=fe2o3_device,fe2o3_macros` is unchanged. CPU8/9,
nice10, 12GiB address-space bound,6GiB target cap,40GiB setup/38GiB active free
floors, metadata120s/other1200s and empty GPU visibility remain unchanged.

Inputs, source maps, stable tools, external dependency manifests and actual
Cargo-selected binaries are rehashed after execution. Failures retain their
actual outcomes and cannot become successful receipts. No native launch, clock
sampling, GPU execution, calibration, numerical acceptance or performance claim
is established. Root owns formatting, pure/Rust execution, publication and any
later separate parent/worker runtime review and GPU-controller generation.
