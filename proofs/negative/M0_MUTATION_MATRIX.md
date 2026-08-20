# M0 Actual-Body Mutation Matrix

This inventory mirrors every active V2 registry row and its assignments in
`M0_PROPERTIES.json`. The registry and property binder machine-check exact
target resolution and require each assigned mutation target to fall within the
property's compiler-rooted paths. The descriptive grouping here is
maintainer-facing; it is not a machine-enforced clause-completeness claim.

| Active mutation | Exact executable target | Assigned `Proved` properties |
| --- | --- | --- |
| `identity-trust` | `Identity::is_present` | Qualification-wide `--no-cheating` control; not assigned to one property |
| `request-id-generation` | `RequestId::new` | `m0.request_generation` |
| `speculation` | `verify_greedy_round` | `m0.greedy_speculation` |
| `speculation-prefix` | `verify_greedy_round` | `m0.greedy_speculation` |
| `speculation-validation` | `verify_greedy_round` | `m0.greedy_speculation` |
| `kv` | `KvPool::append_existing_page` | `m0.kv_transition` |
| `kv-cow-sealed` | `KvPool::append_tentative_key` | `m0.kv_sharing_rollback` |
| `kv-error-frame` | `KvPool::append_tentative_key` | `m0.kv_transition` |
| `kv-free-metadata-bound` | `KvPool::append_fresh_page` | `m0.kv_bounds` |
| `kv-page-generation` | `KvPool::drop_sole_tail` | `m0.kv_generation` |
| `kv-read-initialized` | `KvPool::validate_read_key` | `m0.kv_transition` |
| `kv-request-generation` | `KvPool::retire_empty_request` | `m0.request_generation`, `m0.kv_generation` |
| `kv-rollback-tail` | `KvPool::truncate_writable_tail` | `m0.kv_sharing_rollback` |
| `kv-share-committed` | `KvPool::share_committed_prefix_key` | `m0.kv_sharing_rollback` |
| `kv-stale-page` | `KvPool::page_slot` | `m0.kv_generation` |
| `kv-stale-request` | `KvPool::live_request_index` | `m0.request_generation`, `m0.kv_generation` |
| `kv-storage-length` | `KvPool::append_fresh_page` | `m0.kv_bounds` |
| `scheduler-cancellation-early-ready` | `Scheduler::retire_inflight` | `m0.scheduler_lifetime` |
| `scheduler-completion-boundary` | `Scheduler::complete_exact` | `m0.scheduler_transition` |
| `scheduler-completion-early-ready` | `Scheduler::complete_exact` | `m0.scheduler_lifetime`, `m0.engine_composition` |
| `scheduler-epoch-accounting` | `Scheduler::dispatch_enabled` | `m0.scheduler_transition` |
| `scheduler-exact-rejection` | `Scheduler::retire` | `m0.request_generation`, `m0.scheduler_transition` |
| `scheduler-finalization-transition` | `Scheduler::finalize_slot` | `m0.scheduler_transition`, `m0.scheduler_lifetime` |
| `scheduler-generation-reuse` | `Scheduler::reclaim_detached` | `m0.request_generation`, `m0.scheduler_lifetime` |
| `scheduler-ring-bound` | `Scheduler::dispatch_preflight` | `m0.scheduler_bounds` |
| `scheduler-scan-bound` | `Scheduler::dispatch_scan_commit` | `m0.scheduler_bounds` |
| `system` | `Engine::admit` | `m0.engine_composition` |
| `system-admit-return` | `Engine::admit` | `m0.engine_composition` |
| `system-append-routing` | `Engine::append_tentative` | `m0.engine_composition` |
| `system-completion` | `Engine::complete_exact` | `m0.engine_composition` |
| `system-constructor-routing` | `Engine::new` | `m0.engine_composition` |
| `system-detachment` | `Engine::reclaim_one` | `m0.engine_composition` |
| `system-dispatch-routing` | `Engine::dispatch_ready` | `m0.engine_composition` |
| `system-read-routing` | `Engine::validate_read` | `m0.engine_composition` |
| `system-reclaim-return` | `Engine::reclaim_one` | `m0.engine_composition` |
| `system-retire-routing` | `Engine::retire` | `m0.engine_composition` |
| `system-share-routing` | `Engine::share_committed_prefix` | `m0.engine_composition` |

Qualification requires a fresh authenticated rejection for every active row:

```sh
proofs/negative/run-same-source.sh REPO VERUS_ROOT NEW_OUTPUT_DIRECTORY
```

Anchor checks, ordinary Cargo typechecks, or prior-source transcripts do not
satisfy those rejection runs.
