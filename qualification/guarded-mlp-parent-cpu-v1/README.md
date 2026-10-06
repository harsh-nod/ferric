# Guarded Model Parent Qualification

This checkpoint retains actual MI350 parent-build attempts. The new guarded
parent is not yet qualified. Neither retained attempt ran tests or GPU code.
The separately qualified [worker](../guarded-mlp-worker-cpu-v1/README.md) does
not establish parent or full-model correctness.

| Attempt | Result | Diagnosis |
| --- | --- | --- |
| [1](attempt-v1/evidence/failed.json) | Cargo metadata failed | The private Git cache's default reference was not a branch accepted by the local checkout operation. |
| [2](attempt-v2/evidence/failed.json) | Metadata passed; test compilation refused | The harness selected the worker's Rust 1.96 nightly, but Ferric requires Rust 1.97.1. |

Attempt 1 has four naturally exited phases, ending with Cargo exit 101.
Attempt 2 has five, also ending with Cargo exit 101. Both have clean source,
cache and process postchecks, zero executed tests and no parent artifacts.
The records preserve original commands, diagnostics, source maps, dependency
identities and the 26 proposed parent/worker source bodies. They do not retain
compiler caches, model weights or host executable bodies.

The cache correction changes only three Git HEAD values and their reference
paths to `refs/heads/main`; the three locked commits and package bytes remain
unchanged. The next attempt uses the installed Rust 1.97.1 toolchain, whose
compiler, Cargo and rustdoc hashes match the historical parent toolchain.
No minimum-Rust-version override, source-version downgrade or lockfile edit is
used. Parent qualification still requires all 54 planned phases, 375 selected
passing tests, the 858-name library inventory and all five parent binaries.
This is a selected-test gate, not execution of the entire parent library suite.

The data-only [retention helper](retain.py) verifies capsule and per-file
SHA-256 identities before extracting ordinary bounded files. It never executes
code from an archive. The original V1 export helper encountered an
order-sensitive comparison of formatter paths; its separately retained V1b
correction compares the duplicate-free sets. It does not change the test
attempt or turn a failed build into a passing one.

| Capsule | Bytes | SHA-256 |
| --- | ---: | --- |
| Attempt 1 | 1198066 | `537010a52dd8522f6322214b04d382021b2fc52b85b6af68f053eee6147e6cad` |
| Attempt 2 | 2022193 | `7cfb22fd0d44a1b0af9aeda78cd1b3d4a59a2f16b2d4fe22254aab9da9ea80e9` |

Native model execution, independent numerical acceptance, sustained decode,
performance and production admission remain separate open gates.
