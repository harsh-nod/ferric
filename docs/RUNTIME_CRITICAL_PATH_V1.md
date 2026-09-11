# Runtime Critical Path Options

## Shared Peer Full Currentness

The peer worker supports explicit `--shared-full-currentness` negotiation.
The parent must request it and observe `shared_full_currentness: true` in the
Ready receipt; absent, false, unrequested or malformed acknowledgments reject.
The first command must configure performance, even when the other options are
false. Old launch and Ready bytes remain unchanged by default.

The core API is `configure_performance_v2(cache, operational, shared)` on
`Gfx950EngineeringPeerGroupV1`. It coalesces full topology discovery within one
group fence, not across calls. Every participant still performs the same full
mutable checks before and after that observation, and final root identity and
topology generation are checked after the last rank. Failure remains terminal.
Single-context lifecycle checks and public observation checks are unchanged.

This is a peer scaling/setup/close optimization, not a TP1 dispatch optimization.
The prior six-case host timing matrix implicates repeated full group fences,
but does not measure individual syscalls or GPU duration. Native and matched
model tests are still required before claiming a gain. The producer fixture's
separate `--shared-full-currentness` flag records its opt-in explicitly.

## Runtime Diagnostic Snapshots

`RuntimeOptions.profile` enables the core's existing cumulative wall counters.
It is disabled by default and currently rejected by the shared peer transport.
`runtime_diagnostic_snapshot()` is default-unsupported on other transports and
forwards through the batched driver and resident runtime only when ready.

The independent worker permits exactly two snapshots, with ordinals 0 and 1, for
the controller's before-workload and after-workload boundaries. Receipts bind
worker PID, physical unique ID and rank; counters must be monotonic, and the
driver independently matches completed dispatch counts. Disabled, pending,
closed, poisoned, malformed, missing, timed-out or regressing snapshots fail
closed and preserve normal teardown handling. There are no per-dispatch
snapshot requests or default extra allocations/IPC.

Each receipt has schema `FerricRuntimeDiagnosticSnapshotV1`, authority `none`
and `performance_qualified: false`. The CLI must identify the run with
`performance_profile.runtime_profiling: true` and an explicit diagnostic-only
Setup field so ordinary strict performance comparison rejects it. Output
reference requirements are not relaxed.

Available cumulative counters include commands, full/operational currentness,
immutable kernel admissions, dispatch preparation/publication/wait, completion
polls, and host reads/writes including transferred bytes. Durations overlap:
for example currentness may occur inside dispatch wait. They are not GPU
timestamp queries and must not be added into an exclusive-time total.
Counters from independent ranks can also overlap and must not be summed into
an end-to-end latency.
After-minus-before command counters include the earlier snapshot command;
each snapshot excludes only its own command. Snapshot IPC and counter overhead
make these diagnostics unsuitable for ordinary performance qualification.
