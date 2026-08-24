# M1 fe2o3 Contract Evidence

`validate-fe2o3-contract.py` implements
`ferric.m1-validator.fe2o3-contract.v1`. The production evidence-index checker
owns that path, protocol, and the validator source SHA-256. An index cannot
select another executable.

## Canonical layout

For an evidence artifact named `<artifact-id>`, the report and its two bound
declarations have fixed locations relative to the evidence-index directory:

```text
artifacts/<artifact-id>.fe2o3-contract.json
contracts/<artifact-id>.fe2o3-contract-body.json
contract-sets/<artifact-id>.fe2o3-contract-set.json
```

Every file is canonical, pretty-printed ASCII JSON with one trailing newline
and no duplicate, missing, or extra fields. Every path component must be a
regular non-symlink entry, and the validator checks each file's identity before
and after its bounded read.

The report binds the checker-owned evidence binding, exact still-`Open` roadmap
obligation or assurance property, exact path and `composition`, `kernel`, or
`runtime` profile, requirements SHA-256, ordered Ferric and fe2o3
commit/tree/source-closure identities, fixed `gfx942:xnack-` target, and the
complete ordered compiler/hardware/runtime TCB. It also carries the exact
manifest-declared assurance roster, including each property name, fe2o3 kind,
boundary SHA-256, future closure status, and current `Open` state.

The contract body repeats the authority-relevant binding and identity fields.
Its exact bytes are the `ArtifactIdentityV1` selected by the
`ContractedEvidenceV1` declaration. The contract-set document is a deterministic
Ferric-defined JSON projection of a one-property, one-obligation declaration
with:

- schema `fe2o3-proof-contracts::ContractSetV1` at
  `crates/fe2o3-proof-contracts/src/model.rs`;
- one domain-separated Ferric M1 extension property in status `Contracted`;
- one matching `ContractedEvidenceV1` binding to the contract-body bytes;
- one exactly satisfied obligation requiring status `Contracted`; and
- empty ContractSet-local TCB and correspondence vectors, as required for this
  contracted-evidence variant.

The enclosing report still binds the complete M1 TCB. The declaration names
`ContractSetV1::validate_closed-structural-only` as a descriptive Ferric label,
not as an upstream Rust symbol. The authenticated fe2o3 source establishes the
Rust contract structs and `ContractSetV1::validate_closed()`, but neither the
Ferric producer nor the trusted Python validator instantiates those structs or
executes fe2o3 Rust code. They produce and validate only the fixed JSON
projection; acceptance does not establish that upstream Rust accepted it.

The validator rejects use under a profile that does not require
`fe2o3-contract`, noncanonical paths, traversal, symlinks, replay across
bindings, noncanonical or duplicate-key JSON, report or companion substitution,
source/path/target/TCB drift, property omission or promotion, ContractSet field
or identity drift, stronger self-reported authority, and all schema drift.

## Ferric producer

The planner exposes exactly 52 binding-local `fe2o3-contract` commands. Produce
one selected binding with:

```text
python3 -I proofs/m1-qualification/produce-fe2o3-contract.py \
  FERRIC_REPO FE2O3_REPO PLAN_DIR binding.NNNNN
```

The selected binding must be one of those exact 52 slots. One invocation
projects and publishes exactly these binding-owned files without replacement:

```text
contracts/<artifact-id>.fe2o3-contract-body.json
contract-sets/<artifact-id>.fe2o3-contract-set.json
artifacts/<artifact-id>.fe2o3-contract.json
```

The contract body and contract set publish before the report. The report
publishes last as the transaction's completion marker. A failed transaction
rolls back only the exact files created by that invocation; it preserves a
substituted inode and reports the rollback failure. The producer grants only
`contract-declaration-structure-only` authority and leaves the selected
obligation or assurance property `Open`. It does not invoke or modify the
trusted validator: the checker-owned validator path, protocol, and source
SHA-256 pin remain unchanged.

## Authority boundary

Acceptance authenticates only the exact ContractSet and contracted-property
JSON declaration for the identity-bound Open M1 binding. It makes no
implementation or semantic claim and is not a proof. Acceptance grants no
theorem, machine behavior or refinement, load, launch, queue, completion,
hardware, performance, or qualification authority. It creates neither an
evidence index nor a qualification receipt, and it closes no M1 roadmap
requirement, assurance property, or path obligation.
