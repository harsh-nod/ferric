# M1 Foundation Mutation Matrix

All rows are repository-declared requirements. No checked-in transcript or
M1 evidence binding authenticates them. `run-same-source.sh` can produce fresh
pinned-compiler rejection output outside the repository without changing that
closure status.

| Mutation | Existing direct-Verus target | Distinct body clause | Open association |
| --- | --- | --- | --- |
| `artifact-manifest-commitment-digest` | `auth::validate_manifest_commitment_verified` | the canonical manifest digest must exactly match the retained aggregate identity | `artifact_authenticated` / `bundle-auth` |
| `batching-publish-once` | `continuous_batching::apply_continuous_publish_step` | first publication succeeds; repeated publication is rejected | `scheduler_refined` / `batching-proof` |
| `batching-request-routing` | `continuous_batching::apply_continuous_batch_step` | stale generations are rejected without state change | `scheduler_refined` / `scheduler-proof` |
| `graph-operator-order` | `graph::expected_step` | every layer uses the exact operator order | `graph_refined` / `graph-proof` |
| `graph-role-step-count` | `graph::plan_step_count` | target and draft retain their exact distinct counts | `graph_refined` / `graph-proof` |
| `isolation-other-request-frame` | `continuous_batching::apply_continuous_batch_step` | a selected step preserves every other request slot | `request_isolated` / `isolation-proof` |
| `kernel-memory-read-initialization` | `m1_kernel_safety::validate_m1_kernel_memory_safety` | every modeled read is covered by one unambiguous initialized half-open range | `memory_safe` / `kernel-contract-proof` |
| `kernel-race-conflict` | `m1_kernel_safety::validate_m1_kernel_race_freedom` | distinct same-phase workitems reject overlapping access when either writes | `race_free` / `kernel-contract-proof` |
| `kernel-resource-workitem-bound` | `m1_kernel_safety::validate_m1_kernel_resource_bounds` | the finite workitem count stays within the exact family bound | `resource_bounded` / `kernel-schedule-catalog` |
| `kv-release-generation` | `paged_kv_refinement::release_retired_page` | reuse advances the retired physical generation | `kv_refined` / `kv-proof` |
| `kv-rollback-retirement` | `paged_kv_refinement::rollback_physical_token` | removing a one-token tentative tail retires its exact initialized prefix | `kv_refined` / `kv-proof` |
| `kv-terminal-release-exact-epoch` | `request_isolation::release_isolated_page` | the selected request's recorded quiescent epoch exactly matches the release authority epoch | `lifetime_safe` / `kv-proof` |
| `kv-write-prefix` | `paged_kv_refinement::write_physical_token` | physical initialization advances with logical residency | `kv_refined` / `kv-proof` |
| `model-bundle-record-binding` | `auth::admission_records_equal` | mismatching retained and recomputed record bytes are rejected before proof custody | `model_bundle_well_formed` / `model-bundle-proof` |
| `operator-declared-profile-effect` | `operation_kernel_plan::select_declared_operator_certificate` | the caller-supplied opaque operator profile identity field must be nonempty after the structural match | `operator_refined` / `kernel-contract-proof` |
| `publication-phase-transition` | `step_plan_publication::publish_reserved_delta` | publication moves only from validated to published | `graph_refined` / `graph-proof` |
| `publication-plan-identity` | `step_plan_publication::validate_step_plan` | publication authority binds the exact plan identity | `graph_refined` / `graph-proof` |
| `sampler-lowest-id-publication` | `m1_completion::select_lowest_argmax` | equal scores retain the first, lowest token ID before speculative publication | `sampler_refined` / `speculation-proof` |
| `speculative-accepted-count-binding` | `speculative_step_composition::settle_and_publish_speculative_step` | KV preflight uses the exact publication-derived accepted count | `rollback_refined` / `speculation-proof` |
| `speculative-atomic-failure-frame` | `speculative_step_composition::settle_and_publish_speculative_step` | all publication and KV validation precedes mutation, and any rejection preserves publication and selected state | `request_isolated` / `isolation-proof` |
| `target-catalog-processor-features` | `validation::validate_kernel_catalog_input` | any processor or target-feature drift is rejected before retaining the catalog input | `target_conforming` / `identity-closure` |

The path column is an obligation association, not a claim that the named
future `proofs/m1/*.rs` path exists or has been discharged.
