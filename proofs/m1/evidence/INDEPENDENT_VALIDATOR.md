# M1 Independent-Validator Evidence

`validate-independent-validator.py` implements
`ferric.m1-validator.independent-validator.v1`. The production evidence-index
checker owns that path, protocol, and validator source SHA-256. An evidence
index cannot choose an executable or override the protocol.

## Canonical layout

For an evidence artifact named `<artifact-id>`, the report and its immutable
companions have fixed locations relative to the evidence-index directory:

```text
artifacts/<artifact-id>.independent-validator.json
validator-runs/<artifact-id>.independent-validator.roster.json
validator-runs/<artifact-id>.independent-validator.transcript.json
```

All three files are canonical, pretty-printed ASCII JSON with one trailing
newline and no duplicate, missing, or extra fields. They must be stable regular
files below one nonsymlink evidence root. The checker bounds every read and
compares device, inode, mode, size, modification time, and change time before
and after the read. The report binds the exact size and SHA-256 of both
companions; the transcript also binds the roster SHA-256.

The report authenticates:

- the exact canonical M1 requirements bytes and still-`Open` roadmap or
  assurance obligation;
- the statement, ordered assurance-property roster and boundaries, path
  resolution, evidence profile, and fixed `gfx942:xnack-` target;
- both ordered Ferric and fe2o3 commit, tree, base, and source-closure
  identities;
- the complete ordered compiler, hardware, and runtime TCB identities;
- the independent checker organization, repository, commit, tree, source
  closure, executable path and hash, semantic version, protocol, and exact
  input and output schema hashes; and
- every case input hash, output hash, expected status, observed status, and
  exit code in the checker-owned case matrix.

The external checker protocol is
`ferric.external-independent-validation.v1`. The checker source closure,
executable identity, and input and output schemas must have four distinct
hashes. Its organization and repository cannot name Ferric, fe2o3, or
`harsh-nod`; its commit, tree, source closure, and executable cannot alias a
subject source identity. Its executable path cannot name this trusted
evidence validator. These checks reject declared self-validation. They
authenticate the declaration; they do not establish social or organizational
independence beyond the bound identities.

## Exact cases

Version 1 admits exactly this ordered matrix:

| Case | Expected observation | Exit code |
| --- | --- | --- |
| `canonical-subject` | `PASS` | `0` |
| `boundary-conforming-subject` | `PASS` | `0` |
| `obligation-substitution` | `EXPECTED_FAIL` | `1` |
| `property-substitution` | `EXPECTED_FAIL` | `1` |
| `path-substitution` | `EXPECTED_FAIL` | `1` |
| `profile-substitution` | `EXPECTED_FAIL` | `1` |
| `source-closure-substitution` | `EXPECTED_FAIL` | `1` |
| `target-substitution` | `EXPECTED_FAIL` | `1` |
| `tcb-substitution` | `EXPECTED_FAIL` | `1` |
| `malformed-status` | `EXPECTED_FAIL` | `1` |

The roster and transcript must contain the matrix exactly once and in this
order. All twenty input and output hashes are non-placeholder and mutually
separated by role. A skipped, duplicated, reordered, replayed, or
status-substituted case fails closed. The only accepted aggregate status is
`PASS`, with exactly two passing and eight expected-failure observations.

## Replay and substitution

The roster repeats hashes of the exact evidence binding, requirements,
assurance-property declarations, path resolution, profile, source roster,
target, and TCB roster. The transcript repeats the binding, checker identity,
roster identity, and case observations. The report repeats all of those
identities and the companion byte hashes. Therefore a report, roster, or
transcript copied across obligations, profiles, paths, source closures,
targets, TCBs, or checker identities is rejected. This is identity-bound replay
protection; the protocol does not claim a wall-clock freshness oracle.

The validator additionally rejects unsafe or noncanonical paths, symlinks in
any path component, unstable file metadata, oversized files, duplicate JSON
keys, noncanonical serialization, malformed timestamps or statuses, unknown
fields, source or validator substitution, and authority promotion.

## Authority boundary

Acceptance authenticates an independent checker identity and the exact
observations in its immutable transcript. Those observations are not a theorem,
machine refinement, artifact load, device launch, hardware result, performance
result, or qualification authority. The validator does not inspect or execute
the external checker, and it does not convert an observation into semantic
proof.

This validator creates neither an M1 evidence index nor a qualification
receipt. It does not change `RequiredFuture` path availability and closes no
roadmap requirement, assurance property, or path obligation. All M1 states
remain `Open`.

The hostile policy test covers canonical Ferric and fe2o3 bindings, Roadmap and
Assurance obligations, self-validation, checker/source substitution, case
omission and status drift, replay, malformed canonical JSON, unsafe paths, and
symlinked files:

```text
python3 -I proofs/m1/evidence/test-independent-validator-policy.py FERRIC_REPO
```
