# Guarded Model Parent Qualification

The selected parent CPU gate passes on MI350: **375 tests passed, zero failed
or ignored**, across 54 naturally completed phases. The 858-name library
inventory is preserved, all five selected parent binaries build, and the
default-feature library check passes. This is not execution of the entire
parent library suite, GPU acceptance or full-model numerical correctness.
The [worker](../guarded-mlp-worker-cpu-v1/README.md) has separate qualification.

| Attempt | Result | Diagnosis |
| --- | --- | --- |
| [1](attempt-v1/evidence/failed.json) | Cargo metadata failed | The private Git cache's default reference was not a branch accepted by the local checkout operation. |
| [2](attempt-v2/evidence/failed.json) | Metadata passed; test compilation refused | The harness selected the worker's Rust 1.96 nightly, but Ferric requires Rust 1.97.1. |
| [3](attempt-v3/evidence/complete.json) | Passed all 54 phases and 375 selected tests | Bound the observed Rust 1.97.1 toolchain; no source-version downgrade or lockfile change. |

Attempt 1 has four naturally exited phases, ending with Cargo exit 101.
Attempt 2 has five, also ending with Cargo exit 101. Both have clean source,
cache and process postchecks, zero executed tests and no parent artifacts.
The records preserve original commands, diagnostics, source maps, dependency
identities and the 26 proposed parent/worker source bodies. They do not retain
compiler caches, model weights or host executable bodies.

The cache correction changes only three Git HEAD values and their reference
paths to `refs/heads/main`; the three locked commits and package bytes remain
unchanged. The passing attempt uses the installed Rust 1.97.1 toolchain, whose
compiler, Cargo and rustdoc hashes match the historical parent toolchain.
No minimum-Rust-version override, source-version downgrade or lockfile edit is
used. All selected tests, binaries and postchecks passed. The 390.51-second
build/test duration is not an inference-performance measurement.

The new guarded parent ELF is 13,566,456 bytes, SHA-256
`786c045ebd0565dfb26eac8240aea1af3956a912db692b4406e266621b1f161b`.
The CPU receipt is 3,697,764 bytes, SHA-256
`cfe7adb9e0d375191f41c37e36c9712eeed72c1b7879cd6f96a69ba2359e1c2a`.
The successful capsule retains 274 original evidence files, 97 lineage bodies
and all 26 proposed source bodies, without executable or dependency bodies.

## Source Integration

All seven parent source files are integrated byte-for-byte from the passing
post-format snapshot. The opt-in `guarded-mlp-model-engineering` feature adds
`ferric-qwen3-finite-guarded-mlp-decode-engineering`; existing parent routes and
the lockfile remain unchanged.

The [promotion plan](promotion-plan.json) records pre-integration checks and
is not itself an execution receipt. The parent qualification used eight
pre-format worker files while the canonical worker retains its separately
qualified post-format bodies. Only two of those files are shared with the
parent: the guarded protocol and its tests. An actual MI350
[formatter-only comparison](shared-wire-format-v1/complete.json) confirms
that formatting both parent inputs with the worker's pinned rustfmt produces
byte-identical worker bodies. Neither source tree was changed by that check.
This establishes formatter equivalence, not a second compilation or a claim
that the entire canonical tree is byte-identical to the parent build tree.
The other six differences are worker-private files not imported by the parent.

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
| Attempt 3 | 2180286 | `cb62c38415dc20dee5cc6f6147ec0de07f8c6327a44e910e62b0005bf5d4373b` |

Native model execution, independent numerical acceptance, sustained decode,
performance and production admission remain separate open gates.
