# M0 Actual-Body Mutation Matrix

This matrix records the clause-level assignments for the phase-free scheduler.
It is a qualification input, not proof evidence. A registered row counts only
after `run-same-source.sh` produces an authenticated proof-obligation rejection
transcript for the exact source closure being qualified.

| Property clause | Exact executable target | Required mutation |
| --- | --- | --- |
| Request generations reject stale scheduler identities | `Scheduler::retire` | `scheduler-exact-rejection` |
| Scheduler success implements the finalization transition | `Scheduler::finalize_slot` | `scheduler-finalization-transition` |
| Scheduler rejection matches the exact completion boundary | `Scheduler::complete_exact` | `scheduler-completion-boundary` |
| Completion cannot redispatch a member before KV finalization | `Scheduler::complete_exact` | `scheduler-completion-early-ready` |
| Cancellation cannot redispatch an executing request | `Scheduler::retire_inflight` | `scheduler-cancellation-early-ready` |
| Reclaim advances the scheduler generation before reuse | `Scheduler::reclaim_detached` | `scheduler-generation-reuse` |
| Dispatch scans at most `C` slots | `Scheduler::dispatch_ready` | `scheduler-scan-bound` |
| Dispatch preserves fixed-capacity submission ring bounds | `Scheduler::dispatch_ready` | `scheduler-ring-bound` |
| Engine admission preserves scheduler/KV identity agreement | `Engine::admit` | `system` |
| Engine completion publishes the caller's accepted token count | `Engine::complete_exact` | `system-completion` |
| Engine reclaim consumes detached KV evidence before reporting reuse | `Engine::reclaim_one` | `system-detachment` |

The phase-free migration requires fresh rejection runs for every active row in
`REQUIRED_COMPONENTS`, including the unchanged KV, identity, speculation, and
engine mutations. The remaining proof command is:

```sh
proofs/negative/run-same-source.sh REPO VERUS_ROOT NEW_OUTPUT_DIRECTORY
```

Anchor checks, ordinary Cargo typechecks, or prior-source transcripts do not
satisfy those pending runs.
