# Guarded MLP Compiler Tools: MI350 Loader Audit

Engineering prerequisite for [issue #42](https://github.com/harsh-nod/ferric/issues/42),
performed on MI350 on 2026-10-05. This checkpoint verifies deployment and dynamic
library resolution, not compilation, HSACO emission, GPU execution or performance.

## Result

All seven deployed tools passed `readelf` and `ldd` inspection. All fourteen
bounded commands exited naturally with status zero and were reaped; their process
groups were absent afterward. Thirty input bodies and twenty-one resolved paths
passed integrity postchecks. The extractor resolves the exact deployed backend.

| Tool | Loader Audit |
| --- | --- |
| `cargo-fe2o3` | Pass |
| `fe2o3-rustc-extract` | Pass |
| `librustc_codegen_fe2o3.so` | Pass |
| `fe2o3-llvm-link-worker` | Pass |
| `fe2o3-engineering-lld-proxy` | Pass |
| `clang-22` | Pass |
| `lld` | Pass |

The [tool manifest](tool-manifest.json), [complete receipt](retained/complete.json),
raw commands and logs, controller and [summary](result.json) retain the exact
identities. These transferred tools have different source generations; loader
compatibility does not establish their end-to-end compiler compatibility.

## Retained Failure

The [first audit](attempt-v1/failed.json) omitted the deployed tools directory
from its loader search path. Ten commands completed, but the extractor could not
resolve `librustc_codegen_fe2o3.so`. The second audit adds that directory alongside
the pinned nightly library directory and verifies the backend's exact identity.
No binary or installed system library was replaced to make the audit pass.
The compiler driver must establish its own controlled loader environment during
the subsequent lowering run; this audit's environment is not inherited as proof.

## Shared-Host Cleanup

After preserving their sources and evidence and checking that their process
groups were absent, the two failed CPU attempts' owned `target` and `tmp`
directories were removed. The [cleanup receipt](cleanup.json) records
1,046,052,049 logical bytes reclaimed. The successful CPU build and its products
remain available for the lowering input checks. No unrelated data was removed.

## Next Gates

Prepare locked offline dependencies, run checked gfx950 lowering, inspect actual
ABI and ISA, and execute valid and invalid guard controls. Reusable arena and
full-worker validation follow separately. All M0-M7, independent numerical,
sustained 2,048/256 and 700 tokens/s gates remain open.
