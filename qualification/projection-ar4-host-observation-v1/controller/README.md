# AR4 Host Observation CPU Qualification

Source-only successor of the executed AR4 CPU controller `a4f9fe5d...`.
The author has not imported, tested, built, or executed this proposal.
Root owns pure qualification, storage checks, deployment and all execution.

## Closed Inputs

No new source archive is required. The controller reads and copies the
existing qualified paired tree at
`E/projection-ar4-cpu-v228-v1/sources/{ferric,fe2o3}` into its fresh output.
The complete source map must equal `sources-after.json`, SHA
`22c23c3effbf79c1b38e8c816f43474cdc201aedb971d81bffd75c99dc5ba0a9`.
Both original and copied sources are checked; the original is checked again
after qualification. The actual CPU1037 completion is 583,779 bytes, SHA
`7a3c170ca6ffe6000517588280fcdb99cb58c0cbb22cc3f4be910232f7d51c54`.
All 442 prior raw files and all seventeen selected original executable bodies
must remain available at their original paths and hash-match.

The reviewed source proposal is
`E/p228-projection-ar4-host-observation-v1/source-manifest.json`, SHA
`ab5d055a09c320780a0d0da7d235bb0fe023469c9132b37a877fbc7841738074`.
Its seventeen exact bodies contain eleven replacements and six additions.
Every preimage joins the CPU1037 tree before writes. Rustfmt and its check
may touch only the sixteen Rust files; Cargo.toml must contain only the
single added `tp-batch-engineering` binary declaration. Full relocated
Cargo metadata must remain identical except for that exact target.

Frozen support dependencies remain the same as CPU1037:

- `E/p228-host-policy-cpu-v1/run.py`, `35abf1ed...`.
- `E/run_clean_worker_p228_v1.py`, `2f3ef5c8...` (pin/snapshot helpers only).
- `R/evidence/wave-output-lowering-v216/bounded.py`, `e634e1b3...`.
- `E/p228-projection-residual-runtime-cpu-v1/run.py`, `6fc90b8c...`
  (configuration/old-target helpers only).
- Original AR4 controller, raw commands, toolchain pins and external Cargo
  dependencies; worker nightly and parent 1.97.1 environments stay separate.

Here `R=/home/harmenon/ferric-asrock-42` and
`E=R/evidence/finite-resident-integration-v220`. No Ferric Git checkout,
compiler/provider source, KFD/driver, native image, or live target is edited.

## Scoped Regressions

There are 61 bounded phases: the original 87 minus 28 unchanged runtime
test selections and their list phase, plus one new shared-report selection
and the new parent binary's list/test phases. The old runtime's 208 passing
executions are historical provenance, not rerun or claimed newly qualified.

| Scope | Existing Passes | Added Executions | Conditional Total |
| --- | ---: | ---: | ---: |
| Worker | 505 | 13 | 518 |
| Parent library selections and binaries | 324 | 13 | 337 |
| Total newly executed | 829 | 26 | 855 |

The four historical worker ignores must retain their exact identities.
Its two libtest summaries stay separate: prior library492/4 plus shared_wire13/0
becomes library505/4 plus unchanged shared_wire13/0, totaling518/4.
Full worker inventory must be the old 509 names plus 13; full parent library
inventory must be the old 811 names plus 12. All sixteen existing parent
binary tests are retained. Eight new report tests compile in both crates;
the remaining additions are five worker, four parent-library and one binary
test. No old name is removed or renamed. Actual named outcomes determine
success; the table above is not a result.

Only three production executables are selected and retained from Cargo:
the worker, the original projection-residual decode parent, and new
`ferric-qwen3-finite-projection-residual-decode-host-engineering` parent.
The old sixteen binary test harnesses still compile and run. On full success
the result has 61 phase records and 312 raw entries (305 leaf records and
seven source/configuration/old-target snapshots).

## Bounds And Invocation

The original bounded leaf helper and natural-exit/process-group checks are
unchanged: ASROCK UID9661, CPU8/9, nice10, hidden GPUs, 40 GiB initial and
38 GiB ongoing free-space floors, aggregate fresh target cap6 GiB, per-leaf
address-space cap12 GiB, 64 MiB streams, and original per-command deadlines.
Do not run another build or cleanup concurrently: original target inventories,
including current RPO and indexed targets, are protected before/after.
Cleanup is separate and never performed by this controller.

Root first runs the thirteen synthetic policy tests under existing CPU-only
bounds: `python3 -B -m unittest -v test_run`. They exercise source additions,
Cargo/metadata isolation, exact test sets, recipe selection and formatter
scope without a compiler, GPU, or source-copy operation.

Then root uses the same original host invocation pattern:

```text
taskset -c 8,9 nice -n 10 /usr/bin/python3 -B \
  E/p228-projection-ar4-host-observation-cpu-v1/run.py ACTUAL_CONTROLLER_SHA \
  projection-ar4-host-observation-cpu-v228-v1
```

The output namespace must be fresh. The result schema is
`ferric-p228-projection-ar4-host-observation-cpu-result-v1`; natural test/build
outcomes, raw receipts, source maps and actual Cargo executable identities
are retained. No host latency result, native execution, numerical acceptance,
full CPU1037 cohort rerun, compiler requalification or performance claim is
implied by CPU qualification.
