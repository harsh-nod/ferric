# RoPE Eager Validity Checks: CPU Qualification

The branch-reduced RoPE candidate passed 33 Rust tests on ASROCK through
`mi350-2`. All eleven phases completed naturally and were reaped; source,
dependency, configuration and provider postchecks passed. This remains a CPU
arithmetic qualification, not checked GPU compilation or numerical acceptance.

## Change From V1

The [previous checked-lowering attempt](../rope-materialized-lowering-attempt-v1/README.md)
hit the compiler's existing semantic SSA storage limit. This candidate changes
only the 17 `&&` operators connecting the pair-validity expression's eighteen
finite predicates to boolean `&` in the
[RoPE macro](fixture/src/prefix_rope_materialized_numerics_v1.rs).

All operands are pure checks on already-computed values. Eager conjunction
therefore retains the same validity result while avoiding short-circuit
control-flow joins. No finite predicate, BF16 boundary, arithmetic operation,
task read/write, rejection path or compiler limit is removed or changed. The
kernel entry and all twenty RoPE test bodies are byte-identical to V1.

The [publication record](result.json) checks this exact full-body transformation
against the retained [predecessor](predecessor). The subsequent
[checked-lowering attempt](../rope-materialized-lowering-attempt-v2/README.md)
hit the same semantic SSA partial-move storage limit. This change was insufficient
to produce a GPU image. No GPU speedup is inferred from the source change or
CPU tests.

## Actual Tests

| Invocation | Passed | Ignored |
| --- | ---: | ---: |
| RoPE materialization | 20 | 0 |
| Ordinary reciprocal regression | 12 | 1 |
| Explicit exhaustive reciprocal | 1 | 0 |

The ordinary ignored case is the same exhaustive test explicitly selected and
passed separately. The total is 33, not 34. Rounding ties, signed zero,
cancellation, subnormals, overflow, nonfinite rejection, all lanes/heads and
unchanged value bits remain covered by the existing
[test suite](fixture/tests/rope_materialized_v1.rs).

The numerical harness uses opt2 with debug assertions and overflow checks,
offline locked dependencies, two low-priority CPU cores and hidden GPUs. The
kernel entry is source-contract checked but is not host-compiled by this harness.

## Evidence and Scope

[complete.json](complete.json) and [raw](raw) retain the actual qualification.
All 61 raw records were rehashed during publication; 59 are copied here. The
two dependency snapshots and test executable bodies remain outside Git; the
publication explicitly distinguishes remote executable pins from locally
rehashed bodies. No tests were rerun locally for publication.

The full 15-file [fixture](fixture) retains the numerical Cargo harness;
[lowering-fixture](lowering-fixture) holds the original checked-entry Cargo pair.
These manifests preserve the qualified ASROCK provider paths. Reproduction
requirements are in the [controller recipe](controller/README.md).

Actual CPU completion SHA-256:
`8dcb4f2b326ab909c52039273515f44eb4f88cd91a967c8816f866d5514c864a`.

Checked gfx950 lowering, image inspection, native execution and independent
model comparison remain separate gates. Native rotary-table equality, GPU
denormal behavior, sustained 2,048/256 decoding and 700 tokens/s are not
established. The default device crate and production route remain unchanged.
