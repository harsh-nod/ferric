# Layer Native Admission CPU V2

This fresh qualification retry for the layer-duration native admission
plumbing passed all 304 tests. The first attempt remains preserved as failed.
This is CPU workflow evidence, not a native GPU measurement.

## First Attempt

The original V1 controller completed after 113.07579161401372 seconds with a
failure in `LayerCpuAdmissionTests.setUpClass`. The four inherited leaves
passed 227 tests: legacy 171, forward CPU admission 29, forward retention 12
and forward preparation 15. The new layer CPU test class ran zero tests
because its fixture admission failed; neither later layer leaf ran.

All five launched leaves exited naturally, were reaped and had absent
process groups. The fifth exit code was 1. There were no postcheck errors,
and the source map was unchanged. Natural process retirement does not make
this a successful qualification.

Original failed receipt:

- Remote root: `guarded-mlp-readiness40-tail-layer-duration-native-admission-cpu-v228-v1`.
- Body: `evidence/failed.json`, 341,718 bytes.
- SHA-256: `f87ba11ba93f22308869ae3a41ffd04b038f8601194fb3c8f5c85ddda8af4ff2`.

The [original failed archive](../native-admission-cpu-failed-v1.tar.gz) is
9,022,745 bytes with SHA-256
`49dabea8a4afaaa20506ecfa378f65893a4928177af2cd1307723e30f1568216`.

The failure was an admission-helper expectation error, not a Rust test or
native model failure. The runtime Cargo metadata command is issued from
the workspace and requests the exact feature vector:

`fe2o3-kfd/engineering-currentness-duration-diagnostics,fe2o3-kfd/engineering-gfx950`

The helper incorrectly expected unqualified feature names. The original
command body is 2,526 bytes with SHA-256
`b37f760f468e3563b676beb580f6cb8ee09c44705a0629a862672eda2642fe38`.
Its workspace manifest, feature order, offline/locked arguments and
120-second command bound remain unchanged.

## Narrow Repair

Only the runtime requested-feature expectation changed. It now matches the
original workspace-qualified arguments exactly. Resolved Cargo features,
the full qualified phase argv, source and artifact identities, resource
limits, currentness policy and numerical checks were not relaxed.

The new regression
`LayerCpuAdmissionTests.test_runtime_requested_features_match_exact_workspace_command`
calls the real metadata predicate. It accepts the original command and
rejects bare, mixed and incorrect package prefixes. All 42 existing layer
CPU test methods remain unchanged. The preparer, runner and preparation
test fixture carry only the corrected helper hash.

The repaired helper is 32,495 bytes with SHA-256
`f1105fbef6aad492ef5f32bb7613a4f609134df9395cb1b2416c24e845109d28`.
Original V1 source and failure evidence are not overwritten.

## Retry Status

The fresh root is
`guarded-mlp-readiness40-tail-layer-duration-native-admission-cpu-v228-v2`.
Its staged input manifest is 230,625 bytes with SHA-256
`041543c84df2255a7db7c7bf8d77730ba01d7b23bc478b5caed073be031d9a42`.

| Field | Observed Result |
| --- | --- |
| Original retry terminal | [complete.json](complete.json), 390,455 bytes |
| Process retirement | Seven natural exit-0 leaves, reaped, process groups absent |
| Named test census | 304 passed, 0 failed, 0 errors, 0 skipped |
| Source and postchecks | All 376 source/fixture bodies unchanged; no postcheck errors |
| Elapsed time | 149.409749477054 seconds |
| Original evidence archive | [native-admission-cpu-v2.tar.gz](../native-admission-cpu-v2.tar.gz), 9,034,439 bytes, 417 members |

The original terminal SHA-256 is
`c498645a2a28b140add51448a5d38838436447c91c32fbf70fda3d6790f51234`.
The original archive SHA-256 is
`c33f49214d3b9806ce7fe20abfc0b0b5a552e7655f91b3eadfd0e30418de4e03`.
Its 416 pinned originals retain all 376 source/fixture bodies, the input
manifest, 37 raw evidence files, original terminal and authoring stager;
the archive manifest is the final member. The published directory mirrors
only this README and the exact original terminal, not another source tree.

The observed census is 304 tests across seven separately supervised leaves:
171 legacy, 29 forward CPU admission, 12 forward retention, 15 forward
preparation, 43 layer CPU admission, 16 layer retention and 18 layer
preparation. The unchanged predecessor workflow contributes 227 tests;
the three new layer suites contribute 77.

This workflow uses mixed actual-artifact-backed and synthetic data tests.
The admission and preparation paths authenticate original CPU receipts,
source maps and selected ELF bytes without running those executables.
Synthetic retention fixtures exercise original-byte custody, successful,
failed and absent outcomes, and strict V2 record admission. Host/live-tree
export remains separate from those synthetic retention tests.

## Scope and Bounds

The retry retains GPU-hidden execution on CPUs 8 and 9 at nice 10,
512 MiB address space, a 64 MiB private temporary directory bound,
120-second leaf bounds, a 1,000-second whole deadline and a 50-second cleanup
reserve. It used a closed set of 376 source/fixture bodies. Neither this
document nor the qualified CPU packet grants native launch authority.

The separately qualified 178-test checker suite overlaps inherited tests
in this workflow. Their totals must not be added as a count of distinct
tests. The V2 wire still retains all three original stderr records and
the unchanged whole-stderr cap; individual layer-return containment is
checked by the qualified Rust path, not reconstructed from aggregate wire
rows.

No GPU timing, overlap, speedup, generated-token correctness or Full2303
acceptance is established by this CPU qualification. A retained layer-duration
native GPU result remains pending and will require separate original evidence.
