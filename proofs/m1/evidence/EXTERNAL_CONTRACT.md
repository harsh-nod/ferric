# M1 External Contract Evidence

`validate-external-contract.py` implements
`ferric.m1-validator.external-contract.v1`. The production evidence-index
checker owns that path, protocol, and the validator source SHA-256. An index
cannot select another executable.

## Canonical layout

For an evidence artifact named `<artifact-id>`, the external contract report
has this fixed location relative to the evidence-index directory:

```text
artifacts/<artifact-id>.external-contract.json
```

The report is canonical, pretty-printed ASCII JSON with one trailing newline
and no duplicate, missing, or extra fields. It binds the checker-owned evidence
binding, exact still-`Open` roadmap obligation or assurance property, associated
assurance properties, exact `runtime` profile, path resolution, requirements
SHA-256, ordered Ferric and fe2o3 commit/tree/source-closure identities, selected
source identity, and the complete ordered compiler/hardware/runtime TCB.

The version 1 contract has the fixed target `gfx942:xnack-`, the fixed scope
`external-compiler-runtime-hardware-assumptions`, and exactly these ordered
declarations:

- compiler object emission conforms to the declared target;
- runtime load and dispatch conform to the AMDHSA contract;
- driver and firmware memory, queue, and completion behavior conform to the
  declared ABI; and
- gfx942 execution conforms to the declared ISA and memory model.

The validator rejects use outside the manifest's `runtime` evidence profile,
absolute or noncanonical paths, traversal, symlinks in the report path, replay
across bindings, noncanonical or duplicate-key JSON, source/path/TCB drift,
assumption omission, injection, or reordering, status promotion, self-reported
stronger authority, and any schema drift. It opens the report as a stable
regular non-symlink file and independently checks its size and SHA-256.

## Ferric-owned producer

The planner exposes exactly 15 binding-local declarations. After the three
global TCB reports exist, materialize one with:

```text
python3 -I proofs/m1-qualification/produce-external-contract.py \
  FERRIC_REPO FE2O3_REPO PLAN_DIR binding.NNNNN
```

The producer independently replays the complete plan and queue, authenticates
both clean source repositories and source closures, and holds all three TCB
reports through exclusive publication. It creates the canonical owner-private
report last without replacement and emits no companion payload, evidence index,
receipt, validation result, or status transition. The producer does not invoke
or import the trusted validator. Its ownership is legitimate because it records
Ferric's declared assumptions; it does not attest that an external component
implements or satisfies them.

## Authority boundary

Acceptance authenticates only that the fixed external assumptions were
declared for the exact identity-bound Open obligation. It does not establish
that any assumption is implemented or satisfied. It grants no theorem,
machine-refinement, load, launch, hardware, performance, or qualification
authority. It creates neither an evidence index nor a qualification receipt,
and it closes no M1 roadmap requirement, assurance property, or path
obligation.
