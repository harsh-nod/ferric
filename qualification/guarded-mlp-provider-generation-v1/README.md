# Guarded MLP Provider Generation Checkpoint

The 2026-10-05 MI350 CPU qualification is **incomplete**: 398 tests passed,
then the existing matrix-pipeline extraction test failed. No guarded HSACO,
GPU execution, model correctness or performance result is claimed.

## Source Alignment

The proposed fe2o3 source commit is
`a057bf3b0e9175f481a31181b19d587d342bf8f3`, a local, unpublished descendant
of `097b4f796a283f554339cc0c0ef5c2c8c3858d2a`. Ferric's guarded-kernel
dependency remains unchanged pending qualification.

The proposal imports the exact previously reviewed finite-execution device
sources and updates the in-tree compiler's matching provider digest and tests.
Its complete 53-file device closure is
`037e472ea24d7337f88028ed65cb3d6ea6e0c0681538e6121ab78240c44ef675`,
matching the qualified RPO extractor's reviewed generation. It also adds a
regression rejecting the previous device generation for thread and trap
definitions. Provider authentication is not disabled or broadened. KFD/AQL,
host APIs, dependency versions and the guarded kernel bodies are unchanged.

## Actual Results

The [latest failed receipt](attempt-v3/evidence/failed.json) records unchanged
sources and clean source, lock, dependency, tool and final-product postchecks.
All 31 executed phases exited naturally, were reaped and left no process group:
the first 30 returned zero; the matrix test returned 101. The planned final
attention test was not reached.

| Scope | Passed | Status |
| --- | ---: | --- |
| Full device library | 356 | Passed, none ignored |
| Device API and FFI UI targets | 2 | Both passed |
| Trusted-provider cohort | 28 | Includes old-generation rejection |
| Scalar-pipeline unit controls | 4 | Passed |
| Selected gfx950 atomic extraction controls | 8 | Explicitly ran historical ignored tests |
| Existing gfx942 matrix-pipeline extraction | 0 | One failure |
| Existing gfx942 attention-pipeline extraction | 0 | Not run |

These are CPU tests and checked LLVM extraction controls, not GPU launches.
UI fixtures are exercised within their two libtest targets, not added to the
398-test count. Unselected compiler tests are not claimed qualified.

The [matrix diagnostic](attempt-v3/evidence/matrix-extraction-0.stdout)
reports that the collector cannot authenticate the external
`core::result::Result<T, E>::Try::branch` helper reached by `?` in the existing
matrix example. This is an authentication gap, not evidence that the helper
contains unsafe code. It occurs before workgroup-pipeline construction, so it
does not establish a regression from the new private pipeline representation.
The [matched previous-revision baseline](../guarded-mlp-provider-baseline-v1/README.md)
now reproduces the same refusal, establishing that this specific failure
predates alignment. A narrowly scoped compiler fix remains necessary; the
example and failed test have not been rewritten or skipped to obtain a pass.

## Retained Attempts

- [V1](attempt-v1/evidence/failed.json) stopped before compilation because the
  inventory mistook `cc-1.4.4/src/target` source modules for a build cache. The
  correction includes and hashes those files; it excludes nothing.
- [V2](attempt-v2/evidence/failed.json) built the source but the device test
  aborted on an allocation request within the 12 GiB address-space limit.
  Final checks also detected that the later Cargo test build had replaced the
  earlier backend and extractor at their existing paths.
- The [allocator diagnostic](allocator-diagnostic/failed.json) used the same
  device ELF with `MALLOC_ARENA_MAX=2` and the same 12 GiB limit. Rust reported
  all 356 tests passing, but the receipt remained failed because its old parser
  rejected two standard ` - should panic` labels. The original process's memory
  mappings were not captured, so the allocator cause is not claimed proven.
- V3 retains earlier product hashes as historical build observations and
  authenticates the final rebuilt products. It accepts only the standard
  optional panic annotation, without weakening named-test or summary checks.

[Five source-inventory fixtures](scanner-fixtures/complete.json) and
[eight outcome-parser fixtures](outcomes-fixtures/complete.json) passed on
MI350. The explicit allocator cap limits arena count; it does not increase
the address-space budget. See the [GNU libc allocator documentation](https://sourceware.org/glibc/manual/latest/html_node/Memory-Allocation-Tunables.html).

All attempts keep two CPU cores, two Cargo jobs, offline dependencies, disabled
Cargo automatic cache cleanup, 12 GiB address space, 6 GiB private scratch,
bounded streams and deadlines, and shared-host free-space floors. Raw commands,
outputs, manifests and receipts are retained without changing failed results.

## Remaining Gates

Authenticate only the genuine core error-propagation helper shapes while
preserving normal call traversal and source-safety negatives. Rerun both
unchanged pipeline controls and the provider qualification. Only then publish
the dependency revision, rerun the guarded candidate's 27 tests, and perform
fresh vendoring and checked lowering with the qualified RPO producer.

Actual emitted ABI/ISA inspection, native negative controls, reusable arenas,
full-worker comparison, independent numerical acceptance and sustained
2,048/256 measurements remain subsequent gates. All issue #42 M0-M7 and the
700 tokens/s target remain open.
