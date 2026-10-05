# Core Result Compiler Qualification

This checkpoint implements and tests narrow source-origin recognition for the
genuine core `Result::branch` and `FromResidual` wrappers observed on MI350.
The [measured bodies](../guarded-mlp-core-result-residual-diagnostic-v1/README.md)
are the basis for the checks, not an assumed source lowering. **Qualification
is incomplete. Ferric's dependency and production path remain unchanged.**

## Implementation

The proposed compiler checks genuine core identities, safe Rust signatures,
the finite provider-bound payload/error families, and every operation and
control-flow edge in the observed wrapper bodies. Its provider types remain
bound to the reviewed complete device-source closure. It recognizes neither
arbitrary core helpers nor arbitrary Result types.

The residual wrapper's discriminant assumption and actual `From<E>` call are
checked explicitly. The normal collector still visits that conversion and
applies its independent source-safety checks. The separate inline-origin audit
is unchanged. Diagnostic logging is absent from the production source.

Fifteen authored tests cover the family table, actual compiler callbacks and
172 mutations of real MIR bodies. Structural callback positives deliberately
use scalar families that the full provider authenticator still refuses; they
are not substitutes for provider-positive pipeline extraction. The bounded
controller additionally retains the previous provider cohorts, eight atomic
controls, two existing unsafe/inlined-origin rejection tests, and the unchanged
matrix and attention controls.

## Attempts

| Attempt | Source Revision | Actual Outcome |
| --- | --- | --- |
| [V1](attempt-v1/evidence/failed.json) | `cb234b949405b782e97768ee7a7f66a6dc889761` | Compiler build failed with E0529; no tests ran. |
| [V2](attempt-v2/evidence/failed.json) | `5112b3783e6547d89ca04e1449d3b55bf794ced3` | Production compiler built; test compilation failed with two E0277 diagnostics; no tests ran. |

V1's four phases exited naturally and were reaped; the first three returned
zero, then the compiler build returned 101. Integrity postchecks passed and
the source tree was unchanged. The structured compiler diagnostic is retained
in [Cargo output](attempt-v1/evidence/compiler-products.stdout).

The first error was attempting a Rust slice pattern on rustc's indexed aggregate
operands. V2 uses iterator results `(Some(operand), None)` to
retain the exact one-operand requirement. This is an API correction, not a
relaxed body predicate. The production compiler then built successfully in
40.376 seconds. Its separate test build found two `SwitchTargets::new` calls
that need explicit iterators. The next revision adds `.into_iter()` to the
two mutation-fixture arrays without changing their values or assertions.

All five V2 phases exited naturally and were reaped, with clean integrity
postchecks and unchanged sources. The first four returned zero; the test
build returned 101. Its structured diagnostics are retained in
[Cargo output](attempt-v2/evidence/compiler-tests-build.stdout). Actual test
execution and both pipeline controls remain pending.

The retention manifests pin 31 original V1 files and 36 original V2 files, including each
controller, manifest, raw evidence and four compiler source bodies. Failed
attempts remain failed. Provider qualification, guarded HSACO emission,
GPU/model correctness, sustained single-request BF16 2,048/256 and 700 tokens/s
remain open; no speedup or numerical acceptance is claimed.
