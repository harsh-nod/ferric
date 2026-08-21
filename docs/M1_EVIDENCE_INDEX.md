# M1 Closure Evidence Index

This document specifies the external evidence index accepted by
`proofs/check-m1-evidence-index.py`. The format is
`ferric.m1-evidence-index.v1`.

The repository does not contain an M1 evidence index, qualification receipt,
or completed M1 claim. `proofs/M1_REQUIREMENTS.json` remains an obligation
manifest whose implementation states are all `Open`. An index is an external
qualification input and is accepted only after the implementation, proof,
validation, hardware, and performance work has produced the required
artifacts.

## Invocation

```text
python3 -I proofs/check-m1-evidence-index.py \
  FERRIC_REPO EVIDENCE_INDEX FE2O3_REPO
```

The evidence file must exist. The checker fails closed on a missing file,
noncanonical JSON, duplicate key, unknown field, unknown status, duplicate or
missing record, or unreferenced artifact.

## Closure Rows

The index contains exactly one row for each of the 33 roadmap requirements and
17 assurance properties in `proofs/M1_REQUIREMENTS.json`, in manifest order.
Roadmap rows have status `Closed`. Assurance rows use their exact
`required_status_at_closure`: `Proved`, `Validated`, or `Unsupported`. These
statuses are separate contracts, not an ordered scale:

- `Closed` requires the exact assurance dependency roster and an authenticated
  qualification receipt.
- `Proved` requires the exact theorem and negative-mutation artifacts bound by
  that row's evidence profiles.
- `Validated` requires the exact independent-validator artifacts and explicit
  validator TCB.
- `Unsupported` requires the manifest's exact nonclaim boundary, one bound
  rationale artifact, and explicit nonclaim TCB. It cannot discharge a
  dependency of a `Proved` row.

Changing a required status in either direction is rejected. A self-authored
summary or receipt cannot substitute for a theorem transcript, negative
mutation, independent validator, hardware transcript, or performance report.

## Identity Closure

The index binds the exact requirements-manifest SHA-256 and exact Ferric and
fe2o3 identities. Each source record names its repository, reviewed base
commit, qualified commit, tree, byte-for-byte source-closure artifact, and
source-closure SHA-256. The Ferric requirements base is the commit which
introduced this format; the fe2o3 base is the separately pinned M1 upstream
base from the requirements manifest. Qualified commits may be descendants,
but the supplied repositories must have the exact named `HEAD` commit and
tree. Each repository must be clean, and its measured closure path roster must
equal the committed tree after the checker's narrowly defined generated-file
exclusions.

Every required path resolves through one source identity and must be a regular
file in that repository's measured source closure. Every external receipt,
transcript, contract, rationale, hardware result, performance report, TCB
report, and closure file is a regular non-symlink file below the evidence-index
directory with exact size and SHA-256. Artifact paths and artifact identities
are unique.

## Evidence Products

For every closure row, the Cartesian product of its evidence profiles and each
profile's evidence kinds appears exactly once. Every binding names one
obligation, profile, evidence kind, resolved path, source identity, complete
TCB, statement SHA-256, uniquely owned artifact, and canonical binding
SHA-256. The bindings collectively cover every path named by the obligation.

The compiler, runtime, and hardware TCB entries are explicit and independently
authenticated. Evidence kinds map to distinct artifact kinds; in particular,
theorem, mutation, validator, hardware, and performance evidence cannot be
replaced by `ArtifactIdentityReport`, `CheckerTranscript`, `TcbReport`, or a
qualification receipt.

Artifact labels and hashes are not evidence by themselves. After structural
validation, the production CLI invokes checker-owned validators from the exact
Ferric source closure. The evidence index cannot provide or override these
paths. The version 1 registry is:

| Evidence | Trusted validator path | Protocol |
| --- | --- | --- |
| Artifact identity | `proofs/m1/evidence/validate-artifact-identity.py` | `ferric.m1-validator.artifact-identity.v1` |
| Canonical structure | `proofs/m1/evidence/validate-canonical-structure.py` | `ferric.m1-validator.canonical-structure.v1` |
| External contract | `proofs/m1/evidence/validate-external-contract.py` | `ferric.m1-validator.external-contract.v1` |
| fe2o3 contract | `proofs/m1/evidence/validate-fe2o3-contract.py` | `ferric.m1-validator.fe2o3-contract.v1` |
| Hardware transcript | `proofs/m1/evidence/validate-hardware-transcript.py` | `ferric.m1-validator.hardware-transcript.v1` |
| Independent validator | `proofs/m1/evidence/validate-independent-validator.py` | `ferric.m1-validator.independent-validator.v1` |
| Negative mutation | `proofs/m1/evidence/validate-negative-mutation.py` | `ferric.m1-validator.negative-mutation.v1` |
| Performance report | `proofs/m1/evidence/validate-performance-report.py` | `ferric.m1-validator.performance-report.v1` |
| Qualification receipt | `proofs/m1/evidence/validate-qualification-receipt.py` | `ferric.m1-validator.qualification-receipt.v1` |
| TCB report | `proofs/m1/evidence/validate-tcb-report.py` | `ferric.m1-validator.tcb-report.v1` |
| Unsupported rationale | `proofs/m1/evidence/validate-unsupported-rationale.py` | `ferric.m1-validator.unsupported-rationale.v1` |
| Verus theorem | `proofs/m1/evidence/validate-verus-theorem.py` | `ferric.m1-validator.verus-theorem.v1` |

These validators are `RequiredFuture` M1 implementation obligations. Until
every validator required by an index exists in the exact qualified source
closure, has a reviewed source SHA-256 pinned in the checker-owned registry,
and accepts its canonical context, the production checker cannot print an M1
closure `PASS`. All version 1 registry source pins are intentionally unset at
this scaffold stage. Adding a file at a registered path is therefore
insufficient. The private in-process callback used by synthetic policy tests
is deliberately absent from the CLI and is not a qualification mode.

This checker establishes structural completeness and cryptographic identity
closure. It does not establish the semantic truth of a theorem, the
independence of a validator, hardware correctness, or performance. Those
claims remain obligations of the named tools, reviewers, qualification
environment, and external artifacts.
