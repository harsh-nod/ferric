# M1 Foundation Mutation Matrix

All rows are repository-declared requirements. No checked-in transcript or
M1 evidence binding authenticates them. `run-same-source.sh` can produce fresh
pinned-compiler rejection output outside the repository without changing that
closure status.

| Mutation | Existing direct-Verus target | Distinct body clause | Open association |
| --- | --- | --- | --- |
| `batching-publish-once` | `continuous_batching::apply_continuous_publish_step` | first publication succeeds; repeated publication is rejected | `scheduler_refined` / `batching-proof` |
| `batching-request-routing` | `continuous_batching::apply_continuous_batch_step` | stale generations are rejected without state change | `scheduler_refined` / `scheduler-proof` |
| `graph-operator-order` | `graph::expected_step` | every layer uses the exact operator order | `graph_refined` / `graph-proof` |
| `graph-role-step-count` | `graph::plan_step_count` | target and draft retain their exact distinct counts | `graph_refined` / `graph-proof` |
| `isolation-other-request-frame` | `continuous_batching::apply_continuous_batch_step` | a selected step preserves every other request slot | `request_isolated` / `isolation-proof` |
| `kv-release-generation` | `paged_kv_refinement::release_retired_page` | reuse advances the retired physical generation | `kv_refined` / `kv-proof` |
| `kv-rollback-retirement` | `paged_kv_refinement::rollback_physical_token` | removing a one-token tentative tail retires its exact initialized prefix | `kv_refined` / `kv-proof` |
| `kv-write-prefix` | `paged_kv_refinement::write_physical_token` | physical initialization advances with logical residency | `kv_refined` / `kv-proof` |
| `publication-phase-transition` | `step_plan_publication::publish_reserved_delta` | publication moves only from validated to published | `graph_refined` / `graph-proof` |
| `publication-plan-identity` | `step_plan_publication::validate_step_plan` | publication authority binds the exact plan identity | `graph_refined` / `graph-proof` |
| `speculative-accepted-count-binding` | `speculative_step_composition::settle_and_publish_speculative_step` | KV preflight uses the exact publication-derived accepted count | `rollback_refined` / `speculation-proof` |
| `speculative-atomic-failure-frame` | `speculative_step_composition::settle_and_publish_speculative_step` | all publication and KV validation precedes mutation, and any rejection preserves publication and selected state | `request_isolated` / `isolation-proof` |

The path column is an obligation association, not a claim that the named
future `proofs/m1/*.rs` path exists or has been discharged.
