# Inclusive Host Accounting

Source-only data-analysis proposal. No tests or analysis have been executed by
the author. The sole actual native terminal pin was independently hash-read from
`host-observation-gpu-complete-v1.json`: 614572 bytes,
`91a20d628644eb784d651e4d4d87571cb55cdb367971092fcea40d259ee38ec2`.
It records a successful host diagnostic, not a performance qualification.

## Inputs and Execution

Keep the three Python files together. Root owns bounded execution on MI350:

```text
python3 -I -B run.py INPUT_ROOT FRESH_OUTPUT_ROOT
```

`INPUT_ROOT` is the original host GPU directory or its retained capsule, with
`run_model_gpu.py` and `ar4/complete.json`. Selected reads authenticate the exact
controller, terminal, saved checked host observation, original worker report,
and native completion. The report pin joins both original receipts; identities
and all four completion records join the native summary. No project/controller
module is imported, original file modified, child spawned, or GPU invoked.

The runner checks the authenticated terminal's eleven natural-zero lifecycle
records and false authority fields. This is not another complete 97-member
capsule or raw process audit; that remains the separate retainer's role.

Ten in-process synthetic tests run before analysis. A standalone test-only run
uses `--self-test` instead of `INPUT_ROOT`, with another fresh output directory.
The total wall/CPU bounds are 120 seconds, address-space bound 256 MiB, per-input
bound 8 MiB, and selected readset bound 16 files/32 MiB. Original/source inputs
are rehashed after analysis. Outputs are `tests.stderr`, `analysis.json`,
`summary.md`, and a complete or failed receipt. Test-only runs omit the two
analysis outputs. Only the small analysis and test modules are imported, after
their fixed source pins match.

## Accounting

The closed schedule is 587 snapshots and 586 exact nonnegative interval deltas.
For forward `f`, the 145 intervals beginning at `2 + 146*f` are partitioned into
36 prefix, 36 paired, 36 hidden-read and 37 other intervals. Their union is
disjoint and exactly equals the forward bracket. The six remaining intervals
are `0, 1, 147, 293, 439, 585`. Four brackets plus those six intervals reconcile
to the total snapshot wall. Close host wall is outside that snapshot interval
series and is reported separately.

Every count and nanosecond value is a strict u64, never a bool or float. Integer
sums are checked for u64 overflow. Rank/shared counter deltas must equal exact
successive subtraction with no decreases, fixed device/group/queue identities,
a zero initial baseline and unchanged conservative policy. Forward counter
sums telescope to the two endpoint snapshots. The checked per-layer rows and
final counters must independently match the original report.

All 19 rank counters and four shared counters remain available in JSON. The
Markdown tables show full-currentness counts/time, kernel admissions and
commands separately from shared group/publication checks. These counters are
inclusive and can nest; they are never summed into elapsed wall time. Paired
generic-dispatch timer coverage is incomplete. Disjoint intervals can contain
host work, GPU waits and process scheduling gaps; none is a GPU duration.

Synthetic cases cover the schedule, exact partition, telescoping counters,
decreases/wrong deltas, bool/float/overflow refusal, checked/raw joins, policy
drift, Close/nonclaims and exact integer seconds rendering. This analysis makes
no throughput, overlap, speedup, numerical acceptance or production-authority
claim and does not attribute a causal bottleneck by subtracting nested scopes.
