# M1 Foundation Mutation Scaffold

`REQUIRED_FOUNDATIONS` is the finite
`FERRIC-M1-NEGATIVE-FOUNDATIONS-V1` registry for the direct-Verus foundation
bodies that currently exist for exact graph planning, step-plan publication,
atomic speculative publication/KV composition, logical paged KV, continuous
batching, request-slot noninterference, and retained model-bundle record
composition.

Each row binds one unique body mutation and contract clause to:

- an exact `Open` assurance property and one of its exact `Open` future path
  obligations in `M1_REQUIREMENTS.json`;
- an exact package, source, crate-root `cargo-verus` module, and function whose
  derived compiler path is already recorded as `verified=` in
  `VERIFIED_MODULES`; and
- the exact `postcondition` or `assertion` failure class required from pinned
  Verus after the mutated source first passes ordinary Cargo compilation.

`check-registry.py` hard-codes the exact thirteen-row roster. It first runs the M1
requirements checker, then rejects row omission, addition, reordering,
rebinding, duplicate mutators or clauses, unsafe paths, missing files, targets
outside compiler-rooted coverage, and any property or path that is no longer
`Open`. Every mutator replaces one uniquely occurring executable-body anchor.
The speculative accepted-count row sends a value other than the
publication-derived count to KV preflight; the failure-frame row changes
publication on the KV rejection path. Both are ordinary executable-body edits,
not contract, specification-relation, assertion-only, or proof-tool mutations.
Their paired positive rows select dedicated query-bearing wrappers; negative
rows continue to mutate these actual bodies rather than the wrappers.
The model-bundle mutation removes the equality gate between the retained record
and a freshly recomputed verified seal. That is an internal consistency check,
not independent authentication, `WeightSectionManifest::valid_commitment`,
manifest destination layout, tensor-name semantics, runtime `BTreeSet` roster
completeness, or plan composition.

Run the structural policy directly:

```sh
python3 -I proofs/m1/negative/check-registry.py \
  . proofs/m1/negative/REQUIRED_FOUNDATIONS /tmp/m1-active
python3 -I proofs/m1/negative/test-policy.py .
```

Run every row, or a named subset, against a clean committed worktree and the
exact pinned Verus closure:

```sh
proofs/m1/negative/run-same-source.sh \
  . "$VERUS_ROOT" /tmp/m1-negative-output [MUTATION ...]
```

The runner refuses a dirty worktree, authenticates
`VERUS_CLOSURE_MANIFEST`, measures the exact M1 checker source closure, and
records the exact commit, tree, compiler closure, registry, and runner
identities. It applies the strict mutator, requires `cargo check --all-targets`
to pass, and requires the exact compiler module and function to fail a proof
obligation under `--no-cheating`. Output paths must be new. Each selected row
has an ordered mutation record, compile transcript, Verus transcript, and
`.result` manifest that binds the companion names, sizes, hashes, and exit
statuses. Each scratch source is removed after its sequential row; dependency
build targets are shared between rows and removed when the complete run exits
to bound rebuild time and disk use.

## Non-Claims

Registry membership is only a declared hostile-test requirement. A successful
runner row is a fresh pinned-compiler rejection for that body and clause. The
checker-owned, source-pinned negative-mutation validator can authenticate a
canonical `.result` and its complete dedicated run directory. For an evidence
binding, the selected roster must be exactly every registry row assigned to
that assurance property and path; partial or cross-property replay is rejected.
The validator does not create an evidence index or qualification receipt. It is
an `ExistingFoundation`; all registry-associated proof paths and assurance
properties remain `Open`. Neither a runner result nor validator acceptance
closes a roadmap item, assurance property, or future path obligation.

This scaffold adds no artifact, allocation, address, queue, GPU, kernel,
runner, launch, graph-composition, hardware, numerical, side-channel, or
performance authority. It deliberately does not enter the M0 registry,
property binder, qualification runner, receipt format, or evidence set.
