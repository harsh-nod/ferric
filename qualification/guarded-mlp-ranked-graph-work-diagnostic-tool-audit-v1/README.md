# Graph-Work Diagnostic Compiler Loader

This loader controller requires the exact successful
[diagnostic compiler generation](../guarded-mlp-ranked-graph-work-diagnostic-v1/README.md),
its source lineage and final build products. Only the extractor and compiler
backend may replace their entries in the seven-tool deployment. The other
five entries remain unchanged. The diagnostic adds failure context without
changing graph-work limits or admission predicates.

## Controller Tests

All 26 synthetic admission tests passed on `mi350` in 17.436424 seconds.
The bounded child exited naturally with status zero, was reaped and left
no process group. Sources were unchanged and postchecks were clean, with
no timeout or forced cleanup.

The fixtures reject incomplete qualification, stale products, changed source
bodies, lost tests, changed independent limits and mismatched diagnostic
generation. An unbound qualification receipt refuses before effects.

[controller-tests-v1/attempt-v2](controller-tests-v1/attempt-v2) retains the
executed sources, input manifest, receipt and all seven raw records. Its
archive contains 15 members, 14 pinned bodies and 203,524 body bytes.
Receipt SHA-256:
`1e1fe34a3a6ea48452032e74bd2d12229cb5368caaecf36e986f6fabd4bfdaa5`.
Archive SHA-256:
`1ebc92bb215c9042bee4ce6e3c12a33e03430e6786ca0e23246b5604ae2ffb5e`.

These are synthetic controller tests, not actual binary-loader inspection,
guarded HSACO emission, GPU execution or numerical/performance validation.
All issue #42 milestones remain open.
