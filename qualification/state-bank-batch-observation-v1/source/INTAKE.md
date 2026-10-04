# State-Bank Intake

Input schema: `ferric-p228-state-bank-batch-observation-inputs-v1`.
Package schema: `ferric-p228-state-bank-batch-observation-package-v1`.
Engineering review: `ferric-p228-state-bank-batch-engineering-review-v1`.
Pure receipt: `ferric-p228-state-bank-batch-pure-v1`.

The preceding CPU522 observer's closed plan field list is unchanged:

```text
schema output_label policy deployment prior_deployment image_deployment
standalone_prepared standalone_cases numericals request decode_review
parent_runtime_review worker_runtime_review comparison_baseline
comparison_reference comparison_tests comparison_test_sources
supervisor_tests supervisor_test_sources
```

`deployment` is the new paired-source worker deployment; its `prior_deployment`
must be the explicit actual CPU522 pin in the plan. The new verifier authenticates
the combined source qualification and actual Cargo worker identity. The exact
unchanged `resident_state_portable.py` independently authenticates CPU522.
Both runtime maps must equal their verified deployment maps. Parent CPU633,
base deployment and historical image remain identical. The prior worker must
be the actual 4,780,024-byte CPU522 binary with digest `79d2b50a...`; the new
worker must differ and comes from actual build evidence, not a predicted pin.

`historical_runtime` now records CPU522, not CPU475. The older CPU475/CPU633
chain remains authenticated through the unchanged deployment helpers. Only
after this worker-generation join is the separately qualified V7 image selected.
All six actual prefix GPU cases and six conditional numerical receipts, their
exact policies, assumptions, source/ISA provenance and retained input ledgers
are checked by the unchanged functions. No old arithmetic condition is
promoted to a discharged premise or whole-model numerical acceptance.

Labels are `prefix-state-bank-batch-{tf4|ar4}-{policy}-gpu-v228-vN`; pure labels
are `state-bank-batch-observation-pure-v228-vN`. The three explicit host policies
remain separate, with no combined or default arm. Old schemas/labels fail.
The root pure runner basename is
`run_state_bank_batch_observation_pure_p228_v1.py`.

Root must supply a newly reviewed worker-runtime receipt binding the new
binary, a current parent-runtime review, and substantive engineering review
notes binding the exact new plan and six required topics. Reviews are inputs;
this package does not construct them. Source formatting, CPU tests, prior GPU
success and pure-test success do not confer runtime review authority.

The candidate context, request validation, model/weights, comparison reference,
original topology/device premises and both input ledger rechecks remain
unchanged. Native comparison calls the same structural-only observation helper;
it does not call the paired-bitwise or independent numerical comparison path.

Root stages unchanged helpers from the CPU522 observer, plus all four bank
deployment modules (`bank_batch_portable.py`, `export_bank_batch.py`,
`audit_bank_batch_runtime.py`, `test_bank_batch_portable.py`). The exporter and
auditor require these modules to be in the same frozen manifest and directory
as this intake, without `PYTHONPATH` overrides. `PACKAGE_FILES`, `PURE_TESTS`
and `TEST_RUNNER_SHA` remain fail-closed placeholders until root binds the real
source roster and separately authored pure runner. The preceding package's
pure receipt cannot qualify this successor.

The adapted intake file contains 28 authored tests, preserving the preceding
25 scenarios and adding three generation/refusal checks. The controller file
retains 16 orchestration tests. These are unexecuted author declarations;
root must discover and run the complete assembled package, including unchanged
regressions and new verifier tests. No author import, syntax check, build,
test, SSH operation or GPU launch was performed.
