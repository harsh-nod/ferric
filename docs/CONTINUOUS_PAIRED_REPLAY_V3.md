# Continuous Paired Replay V3

`competitive_continuous_series.py` is a separate, bounded analyzer for the
continuous-timeline collector. It does not change the collector, the V1/V2
client, or the existing V1 paired analyzer. It launches no model or server.

Every report has `authority: none`, `qualification: false`, and
`framework_win_claim: false`. Confidence intervals are descriptive; neither an
interval excluding one nor a passing structural check establishes a framework
win. Continuous fixed windows still do not prove empirical stationarity, and
text-chunk TPOT is not token-level ITL evidence.

## Frozen Inputs

The plan has schema `FerricCompetitiveContinuousSeriesPlanV3` and exactly these
fields:

```text
schema, cell, comparison_scope
workload_sha256, tuning_policy_sha256, settings_sha256, settings
collector_sha256, client_sha256, replay_verifier_sha256, aggregator_sha256
engines, baseline, starts, engine_order, bootstrap_seed, bootstrap_samples
```

`settings` is the exact `FerricContinuousTimelineSettingsV3` object, including
fixed windows, warmup, arrival rate/seed, cyclic original workload order,
request cap, queue bounds, deadline, and SLOs. Its separately hash-bound JSON
file must be identical across all runs. Every paired start uses the same full
schedule. This slice does not search rates or tune configurations.

`engines` maps `ferric` and one explicitly chosen `vllm` or `sglang` baseline to
their distinct identity-file SHA256 values. Both identities must have the exact
common `comparison_scope` described in `COMPETITIVE_BENCHMARK_HARNESS.md`.
Engine-specific launch configurations may differ; the model, tokenizer, weights,
hardware, environment policy, cache, sampling, and speculation scope may not.
The existing held-out/calibration and equal tuning-budget contract is unchanged.

The four source hashes bind the actual imported collector, HTTP client, existing
raw SSE verifier, and this new analyzer. Local source bytes must match before any
run is loaded or replayed. The report records these hashes again.

The manifest has schema `FerricCompetitiveContinuousSeriesRunsV3`, the externally
supplied frozen `plan_sha256`, and `runs`. Each run entry contains:

```text
engine, start_index
run, identity, workload, settings, tuning_policy, start_evidence, observations
```

Each evidence binding is exactly `{ "path": "relative/file.json", "sha256": "..." }`.
Paths must resolve within the manifest directory. Entries follow the plan's
engine order for every start pair, with no omissions. The collector's optional
start/tuning inputs become mandatory for paired replay and must match their
original files and hashes.

Existing `FerricCompetitiveServerStartEvidenceV1` and
`FerricCompetitiveExternalObservationsV1` contracts apply. Each boot-ID/PID/start-
tick tuple and start receipt hash must be distinct. Raw run hashes must also be
distinct. Readiness precedes each run, and wall-clock intervals must not overlap
or contradict the frozen engine order. These are hash-bound operator receipts,
not authenticated or independently proved observations. An operator's
`steady_state_windows` boolean does not upgrade qualification.

## Replay and Statistics

The analyzer recomputes the entire schedule, replays raw SSE through the frozen
verifier, and reruns the V3 reducer. The saved reduction must match exactly.
Original-ID prompt token counts must agree across repeated occurrences, starts,
and engines wherever successful usage exists. Fixed requested completion counts,
all occurrences, all boundaries, and all windows remain mandatory.

Each successful fixed-length completion is credited once in its half-open DONE
window. Intended-arrival cohort outcomes remain separate. An ordinary failed
measurement window retains zero accepted goodput, including a late failure
charged to its original arrival window. No failed window is removed from a pair.
Client budget/cancellation faults, warmup/guard/drain invalidity, or external
faults, failed admission gates, link/ECC errors, environment changes, or more than
3% clock/thermal drift make `comparison_valid: false` and `paired: null`.
Invalid input, source drift, incomplete runs, and resource failures exit nonzero
with an incomplete failure report and no paired statistics.

At least three fresh pairs are required before any descriptive bootstrap is
emitted. Each resample selects complete paired start timelines with replacement;
it preserves all their adjacent windows and both engines together. It does not
treat correlated adjacent windows as independent samples. The point statistic is
median per-window SLO-filtered completed output tokens/second, with paired ratio
and difference percentile 95% intervals. Any zero-denominator resample makes the
ratio interval null; the difference interval remains available.

Structural sampling checks additionally require at least ten measurement
windows, ten declared warmup windows with their actual completion counts,
rotated engine order, no request failures, and at most 5% per-start goodput CV.
CV is a diagnostic, not proof of stationarity. Three pairs remain a small number
of independent observations regardless of the number of adjacent windows.
`qualification_preconditions_satisfied` is always false in this implementation.

## Resource Bounds and Invocation

Run this as its owned POSIX CLI process:

```sh
python3 -B adapters/m1-engineering-execution-v1/tools/competitive_continuous_series.py \
  --plan frozen-plan.json --plan-sha256 PLAN_SHA256 \
  --runs paired-runs.json --output paired-report.json
```

There are at most 32 pairs and 100 measurement windows per start. Small JSON
inputs are capped at 1 MiB. Each serialized run is capped at 256 MiB, and all
referenced evidence reads together at 2 GiB, counting reused files each time.
Plans and manifests have their separate 1 MiB bounds. The analyzer parses and
replays one run at a time, retaining only compact per-start summaries; it does
not accumulate the raw SSE trees from multiple runs.

The collector's **64 MiB response-byte budget is unchanged**. It is not a JSON
file-size or Python-heap estimate: pretty printing, escaping, repeated text and
derived fields can expand the saved report. The larger 256 MiB serialized-run
ceiling accounts for expansion without promising that every possible 64 MiB
stream fits. A larger report is invalid evidence for this bounded tool, never a
zero-goodput engine result. Calibrate feasible duration/rate/token lengths before
freezing the held-out plan; do not truncate reports or raise only one engine's
budget after observing its outcome.

Before parsing inputs, the CLI lowers its hard and soft address-space limits to
at most 2 GiB, respecting any smaller inherited limit. This also bounds JSON
object expansion rather than assuming a fixed byte-to-object ratio. Exceeding a
bound fails without ranking an engine. The importable replay helpers do not
change their caller's process limits; callers needing the memory contract must
use the owned CLI. Output creation is exclusive and never overwrites evidence.
