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
runners are represented as available commands. All other evidence producers,
the three TCB reports, and the shared receipt remain explicitly missing.

This command never emits an evidence index or qualification receipt. Those
outputs remain forbidden until every external artifact exists and the complete
candidate closure passes `proofs/check-m1-evidence-index.py`. The plan has
`planning-only-no-evidence` authority and changes no `Open` M1 obligation.

Run the focused hostile policy with:

```text
python3 -I proofs/m1-qualification/test-policy.py FERRIC_REPO FE2O3_OBJECT_REPO
```

`FE2O3_OBJECT_REPO` may have any checked-out branch, but its local Git object
store must contain the exact revision pinned by `FERRIC_REPO`. The policy uses
disposable shared clones and leaves both supplied repositories unchanged.
