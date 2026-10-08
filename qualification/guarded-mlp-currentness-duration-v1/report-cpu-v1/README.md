# Duration Report CPU Qualification

All thirteen tests passed on `mi350` in 0.053222963004373014 seconds. The
single-process harness used CPUs 8/9 at nice 10, with GPU visibility disabled.
It launched no child processes, model or GPU work. The original source maps
before and after execution are identical, and its private temporary directory
was empty at completion.

The tests cover exact integer arithmetic, all twelve callback categories,
unmeasured versus zero-duration rows, per-forward nonnegative residuals,
bank containment without double counting, closed timing spans, strict input
types and source identity, and CSV/table/SVG consistency. They exercise the
report reducer and rendering logic with synthetic records, not the complete
original-archive command. That separate actual command also passed; see the
[published report](../report-v1/README.md).

## Evidence

The [original terminal](evidence/complete.json) lists all thirteen test names
and outcomes; the [raw test output](evidence/stderr) retains each passing row.
The [manifest](manifest.json) authenticates twelve bodies, including all five
staged sources, in a thirteen-member original capsule. Independent data-only
review reconciled every body, the unchanged source maps, AST test names and
raw outcomes without executing helpers locally.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Original terminal | 4019 | `c9d5074e64f44023dc95f56d2e3fa32eeb8c7f7a4d08eb0b67620393894b7253` |
| Original exported archive | 22541 | `8e2493808991e2598b16e4cec71a5bc1450ecda965dabbc296de2cca425304eb` |
| Qualified report source | 22586 | `dedca35993c62acce84f8d2286b2dbce30f2f6f046afcebd6427ff69c83d9b6e` |

Bounds were 60 seconds wall time, 30 seconds CPU, 512 MiB address space,
8 MiB per file and 4 MiB per captured stream. The exporter recorded the
original process absent after SSH returned zero; that is retained post-run
evidence, not a new live observation or a child-retirement claim.

This README is subsequent commentary, not an original manifest member.
These tests do not establish model correctness, native feasibility or speedup.
