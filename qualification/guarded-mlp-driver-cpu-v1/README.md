# Guarded MLP Driver Qualification

On 2026-10-06 UTC, the refreshed `cargo-fe2o3` driver built and passed its
18 engineering-route tests on MI350. All seven phases exited naturally with
zero status and were reaped, with absent process groups, unchanged source and
dependency maps, and clean integrity postchecks. Whole-run time was 98.461342
seconds. This is CPU driver qualification, not HSACO emission or GPU execution.

The previous [guarded lowering attempt](../guarded-mlp-lowering-attempt-v2/README.md)
rejected a core atomic load without a concrete ordering argument. Its old
driver hardcoded minimal MIR normalization and did not support the newer
`--mir-normalization optimized-inline-v1` option. The existing passing atomic
extraction control uses inlining and optimization. Qualifying the newer driver
enables a matching retry without relaxing atomic-ordering or source-origin
checks. This explanation is source-grounded; it is not a captured MIR operand
from the rejected kernel.

## Scope And Products

The fresh workspace contains the 5,297 source files of published fe2o3 commit
`5a500d63c29b78f8788356b20dfcbb5c41ec20c9`, plus the controller and unchanged
owned-process supervisor. Its source map joins the previously qualified
[candidate dependency refresh](../guarded-mlp-dependency-refresh-v1/README.md).
Builds were offline and locked, on CPUs 8/9 with two Cargo jobs and GPUs hidden.

| Phase | Result |
| --- | --- |
| Rust toolchain identity and Cargo metadata | Both passed |
| Driver test-binary build | Passed |
| Full compiled and ignored inventories | 402 compiled names; 5 ignored names |
| Engineering-route tests | 18 passed; 0 failed; 0 ignored; 384 filtered |
| Final driver build | Passed |

The full driver test suite was **not executed**. The five ignored names are
inventory entries, not skipped executions in the focused run. An unrelated
full-suite test deliberately detaches a child from its process group; this
runner does not claim cleanup coverage for that test.

The deployable driver is selected from the final `driver-build.stdout` Cargo
artifact, not from the test build. Its size is 81,290,624 bytes and SHA-256 is
`3513cb03e8a8fc2ec54f5d65c5d8a620ca614fadfed3d65a991e5b4a5243db0a`.
The separate test ELF is 57,561,728 bytes with SHA-256
`a3a3ffaeeb7fcb3e4f6f07d4013049ab7becee72dd301ba3ff0060fbaa25b597`.
Neither executable body is committed here; exact Cargo records and pins are.

## Evidence And Limits

[attempt-v1](attempt-v1) retains 43 original bodies and a retention manifest:
both controller sources, input manifest, complete receipt, and 39 raw records.
The source maps inside the complete receipt are inline maps, not file pins.
The 4,150,183-byte receipt SHA-256 is
`a869f0f4aa0572c82492cb3d90bdb0dc9bb2625a7b441ff1f3e9a876a87079d9`.
The transfer archive was 3,450,693 bytes, SHA-256
`78c64bb7e4111ebf9c96a27e827f376185862935e10ca30d50d9aa448655e66f`;
all 44 members and 20,634,053 uncompressed bytes were verified after transfer.

Next is a fresh loader audit replacing only the driver, preserving the other
six tools and the full combined compiler qualification. Guarded gfx950
lowering, native guard behavior, model correctness, sustained 2,048/256 decode,
and the 700 tokens/s target remain unqualified. All issue #42 milestones stay
open, and production execution is unchanged.
