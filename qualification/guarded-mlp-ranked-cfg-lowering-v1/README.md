# Guarded MLP Ranked CFG Diagnostic Result

The qualified diagnostic compiler identifies the actual guarded gfx950
lowering rejection on MI350:

| Diagnostic Field | Observed Value |
| --- | --- |
| Kernel export | `ferric_qwen3_mlp_state_guard_v1` |
| Function role | `KernelRoot` |
| Declared control-flow blocks | 1,121 |
| Current block limit | 1,024 |
| Rejecting site | `loop-switch-inventory` |
| Reason | `above-limit` |
| Projection phase | `root-projection` |

The [raw compiler log](attempt-v1/evidence/compile.stderr) also retains the
function identity, source location and root/body indices. This is a measured
count for the state-guard kernel, not an attribution of each block to a
particular source operation.

The compiler exited naturally with status 1 after 117.362263861 seconds and
was reaped with an absent process group. Whole-run time was 120.964602764
seconds. There were no timeouts, forced cleanups or integrity postcheck errors;
sources and consumed inputs remained unchanged. **No HSACO was emitted and
no GPU kernel ran.** These durations are compilation timings, not decode speed.

## Changes And Next Work

This attempt binds the [new fourteen-check loader audit](../guarded-mlp-ranked-cfg-tool-audit-v1/README.md)
and [fully qualified diagnostic compiler](../guarded-mlp-ranked-cfg-diagnostic-v1/README.md).
The candidate Rust kernels, offline vendor tree, driver, optimized MIR
normalization and owned-process lifecycle remain unchanged. The 1,024-block
limit, atomic ordering and source-origin checks were not relaxed.

The next experiment is a separately reviewed, bounded increase in permitted
blocks while keeping the existing fact, edge and work budgets explicit.
Boundary tests and full compiler qualification must pass before retrying
lowering. This experiment has not yet run. The guard protocol and all 548
state checks must remain intact. The possible shared-state/guard alias issue
is still unestablished, not passed or failed by this diagnostic.

## Evidence

[attempt-v1](attempt-v1) retains the exact executed controller, failed receipt,
ten raw records and retention manifest. Receipt: 6,381 bytes, SHA-256
`e4547a1269e419952bba67914e76e69266261036ef8b4ae273180a0f08ffafc7`.
The 669,122-byte archive SHA-256 is
`3ccddd03dc23a8c5b68fb7b4da1f679dbd3f37af28bc706dfb6900306fc68d5f`;
all 13 members and 4,192,948 body bytes were verified after transfer.

[controller-tests-v1/attempt-v1](controller-tests-v1/attempt-v1) retains the
separate run of thirteen admission and four normalization fixtures. All
seventeen passed in one natural zero/reaped child, with unchanged inputs and
clean postchecks, in 1.585517 seconds whole. Receipt SHA-256:
`04e34661324f10e83767d3ca08943b8719aa7adbabd7a484accd5d3b3ac9c768`.
These synthetic fixtures are not an HSACO or GPU test.

The tested controller SHA-256 was
`6f91a4e547a0ae740d0b3cb7605cbb22c899c6178ab31b5f7425f3ee50bea795`.
Only its pending audit-receipt constant was subsequently bound to the actual
loader receipt. The executed controller SHA-256 is
`cf29a932d5475654709b82a13625ba3c55e54c76dd4022ca873819933a603f8a`.
Reversing that single binding reproduces the tested source exactly; the
seventeen fixtures are not claimed against the later bound hash.

All issue #42 milestones remain open. Guarded GPU behavior, independent model
numerics, sustained BF16 2,048/256 decode and 700 tokens/s remain unqualified.
