# Guarded MLP Refreshed Driver Loader Audit

On 2026-10-06 UTC, the seven-tool compiler deployment with the newly qualified
driver passed all fourteen bounded loader inspections on MI350. Every leaf
exited naturally with zero status and was reaped with an absent process group.
Input/tool/library postchecks were clean. Whole-run time was 2.306678 seconds.

Only `cargo-fe2o3` changes from the
[previous deployment](../guarded-mlp-s-rpo-tool-audit-v1/README.md).
Its source and final Cargo artifact join the separately passed
[18-test driver qualification](../guarded-mlp-driver-cpu-v1/README.md).
The other six manifest entries are identical, preserving the combined S/RPO
extractor/backend and their full qualification provenance. The extractor still
resolves the exact deployed backend. This is loader inspection, not a compiler
invocation, HSACO qualification, GPU launch or production admission.

## Checks And Evidence

| Check | Observed result |
| --- | --- |
| `readelf -l -d` for seven tools | 7 passed |
| `ldd` and resolved dependency identity for seven tools | 7 passed |
| Full compiler producer/source/final-Cargo joins | Preserved |
| New driver producer/source/final-Cargo joins | Passed |
| Six unchanged tool manifest entries | Exact match |
| Synthetic loader-admission fixtures, separate run | 18 passed |

The driver test ELF and compiler rlib remain metadata-only provenance products;
their bodies were not deployed or inspected by `ldd`. Driver qualification
covers exactly 18 engineering-route tests, not its full 402-name compiled suite.
The separate synthetic fixtures cover the prior ten producer checks and eight
new driver checks, including wrong scope, source, phase, artifact and tool
substitutions. They do not themselves execute a compiler or loader inspection.

[attempt-v1](attempt-v1) retains 87 original bodies plus its retention manifest,
including all 72 raw records, both producer readsets, controller, input and
three tool manifests. Receipt: 110,229 bytes, SHA-256
`d79c1b846be0f8a9a466d15a6d662949a761bdb1397771a0a47b9699468439f0`.
New seven-tool manifest SHA-256:
`b38ef216900209ea59fbd53a1cb571447f983eb8984ca4a7ce57c1fb10f521fb`.
The 2,194,715-byte archive SHA-256 is
`1e9ac9b6c75a77d4e2908940158f67c56f33f4b83884bfbe5d08b16c8f869490`;
all 88 members and 12,886,885 body bytes were verified after transfer.

[controller-tests-v1/attempt-v1](controller-tests-v1/attempt-v1) retains the
18-fixture run: one natural zero/reaped child, unchanged sources and clean
postchecks, 2.819481 seconds whole. Receipt SHA-256:
`fa01a5a7316ca30176946d4fc477d6a6c194b0267300ec153dd776caa66f0937`.

The next lowering attempt can explicitly select optimized MIR inlining with
this driver. Guarded gfx950 HSACO emission and native behavior, independent
model correctness, sustained decode and the 700 tokens/s target remain open.
No issue #42 milestone or production execution path changes here.
