# Prefix Raw Timestamp CPU Adapter

Unexecuted controller proposal. Root owns source formatting, freezing, unit
tests, Cargo execution, retention, integration, and publication. This package
does not run a GPU, install a runtime, or modify a live checkout. The runtime
overlay and actual source archive manifest must be pinned before freezing;
missing values refuse execution rather than borrowing a prior result.

## Sources

Use full, clean Git archives with `ferric/` and `fe2o3/` prefixes:

- Ferric `44e308d72815405f9127473368671e9696c5ebaa`, tree
  `fb509bfe4adb1afb98a8abb788f2ac74a7b3dc29`.
- fe2o3 `3d217aabc3c48e9c767fa28a05bd596987c29f9d`, tree
  `e549d1ef8c8c2abec85d52838eebfe0f5a2ff46d`.

The root-authored `prefix-raw-timestamps-source-inputs-v228-v1.json` uses the
existing `ferric-p228-clean-worker-sources-v1` schema. Its exact file pin belongs
in `overlay.json`; its archive pins must match the retained archive bytes.
There is no reconstruction from older uncommitted overlays. Apply only the
three formatted bodies in `p228-prefix-raw-timestamps-runtime-v2`: two existing
prefix files and one new timestamp test module. All preimages are checked
before the first replacement. Full source inventories before and after Cargo
must agree; the new source count is exactly the extracted count plus one.

Historical CPU553 completion and all 109 raw files are independently pinned
and replayed for the exact 21 original commands, environments, natural exits,
reaped process groups, named tests, and outcomes. That historical receipt is
not relabeled as a build of these newer clean snapshots. Retired old compiled
source trees, target dependencies, and old worker execution are unnecessary.

## Coverage

The new compiled runtime inventory must equal the complete actual CPU553
inventory plus exactly nine named tests. The original runtime selections cover
bank bounds, token currentness, typed prefix and MLP state, resident dispatch,
publication, host policy, poisoning, and mapped atomic operations.

The planned selections are:

| Selection | Planned Ordinary Tests |
| --- | ---: |
| All prior CPU553 runtime selections | 144 |
| New prefix public-entry refusals | 2 |
| New typed prefix timestamp joins | 7 |
| Existing raw timestamp policy tests | 8 |
| Existing mapped raw signal tests | 6 |
| Existing queue timestamp-control tests | 2 |
| Full worker library and shared-wire tests | 409 |
| Planned total | 578 |

The original seven typed MLP timestamp tests are included in the first row.
All four historical worker ignores must remain exactly the same named cases;
they are not passes. No selected runtime test may be ignored. The exact full
worker inventory stays at 413 names. Counts above are acceptance expectations,
not test results: actual compiled inventories and every raw result line are
validated before a passing receipt can be written.

There are 25 bounded Cargo phases: metadata, runtime inventory, 20 disjoint
runtime selectors, worker inventory, full worker tests, and worker build.
The executable comes only from the actual worker Cargo artifact message.
The parent is not rebuilt. This proposal contains 16 authored pure policy
tests; they have not been executed by the author.

## Bounds And Execution

Reuse exact, unchanged `bounded.py`, extraction, inventory, metadata, artifact,
environment, and recipe helpers from the qualified prior pipeline. Root must
stage their pinned bytes and the CPU553 raw evidence. Run only after other
task-owned Cargo activity is terminal. Use a fresh output directory and empty
target; no old target is adopted. Reuse only the disclosed external Cargo
registry/git cache. Metadata must resolve every local crate into this new
paired source tree. Source, input, tool, external dependency manifest, and
selected binary pins are rechecked after all phases, including failure paths.

Unchanged limits: CPU affinity 8/9, nice 10, two Cargo/test threads, 12 GiB
address space per leaf, 6 GiB target cap, 40 GiB setup and 38 GiB running disk
floors, 64 MiB streams, 120 seconds for metadata and 1,200 seconds per remaining
leaf. GPU visibility is empty. Python optimization is rejected before loading
the older assert-based helpers. No automatic cleanup is performed.

Root freezes exactly `run.py`, `overlay.json`, `test_run.py`, and `README.md`
under schema `ferric-p228-prefix-raw-timestamps-cpu-package-v1` with file rows
`path`, `bytes`, `sha256`; then runs the pure tests and the controller through
the existing root-owned bounded launch pattern:

```text
run.py ACTUAL_PACKAGE_MANIFEST_SHA prefix-raw-timestamps-cpu-v228-vN
```

Actual success writes `ferric-p228-prefix-raw-timestamps-cpu-result-v1` with raw
phase records, exact source/archive identities, named results, and the new
worker pin. A Cargo/test/postcheck failure writes `failed.json` and returns
nonzero. Early prerequisite refusal cannot produce a completion record. All
GPU, numerical, performance, timestamp-calibration, and production claims stay
false. Raw timestamp CPU coverage is not device-clock qualification.
