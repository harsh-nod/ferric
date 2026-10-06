# Bounded CFG Capacity Tool Audit

The controller for the [bounded block-cap experiment](../guarded-mlp-ranked-cfg-capacity-v1/README.md)
has passed 26 synthetic admission tests on MI350. The actual deployed-binary
audit is still pending; these fixtures do not inspect or qualify a compiler
deployment.

## Controller Tests

The fixtures check the exact full-suite and source lineage, final Cargo
product selection, unchanged legacy tools, preserved independent resource
limits and rejection of failed, earlier or diagnostic-only producers.
Missing receipt bindings must reject before output creation.

The fixed runner completed in 8.595860 seconds. Its sole child exited
naturally with status zero, was reaped and left no process group. All 26
names passed, with no skips or unexpected successes. Source and tool
postchecks passed. CPU affinity, memory, storage and wall-time bounds were
unchanged from the preceding controller qualification.

[controller-tests-v1/attempt-v1](controller-tests-v1/attempt-v1) retains the
executed sources, exact inputs, seven raw records and receipt. All 15
archive members were verified after transfer. Receipt SHA-256:
`daa80666a02fdc3be9a4d6417501ed459c5ce8cda4e23c58c76361b3993e2aa3`.

No HSACO, GPU execution, model correctness or performance result is claimed.
