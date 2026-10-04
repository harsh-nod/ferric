# Independent Four-Forward Intake

Unfrozen implementation. Eighteen new pure test methods are authored and have
not been run by the author. No imports, subprocesses, tests, SSH, or GPU work was
performed while authoring these files. Root owns package finalization, actual
test results, scoped reviews, and launch.

## Closed Plan

Schema: `ferric-p228-independent-decode-observation-inputs-v1`.
The exact fields are:

```text
schema output_label policy deployment image_deployment standalone_prepared
standalone_cases numericals request decode_review parent_runtime_review
worker_runtime_review comparison_baseline comparison_reference comparison_tests
comparison_test_sources supervisor_tests supervisor_test_sources
```

All non-label/policy/schema fields are FilePins, except `standalone_cases` and
`numericals`, which are six ordered FilePin lists. Case order is genuine-pos0,
genuine-pos4, patterned-pos15, patterned-pos16, patterned-pos2047, patterned-pos2048.
The numerical list is the actual retained `prefix-independent-numerical-v228-v1`
through `v6` cohort; its exact observed completion SHAs are constants, not
predictions of future output.

Fresh labels are `prefix-independent-decode-{tf4|ar4}-{policy}-gpu-v228-vN`.
The policy must be explicitly one of baseline, immutable-admission-cache, or
shared-full-currentness. The label, request, review, and mode must agree. There
is one attempt and no retry.

## Reused Boundaries

The frozen `p228-group-fence-gpu-v3` intake is loaded by exact manifest and source
pins for its unchanged historical deployment, comparator, request, model/prompt,
and baseline-image checks. Its `context`, old parity prerequisites, and bitwise
success gate are never used. The original GP verification still authenticates
CPU633 parent, CPU475 worker, and the historical image without changing that
record. `historical_runtime` preserves that returned mapping.

The separately frozen numerical CLI loader authenticates the new standalone
observer, adapter, reference-source closure, and their actual pure evidence. Its
`prepare.load` independently verifies the V7 deployment, reviewed artifact,
current standalone native runtime, prepared requests, and image provenance.
Every requested standalone completion is replayed with the frozen
`run_case.replay_case`, including complete capture/child custody and all six
recorded audits. No numerical references are executed by this intake.

Each actual numerical completion is joined to that replay, source packages,
exact CLI controller, arithmetic assumptions review, both independent profiles,
both ranks, capture/stage hashes, and output weights. Both outer ledgers and both
profile ledgers are rehashed. The fixed original conditional completion hashes
authenticate the existing bounds and outcomes; this intake neither re-proves
those results nor grants unconditional arithmetic acceptance.

`selected_runtime` and `runtime` are a new mapping containing the historical
parent and worker plus the independently verified V7 image. The exact original
request checker then binds the supplied prefix override to this selection and
the new prepared object, retaining all other model, prompt, device, setup image,
MLP tile image, timeout, and output-path checks. Mode-specific native/framework
reference files remain authenticated and available for later CPU diagnostics,
but no equality with them is required for a structural observation.

## Root Review And Builder Inputs

Root must provide the actual old group-fence deployment, the actual new V7
deployment, new prepared pin, six actual GPU completions, six actual conditional
numerical completions, selected mode's actual baseline/framework references, and
existing comparator pure receipt/source snapshot. The actual new supervisor
pure receipt and source snapshot are required after root freezes/tests this
package. `PACKAGE_FILES` and `PURE_TESTS` intentionally remain `None` until root
finalizes the roster; `context` refuses without them. The separately authored
pure-runner SHA is explicitly pinned, not borrowed from the old runner.

The request uses the existing V2 host-policy wrapper and V1 decode body. Preserve
the actual CPU475 worker, original setup images, MLP tile image, model, prompt,
device IDs, and finite limits. Select only the actual V7 `decode.prefix_image`,
a fresh session, fresh native evidence directory, and explicit TF/AR mode.

Root must supply freshly reviewed selected-parent and selected-worker runtime
reviews on the same current platform. The new engineering review schema is
`ferric-p228-independent-decode-engineering-review-v1`; its exact fields are:

```text
schema reviewed authority policy parent worker image historical_runtime
image_provenance review_topics notes gpu_attempts deployment image_deployment
standalone_prepared standalone_cases numericals request parent_runtime_review
worker_runtime_review comparison_baseline comparison_reference
production_authority full_model_acceptance independent_numerical_acceptance
arithmetic_prerequisites_verified runtime_premises_discharged performance_claim
```

`reviewed` must be true, `authority` must be `none`, `gpu_attempts` must be integer
1, and all six trailing authority flags must be false. `image_provenance` must be
the independently verified standalone context's complete `provenance` mapping;
it is not a newly manufactured receipt. The six `review_topics` keys are
source_lineage, formal, isa, coherence, lifecycle, and selected_device. Each value
and `notes` must contain substantive root-authored scoped notes. This intake
never creates reviews or marks unproved runtime/arithmetic premises discharged.

## Controller Contract

`context(plan_pin)` returns the existing controller API plus `selected_runtime`,
`historical_runtime`, `image_deployment`, and `image_provenance`. There is no
`layer_receipt`. `deployment` and `base_deployment` are still the original worker
and parent deployment cohorts. `standalone_receipts` equals the new six case
pins, and `standalone.pins` remains a separate immutable custody ledger.

`compare_native(c, records, host_sidecar, leaf_record, value)` calls only the new
data-only `observation.observe` helper. It does not call legacy paired comparison
or numerical mathematics. `guard` rechecks both ledgers and the current selected
runtime library aliases. The controller must preserve its existing one-attempt
ownership, Close/reap, full capture retention, all three pre-/three post-audits,
and resource limits. The top-level root must serialize and reap tasks as before.

The pure tests exercise schema/refusal, immutable selection, review joins,
ledger custody, and new-helper routing. Numerical routing tests use explicitly
synthetic result bodies and mock only document/ledger I/O, while using the actual
frozen CLI/observer interfaces. They are not numerical re-executions, deployment
replays, or GPU evidence. Full actual intake and native execution remain root
validation gates.
