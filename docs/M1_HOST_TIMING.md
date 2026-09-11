# Opt-In Host Timing

`ferric-qwen3-tp-batch-engineering --host-timing /private/run/host-timing.json`
enables a separate `FerricHostTimingV1` JSON sidecar. The file is created
exclusively with mode 0600 before setup. Existing files are never overwritten.
Without this flag the recorder has no shared state, allocation or clock reads;
the normal correctness JSONL schemas, request IDs and workload clocks are unchanged.

This is **controller wall latency, not GPU duration**. It uses `std::time::Instant`
on the controller thread, not a device clock or the replica cohort RAW epoch.
The profiling flag must be recorded in the externally frozen launch receipt.
Profiling overhead is present in all compared profiled variants; it is not subtracted.

## Coverage

- Controller lifetime, setup, workload and close are distinct outer spans.
- Setup separates workload/artifact admission, model load/tokenization, scheduler,
  worker spawn/kernel admission, resident allocation/weight verification/upload,
  execution policies and transposed projection preparation. Projection is nested
  inside policy setup; those two spans must not be added together.
- Each controller batch includes scheduler/pool preparation, execution and commit.
  Each physical execution is separately bound to its existing pool batch ID.
- Physical batches cover metadata, embedding/broadcast, attention, feed-forward,
  both collective sites, output head and choice readback. Layer instances aggregate
  under the same phase; there is no per-layer event stream.
- Dispatch-group, rank-zero dispatch, sequence-flush and broadcast spans include
  surrounding host work. Sequence execution is attributed to its actual flush/wait
  location, not the earlier logical enqueue site.
- Both independent and shared-process peer parent transports record send through
  writer acknowledgement, response-wait call latency and request-to-receipt latency.
  Shared-process concurrent rounds retain one ticket for the complete mixed-rank
  frame; its dispatch count is the number of entries, not the number of frames.
  Deferred rank queueing is covered by dispatch-group wall time, not frame latency.

Transport latencies include checks, child execution, OS scheduling, JSON work and
pipe I/O. A response may already be buffered when the controller begins waiting.
No metric here isolates pure IPC cost or device execution. Payload-byte counters
exclude wire headers. A successful ticket means a successful transport receipt,
not independently verified model outputs. Existing transport state/ownership checks
and failure/cleanup paths remain responsible for execution safety.

## Bounded Failure Semantics

At most 65,536 distinct `(batch, phase, category, label, rank)` keys are retained;
each contains count, failed count, elapsed/max nanoseconds, payload bytes and
dispatch count. Repeated calls do not append events. There are no per-dispatch
timestamps or new request identities. Aggregate overflow, record overflow, invalid
key bounds or active tickets at snapshot mark the sidecar incomplete. Abandoned
tickets record failure. Such diagnostics are not accepted for comparison.

Normal run failures still close/reap workers using the existing logic and produce
a failed diagnostic receipt when possible. The sidecar copies the exact Setup and
Closed records plus the loaded workload SHA256. Early failures may have no setup
or close. Abrupt termination can leave an empty/truncated reserved file, which the
summary rejects. A sidecar write failure is reported without masking an original
execution error; a completed correctness stream alone does not imply exit status 0.

## Strict Summary

Run the existing `compare_tp_batch.py` with externally pinned controller, worker,
artifacts, workload, reference, world and policy flags first. Freeze the raw trace,
idle snapshots, successful status, comparison report and sidecar SHA256 values.

`tools/host_timing_summary.py --manifest PLAN.json --output SUMMARY.json --markdown SUMMARY.md`
uses the performance-ledger manifest shape, except its schema is
`FerricHostTimingManifestV1` and every run additionally requires absolute `timing`
and externally pinned `timing_sha256` fields. It re-executes the strict reference
and physical-idle comparator and requires the pinned prior report to equal the
new report, rather than trusting an old `passed` label.

The summary verifies sidecar setup/close/workload identity, complete physical and
controller batch counts, exact total completed dispatches, paired send/receipt
counts and bytes, zero failed tickets, and compatible workload/world/roster/cache/
row/chunk policies. New `device-peer-concurrent-round-v1` comparisons retain the
serial peer artifact, same-PID and dispatch-count requirements, but require their
own exact label and an explicit profile with legacy sequences disabled.

Per-operation repetitions retain raw totals/counts and report a mean host elapsed
ratio to the named baseline. Different request identities are never pooled into
latency percentiles. Nested phases and cross-rank round trips overlap: **do not
sum these rows into a GPU duration or workload throughput**. TTFT, TPOT and output
rate remain derived from the independently accepted original correctness trace.
This is a short fixed-workload engineering diagnostic, not serving qualification.
