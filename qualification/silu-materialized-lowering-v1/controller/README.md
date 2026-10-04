# SiLU Materialization Checked Lowering

Source-only, unfrozen proposal. The author has not imported or executed this
package, its 27 synthetic policy tests, a compiler, SSH, or a GPU. Root owns
review, pure tests, freeze, transport and serialized execution. The actual CPU
completion is a required caller input, not a predeclared successful result.

This is a narrow successor of `p228-down2-lowering-v2`. `contracts.py` is
byte-identical. The nine checked stages, callbacks, finalizer, resource limits,
source-map custody and owned-process outer/inner lifecycle are retained.
Historical packages, compiler sources and live kernel selection are untouched.

## Exact Source Generation

Admission requires the completed CPU runner `c083d3fc...` and candidate proposal
`ca0bafcd...`. The actual receipt must show all 38 named tests passing with none
ignored: old Down2 4 + 10, existing claimed-numerics 8, and candidate SiLU 16.
All 16 natural CPU phases, 86 raw files, four actual Cargo-selected test ELFs,
the three formatted source pins, and the unchanged before/after source snapshots
are rehashed. The full 31-file prior CPU fixture plus exactly three additions is
joined to the new 34-file fixture. Every CPU input pin is retained and rechecked.

The new compiler fixture has exactly seven files, five of them Rust:

| Destination | Qualified source |
| --- | --- |
| `Cargo.toml`, `Cargo.lock` | Unchanged checked MLP `row-source-v225-v9` |
| `src/lib.rs` | CPU-formatted `finite_mlp_tiles_silu_materialized_v1.rs` |
| `src/wave_numerics_v1.rs` | Unchanged checked Down2 include |
| `src/mlp_numerics_v1.rs` | Unchanged checked Down2 include |
| `src/mlp_tile_numerics_v2.rs` | Unchanged checked Down2 include |
| `src/mlp_silu_materialized_numerics_v1.rs` | CPU-formatted SiLU include |

Test bodies are not kernel inputs. No library source is downloaded or substituted.
The original provider53 digest `037e472e...` remains mandatory. The separate
validity-reduction provider54 proposal is not selected. The expanded compiler
generation is independently reauthenticated by the frozen V7 helper and must
equal the actual Down2 lowering receipt `0ba363b9...` generation, including its
source roster, compiler products, finalizer prerequisites and reviewed tools.

## Root Commands

First run the 27 synthetic tests in the usual bounded CPU-only environment:

```sh
cd E/p228-silu-materialized-lowering-v1
/usr/bin/python3 -B -m unittest -v test_run
```

Root must freeze the four package files in `manifest.json`, schema
`ferric-p228-silu-materialized-lowering-package-v1`, with exact `files` rows
(`path`, `bytes`, `sha256`) and the actual authored test census. The wrapper is
not a package member. The package directory must contain only those four files
and its manifest. Then invoke the owned outer entry, not private `--child`:

```sh
taskset -c 8,9 nice -n 10 /usr/bin/python3 -B \
  E/p228-silu-materialized-lowering-v1/run.py ACTUAL_PACKAGE_SHA \
  E/silu-materialized-cpu-v228-vN/complete.json ACTUAL_CPU_COMPLETE_SHA
```

`E` means `/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`.
The host must be ASROCK `asrock-1w300-g2-2b`, UID 9661, with Python optimization
absent and all three GPU visibility variables empty in each compiler leaf.
No compiler/GPU job may overlap the protected-target run. No retries occur.

Fresh outputs are `row-silu-materialized-checked-probe-v228-v1` and
`silu-materialized-checked-probe-owner-v228-v1`; both must be absent. Bounds stay
CPU 8/9, nice 10, 12 GiB address space, 1 GiB file size, 6 GiB private target,
40 GiB initial free space and 38 GiB running floor. Checked lowering and actual
semantic replay each retain 1800 seconds; other stages retain 900 seconds.
The inner/outer deadlines remain 10500/10800 seconds plus owned cleanup.

## Preserved Checked Stages

1. Offline locked fixture metadata and dependency/sysroot source snapshot.
2. Checked `cargo check -Zbuild-std=core` through the retained extractor/backend,
   with completed pre-ranked, neutral and target-before-LLVM captures.
3. Actual MLP V39 semantic/ranked/formal/outer-lineage replay, one real pass.
4. Actual retained full MLP inert join, one real pass.
5. Normal `--wave-mlp-tiles-v2` engineering finalization, unchanged LLVM worker
   and reviewed nine-file OCML provider with exactly `__ocml_exp_f32`.
6. Retained formal archive and LLVM extraction.
7. Actual image descriptor metadata, same eleven roots and 88/344-byte ABI.
8. ELF notes, requiring wave64, LDS512 and private0 for the current launch path.
9. Actual gfx950 disassembly with the expected symbol and `s_endpgm`.

Each leaf retains command/environment, start, result and both streams. Whole
source/tool/library/configuration, compiler source roster, fixture, dependency,
sysroot and old-target checks remain before/after. The candidate CPU target is
added to the protected inventory. Missing retained dependencies or artifacts
are failures, not grounds to rebuild the preserved compiler or widen budgets.

Required retained paths are explicit in `run.py`: the prior CPU14 and Down2
lowering receipts, original MLP Cargo fixture, candidate source proposal and
CPU fixture/raw/inputs, V7 recipe/maps/helpers, actual provider53, qualified
ordinary-induction compiler/finalizer generation, reviewed LLVM/OCML closure,
nightly rust-src and all existing `OLD_TARGETS` directories.

## Scope

Successful emission would establish only this fresh checked compiler path for
the CPU-tested source. All eight runtime requirements remain undischarged.
The symbol and scheduling ABI remain the existing MLP tiles symbol in a separate
image. No HSACO hash, GPU result, numerical acceptance, calibrated timing,
performance improvement, full-model acceptance or production authority is
predicted here. After emission root must inspect actual LLVM/ISA materialization
and spills, then separately qualify the image in the owned native capture route.
