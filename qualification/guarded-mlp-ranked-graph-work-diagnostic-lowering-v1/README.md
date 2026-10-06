# Graph-Work Diagnostic Guarded Lowering

This controller prepares a guarded gfx950 compile using the
[diagnostic compiler generation](../guarded-mlp-ranked-graph-work-diagnostic-v1/README.md).
It requires that successful compiler qualification and a fresh
[binary-loader inspection](../guarded-mlp-ranked-graph-work-diagnostic-tool-audit-v1/README.md).
The guarded sources, optimized-inlining profile and independent resource
limits remain unchanged.

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
