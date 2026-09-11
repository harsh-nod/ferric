# Matched V7 Host Timing

`summarize_tp_head_precision_v7.py` emits a distinct
`FerricTpHeadPrecisionTimingSummaryV1` report after revalidating every retained
v7 correctness report, unchanged raw trace, and original host-timing sidecar.
It never invokes legacy BF16 ledger admission or changes a frozen helper.

The manifest schema is `FerricTpHeadPrecisionTimingManifestV1`, with exactly
`schema`, `warmup_policy`, `variants`, and `pairs`. The warmup policy is
`fresh-worker-no-warmup`. Each variant contains a name, canonical absolute path
to its separately pinned v7 expectation JSON, that file's SHA-256, and 1..8 runs.
Each run has `id`, `run_dir`, `workload`, `reference`, `comparison`,
`comparison_sha256`, `timing`, and `timing_sha256`. Paths must be canonical and
absolute; run, comparison, and sidecar paths cannot be reused across repetitions.

Only these named profiles are accepted:

| Variant name | Head precision | Projection |
|---|---|---|
| `bf16-control` | `bf16-v7-control` | baseline |
| `fp32-baseline` | `fp32-v7` | baseline |
| `fp32-mfma` | `fp32-v7` | MFMA |

One variant can be summarized independently while later runs are pending. Pairs
are explicit objects with `baseline` and `candidate`, and only these transitions
are admitted: `bf16-control` to `fp32-baseline`, and `fp32-baseline` to
`fp32-mfma`. All controller/worker/base/v7 artifact IDs, workload/reference,
physical GPU roster, cache/pruning/collective/runtime policy and physical
execution geometry must match. A direct BF16-baseline to FP32-MFMA comparison
changes two dimensions and is rejected as an isolated pair.

```bash
python3 -B adapters/m1-engineering-execution-v1/tools/summarize_tp_head_precision_v7.py \
  --manifest /absolute/pinned-v7-timing-manifest.json \
  --json-output /absolute/new-v7-timing.json \
  --markdown-output /absolute/new-v7-timing.md
```

The wrapper re-runs the pinned v7 semantic/reference check, requires the retained
comparison to exactly equal that fresh result, and verifies raw hashes again
before reading timing. The frozen sidecar validator compares the complete raw
v7 Setup and Closed objects directly; no sidecar normalization is necessary.
Altered head IDs, stale reports, sidecar identity/dispatch failures, nonzero
status or failed exact reference rejects before aggregation.

TTFT starts at actual admission. TPOT is each named request's mean adjacent
output gap, absent for the one-output cancellation. Throughput counts all eight
outputs, including the cancelled request's output, from first admission to last
terminal event; terminal means last output for completion and cancellation time
for cancellation. Request identities and interval counts remain explicit and are
never pooled. Repetition p50/p95 use nearest rank, so a small-n p95 is not a
stable serving tail. Setup and whole time stay separate from the workload window.

Full host observations, matched phase ratios, and selected named span tables are
retained. Nested scopes and cross-rank observations overlap; these are host wall
latencies including instrumentation, not GPU durations or individual syscall
costs. Dependency source hashes and this generator's source hash are distinct
from generated JSON/Markdown hashes and must be labeled accordingly in Pages.
