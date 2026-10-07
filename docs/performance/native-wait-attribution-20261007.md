# Native Completion Attribution

## Result

The completion-observation question is resolved for this capture: **late CPU
observation is not the source of the tens-of-milliseconds decode wait**.
The final program signal remains Pending until close to the end of that wait.
The bottleneck is on the published-program side of completion, not the small
host return tail. This does not yet distinguish shader execution, device-side
dispatch gaps, dependencies or fences within that program.

One cold instrumented Qwen3-8B BF16 / FP32-head request completed on MI350 on
October 7, 2026, 19:42-19:45 UTC: TP1/C1, 128 input and 128 output tokens,
context 8192, greedy fixed length, no prefix caching or speculation. All 128
token IDs and UTF-8 bytes match the frozen independent reference. The worker,
controller and supervising process group closed without signals or errors.

| Decode Region | Mean ms/Step |
| --- | ---: |
| Registered decode span | 46.932647 |
| Worker completion wait | 45.721637 |
| Publication return to final signal completion, bounded | 45.619859-45.719894 |
| Signal completion to diagnostic return point, bounded | 0.022621-0.122656 |
| All recorded regions outside completion wait | 1.211010 |
| Retirement-signal checks, nested within return tail | 0.001691 |

There are 127 decode executions, each with 652 dispatches. Completion wait
occupies **97.4197%** of the registered span. The mean upper bound on the
post-signal tail is **0.2613%** of that span; its maximum is **0.142710 ms**.
This bounds that tail, not the benefit of arbitrary runtime changes.

Actual sleeps total 45.601012 ms per decode step, but almost all of that time
overlaps outstanding device work. Subtracting accumulated sleep from latency
would be incorrect. The mean final signal bracket is 0.100035 ms wide.
Currentness inside polling averages 0.009279 ms, nested within 0.052988 ms of
post-read work; neither is a disjoint shader-time bucket.

The four prefill32 executions contain 649 dispatches each. Their aggregate
publication-to-signal interval is bounded by **359.789264-360.196504 ms**;
the aggregate post-signal tail is bounded by **0.087890-0.495130 ms**.
These are not complete TTFT: preparation, registration, singleton head work,
caller work and output transport are not all inside these intervals.

## Scope And Validation

The worker is freshly built from published fe2o3 `179629a7310e31dde354afa6465f304f09ec87fb`
with default-off `engineering-native-wait-diagnostics`. It preserves polling,
currentness and retirement behavior. The previously qualified controller and
kernel set remain pinned; they are not relabeled as current-source products.
All builds and the 28 parser/capture tests ran on mi300x-2. No local build ran.

The capture validates all 131 native records, queue epoch, dispatch counts,
frontiers, enclosing controller durations, legacy counter replay, token/byte
parity, live executable endpoints and clean supervised closure. A separate
stdlib byte/protocol/arithmetic audit rechecks all 177 retained files, raw
tokens and all reported completion-bound statistics. Isolation checks are
sampled, not continuous proof. See the [evidence record](native-wait-attribution-20261007.json).

The bounds use CPU monotonic timestamps around acquired completion-signal
reads. They are **not calibrated GPU timestamps or kernel-family shares**.
The diagnostic return point excludes later JSON serialization/stderr output
and remaining caller return work. Instrumentation overhead is not calibrated.
This one request is not a TTFT/TPOT benchmark, speedup result or vendor comparison.
Its lower span than the October 6 diagnostic must not be promoted as a gain.

MI350 disk headroom was restored by removing only inactive, over-seven-day-old,
single-link Cargo `debug/deps` `.rlib`/`.rmeta` files, after process-reference
checks. Model files, source, kernel images, executables and result logs were
not removed. The 64 GiB native root floor was unchanged. An initial launch
stopped before execution because logind removed shared-memory staging on
logout. The successful retry keeps restore, capture and durable archive
retention in one SSH session; no shared-host configuration was changed.

## Next Steps

1. **Separate kernel work from device dispatch gaps in the exact native path.**
   Profile the unchanged prefill649/decode652 programs by projection, attention,
   normalization/elementwise, KV and head families. Retain queue/fence gaps as
   a separate category. Add only default-off core observation support in
   fe2o3 if needed; keep model labels and analysis in Ferric. Do not substitute
   ordinary ordered64 packet ticks or hot-buffer microbenchmarks for native
   wall-time shares. Use the result to rank larger changes.
2. **Finish the existing native688 down-projection ABBA campaign.** The split-K8
   partial-plus-merge component already has a favorable scoped result, and
   the model integration exists. Earlier interrupted campaigns remain
   incomplete, not wins. Use the same non-diagnostic worker in both arms,
   retain clock/power/temperature telemetry, exact token parity and the existing
   full paired-consistency and regression gates.
3. **Address projection load serialization.** Implement the bounded checked-load
   grouping design in fe2o3; verify actual ISA load overlap and register/spill
   cost before re-emitting Ferric O/down/gate-up candidates. Preserve guards
   and reduction order. A source rewrite without changed ISA is not progress.
4. **Reduce prefill program cost for TTFT.** Complete same-compiler emission,
   native numerical checks and isolated measurement of the existing prefill
   Q/K Wave64 RMSNorm candidate. Use the exact-native family profile to decide
   whether it or matrix/attention work deserves priority. Keep cold setup and
   warmed request TTFT separate; do not infer a full TTFT gain from the four
   intervals above.
5. **Measure each accepted change end to end.** Record its isolated native
   TTFT/TPOT delta, then its composed delta, before a refreshed matched HTTP
   comparison. Keep model, precision, prompt/output lengths, concurrency,
   context, cache/speculation and hardware fixed. Promote only completed
   correctness and performance gates. Small host-path optimizations can be
   revisited after published-program cost falls.

The last complete matched HTTP comparison remains unchanged: Ferric
**431.033 ms TTFT / 53.119 ms TPOT**, vLLM **18.969 / 4.352 ms**. No new
vendor win, serving qualification or default promotion is claimed.
