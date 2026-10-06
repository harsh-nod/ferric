# Bounded CFG Capacity Tool Audit

The actual seven-tool deployment for the
[qualified block-cap experiment](../guarded-mlp-ranked-cfg-capacity-v1/README.md)
passed all 14 loader inspections on MI350. Separate controller qualification
passed 26 synthetic admission tests; those are not binary inspections.

## Actual Deployment

Only the final extractor and backend are redeployed from the successful
capacity qualification. The other five tools remain byte-identical to the
qualified driver deployment. The audit verifies the full source/test lineage,
final Cargo product roles and actual library resolution before allowing the
separate engineering lowering controller to consume this deployment.

All 14 inspection children exited naturally with status zero and were reaped
with absent process groups. The audit completed in 2.352120 seconds, with
clean input/provider postchecks and no timeout or forced cleanup. This
inspection ran `readelf` and `ldd`, not the compiler or a GPU kernel.

[attempt-v1](attempt-v1) retains 72 raw records, the executed controller,
tool manifests, producer/driver lineage and the successful receipt. Its
manifest pins 92 bodies in 93 archive members totaling 18,007,366 bytes;
all were verified after transfer. Binary bodies are not included. Receipt:
`3fd116bd1a1e7345cd0573cc4e769d5a98b11ea34f842ec78112f20849022291`.

The 3,083,013-byte archive SHA-256 is
`b9da258e3b470693c66149389b24f736d2d4eeac7db4cd4898f3504dd99070c3`.

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
