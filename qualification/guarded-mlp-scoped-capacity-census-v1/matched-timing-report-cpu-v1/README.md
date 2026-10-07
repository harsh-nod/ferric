# Census Report Qualification

Executed on `ssh mi350`, 2026-10-07 UTC. All 11 synthetic report tests passed,
with zero failures, errors or skips, clean natural process retirement and
unchanged source/tool postchecks. All nine previous regression tests remain;
two new tests reject mislabeled Bank/Census cases and missing census flags.

The tested reporter is the final bound source, not an unbound template:
24,744 bytes, `bdf61482dba71a3e07736ea9f33762e809f96c945f572944275b7055ea1bb21e`.
It preserves integer nanosecond accounting, six-place half-even ratios,
disjoint category totals, overlapping forward-group reconciliation and plots.

All 16 original archive members are retained, including six exact source
bodies and seven raw evidence files. Terminal (11,878 bytes):
`73395625e7d28fda2b69418274f68cdebeea6ceeccb347a0320d5b0c6a9e97c6`.
Archive (40,436 bytes):
`565e16dc54f9526e789a2bcbb9b4877774dfaa28e59e36b750d3cb2bf3d2e76e`.

See [original receipt](evidence/complete.json), [manifest](manifest.json),
[retention](retention.json), and [report source](paired_analysis.py).
This README is publication commentary outside the original manifest.
These tests use synthetic data. They neither execute a GPU nor establish
an actual pair's correctness, timing, throughput or numerical acceptance.
