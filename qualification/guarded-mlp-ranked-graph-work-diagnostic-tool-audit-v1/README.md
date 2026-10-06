# Graph-Work Diagnostic Compiler Loader

This loader controller requires the exact successful
[diagnostic compiler generation](../guarded-mlp-ranked-graph-work-diagnostic-v1/README.md),
its source lineage and final build products. Only the extractor and compiler
backend may replace their entries in the seven-tool deployment. The other
five entries remain unchanged. The diagnostic adds failure context without
changing graph-work limits or admission predicates.

## Actual Loader Inspection

The deployed seven-tool generation passed all 14 `readelf` and `ldd`
inspections on `mi350` in 2.445856 seconds. Each child exited naturally with
status zero, was reaped and left no process group. Inputs and tool postchecks
were clean, with no timeout or forced cleanup.

Only the extractor and compiler backend entries take provenance from the
successful diagnostic build. The extractor bytes remain unchanged; the
backend is the new diagnostic binary. The other five deployment entries
retain their exact previous bytes and provenance.

[attempt-v2](attempt-v2) retains the actual receipt, bound controller, input
and tool manifests, original producer/driver lineage, diagnostic lineage and
all 72 raw records. The archive contains 93 members, 92 pinned bodies and
18,158,516 body bytes. Receipt SHA-256:
`364a5e3e693c0a10e13947df6f1695294076f3621e6b40406e7cc32cd47ee3ab`.
Archive SHA-256:
`617750463eccd700406d96a21522364da99d2591715b4b7b160ce73b7abbbbe4`.

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

Synthetic controller tests and successful binary-loader inspection do not
establish guarded HSACO emission, GPU execution or numerical/performance
validation. The [actual guarded compile](../guarded-mlp-ranked-graph-work-diagnostic-lowering-v1/README.md)
is a separate check. All issue #42 milestones remain open.
