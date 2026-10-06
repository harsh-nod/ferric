# Chain Fusion Binary Audit

Status: all 14 binary inspection and dynamic-dependency checks pass on MI350
for the [fully qualified chain-fusion compiler](../guarded-mlp-ranked-cfg-linear-fusion-v1/README.md).
This is not a guarded-kernel compile, GPU execution or numerical acceptance.

The [actual receipt](attempt-v3/evidence/complete.json) reports 14 naturally
completed, reaped checks with no remaining process groups and clean postchecks.
The audit takes 2.450390 seconds. It authenticates the complete current CPU
qualification and the independently pinned historical tool and driver lineage.

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
is the new qualified build. The current backend is 206,470,200 bytes, SHA-256
`c7facc39cb88f4ddae11e2bd522a5e0298dd3b38c64fe112180d1397c1aa9210`.

The [controller fixtures](controller-tests-v1/attempt-v3/evidence/complete.json)
separately pass all 26 tests, including current-versus-historical helper
identity, malformed provenance, omitted phases and changed-tool refusals.

The selected evidence capsule has 100 members, 99 pins and 72 raw files:
4,128,239 compressed bytes, SHA-256
`936f8aae165b68da28855ca3ead6d65254966366383489fcf2d9fb56b2f86278`.
It retains the selected provenance, controller and inspection outputs, not the
tool binaries or the entire transitive readset. The actual receipt is 163,135
bytes, SHA-256 `8c6864383f1c6ba0c8e05c8e2692aac8c095335764728fb7ac24a4f41b7fb792`.
