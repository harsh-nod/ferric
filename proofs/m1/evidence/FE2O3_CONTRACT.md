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
one-property, one-obligation declaration with:

- schema `fe2o3-proof-contracts::ContractSetV1` at
  `crates/fe2o3-proof-contracts/src/model.rs`;
- one domain-separated Ferric M1 extension property in status `Contracted`;
- one matching `ContractedEvidenceV1` binding to the contract-body bytes;
- one exactly satisfied obligation requiring status `Contracted`; and
- empty ContractSet-local TCB and correspondence vectors, as required for this
  contracted-evidence variant.

The enclosing report still binds the complete M1 TCB. The declaration names
`ContractSetV1::validate_closed-structural-only` because the upstream method
checks bounded structure, identities, evidence/status agreement, and closure;
it does not authenticate a digest or establish the declared semantics.

The validator rejects use under a profile that does not require
`fe2o3-contract`, noncanonical paths, traversal, symlinks, replay across
bindings, noncanonical or duplicate-key JSON, report or companion substitution,
source/path/target/TCB drift, property omission or promotion, ContractSet field
or identity drift, stronger self-reported authority, and all schema drift.

## Authority boundary

Acceptance authenticates only the exact ContractSet and contracted-property
declaration for the identity-bound Open M1 binding. A contract is not an
implementation or a proof. Acceptance grants no theorem, machine-refinement,
load, launch, queue, completion, hardware, performance, or qualification
authority. It creates neither an evidence index nor a qualification receipt,
and it closes no M1 roadmap requirement, assurance property, or path obligation.
