# Continuous Timeline Collector V3

`tools/competitive_continuous.py` is a separate, bounded engineering collector.
V1/V2 client behavior is unchanged, and the existing paired-series analyzer
rejects V3. This does not qualify serving performance, prove stationarity, infer
token ITLs or claim a framework win. It launches no model or baseline server.

Supply an already-running loopback endpoint and the existing canonical workload
and engine identity files, plus a strict settings file:

```json
{
  "schema": "FerricContinuousTimelineSettingsV3",
  "arrival_policy": "constant",
  "arrival_rate": 1,
  "arrival_seed": 7,
  "concurrency": 4,
  "pending_capacity": 8,
  "timeout_seconds": 2,
  "window_seconds": 2,
  "warmup_windows": 10,
  "measurement_windows": 10,
  "max_requests": 64,
  "ttft_slo_ms": 1000,
  "tpot_slo_ms": 100,
  "workload_repetition": "cyclic-in-file-order"
}
```

```sh
python3 -B adapters/m1-engineering-execution-v1/tools/competitive_continuous.py \
  --endpoint http://127.0.0.1:18980/v1/completions --engine ferric \
  --workload heldout.json --identity identity.json --settings continuous.json \
  --start-evidence start.json --tuning-policy tuning.json --output run-v3.json
```

There is one seeded constant or Poisson schedule and one bounded executor.
No boundary drains or clock resets occur between warmup and measured windows.
A loaded guard of one request timeout follows measurement; arrivals continue
during it. Only then do admissions stop and a final drain of at most one more
request timeout begins. Requests keep their original absolute arrival deadline.
The warmup must cover at least one full deadline; elapsed warmup alone does not
establish empirical stationarity. The tool retains observations, not that claim.
At least `warmup_windows` requests must actually complete before measurement;
elapsed warmup without that completion count invalidates accepted metrics.

Workload entries cycle in their frozen file order, with unchanged prompts and
output limits. Every occurrence has a unique sequence/ID plus its original ID.
The complete schedule, token demand and all timing boundaries are recorded.
Reported prompt token usage must remain consistent for each repeated original ID.
Counter snapshots include their actual observation time and lag, rather than
pretending a delayed observer sampled the exact planned boundary. Active counts
refer to outstanding client futures, not server/GPU occupancy.

Each completed request contributes its exact final completion usage once to the
half-open time interval containing `[DONE]`. This is completed fixed-output work
per second, not a claim about the instant individual tokens were generated.
Completion metrics and intended-arrival latency/failure cohorts are separate.
A late failure still zeros the originating measurement window, even when it
appears in the guard or drain. Every window remains in the output. Warmup/guard
failures, client cancellation/budget faults or an expired drain make all accepted
window goodputs null; partial diagnostics are retained. An interrupted run is
explicitly incomplete and never receives an accepted reduction.

Resource limits remain 32 active requests, 256 pending requests, a declared
total cap of at most 16,384, and one hour including final drain. A schedule that
exceeds the cap is rejected before network activity, not truncated. The shared
raw SSE budget remains **64 MiB**, including warmup and guard. Exhaustion does
not drop future planned occurrences: they remain explicit failed evidence and
cannot support a comparison. Different engines' SSE metadata sizes can exhaust
this budget differently; that is a client measurement fault, not an engine loss.

For constant rate R, window duration D, W warmup windows, M measured windows
and timeout T, the offered interval is `(W + M) * D + T`; its request count is
approximately R times that interval. The final deadline adds another T, and
`W * D >= T` is required. The example offers 42 requests over 42 seconds, with
a final bound of 44 seconds. Output token count and SSE verbosity determine
whether it also fits the unchanged evidence budget. Large rates/output lengths
can require a future separately reviewed streaming-evidence design; the tool
does not silently raise the budget to make a workload fit.

Reports bind the exact collector, shared client and replay-verifier source
hashes along with raw input hashes. External start/tuning files remain optional
operator evidence, not authenticated proof. A future V3 paired-plan contract,
three fresh starts, stable measured load and real token-ITL evidence are still
required before any serving qualification decision.
