# Bounded Streaming Measurements

The Python tools under `adapters/m1-engineering-execution-v1/tools` collect and
check engineering evidence. They do not start server processes or directly
access GPUs. The client sends requests only to an explicitly supplied loopback
`/v1/completions` endpoint. Starting or stopping any server remains an operator
responsibility. Nothing in these tools authenticates external evidence, proves
numerical correctness, qualifies a release, or asserts a framework win.

## Arrival Modes

`competitive_benchmark.py` retains the closed-loop default and
`FerricCompetitiveStreamingRunV1` report. Its executor now holds at most
`--concurrency` active futures, rather than submitting the whole workload at
once. Closed-loop request TTFT still begins at the client send attempt; executor
wait remains included only in the whole window duration.

For open-loop measurements, use `--arrival constant` or `--arrival poisson`,
`--arrival-rate REQUESTS_PER_SECOND`, `--arrival-seed INTEGER`, and
`--pending-capacity INTEGER`. These modes produce
`FerricCompetitiveStreamingRunV2`, with at most 32 active requests, at most 256
pending requests, and the existing workload cap of 256 requests per window.

For example, against an already-running server:

```sh
python3 -B adapters/m1-engineering-execution-v1/tools/competitive_benchmark.py \
  --endpoint http://127.0.0.1:18980/v1/completions \
  --workload heldout.json --identity ferric-identity.json --engine ferric \
  --output ferric-start-0.json --arrival poisson --arrival-rate 4 \
  --arrival-seed 17 --concurrency 4 --pending-capacity 8 \
  --warmups 10 --samples 10 --timeout 120 --ttft-slo-ms 500 --tpot-slo-ms 50 \
  --start-evidence ferric-start-0-evidence.json --tuning-policy tuning.json
```

Constant arrivals start at offset zero. Poisson arrivals use seeded exponential
interarrival intervals, including the first arrival. The exact nanosecond offsets
are stored and replayed for every window and engine. Schedules longer than one
hour and request timeouts longer than one hour are rejected. The scheduling loop
never moves a missed intended arrival to a later free-capacity instant.

Each V2 record preserves `started_ns`, the actual client send-attempt start, and
adds `intended_arrival_ns`, `send_started_ns`, and `send_delay_ns`. V2 `ttft_ns`
and `e2e_ns` begin at intended arrival, including both client scheduling and
pending-queue delay. `wire_ttft_ns` and `wire_e2e_ns` retain the send-attempt-based
durations. Request deadline accounting also begins at intended arrival. An unsent request
has null send timestamps and explicit `client_overload`, `queue_timeout`, or
`client_budget` failure; sent failures are `deadline`, `request_error`, or
`client_budget`. No overload is silently retried or dropped from the records.

The shared open-loop raw SSE byte budget is 64 MiB per whole invocation, including
warmups. Individual responses remain limited to 16 MiB, lines to 1 MiB and parsed
chunks to 16,384. Exhaustion becomes failed evidence, and subsequent requests do
not reach the server once they observe exhaustion. A legitimate large run can
exhaust this budget, and different engines can emit different amounts of SSE
metadata. Such a run is an invalid comparison, not an engine-performance loss.
The client retains every planned window after ordinary
request failures. Interrupted invocations retain a partial report marked
incomplete; aggregation rejects it. Output paths are exclusive and cannot
overwrite earlier runs.

## Timing Limits

Every window is a finite arrival cohort measured from its fixed clock origin
until its last admitted request finishes or fails. Its duration includes arrival
spacing and drain time. Successive windows do not overlap. These are **not
continuous steady-state serving windows**, regardless of sample count.

SSE text chunks can contain zero, one or many tokens. TPOT remains first-to-last
nonempty text-chunk duration divided by final completion usage minus one; it is
not a distribution of token ITLs. Usage must honor the fixed requested output
length and the stream must finish with `length` and `[DONE]`. Raw SSE events and
receive timestamps remain available. No token ITL is invented from chunk timing.

V2 explicitly records
`deadline_semantics: soft-absolute-checks-with-per-read-socket-timeout`. Absolute
clock checks reject late reads when they return, but urllib's socket timeout is
per blocking read; it does not interrupt an in-progress line that stalls or
slowly drips bytes across the absolute deadline. Queue expiry is checked before
send. HTTP timeout accounting is not a hard wall-clock termination guarantee,
and these reports cannot establish hard-timeout qualification. Operators must
retain their separately owned process-level cancellation/supervision policy for
a server or client that does not return; this tool starts no supervisor.

## Frozen Paired Series

`competitive_series.py` accepts a frozen plan, an explicitly supplied SHA-256 of
that plan, and a post-run evidence manifest. It reads files only and launches
nothing. A plan compares Ferric with one explicitly selected pinned baseline,
either vLLM or SGLang; it does not select whichever baseline happens to make
Ferric look better. Comparing with the faster tuned baseline still requires the
operator to collect and review both applicable baseline series.

The exact plan fields are:

```text
schema: FerricCompetitiveSeriesPlanV1
cell: bounded descriptive string
comparison_scope: model_sha256, tokenizer_sha256, config_sha256, weights_sha256,
  hardware_topology_sha256, environment_policy_sha256,
  cache_policy, sampling_policy, speculation_policy
workload_sha256, client_sha256, tuning_policy_sha256
settings: concurrency, pending_capacity, arrival_policy, arrival_rate,
  arrival_seed, timeout_seconds, ttft_slo_ms, tpot_slo_ms
engines: {ferric: IDENTITY_FILE_SHA256, BASELINE: IDENTITY_FILE_SHA256}
baseline: vllm or sglang
starts, windows, warmups
engine_order: one ordered two-engine list per start pair
bootstrap_seed, bootstrap_samples
```

Engine identity JSON must include its `engine` and the exact common
`comparison_scope`; all engine-specific versions, artifacts, flags and config
are permitted to differ but are pinned by that engine's full identity-file hash.
The hardware and environment scope digests should bind the reproducibility
inventory in [PERFORMANCE.md](PERFORMANCE.md), not merely a GPU model name.

The tuning policy has exactly `schema: FerricCompetitiveTuningPolicyV1`,
`calibration_workload_sha256`, `heldout_workload_sha256`, `trials_per_engine`, and
`selection_rule`. Calibration and held-out hashes must differ, the held-out hash
must match the plan, and both engines must receive equal positive bounded trial
budgets. This is a pinned policy, not evidence that every tuning trial occurred.

Each server start file has exactly:

```text
schema: FerricCompetitiveServerStartEvidenceV1
engine, identity_sha256, fresh_start: true
boot_id: Linux boot UUID
pid, process_start_ticks, ready_unix_ns
launch_evidence_sha256
```

The V2 client embeds this file and its raw-byte hash. The analyzer rejects reused
raw start-file hashes or repeated `(boot_id, pid, process_start_ticks)` identities,
even when a caller changes an arbitrary name. Readiness must precede the report
wall-clock interval, and engine intervals must follow the frozen rotated order.
These are operator-supplied process receipts, not authenticated proof of a restart.

External observation JSON has exactly
`schema: FerricCompetitiveExternalObservationsV1`, `steady_state_windows`,
`clock_drift_percent`, `thermal_drift_percent`, `environment_unchanged`,
`link_errors`, `ecc_errors`, `faults`, `admission_gates_passed`, and
`evidence_sha256`. Link/ECC errors, faults, failed admission gates, environment
drift or clock/thermal drift above 3% prevent structural acceptance. The final
evidence digest references retained external raw observations; the analyzer does
not authenticate them or upgrade an operator assertion into a proved gate.

The evidence manifest has exactly `schema: FerricCompetitiveSeriesRunsV1`,
`plan_sha256`, and `runs`. Its runs are ordered according to the frozen plan.
Each entry contains `engine`, zero-based `start_index`, and bindings for `run`,
`identity`, `workload`, `tuning_policy`, `start_evidence`, and `observations`.
Every binding has exactly `path` and `sha256`; relative paths are resolved from
the manifest directory. Inputs are bounded to 1 MiB each, reports to 64 MiB each,
and the whole series to 512 MiB. The test fixture in
`tools/test_competitive_series.py` constructs a complete synthetic example without
launching a model, server, or GPU.

```sh
python3 -B adapters/m1-engineering-execution-v1/tools/competitive_series.py \
  --plan frozen-plan.json --plan-sha256 PLAN_SHA256 \
  --runs paired-runs.json --output paired-report.json
```

## Statistical Checks

The analyzer replays successful raw SSE records and recomputes request clocks,
token usage, SLO membership and per-window goodput. It rejects late successes,
missing or reordered requests/windows, incomplete runs, hash drift, mismatched
workloads/settings/SLOs and unpaired server starts. A complete window containing
any request failure remains in its pair with **zero accepted goodput**, while its
observed partial-success metrics remain in the raw report. Missing data is never
invented as a successful sample.

Before replay, the analyzer hashes the actual imported
`competitive_benchmark.py` and requires it to match the frozen plan's client
hash. It also records its own `aggregator_sha256`, so future implementations
cannot silently reinterpret older runs under an unchanged claimed client hash.

Any `client_budget` record, `response_budget_exhausted: true`, or externally
recorded fault, environment/clock/thermal drift or failed admission gate makes
`comparison_valid: false` and `paired: null`. Raw per-window metrics remain for
diagnosis, but no ratio, difference or confidence interval is emitted from such
a comparison. Declared offered-load `client_overload` outcomes are distinct from
client evidence-budget faults: ordinary overload windows retain zero accepted
goodput within their pairs.

The statistic is median per-window SLO-filtered output tokens/second. Bootstrap
resampling first selects paired starts, then paired windows within each selected
start. The same sampled indices are used for both engines. The report includes
median ratio/difference and percentile 95% confidence intervals. If any bootstrap
sample has a zero baseline median, the ratio interval is undefined rather than
silently discarding that sample; the difference interval still includes it.

Structural sampling acceptance requires at least three distinct fresh starts,
ten warmups and ten recorded windows per start, rotated engine order, no request
failures or external faults, and at most 5% within-start coefficient of variation
of goodput. CV is an explicit conservative diagnostic, not a universal definition
of the release policy's persistent variance. The exact fixture/rate/SLO selection
still needs human review.

`qualification` and `framework_win_claim` are always false. Full
`qualification_preconditions_satisfied` is also false for these cohort reports:
the steady-state, token-ITL and hard-timeout requirements remain unmet. `--require-preconditions`
therefore writes the diagnostic report and exits unsuccessfully. It must not be
used to relabel a 3-by-10 cohort series as qualified serving evidence.
