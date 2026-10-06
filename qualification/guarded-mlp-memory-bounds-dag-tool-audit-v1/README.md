# Memory-Bounds DAG Binary Audit

Status: all 14 binary inspection and dynamic-dependency checks pass on MI350
for the [CPU-qualified DAG compiler](../guarded-mlp-memory-bounds-dag-v1/README.md).
This is not a guarded-kernel compile, GPU execution or numerical acceptance.

The [actual receipt](attempt-v1/evidence/complete.json) reports 14 naturally
completed, reaped checks with no remaining process groups and clean postchecks.
The audit takes 2.541485 seconds. It authenticates the current CPU qualification
and the independently pinned historical tool and driver lineage.

| Tool | ELF Inspection | Dynamic Dependencies |
| --- | --- | --- |
| cargo-fe2o3 | Pass | Pass |
| clang-22 | Pass | Pass |
| fe2o3-engineering-lld-proxy | Pass | Pass |
| fe2o3-llvm-link-worker | Pass | Pass |
| fe2o3-rustc-extract | Pass | Pass |
| librustc_codegen_fe2o3.so | Pass | Pass |
| lld | Pass | Pass |

Only the extractor and backend are sourced from the final qualified compiler
build. The other five tool bodies remain byte-identical to their pinned
predecessors. The extractor itself is also byte-identical, but its provenance
is the new qualified build. The current backend is 206,535,552 bytes, SHA-256
`1b2c0ab159ef3193febe0b4c1e85fd3c438cbfac1a51a360b5ba628c34a9b806`.

The [controller fixtures](controller-tests-v1/README.md) separately pass all
26 tests. The selected actual-audit evidence capsule has 101 members, 100 pins
and 72 raw files: 4,328,804 compressed bytes, SHA-256
`c6998422a3ab7afa40623d485d5a11376798feb4532f8d87b321f7a1b2dc1fd0`.
It retains selected provenance, controller and inspection outputs, not the
tool binaries or the entire transitive readset. The actual receipt is 165,663
bytes, SHA-256 `bbcfc8228c4b8a34bfe1da740662bbee8fe81c878f8bfae5d1be0be699208d76`.
