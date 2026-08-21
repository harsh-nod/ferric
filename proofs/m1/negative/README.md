# M1 Foundation Mutation Scaffold

`REQUIRED_FOUNDATIONS` is the finite
`FERRIC-M1-NEGATIVE-FOUNDATIONS-V1` registry for the direct-Verus foundation
bodies that currently exist for exact graph planning, step-plan publication,
logical paged KV, continuous batching, and request-slot noninterference.

Each row binds one unique body mutation and contract clause to:

- an exact `Open` assurance property and one of its exact `Open` future path
  obligations in `M1_REQUIREMENTS.json`;
- an exact package, source, crate-root `cargo-verus` module, and function whose
  derived compiler path is already recorded as `verified=` in
  `VERIFIED_MODULES`; and
- the exact `postcondition` or `assertion` failure class required from pinned
  Verus after the mutated source first passes ordinary Cargo compilation.

`check-registry.py` hard-codes the exact ten-row roster. It first runs the M1
requirements checker, then rejects row omission, addition, reordering,
rebinding, duplicate mutators or clauses, unsafe paths, missing files, targets
outside compiler-rooted coverage, and any property or path that is no longer
`Open`. Every mutator replaces one uniquely occurring executable-body anchor.
It does not edit a function contract, specification relation, or proof-tool
configuration.

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
`VERUS_CLOSURE_MANIFEST`, records the exact commit/tree/compiler/registry
identity, applies the strict mutator, requires `cargo check --all-targets` to
pass, and requires the exact compiler function to fail a proof obligation
under `--no-cheating`. Output paths must be new. Each scratch source is removed
after its sequential row; dependency build targets are shared between rows and
removed when the complete run exits to bound rebuild time and disk use.

## Non-Claims

Registry membership is only a declared hostile-test requirement. A successful
runner row is a fresh pinned-compiler rejection for that body and clause, but
it is not an M1 negative-mutation evidence product: the checker-owned M1
negative-mutation validator remains a `RequiredFuture` path with no admitted
source pin. Neither state closes a roadmap item, assurance property, or future
path obligation.

This scaffold adds no artifact, allocation, address, queue, GPU, kernel,
runner, launch, graph-composition, hardware, numerical, side-channel, or
performance authority. It deliberately does not enter the M0 registry,
property binder, qualification runner, receipt format, or evidence set.
