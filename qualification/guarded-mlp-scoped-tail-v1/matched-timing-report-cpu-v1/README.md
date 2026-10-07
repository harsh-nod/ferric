# Tail Matched Report CPU Qualification

The final bound Census V3/Tail V4 report tool passed all 13 synthetic tests on
MI350. The controller completed in 0.22108176699839532 seconds. Its sole child
phase exited naturally, was reaped, and left no process group; no timeout,
forced cleanup, failure, error, skip or postcheck error occurred.

The tests cover disjoint host-wall accounting, overlapping forward totals,
exact integer nanoseconds beyond floating-point precision, output tables/plots,
strict policy and pair admission, and independent Tail diagnostic counters.
The report pins the actual V2 native collector at 49,363 bytes, SHA-256
`e5f4703652389327a0ef8722cc459ed9f433eb2aa4deafc98847c1728d0d76ff`.

The original terminal is 12,237 bytes, SHA-256
`618fa90eeaf6a058387487efb79ffded49560605c60b6985cb1efc5948167e63`.
The original archive is 41,471 bytes, SHA-256
`2089211faa00ebec30e4b7730e008c1859198df5f527d56f644cef140c02c5e9`.
All 16 archive originals, including six source bodies and seven raw evidence
bodies, are retained; the original manifest pins the other 15 members.
`retention.json` records the separate local custody check. Independent review
rejoined all original pins, test outcomes and retirement metadata.

This qualification uses synthetic data only. It runs no model or GPU work,
renders no actual native pair, and establishes no numerical acceptance,
Full2303 feasibility, GPU overlap, sustained throughput or performance gain.
