# Parent Device-Observation CPU Qualification

Draft controller and sixteen authored policy tests. The author has not imported
the controller, run its tests, built Rust, used SSH, or launched any native or GPU
work. Root owns source formatting, package freeze, execution and publication.
Successor V2 retains the first actual failure rather than relabeling it. V1
passed metadata and compiled inventory, then its new mutation fixture overflowed
at `u64::MAX += 1`: the actual parent-client run had 146 passes and one failure.
Root changed only that test mutation to `^= 1`, preserving the intended refusal
and all test names. Failed receipt SHA256 is
`d5fae24609182d0b8cb3818b3fd67c9647fb7efe8d32f0046440dbb540b52e57`.
The seven source V2 bodies were rehashed against actual formatter receipt
`device-parent-source-format-v228-v2/complete.json`, SHA256
`bebe48b8158de37308b54bf6a6c41027644802d0ed21ae679dd57fd7c2ed588d`.
Only the intended test file differs from formatted V1. V1 pure success does not
qualify this V2 package; its fresh CPU and pure executions remain root gates.

## Inputs

This is a parent-only successor to the parent portion of actual CPU633:
`host-policy-cpu-v228-v1/complete.json`, 290447 bytes,
SHA256 `1fc4d17534161e6e7f96e6d0eba0a2455227ba2166623823f6a74f845b93deae`.
The historical result contains 398 worker passes and 235 parent passes. This
controller replays the 38 parent command/result/stream records, exact stable
environment, all old named parent test outcomes, and all 213 historical raw
pins. It does not rerun or add those historical worker passes to a new total.

Unchanged pinned helpers are `p228-host-policy-cpu-v1/run.py` (inventory, outcome,
metadata and Cargo artifact parsing), `run_clean_worker_p228_v1.py` (bounded
archive extraction and source inventories), `bounded.py` (owned process groups
and resource limits), and `metadata_p220_v2.py` (stable 1.97.1 environment only).
The latter's old subprocess implementation is not called.

Fresh source manifest `device-parent-source-inputs-v228-v1.json` uses the existing
`ferric-p228-clean-worker-sources-v1` schema, with exact archive FilePins plus
commit/tree fields for Ferric `dc04a484a2424baa453f73fc832b2fa890875b44` and
fe2o3 `9a321f3f98e597a75e8ebeafdda169ec10e12e9e`. Expected archive basenames:

- `device-parent-ferric-dc04a484-v228-v1.tar.gz`
- `device-routing-fe2o3-9a321f3f-v228-v1.tar.gz`

The closed `overlay.json` schema is `ferric-p228-device-parent-cpu-overlay-v1`:
`schema`, `source_manifest` FilePin, `files`, and `added_parent_tests`.
Each of exactly seven files has `path`, `source`, `before`, and `after`; content
pins have `bytes` and `sha256`. Source bodies are exclusively
`p228-device-parent-source-v2/draft/<repo-relative-path>`. Exactly three files
are new. The full seven-path allowlist is in `run.py`; worker, runtime, lockfile,
and unrelated parent changes are refused. The parent Cargo manifest may gain
only the single exact opt-in bin entry; parsed dependencies, features, package
configuration and old bins must remain unchanged.

`added_parent_tests` contains sorted unique `lib` and `bin` name lists. Freeze
the actual nine device-module names, twelve shared device-schema names, and
one new bin name from reviewed source. Compiled inventories must confirm them;
the controller does not accept passing totals without the exact test names.

## Coverage and Counting

The complete parent library inventory must equal the actual historical
inventory plus the frozen new library names. All twelve existing selectors
are retained. The existing `parent-client` selector already includes the nine
nested device tests, so they run and count once there. The new shared-schema
selector is disjoint. The eleven historical binaries retain their exact old
test names; the new bin must have the one frozen opt-in test. All twelve
binaries are built and selected from actual non-test Cargo compiler-artifact
records. The old default-feature library check is retained.

With the currently reviewed names this means 245 selected library tests and
12 bin tests, or 257 parent passes with no ignores, only after actual success.
The expected total is derived from the authenticated old 235 plus the frozen
additions. There are 41 bounded phases (38 old plus shared-schema and new-bin
list/test), producing 208 raw files on a full successful run: five per phase
and three full source inventories. A failed phase is not inserted into the
successful `phases` map; its actual raw failure files remain retained.

Parent Cargo metadata is not the worker's 39-package graph: the retained parent
record has 209 packages, 28 local and 181 external. The new metadata must have
the exact relocated local manifest map and identical external manifest/source
identities and hashes. Actual dependency manifests are rehashed after work.

## Bounds and Invocation

The package freezes exactly `run.py`, `test_run.py`, `README.md`, and `overlay.json`
under manifest schema `ferric-p228-device-parent-cpu-package-v1`. The manifest
is supplied by its actual SHA, not inferred from a directory name. The first
non-assert guard refuses optimized Python before any authenticated helper load.

```text
/usr/bin/python3 -B E/p228-device-parent-cpu-v2/run.py ACTUAL_MANIFEST_SHA device-parent-cpu-v228-v2
```

`E` denotes `/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`.
Launch with the existing ASROCK CPU8,9 affinity, nice10 and ownership wrapper.
The target and extracted source pair are fresh; only the disclosed external
Cargo registry/git cache is reused. No prior build target is written or retired.
Each Cargo command is offline/locked/jobs2 with the exact stable 1.97.1 parent
environment, including `RUSTC_BOOTSTRAP=fe2o3_device,fe2o3_macros`, opt-level2,
debug-info0 and empty GPU visibility. No caller environment is inherited.

The unchanged bounded helper keeps 12 GiB address space per leaf, a 6 GiB target
cap, 40 GiB setup and 38 GiB active free-space floors, 64 MiB streams, core0,
metadata120 seconds and other phases1200 seconds. Root must reclaim only its
verified unused caches if the floor is not met; this controller never deletes
anything or relaxes the bounds. All native/GPU work remains outside this run.

Before and after execution, exact input/tool pins, extracted source snapshots,
Cargo.lock contents, dependency manifests and selected binaries are checked.
All preimages are verified before any overlay write. Failure writes `failed.json`
and returns nonzero; success alone writes `complete.json`. These receipts use
`ferric-p228-device-parent-cpu-result-v1` and preserve actual phases, test names,
summaries, Cargo artifact records and raw FilePins. A successful result builds
the parent only: `worker_rebuilt`, GPU execution, numerical acceptance, timestamp
calibration, compiler/HSACO reproduction, performance and production authority
remain false. This does not validate a device timeline or the 700 tokens/s goal.
