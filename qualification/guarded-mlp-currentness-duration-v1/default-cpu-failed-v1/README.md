# Original Default CPU Attempt Failure

The first MI350 qualification of the optional currentness-duration diagnostic
stopped at `worker-tests-build`. The stable Rust compiler rejected a `cfg`
attribute directly on an assignment expression with E0658. The runtime suite
passed 1,180 tests with eight ignored; all ten focused scopes and ten facade
doctests passed. Worker tests and the final worker build did not run.

All 23 phase processes were reaped and their process groups absent. The first
22 phases exited zero; Cargo exited 101 in the final phase. Postcheck errors
are empty. There is no successful worker executable or GPU qualification here.
The separately staged diagnostic-mode attempt was not run.

The [original failed receipt](evidence/failed.json) is 2,267,201 bytes, SHA-256
`a3f48e67451287afca3ee91000209a2c3d9c65b693659a4afa73882ab2615d8b`.
The original archive is 1,755,405 bytes, SHA-256
`b1adcb8d14ab5284d5ceaebcdf52e1a44a0ff1088d38edde39806f9deb7ec660`:
172 members, 171 manifest-pinned bodies, 12,475,437 expanded bytes. It preserves
122 raw files, the original terminal, selected final sources and lineage.

Export on MI350 rehashed all 1,069 live sources, tool and dependency inputs,
and the six recorded runtime products. Executable bodies are not in the
capsule. Local extraction checked the original archive and every member pin;
an independent data-only review checked the phase and compiler-error joins.
No retained project code was executed locally.

The repair puts the same assignment inside a conditionally compiled block.
A fresh qualification must establish its result; this attempt remains failed.
This README is commentary added after original-only retention, not a member
of the original archive or a replacement test result.
