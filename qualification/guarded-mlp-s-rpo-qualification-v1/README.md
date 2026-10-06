# Full Runtime Compiler Qualification

The combined runtime compiler and reverse-postorder (RPO) dataflow source
passed its full CPU qualification on MI350. This joins the previously tested
ordinary-induction/runtime changes with the exact core-wrapper recognizers
from the [445-test scoped run](../guarded-mlp-core-kernel-error-identity-qualification-v1/README.md).
It does not yet qualify guarded gfx950 HSACO, GPU execution, independent
model numerics or performance. All issue #42 milestones remain open.

## Actual Results

The [complete receipt](attempt-v1/evidence/complete.json) records 32 naturally
successful, reaped phases and 19 test scopes in 643.960 seconds. Every process
group was absent afterward; there were no timeouts, forced cleanups or
integrity postcheck errors. Source and dependency maps remained unchanged.

| Scope | Passing Executions | Historical Ignores |
| --- | ---: | ---: |
| Full compiler library | 1,239 | 24 |
| Full pliron library | 1,504 | 1 |
| Five focused core-wrapper cohorts, repeated | 43 | 0 |
| Selected atomic extraction controls | 8 | 0 |
| Unsafe-source rejection controls | 2 | 0 |
| Matrix and attention extraction | 2 | 0 |
| Total | 2,798 | 25 |

The total counts test executions, not unique tests: the 43 focused checks
repeat tests in the compiler suite. The historical passing and ignored
identities are checked against retained prior inventories; the 25 ignores
are not counted as passes. All seventeen prior RPO controls remain covered.
The twelve selected extraction/rejection controls are invoked explicitly.

The compiler and pliron suites took 68.838 and 25.058 seconds respectively.
Matrix and attention extraction took 21.780 and 42.827 seconds. These are
CPU qualification timings, not kernel or decode performance. Matrix and
attention controls target gfx942 LLVM; selected atomic controls check gfx950
LLVM. None of these results establishes a guarded gfx950 executable or launch.

## Source And Product Provenance

The input pins 5,793 compiler source bodies plus the controller and helper.
The assembled source preserves the runtime compiler's existing changes,
applies the four qualified RPO files, and introduces ten exact core-wrapper
files through two reviewed insertion points. It is a derived source tree,
not the separately published fe2o3 `5a500d63` commit. Prior source maps,
insertion preimages, patch, donor receipts and inventories are retained.

The dependency graph must match the earlier RPO graph after four explicit
path relocations. No general path normalization or dependency substitution
is allowed. The nightly Rust library map contains 2,247 verified files.

Seven final build products are recorded and posthashed: extractor, backend,
backend rlib, and four test executables. The final compiler products come
from `compiler-tests-build`, not the earlier production-build observation.
Earlier product hashes are historical observations, not final qualified
products. The rlib is build provenance, not a deployable ELF tool.

## Evidence And Reproduction

The [retention manifest](attempt-v1/retention-manifest.json) pins 202 original
files: 164 raw records, sixteen compiler/RPO source bodies, lineage inputs,
controller, helper, exporter, input manifest and receipt. Binary bodies are
not included. The complete receipt SHA-256 is
`e30712ef17c7094efce9bd13bee2bb172823e94f4817c812cfb8b1b20cefb847`.
Commands, environments and lifecycle records are retained per phase.

Builds ran offline and locked on CPU cores 8/9, with two Cargo jobs, GPUs
hidden and bounded process, memory, output and scratch lifetimes. The
[eleven controller tests](controller-tests-v1/attempt-v1/evidence/complete.json)
also passed separately on MI350, checking insertion, relocation, cohort and
context handling. These synthetic tests are not compiler-test passes. The
[sixteen helper tests](../guarded-mlp-s-producer-helper-tests-v1/README.md)
remain a separate earlier checkpoint.

Next are loader inspection of the actual final compiler binaries, the
dependency-updated guarded candidate and fresh offline vendoring, then
guarded gfx950 lowering. The GPU worker, independent model validation and
sustained single-request Qwen3-8B BF16 2,048/256 benchmark remain open.
The 700 tokens/s target has not been reached.
