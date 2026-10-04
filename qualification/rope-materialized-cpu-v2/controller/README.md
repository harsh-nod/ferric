# RoPE Materialization V2 CPU Qualification

Source-only controller draft. The author has not imported, run or tested it.
Root owns execution, resource admission and any actual qualification claim.

This is the narrow successor to the V1 controller: only its accepted source
directory and source-proposal schema change to V2. The result schema, guards,
phase/test inventories and provider/compiler/tool identities are unchanged.
The source changes only 17 pure finite-check `&&` operators to boolean `&`;
all 20 candidate tests are byte-identical to V1. Fresh execution is required.

## Invocation

Use the existing ASROCK bounded launch environment: CPU affinity 8,9, nice10,
ordinary Python with `-B`, hidden GPUs, retained toolchain and owned helper.
The unchanged bounded helper keeps its 40GiB setup/38GiB active free-space
floors, 12GiB address space, 6GiB target, 1GiB file and 64MiB stream limits.
The controller's cumulative phase budget is1650s; retain the existing outer
timeout/ownership wrapper, rather than launching an unowned controller.

```text
python3 -B E/p228-rope-materialized-cpu-v2/run.py \
  E/p228-rope-materialized-source-v2/source-manifest.json \
  7f1f447852f01bad9b6dedb9b40a7969961d45861592465548e6401615318d23 \
  rope-materialized-cpu-v228-v3
```

`E` above is `/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`.
An existing output directory is rejected. The actual source-manifest digest is
an explicit invocation input, never a fabricated future CPU or image digest.

## Exact Scope

The baseline is the actual V7 checked-lowering fixture: seven Rust sources and
its Cargo pair, authenticated through the retained V7 completion and exact
before/after maps. All nine original bodies are rehashed. Only the candidate
entry, additive macro, twenty-test suite and exact original-entry test fixture
are overlaid. Rustfmt may change only the three candidate source/test bodies;
the original-entry fixture and every inherited body must remain byte-identical.

CPU compilation uses the unchanged device-only numerical Cargo/lock/fixture-lib
harness from the qualified Down2/reciprocal controllers. It does not compile
the GPU entry point as a host library or pull in its unrelated host compiler
dependency closure. The candidate entry is checked by the authored source
contract; compiling that entry is a separate checked-lowering obligation.

The unchanged reciprocal macro is tested from the original exact test source
and1007-vector TSV against the V7 head/reciprocal sources. The original named
inventory is authenticated from actual CPU-v3 raw records, not inferred from a
passing count. The candidate inventory is the twenty names in the frozen
source manifest. There are eleven phases:

- Rustfmt and rustfmt-check, metadata and two-test-binary build.
- List, ignored-list and execution for each of the two test binaries.
- Explicit execution of the reciprocal's existing ignored exhaustive test.

Expected observations, not results:32 ordinary passes, one ordinary ignored
test, then that exact exhaustive test passes separately;33 total passes and
one recorded ignore across the three test rows. Success must come from actual
Cargo artifacts, named inventories and actual test transcripts. All eleven
owned leaves must naturally complete. There are61 direct raw records on full
success, excluding the final completion. No controller-only synthetic test
suite is added to this copied harness.

Provider53's exact closure digest, retained tools, shared configurations,
workspace Cargo pair, locked dependency identities, actual dependency source
trees, copied fixtures and consumed inputs are checked before/after. Neither
the provider, runtime nor compiler is edited or rebuilt by this controller.

## Lowering Handoff

The receipt schema is `ferric-p228-rope-materialized-cpu-result-v1`. It retains
the SiLU-shaped `raw`, `phases`, `tests`, `binaries`, `formatted_sources`,
`lowering_sources`, `input_pins`, `source_unchanged` and `postcheck_errors`.
`formatted_sources` has the four actual copied candidate files;
`lowering_sources` has exactly eight `src/*.rs` FilePins. The separate
`lowering_fixture_pins` holds the original V7 Cargo.toml and Cargo.lock pins,
not the CPU-only Cargo files. A later checked compiler must use that Cargo pair
and the eight actual source bodies, retaining the unchanged provider53 and
checked pipeline. CPU macro execution is not HSACO reproduction, native
execution, table-generation equality, full-model acceptance or performance.
