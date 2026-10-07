# Native Decode Attribution

Measured on MI350, 2026-10-06. Both arms completed exact 128-token/UTF8 replay,
127 registered decode executions and clean unsignaled child/outer shutdown.
One cold instrumented 128-input/128-output request per arm is diagnostic evidence,
not a speedup qualification or matched vendor comparison.
The [independent raw audit](native-decode-a001-independent-review.json) passes
all 493 retained files, input bindings, token output and counter arithmetic.

The baseline uses 652 dispatches per decode. The default-off gate/up candidate
uses 724. Both use unchanged prefill32, the same current-runtime worker, model,
ten images, scratch allocation and greedy uncached fixed-length workload.

| Host-Wall Scope | Control, ms/Step | Gate/Up Candidate, ms/Step |
| --- | ---: | ---: |
| Registered decode span | 52.211899 | 43.964788 |
| Worker completion wait | 50.880113 | 42.528147 |
| Execute residual | 0.429083 | 0.437441 |
| Warm transport planning | 0.340089 | 0.395284 |
| Worker dispatch preparation | 0.189804 | 0.197885 |
| Metadata writes | 0.175111 | 0.201724 |
| Span residual | 0.106777 | 0.110007 |
| Token readback | 0.060350 | 0.062751 |
| Publication | 0.015917 | 0.016656 |
| Token staging | 0.014655 | 0.014894 |

These values divide each aggregate by 127; they are not reported TPOT.
The first cold transport plan and metadata uploads, registration, endpoint
snapshots, setup, prefill and teardown are outside the registered span.
The warm-planner aggregate contains 126 plans. General command/currentness and
worker read/write counters overlap these scopes and must not be added again.

## Consequence

Completion wait occupies **97.4493%** of the control span and **96.7323%** of the
candidate span. With this measured wait unchanged, eliminating every other
recorded region gives conditional arithmetic ceilings of **1.02617x** and
**1.03378x**. This is not a ceiling on all runtime optimizations, a prediction,
or a reason to stop improving the engine. It shows that host planning and IPC
alone are not the principal route to closing the existing matched TPOT gap.

The core has no fixed 50 ms delay. It polls the final program signal, requests
a 50-microsecond sleep after a Pending result, then checks all retirement
signals once without sleeping. The wait timer includes those polls, scheduler
delays, currentness checks and retirement while the published program executes.
Summed sleep requests overlap device progress and are not recoverable latency
that can simply be subtracted. These are not calibrated GPU timings.

## Next Experiment

Reuse the existing unpaired V16 split-K8 **down-projection** kernel with original
KN BF16 weights and 128 KiB scratch. It already passed component parity and
timing, including partial and merge together. The paired-load successor showed
no useful gain, and packed-down requires a different storage layout.

The separate private native688 decode route now passes its CPU qualification,
including actual-image admission, captured graphs, ABI checks, strict lint,
feature checks and both controller releases. It preserves prefill32 and excludes
the unpromoted gate/up experiment. Native attempt a002 passes both counters and
two ABBA blocks, then stops before third-block Setup after a foreign Docker
process opens KFD. It is an interrupted campaign, not a qualified speedup or
main overlay. The kernel retains its actual
compiler provenance. See the
[integration checkpoint](native-down-integration-a001-checkpoint.json).

Use the new read-only clock/power/temperature telemetry on the next latency
campaign to investigate the earlier middle-block variability. Keep telemetry
and diagnostic samples separate from the timing comparison. Finer native wait
instrumentation belongs in fe2o3 if completion-observation delay remains unclear;
existing ordinary-ordered active polling is not a supported native-mode flag.
The source audit also finds only 40.165411 ms of aggregate currentness work
across the control's 127 steps, versus 6,461.774339 ms of wait. Those counters
overlap and must not be subtracted. A bounded default-off observation diagnostic
could record the last-Pending/first-Completed signal brackets, actual pause
durations and post-observation checks without changing the polling policy.
It would bound CPU observation uncertainty, not measure shader execution.

## Status

The prior 14-cell gate/up campaign remains **inconclusive** under its fixed
paired-consistency gate. These two diagnostic requests do not override it.
Defaults and HTTP admission remain unchanged. The last complete matched HTTP
comparison still shows Ferric 22.72x slower in TTFT and 12.20x slower in TPOT
than vLLM; no fresh vendor launch was performed for this diagnostic.
