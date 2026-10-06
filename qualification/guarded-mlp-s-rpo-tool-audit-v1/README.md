# Runtime Compiler Loader Audit

The seven-tool deployment using the
[qualified runtime compiler](../guarded-mlp-s-rpo-qualification-v1/README.md)
passed its loader checks on MI350. This is an executable/library inspection,
not guarded-kernel compilation, HSACO acceptance, GPU execution or performance.

## Actual Results

All fourteen `readelf`/`ldd` commands exited naturally with status zero, were
reaped and left no process group. The complete audit took 2.259 seconds;
integrity postchecks were clean. The extractor resolves the exact deployed
new backend, and the resolved libraries/interpreters are recorded and
posthashed. There were no timeouts or forced cleanups.

| Deployed Tool | Change From Previous Deployment |
| --- | --- |
| `fe2o3-rustc-extract` | Final qualified compiler build |
| `librustc_codegen_fe2o3.so` | Final qualified compiler build |
| `cargo-fe2o3` | Unchanged |
| `clang-22` | Unchanged |
| `fe2o3-engineering-lld-proxy` | Unchanged |
| `fe2o3-llvm-link-worker` | Unchanged |
| `lld` | Unchanged |

The new two products are selected from the producer's final
`compiler-tests-build` Cargo records, not its earlier build observation.
The audit joins the actual successful producer receipt, input, source map
and final Cargo stream. The backend rlib and four CPU test executables are
retained as metadata provenance only; they are not deployed or inspected
as part of this seven-tool audit. Their bodies are not rehashed here.

Ten separate [synthetic admission tests](controller-tests-v1/attempt-v1/evidence/complete.json)
also passed on MI350. They check failed/incomplete producer refusal,
source/phase/product identity, final Cargo selection and the two-new/five-old
tool policy. They do not replace the fourteen actual inspection commands.

## Evidence And Limits

The [complete receipt](attempt-v1/evidence/complete.json) has SHA-256
`66c3b4d0bfca1f58990cb4b8dba883c05f4bb7c1723ff4032b28b349bcca4187`.
The [retention manifest](attempt-v1/retention-manifest.json) pins 81 original
files, including 72 raw inspection/lifecycle records, the controller, manifests
and consumed producer metadata. No executable bodies are exported.

Inspection used a controlled library path, CPU cores 8/9 and nice 10, with
30-second leaves, a 600-second whole bound and a 50-second cleanup reserve.
The inspection library path must not leak into managed device compilation.
The source and dependency maps are authenticated producer metadata, not a
second full CPU qualification. This check grants no load/launch authority.

Fresh offline dependencies and guarded gfx950 lowering remain separate
prerequisites to ABI/ISA inspection and GPU validation. All issue #42
milestones, independent model numerics and the 700 tokens/s target remain open.
