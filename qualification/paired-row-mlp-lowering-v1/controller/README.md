# Down2 Checked Lowering Probe

Draft only. The author has not imported this package, executed its 17 synthetic
policy tests, built code, run SSH, or launched a GPU. Root must review, test,
freeze, transport, and serialize execution. No manifest or result is invented.

This is the next compiler step for the actual CPU14-qualified Down2 candidate,
not a production replacement, numerical GPU qualification, or performance claim.
The live Ferric kernel selection and all historical packages remain unchanged.
This V2 successor adds exact source-map byte/stamp rechecks to both inner and
outer completion paths. Frozen V1 is preserved; no V1 lowering was launched.

## Existing Path Reused

The successful `row-source-v225-v9` MLP fixture supplies Cargo.toml, Cargo.lock,
`mlp_numerics_v1.rs`, and `wave_numerics_v1.rs`. Only its entry and tiled-numerics
include are replaced by their exact CPU14-tested formatted Down2 counterparts.
There are four Rust fixture files. CPU-only test source is not a kernel input.

The retained V7 checked-probe recipe supplies the unchanged nightly tools,
extractor/backend, LLVM worker and nine reviewed OCML libraries. Its exact
ordinary-induction compiler generation is reauthenticated using the retained
helper, including the compiler/finalizer CPU ownership receipts and full source
snapshots. No compiler rebuild, compiler edit, cap increase, source admission
exception, or numerical trust expansion is included.

The kernel remains `ferric_qwen3_claimed_mlp_tiles_bf16_f32_v2`: V39 semantic
ownership, eleven roots, 88-byte carrier, 344-byte kernarg, 512-byte static LDS,
64 lanes, maximum 64 workgroups, 548-word task state, and fixed 512 scheduling
rounds. The CPU source-contract tests bind the fixed-round entry. The actual
retained MLP replay independently checks its storage and formal profile.

## Root Invocation

Freeze these four files under
`E/p228-down2-lowering-v2/manifest.json`, schema
`ferric-p228-down2-lowering-package-v2`, with exact `files` FilePin rows and the
authored test census (`test_run.py`: 17). Run the pure suite separately first.
Then, with Python optimization variables absent:

```sh
taskset -c 8,9 nice -n 10 /usr/bin/python3 -B \
  E/p228-down2-lowering-v2/run.py ACTUAL_MANIFEST_SHA
```

`E` above denotes `/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`.
Run on ASROCK as UID9661. The public entry uses the retained `run_coordinator`
owned-process implementation to launch its private `--child` mode. Do not launch
the child directly. No GPU/compiler job may overlap this protected-target run.

Outputs are exclusively fresh `row-down2-checked-probe-v228-v1` and
`down2-checked-probe-owner-v228-v1`. Both must be absent. Limits remain two CPU
cores, nice10, 12GiB address space, 1GiB file size, 6GiB fresh target, 40GiB initial
free space and 38GiB running floor. Lowering has its unchanged 1800s deadline;
the inner/outer bounds are 10500/10800s plus retained cleanup. A timeout is a
retained failure, never justification to bypass a gate.

## Required Retained Inputs

All remote paths and exact content hashes are explicit in `run.py` and the
pinned V7 recipe. Root should confirm their presence, not replace missing files:

- CPU14 receipt `down2-cpu-v228-v1/complete.json` (`96ef9912...`) and all 45 raw
  records, exact source snapshots, the frozen CPU runner, and formatted source
  manifest `26ac584b...` with the candidate bodies.
- `row-source-v225-v9/complete.json` (`d3bd29c6...`) and all six old fixture files.
- Frozen `p228-reciprocal-checked-probe-v7/run.py`/recipe and
  `p228-reciprocal-outer-v6/run.py`; no historical prefix image is reused.
- Exact ROOT/PIPE map input `reciprocal-checked-probe-inputs-v228-v7.json`
  (`08a85adc...`), all mapped source bytes and canonical Cargo configurations.
- The actual provider53 at `R/fe2o3/crates/fe2o3-device`, closure `037e472e...`.
  The candidate CPU snapshot must match the same provider files.
- Preserved `ordinary-induction-compiler-cpu-v228-v1` source, compiler tests,
  extractor, backend rlib/so, finalizer/test/metadata executables and generation
  receipts. The backend's `debug/deps` alias and dynamic library closure must
  still satisfy the original pin checks after cache retirement.
- The pinned nightly rust-src tree, cached Cargo dependency sources, LLVM worker,
  LLVM shared library, OCML support files, `llvm-readobj` and `llvm-objdump`.
- Every `OLD_TARGETS` directory in `run.py`, including terminal CPU669, parent275,
  CPU14 and prior compiler/probe targets. Their current stat inventories must
  stay unchanged; this probe does not require deleted disposable deps to return.

Only data/helper APIs from the old runners are imported. Their original main
functions are not run. A changed map, missing binary/dependency, stale source
stamp, altered config, or old-target drift fails closed before or after the probe.

## Nine Actual Stages

1. Offline locked fixture Cargo metadata; selected provider and dependency/sysroot
   source bodies are snapshotted before compilation.
2. Normal `cargo check -Zbuild-std=core --target amdgcn-amd-amdhsa`, using the exact
   checked rustc workspace extractor, gfx950 flags and fresh target.
3. Actual MLP V39 capture replay through ranked/formal/outer-lineage checks:
   `actual_mlp_v39_reaches_formal_archive_and_outer_lineage_replay`.
4. Actual retained MLP full inert join:
   `wave_emission_actual_retained_v1_passes_full_inert_join`.
5. Normal `finite_join_engineering_hsaco_v1 --wave-mlp-tiles-v2` finalization with
   the unchanged reviewed LLVM worker and OCML provider.
6. Byte-preserving retained formal-archive/LLVM extraction.
7. Actual HSACO descriptor metadata and unchanged MLP ABI checks.
8. ELF notes/symbols, requiring LDS512, wave64 and private0 for this existing
   launch path. Register counts and spills are retained, not guessed.
9. Actual gfx950 disassembly, including the expected symbol and `s_endpgm`.

Every leaf retains argv, environment, PID/PGID, stdout/stderr and natural reap
result. Whole source/tool/library/config, compiler source roster, fixture,
dependency/sysroot and preserved-target checks remain before/after. On a failure,
available captures and raw logs remain; no retry or live source promotion occurs.

## What Root Must Inspect

Paired accumulators can increase MIR locals, CFG blocks, ranked proof work and
VGPR live ranges. They may hit existing storage/work/deadline gates or spill;
CPU arithmetic success cannot predict compiler acceptance. Do not increase caps
or reinterpret a failed capture as accepted.

After successful emission, compare LLVM and ISA against the old MLP image:
per-row product/add order, two independent finite flags, row0 reduce/write before
row1, and sticky row0 rejection poisoning row1 before its reduction. Confirm no
new reassociation, FMA contraction, fast-math or precision change. Inspect actual
VGPR/SGPR counts, scalar/vector spills, private bytes, LDS and instruction counts.
The baseline had 104 VGPR, 106 SGPR, no recorded spills and private0; these are
comparison facts, not predeclared candidate results.

All eight conditional runtime requirements remain undischarged. This probe does
not rerun the old six-budget measurement callback or claim its measurements for
Down2; the normal production-budget actual join/finalizer gates remain intact.
Actual GPU numerical captures and an equal-workload timing ablation are separate
root-owned next steps, including checking whether reduced input calls actually
reduce loads and improve duration without a pressure regression.
