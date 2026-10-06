# Guarded Model Worker CPU Qualification

This checkpoint tracks the standalone Ferric worker for the guarded TP2 model
route. It is not native whole-model acceptance, an independent numerical
reference, or a throughput result. The production execution path is unchanged.

## Scope

The candidate connects the qualified runtime allocation, binding and dispatch
facade to the real-model worker. It adds a two-bank roster for 36 layers,
guarded reduction/MLP execution, a distinct four-forward control protocol and
an opt-in engineering CLI. The shared protocol has no dependency on the
worker's private GPU implementation. Existing execution routes remain intact.

The 19 worker source files are integrated after MI350 qualification: **586
tests passed, zero failed, four deliberately ignored**. All 24 new library
tests pass. The library accounts for 573 passes and the shared-wire integration
target for 13; the binary test target contains no tests. All nine phases,
including the final worker binary build, exit naturally with clean source,
dependency, cache and process postchecks.

## Attempts

| Attempt | Outcome | Cause | Tests executed |
| --- | --- | --- | --- |
| [1](attempt-v1/evidence/failed.json) | Failed during Cargo metadata | Runtime staged one directory too deep for the locked relative dependency | 0 |
| [2](attempt-v2/evidence/failed.json) | Failed during Cargo metadata | Selected MI350 cache lacked the locked crate versions | 0 |
| [3](attempt-v3/evidence/failed.json) | Failed during compilation | New private constructor reused an existing method name | 0 |
| [4](attempt-v4/evidence/failed.json) | Failed during test compilation | A test module needed an explicit path | 0 |
| [5](attempt-v5/evidence/failed.json) | Test process aborted | Large inline control structure overflowed the test-thread stack | 197 observed passes; no suite result |
| [6](attempt-v6/evidence/complete.json) | Passed | Heap-backed control records; unchanged wire format and test stack | 586 passed, 0 failed, 4 ignored |

The five failures have clean source and process postchecks. None produced the
final worker executable or ran GPU code. Attempt 5 built three test executables
before aborting; its unfinished test and missing suite totals remain explicit.
Their receipts, raw diagnostics and source
identities are retained unchanged; later attempts use fresh directories.

The previously qualified worker used an ASRock-local Cargo cache. The current
MI350 run must not assume that historical cache path exists. All 29 locked
registry archives and their sparse-index entries were found in the local
cache and verified against the unchanged worker lockfile. The passing attempt
uses a task-owned MI350 cache, without modifying a shared cache or lockfile.

The subsequent test run compiled all three test executables and confirmed the
590-name inventory, but aborted on a stack overflow in the four-forward CLI
test. The candidate now stores the 36 layer observations on the heap rather
than copying a roughly 243 KB inline control structure through nested calls.
It keeps the 242,824-byte wire format, exact 36-layer validation, all state
checks and the default test-thread stack size. The existing wire test also
checks the small control-structure size and rejects short or oversized layer
lists. Attempt 6 passes these tests with the default stack; attempt 5 remains
an aborted run, not a successful suite.

## Reproducible Pair

Use fe2o3 revision `e9ccc629cf1d445bda4d792276a1defb2e386d88` from
`codex/p228-finite-runtime-integration-v1`, beside the Ferric checkout. The
worker's existing `../../../fe2o3` dependency then resolves to the promoted
runtime. Those 29 cumulative runtime source files match the earlier
[full KFD/default-feature/API qualification](../guarded-mlp-model-interface-v1/README.md).
No thin-workspace Cargo manifests or lockfiles were promoted.

The complete canonical worker source tree matches all 181 worker rows in
[the passing source map](attempt-v6/evidence/sources-after.json). The retained
controller, input manifest, dependency maps and raw Cargo commands identify
the exact offline toolchain and build settings. Host executable bodies are
not committed. The final worker ELF is 5,727,168 bytes, SHA-256
`ac265ab2545a534c228edcb926458f95cca20243ba106b17eb2f60f58f6552ff`.
The CPU receipt SHA-256 is
`927e6519923ab44aa5f5616886ce9b539ed5b5f77804a48fe5d42f6a37974cd2`.

This validates the focused runtime/worker pair, not the entire fe2o3 workspace.
The new parent entry point retains its separate locked dependency graph and
is not yet qualified by this checkpoint.

## Demo Gates

1. Complete: compile and test the worker, including the 24 new tests.
2. Compile and test the parent guarded-decode entry point.
3. Run four real-model forwards, including both alternating bank reuses,
   with healthy worker shutdown and complete output evidence.
4. Compare model outputs against an independent reference before claiming
   numerical acceptance.
5. Measure the requested BF16 Qwen3-8B, single-request 2,048/256 workload.
   Host control durations are not GPU overlap traces or throughput results.

All issue #42 milestones remain open. The 700 tokens/s target is not met by
this checkpoint.
