# Partial-Move RPO CPU Qualification Draft

This V2 successor preserves the failed V1 run and all original source files.
V1 failed at Rust test compilation with E0308 in the new CFG fixture, before
any Rust test executed. V2 wraps the fixture's `u16` edge role in the existing
`SsaEdgeRoleV1::new` constructor. Both production files, FIFO oracle, all
seventeen added Rust test names, all limits and qualification checks remain
unchanged. The controller delta is only fresh output/owner/package paths and
the exact V2 source schema/manifest pin. Result and owner schemas remain V1
because their fields and interpretation are unchanged. This V2 package and
its sixteen pure tests have not been executed.

Source-only controller and 16 authored policy tests, not an executed
qualification. Root owns package freeze, tests, remote execution and cleanup.
No source import, formatting, build, test or native execution by the author.

New outputs are `E/rpo-compiler-cpu-v228-v2` and
`E/rpo-compiler-cpu-owner-v228-v2`. After root freezes a flat manifest over
`run.py`, `test_run.py` and this README, the entry is:

```text
/usr/bin/taskset -c 8,9 /usr/bin/nice -n 10 /usr/bin/python3 -B E/p228-partial-move-rpo-cpu-v2/run.py MANIFEST_SHA
```

Here `E=/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`.
The CPU schema is `fe2o3-p228-rpo-compiler-cpu-result-v1`; the owner schema is
`fe2o3-p228-rpo-compiler-owned-result-v1`. Actual phases/tests/artifacts/raw,
formatted_sources, patch, package and qualified_generation are retained.

## Reuse the Ordinary Compiler Pipeline

Use `p228-ordinary-induction-compiler-cpu-v1/run.py` as the immediate template
(SHA `7d14e9ecdf50f4e81c8f65469b630cdcbc118d67bae9ce879e4455a351885e18`).
It already retains the idempotent partial-move fix and ordinary-induction
recognition. The older `p228-idempotent-compiler-cpu-v2/run.py`
(`e41ee313f025c1c3840d82f08dfba5bf6486e0cea100707f7fc84b2140f79982`)
predates the eight ordinary-induction tests and is not the closest baseline.

The actual ordinary CPU receipt is
`E/ordinary-induction-compiler-cpu-v228-v1/complete.json`, SHA
`c45aa83b01bf76fdfd9962b06884611658dab607b001ba67fecc66db9fd5ba97`.
Its owner receipt is
`E/ordinary-induction-compiler-cpu-owner-v228-v1/complete.json`, SHA
`85de66610631612ed2473f9d5fbb80ce89ea7ffc6064c1784834a6751d623609`.

Copy only the actual qualified `source/fe2o3` into a fresh candidate row; do not
copy a target, rebuild from current RT, or reapply old overlays to this already
overlaid source. Match all 5,781 files / 82,852,470 bytes by relative path to
`sources-before.json` (`5aad4c8ba6520c323310424ce343de2ae5088e7f8c1c0b1fd67853b5fd3bd9f9`).
Then apply only the frozen RPO preimages/additions and record the full new
before/after tree. Keep original-source, dependency, configuration, rust-src,
tool, overlay and old-target postchecks.

The source author identifies two existing files plus new test sources:
`crates/fe2o3-pliron/src/production/semantic_ssa.rs`
(`ffc3046cf078e1b1c82460a528a23ed86fbe97b6355be46f638f3457e20d7797`),
`semantic_ssa/partial_moves.rs`
(`2a04c9bdf64531a6d5d43f8bb7e6316c5843d2c5d56f1123f31e0b7ef30d4036`),
and new `semantic_ssa/partial_move_rpo_v1_tests.rs` and test-only
`semantic_ssa/partial_move_fifo_oracle_v1.rs`. The exact four-body source
manifest is `p228-partial-move-rpo-v2/source-manifest.json`, SHA
`939b76eb28df8d7e2b53ae0b4f5034257185f12f077d20e581d9880b006a252c`.
It declares 17 full new test names. Its original local provenance paths remain
unchanged; byte/extent identities join them to the retained E predecessor.
The retained partial-move source includes fixes absent from current RT.

## Preserve Exact Test and Build Recipes

Two new `rustfmt` and `rustfmt-check` phases operate only on those four copied
Rust bodies with `--edition 2024 --config skip_children=true`; the nightly
formatter SHA is `a9137d0c198ceb6c72193d517d3c9007b3ec7a90d3d10ec6889773eca48261b4`.
The unformatted map is retained, and the full formatted before/after roster
must differ only at those four paths. Retain these ten original phases after
formatting, for 12 total, without adding a second baseline build:

```text
metadata
pliron-build-tests, pliron-list, pliron-ignored-list, pliron-tests
compiler-build-tests, compiler-list, compiler-ignored-list, compiler-tests
compiler-build
```

Baseline pliron: 1,487 passed / 1 ignored, 1,488 named tests.
Baseline compiler: 1,196 passed / 24 ignored, 1,220 named tests.
Require the actual new pliron inventory to equal every old name plus the
frozen RPO additions; preserve all old ignored names. The compiler inventory
must remain exact. Candidate expectations are 1,504 passed/1 ignored and
1,196 passed/24 ignored, not results until actual compiled inventories and
named outcomes are retained. No count-only fallback exists. The entire Cargo
metadata document must equal the prior document after only source/target
namespace relocation.

Retain the exact `--offline --locked --jobs 2 --manifest-path COPY/Cargo.toml`
Cargo arguments. Build each crate's `--lib` test executable with
`--no-run --message-format=json`, list ordinary/ignored names, then execute
`--test-threads=2`. Finally build `rustc-codegen-fe2o3 --lib --bin
fe2o3-rustc-extract`. Select actual Cargo artifacts: both test ELFs, extractor,
backend `.so` and `.rlib`. No guessed executable hashes or copied old products.

Use the same bounded helper `e634e1b3be3b122b2231ad134c770f25d7d71d10ec767d61a16d03e726bdf4f1`,
probe `bb735600...`, outer `a024e17e...`, driver `244c12f6...`, owned
`ab41af45...`, and inventory `48d61daa...` at the exact paths in the old
controller's `HELPERS`. Preserve the non-assert optimization guard before
loading them. CPU affinity 8/9, nice 10, hidden GPUs, 12 GiB AS, 6 GiB target,
40/38 GiB free-space floors, 64 MiB streams, 1,800-second phase maximum and
10,800-second owned deadline remain unchanged. Nested Cargo TMPDIR stays
inside the new target.

Toolchain remains `nightly-2026-04-03-x86_64-unknown-linux-gnu`:

```text
rustc   08dfef109ad22d90556dbd2f964543cd93843dcd75a2e9792c173667392a1950
cargo   c9ad606cb1dbb4a65aa27c80be88ed61eb2b811b6450eeec6794f60ed78b94a3
rustdoc 475ad4924e815034ed7ec5f85edfc8a99b2df99eab8695a76debb1eb64a9bd44
```

Keep the full toolchain library/rust-src and locked dependency snapshots,
provider source map and configuration absence checks; these three binaries
alone are not the complete compiler provenance closure.

## Matching Finalizer Qualification

After actual CPU success, adapt
`p228-ordinary-induction-finalizer-tools-v1/run.py`
(`564b70189259bbdb6fbf6eec42d424ecb1b2ffc912a4d5a76e1c4c4b63a14de3`).
Bind the new actual CPU/owner/source/artifact pins, replay both full CPU logs,
and use that same new source and target. Do not reuse old finalizer binaries
as candidate-generation products.

Retain six phases: metadata, finalizer-build, finalizer-build-tests,
finalizer-list, finalizer-ignored-list, finalizer-tests. Build both examples
`finite_join_engineering_hsaco_v1` and `finite_join_request_metadata_v1`, plus
the finalizer test ELF. Preserve the exact 205-name inventory: 190 default
passes / 15 ignored. Actual retained-handoff inert join remains a separate
selected gate in checked lowering, not an ignored test counted as passed.

Baseline finalizer receipt SHA is
`874a6d2026029619a02283ee9650d682690941c14904d9fa59c101b58b89d6bc`.
Keep replay/certificate checks, source-bound proof generation, unchanged
resource limits and authentic lowering stages. New compiler CPU success is
not a new RoPE certificate or HSACO. A fresh checked lowering must bind the
new generation and still complete replay/inert join/emission/inspection.

## Storage and Custody

Historical target peaks were 2,101,878,889 bytes after CPU qualification and
2,401,516,660 after the six tool phases. A single fresh target, reused only
between its own qualified CPU and finalizer phases, is the smallest direct
pipeline. Allow roughly 2.5 GB plus cushion above the 38 GiB floor; the exact
new size is not known. Measure the actual 40 GiB admission floor immediately
before launch; a historical free-space observation is not admission.

Root-only inventory candidates, never blanket cache clearing:

- `rope-materialized-cpu-v228-v3/target`, completed CPU `8dcb4f2b...`.
- `row-rope-materialized-checked-probe-v228-v2/target`, terminal failure
  `e057e772...` plus owner `77d6e1d0...`.
- `row-reciprocal-checked-probe-v228-v7/target`, inner `6ba25826...`, owner
  `reciprocal-checked-probe-owner-v228-v6/complete.json`
  (`8d55d89d06fb1a3a6966c25ed52e3f5274b6c69b6cef9fffcb2363929305cb62`);
  historical 257,919,222 bytes, not a current reclaim estimate.

Use exact terminal receipts and a fresh plan of disposable single-link,
non-executable `.rlib`/`.rmeta`/host-ET_REL `.o` only. Preserve every non-target
row/owner file, all executable/shared-library/code-object files and explicit
product pins, source/HSACO/toolchains and finalizer artifacts. Current eligible
bytes and quiescence require root measurement; prior retirement may leave
little to reclaim. Root's actual cleanup V2 removed 232 disposable files /
651,563,008 allocated bytes while preserving 1,028 protected files; its reported
free space was 43,300,532,224 bytes before the separate AR4 build. The historical
validity target was already absent and is excluded from that cleanup. No agent
cleanup execution occurred, and this compiler controller never deletes caches.

Existing custody archives, not new build inputs to extract wholesale:
`ordinary-induction-compiler-cpu-evidence-v228-v1.tar.gz`
(`5ec28cafc6ef37f7eea14b979c23130d4fbe7a37a3ba75e7a3031f282c606215`),
and `ordinary-induction-finalizer-tools-evidence-v228-v1.tar.gz`
(`f243be26fe9c1196179af078f37c02c6fbf8396eea54e1ae820ac4048c84b531`).
The qualified source is already retained under the original E/L namespace;
rehash it against the pinned source map before copying. If missing remotely,
transport only the required authenticated source/metadata, not a multi-GB
historical target.
