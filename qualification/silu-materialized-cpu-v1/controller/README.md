# SiLU Materialization CPU Qualification

Source-only controller draft. No Python policy test, Rust build or GPU execution
has been performed by its author. The frozen three-file candidate remains
unchanged; this controller formats and tests fresh copies only.

## Run

On ASROCK, stage this directory at `E/p228-silu-materialized-cpu-v1` and the
unchanged candidate at `E/p228-silu-materialized-kernel-v1`, where
`E=/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`.
The root agent owns execution and must serialize it with other compiler work.

```sh
taskset -c 8,9 nice -n 10 /usr/bin/python3 -B \
  "$E/p228-silu-materialized-cpu-v1/run.py" \
  "$E/p228-silu-materialized-kernel-v1/source-manifest.json" \
  silu-materialized-cpu-v228-v1
```

The existing bounded helper checks ASROCK UID9661, CPUs8/9, nice10, a 40GiB setup
floor and 38GiB active floor. Each owned child has a 12GiB address-space limit,
1GiB file limit, 64MiB stream limit, hidden GPUs and a fresh 6GiB target cap.
Phase deadlines share a 1650-second budget, leaving room for cleanup/postchecks
under the root's 1800-second outer limit. No old target is opened for writing.
Cargo uses stable1.97.1, offline/locked/two jobs, opt2 with explicit debug
assertions and overflow checks. The already-retained nightly rustfmt is pinned
separately and only formats the three new copied Rust bodies.

## Actual Dependencies

- `E/p228-down2-cpu-v1/run.py`, SHA `a48976422c3eba35308d3f210022bcd2b736dcd9928f7b4f40d9f417293f9f24`.
- `E/down2-cpu-v228-v1/complete.json`, SHA `96ef991246b90f9f02b509b3302df0215309bb172f9cae4340c21eba27bda563`, its retained raw files, and its complete 31-file `fixture`.
- `E/row-down2-checked-probe-v228-v1/complete.json`, SHA `0ba363b9b4106e5293e4e6152d719c795c2d58f05e062b6a67570f194e0eb68e`, and its four `fixture/src` Rust bodies.
- `E/p228-silu-materialized-kernel-v1/source-manifest.json`, SHA `ca0bafcdd0297d2d849770791cf04686b33f048402f0fb5cc451183217acb276`, and its three mirrored draft bodies.
- The unchanged helpers loaded by the pinned Down2 runner: `E/run_clean_worker_p228_v1.py`, `E/metadata_p220_v2.py`, and `/home/harmenon/ferric-asrock-42/evidence/wave-output-lowering-v216/bounded.py`.
- The actual provider at `/home/harmenon/ferric-asrock-42/fe2o3/crates/fe2o3-device`, its unchanged 53-file closure digest `037e472ea24d7337f88028ed65cb3d6ea6e0c0681538e6121ab78240c44ef675`, original workspace files, cached locked Cargo dependencies and configurations.

No Ferric archive is extracted. The small actual Down2 fixture is rehashed
against the original successful source snapshots before exclusive copying.
The old checked entry is also authenticated and supplied to the candidate AST
test through `FE2O3_SILU_BASELINE_ENTRY`. Original fixtures and all consumed
inputs, tools, provider sources, dependency bodies and configurations are
rechecked after the run.

## Test Cohort

Sixteen phases: format/check, metadata, build, then list/ignored-list/run for
each of four actual Cargo-selected test executables. Complete named inventories
and individual outcomes must agree; missing, duplicated or ignored tests fail.

| Suite | Required Named Tests |
| --- | ---: |
| `mlp_tiles_numerics_v2` | 4 |
| `mlp_down_two_row_v1` | 10 |
| `mlp_claimed_numerics_v1` | 8 |
| `mlp_silu_materialized_v1` | 16 |

The expected total is 38, not a preclaimed result. All eight original claimed
MLP tests run, including original fused-SiLU coverage and failure propagation.
The new tests isolate the BF16 materialization using controlled exponential
results; neither they nor the old host-exp tests prove OCML implementation
equality. `test_run.py` separately contains 12 synthetic controller-policy tests:

```sh
cd "$E/p228-silu-materialized-cpu-v1"
/usr/bin/python3 -B -m unittest -v test_run
```

## Lowering Handoff

Only a successful actual `complete.json` with schema
`ferric-p228-silu-materialized-cpu-result-v1` may be supplied to the later checked
lowering controller by exact path/SHA. It records:

- `prior_cpu`, `prior_lowering`, `overlay`, `runner`, `formatter`, `input_pins`;
- `tests`, `tests_passed`, `tests_ignored`, `phases`, actual Cargo `binaries`;
- `fixture` and three `formatted_sources` keyed by original project-relative paths;
- `lowering_sources`, five FilePins keyed by destination source name: `src/lib.rs`, `src/wave_numerics_v1.rs`, `src/mlp_numerics_v1.rs`, `src/mlp_tile_numerics_v2.rs`, `src/mlp_silu_materialized_numerics_v1.rs`;
- `raw`, retaining each command/start/result/stdout/stderr, source snapshots, dependency snapshots and consumed input map.

The CPU `fixture/Cargo.toml` is the numerical harness, **not** the GPU crate
manifest. Later lowering must retain the old Down2 checked Cargo.toml/lock and
install precisely these five source bodies into a separate fresh fixture. The
two original Down2 numerical bodies and wave helper are never reformatted.

Successful raw roster: 80 phase leaves plus `sources-unformatted.json`,
`sources-before.json`, `sources-after.json`, `dependencies-before.json`,
`dependencies-after.json`, `inputs.json` (86 files). The completion is additional.
Failures retain their own raw leaves and `failed.json`; no success count is
inferred from compilation alone. All GPU, lowering, model-numerical, production
and performance flags remain false. The default kernel and existing images are
not replaced by this CPU experiment.
