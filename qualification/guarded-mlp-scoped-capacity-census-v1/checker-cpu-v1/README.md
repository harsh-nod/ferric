# Scoped Census Data Checker Qualification

Executed on `ssh mi350`, 2026-10-07 UTC. All 98 named synthetic tests passed,
with zero failures, errors or skips and clean owned-process/source postchecks.
The 82 previous tests remain; 16 tests cover the new explicit census policy,
ranked owner counts, subset accounting, exact integer counters, original
policy bytes and same-binary pair admission.

The capsule retains all 31 original archive bodies, including 21 source files
and seven raw evidence bodies. Original terminal (29,548 bytes):
`50d1268c73ea34d2ebfe98b995cf56900cd51255dd44234017bbb67f94cac729`.
Original export (66,012 bytes):
`84af74698b7601844e13d2eff3e08262f5a15c4cf455c816f43e946281220b6a`.

See [complete receipt](evidence/complete.json), [manifest](manifest.json),
and [retention](retention.json). This README is commentary outside the original
manifest. Synthetic-data acceptance does not execute the GPU, validate a real
pair, establish independent numerical acceptance or demonstrate a speedup.
