# Graph-Work Diagnostic Guarded Lowering

This controller prepares a guarded gfx950 compile using the
[diagnostic compiler generation](../guarded-mlp-ranked-graph-work-diagnostic-v1/README.md).
It requires that successful compiler qualification and a fresh
[binary-loader inspection](../guarded-mlp-ranked-graph-work-diagnostic-tool-audit-v1/README.md).
The guarded sources, optimized-inlining profile and independent resource
limits remain unchanged.

## Actual Lowering Result

The actual guarded gfx950 compile still fails at the unchanged graph-work
limit, but the new diagnostic identifies the operation and function:

| Field | Measured value |
| --- | --- |
| Kernel | `ferric_qwen3_mlp_state_guard_v1` |
| Semantic root / body / function index | `1` / `1` / `1` |
| Phase | Root projection |
| Charge site | `indexed_atomic_v1.rs:56:14` |
| Work before charge | 3,144,193 |
| Requested charge | 1,656 |
| Attempted sum | 3,145,849 |
| Work limit | 3,145,728 |

The measured site is `AuthenticatedAtomicAllocationsV1::contains_local`,
which charges the entire `indexed_locals` length before a linear membership
scan. Source inspection shows that the constructor already sorts and
deduplicates this list before the escape checks query it. The next scoped
optimization is a bounded logarithmic lookup over that existing sorted list,
with differential membership and accounting tests. It must retain escape,
origin, alias and atomic predicates and the existing work/storage limits.
This is not the definition-block scan previously considered as a possibility.

The attempted sum is diagnostic information, not a committed atomic-inventory
counter: the forwarding method updates its `Cell` only after a successful
charge. The controller's generic `actual_failure_caller_identified=false`
field is preserved verbatim; the attribution above is a reader's interpretation
of the retained raw compiler diagnostic, not a rewritten receipt.

The compiler exited naturally with status one after 117.449253 seconds; the
whole attempt took 121.097447 seconds. It was reaped and left no process group.
Sources remained unchanged and input postchecks were clean, with no timeout
or forced cleanup. No HSACO or GPU execution resulted. This refusal does not
establish whether the remaining alias checks will pass.

[attempt-v2](attempt-v2) retains the failed receipt, exact bound controller
and all ten raw records. The archive contains 13 members, 12 pinned bodies
and 4,212,851 body bytes. Receipt SHA-256:
`7727462e3e8f3744e22926546da24ec9eec2e55a6f7b9c3fcf6036168ae0a52e`.
Archive SHA-256:
`4813a8600a545ea4d545bf0223ec631a8a5765838269d6d51e489c7c4f88d634`.
The 8,533-byte `compile.stderr` SHA-256 is
`a52d57b5b340b8b7223d35bfaa441dcf88c0febc74bc3924925647cf1d91e99c`.

## Controller Tests

All 19 synthetic tests passed on `mi350` in 4.386260 seconds: 15 admission
tests and four command/output-normalization tests. The child exited naturally
with status zero, was reaped and left no process group. Source and tool
postchecks passed, with no timeout or forced cleanup.

The fixtures reject earlier producer generations, changed limits, incorrect
diagnostic flags, stale final products and mismatched loader lineage.
Missing actual receipt bindings refuse before helper or output effects.
The flags test also inspects the frozen controller's actual output
declarations, so the diagnostic cannot claim a new graph optimization or
changed resource admission.

[controller-tests-v1/attempt-v2](controller-tests-v1/attempt-v2) retains the
executed sources, input manifest, receipt and all seven raw records. Its
archive contains 16 members, 15 pinned bodies and 167,441 body bytes.
Receipt SHA-256:
`2c01a4e073c6f5dcda7a3967ce97aa3451e2e474508fcd1c87d4a02d2418202c`.
Archive SHA-256:
`907c8f22a020aa9e56656ed07abde7b82f2b64de6a8eb91f25ab960b62b4dd1f`.

These fixtures emit no HSACO and establish neither GPU execution nor
independent model correctness or sustained performance. All issue #42
milestones remain open.
