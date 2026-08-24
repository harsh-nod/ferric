# M1 External Evidence Planner

`planner.py` is the first planning-only slice of the external M1 qualification
orchestrator. It authenticates clean, exact Ferric and fe2o3 source identities,
the admitted 12-package direct pin roster, 27-package resolved pin roster and
19-edge dependency topology, all 39 source paths, the requirements manifest,
and the checker-owned validator registry. It runs the existing source closure
producer once for each repository and creates a new external planning bundle:

```text
python3 -I proofs/m1-qualification/planner.py \
  FERRIC_REPO FE2O3_REPO NEW_OUTPUT_DIR
```

The bundle contains `plan.json`, `missing-work.json`, two exact source closure
records, and their preflight transcripts. The planner allocates the exact
minimum 354 realizable bindings: 168 Roadmap bindings and 186 Assurance
bindings. The extra Assurance binding beyond the unconstrained profile/path
count is required because both `graph_refined` foundation kinds can bind only
`graph-proof`; two flexible kinds must repeat on distinct remaining paths.

The work queue names every expected primary artifact, its producer role, and
whether an in-repository producer exists. The theorem and negative-mutation
runners, the three declaration-only TCB reporters, all 74 artifact-identity
bindings, all 14 canonical-structure bindings, all 15 external-contract
bindings, and the five exact unsupported-rationale bindings are represented as
available commands. The queue therefore has 167 available producer items and
191 missing producer items. All other binding-evidence producers and the shared
receipt remain explicitly missing.

After producing a plan against the final clean source identities, materialize
the three global TCB declarations independently:

```text
for subject in tcb.compiler tcb.hardware tcb.runtime; do
  python3 -I proofs/m1-qualification/produce-tcb-report.py \
    FERRIC_REPO FE2O3_REPO PLAN_DIR "$subject"
done
```

Each invocation reauthenticates the plan/work-queue identity, exact clean
source trees and closures, requirements, path resolutions, and checker-owned
validator registry before publishing one canonical report without replacement.
The producer does not import the trusted TCB validator, emit an evidence index
or receipt, or change an `Open` obligation. Once all three files exist, their
SHA-256 values form the outer TCB roster supplied to every trusted evidence
validator.

After all three TCB declarations exist, materialize one identity-only payload
and report for an exact planner-selected binding:

```text
python3 -I proofs/m1-qualification/produce-artifact-identity.py \
  FERRIC_REPO FE2O3_REPO PLAN_DIR binding.NNNNN
```

The binding selects one authenticated path and source repository. The producer
copies that exact stable regular source file as an opaque payload at
`identified-artifacts/artifact.binding.NNNNN.bin`, then publishes the canonical
report without replacement. This source-file snapshot is the producer's
operational convention; the unchanged validator grants only opaque byte
identity and canonical structure, not source provenance or semantic authority.
Each invocation reauthenticates the clean source repositories, exact complete
plan and work queue, source closures, requirements and validator pins, and all
three canonical TCB reports. It leaves every obligation `Open`.

Materialize one of the planner's 14 canonical-structure reports and its typed
companion payload with:

```text
python3 -I proofs/m1-qualification/produce-canonical-structure.py \
  FERRIC_REPO FE2O3_REPO PLAN_DIR binding.NNNNN
```

The payload is an exact six-record operational projection of the selected
stable source path: normalized declared availability, source-file SHA-256 and
size, source identity, regular-file observation, and relative path. The
producer holds and revalidates the exact plan, work queue, source closures,
source file, clean repository identities, and three TCB reports while it
publishes the owner-private canonical payload and then its report without
replacement. The report is the completion marker. The trusted validator grants
only `canonical-structure-only` authority: it validates the record encoding and
binding, not the truth or semantics suggested by record names, source code,
kernel behavior, runtime behavior, hardware, or qualification.

Materialize one of the planner's 15 external-contract declarations with:

```text
python3 -I proofs/m1-qualification/produce-external-contract.py \
  FERRIC_REPO FE2O3_REPO PLAN_DIR binding.NNNNN
```

The selected binding is an exact `runtime`-profile Roadmap or Assurance slot.
The producer replays the complete source-pinned plan, holds and revalidates the
plan, work queue, both source closures, clean source repository identities, and
all three TCB reports, then publishes one owner-private report without
replacement. It independently projects only the validator's four fixed
compiler, runtime, driver/firmware, and hardware assumptions. It does not invoke
the trusted validator and grants only `declared-assumptions-only` authority: the
report does not establish implementation, satisfaction, external review,
machine refinement, load, launch, hardware, performance, or qualification.

The three M1 properties whose required closure status is `Unsupported` have
five exact path-bound nonclaim reports. Materialize one selected report with:

```text
python3 -I proofs/m1-qualification/produce-unsupported-rationale.py \
  FERRIC_REPO FE2O3_REPO PLAN_DIR binding.NNNNN
```

The selected binding must be one of the planner's five `unsupported-rationale`
slots. The producer independently replays the complete source-pinned plan,
reauthenticates the exact clean source closures and all three canonical TCB
reports, and projects only the validator's fixed rationale, reason code, and
excluded-claim roster. It publishes one owner-private canonical report without
replacement and grants only `nonclaim-only` authority. It emits no companion
payload, evidence index, receipt, positive validation result, or status closure.

This command never emits an evidence index or qualification receipt. Those
outputs remain forbidden until every external artifact exists and the complete
candidate closure passes `proofs/check-m1-evidence-index.py`. The plan has
`planning-only-no-evidence` authority and changes no `Open` M1 obligation.

Run the focused hostile policy with:

```text
python3 -I proofs/m1-qualification/test-policy.py FERRIC_REPO FE2O3_OBJECT_REPO
python3 -I proofs/m1-qualification/test-tcb-producer-policy.py \
  FERRIC_REPO FE2O3_OBJECT_REPO
python3 -I proofs/m1-qualification/test-artifact-identity-producer-policy.py \
  FERRIC_REPO FE2O3_OBJECT_REPO
python3 -I proofs/m1-qualification/test-canonical-structure-producer-policy.py \
  FERRIC_REPO FE2O3_OBJECT_REPO
python3 -I proofs/m1-qualification/test-external-contract-producer-policy.py \
  FERRIC_REPO FE2O3_OBJECT_REPO
python3 -I proofs/m1-qualification/test-unsupported-rationale-producer-policy.py \
  FERRIC_REPO FE2O3_OBJECT_REPO
```

`FE2O3_OBJECT_REPO` may have any checked-out branch, but its local Git object
store must contain the exact revision pinned by `FERRIC_REPO`. The policy uses
disposable shared clones and leaves both supplied repositories unchanged.
