# DAG-Qualified Guarded Lowering

Status: the actual guarded compile still fails. The CPU-qualified DAG solver
clears the previous memory-bounds preflight refusal, and compilation reaches
allocation alias analysis. No HSACO, GPU execution, numerical acceptance or
performance result is produced. All issue #42 milestones remain open.

## Actual Failure

The [original stderr](attempt-v1/evidence/compile.stderr) identifies
`ferric_qwen3_mlp_state_guard_v1` and reports:

```text
error[FE2O3-RACE-002]: allocation alias analysis is incomplete: potentially aliasing class 1 in Global memory contains 2 distinct allocation origins (first Some(1)); ranked IR does not retain their relative base offsets
```

This is a missing-proof refusal, not an observed GPU data race. The two
rendered views have allocation origins 1 and 2 in the same potentially aliasing
class. The kernel reads 548 state words with Acquire/System atomics and
publishes four guard words with three Relaxed/System writes followed by one
Release/System write. The analysis cannot establish the relationship between
the two bases from the retained ranked metadata.

Independent comparison finds the rendered graph byte-identical to the
[previous memory-bounds failure](../guarded-mlp-ranked-cfg-linear-fusion-lowering-v1/README.md):
567 blocks, 1,122 raw edges, 2,240 operations and 552 bounds guards. The graph
and atomic effects did not change between these attempts; the rejecting
analysis changed. This attribution is a separate inspection of retained raw
data, not a rewrite of the controller's predeclared diagnostic flags.

The exact graph slice starts immediately after the line
`  = ranked PLIRON before rejected lowering:` and ends immediately before
`  = lowering stopped before target IR or artifact emission`, retaining the
closing function line's newline. Both stderr bodies contain the same 115,155
bytes, SHA-256
`3e884e8c85bfdd71b2f734c43813274462a3b963e351a13fe1c866bf850ae1cb`.

## Qualification And Evidence

The compiler passed [45 phases / 32 scopes / 2,990 passing executions](../guarded-mlp-memory-bounds-dag-v1/README.md).
All [14 actual binary-loader checks](../guarded-mlp-memory-bounds-dag-tool-audit-v1/README.md)
and [19 lowering controller fixtures](controller-tests-v1/README.md) also pass.

The [actual receipt](attempt-v1/evidence/failed.json) records a natural exit 1
after 132.322763 seconds for the compiler child, reaped with no remaining
process group. The whole attempt takes 136.065298 seconds. Sources and input
byte maps are unchanged, all postchecks are clean, and no timeout or forced
cleanup occurs. The artifact field remains null.

The selected evidence capsule has 13 members, 12 pins and 10 raw files. Its
compressed archive is 693,591 bytes, SHA-256
`831b3975791f5d6a71d9efd1df3e83ad1f20db62e9a8d4c473f253c035e90057`.
The original receipt is 19,105 bytes, SHA-256
`1943926bd4d138d79505f6d0a4d97722d7efa54af6323cccab319be9d53c9140`.
The original stderr is 123,593 bytes, SHA-256
`b7c4435210a46414e49ed798624eff224c0436935a51d6715eccd8ee61c72f0a`.

Next work compares a sound compatible-atomic alias proof with an explicit
single-allocation state/guard contract. Neither solution is accepted yet;
no alias fact will be invented and no safety check or resource ceiling will
be weakened to admit this kernel.
