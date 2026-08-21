# M1 Canonical-Structure Transcript

`proofs/m1/evidence/validate-canonical-structure.py` implements protocol
`ferric.m1-validator.canonical-structure.v1`. The evidence index invokes this
checker-owned program for a `canonical-structure-check` binding whose artifact
kind is `CheckerTranscript`.

The validator accepts one canonical JSON context on standard input and the
protocol as its only argument. It independently loads the exact canonical
`proofs/M1_REQUIREMENTS.json`, the bound report, and its companion payload. It
rejects duplicate keys, unknown fields, noncanonical serialization, unsafe or
symlinked paths, byte or digest drift, oversized inputs, replay across
bindings, and incomplete source or TCB rosters.

## Bound identities

Every report is bound to the exact requirements SHA-256, Open roadmap or
assurance obligation, statement, evidence profile, path resolution, source
identity roster, `gfx942:xnack-` target, and compiler/runtime/hardware TCB
roster supplied by the already structurally validated evidence index. Roadmap
reports repeat the exact ordered assurance-property dependencies; assurance
reports name only their bound property. A profile is accepted only when the
requirements manifest assigns `canonical-structure-check` to that profile.

The artifact path is fixed by its artifact id:

```text
artifacts/<artifact-id>.canonical-structure.json
```

Its referenced payload path is likewise fixed:

```text
canonical-payloads/<artifact-id>.json
```

Both files must be stable regular files below the same nonsymlink evidence
root. The report uses format `FERRIC-M1-CANONICAL-STRUCTURE-V1`; the payload
uses format `FERRIC-M1-CANONICAL-RECORDS-V1` and checker-owned schema id
`ferric.m1-canonical-records.v1`.

## Canonical records

The payload repeats the exact binding, obligation, profile, path, source, and
target identities. It contains a nonempty, lexicographically ordered roster of
at most 1,024 uniquely named typed records. Record names are bounded lowercase
identifiers. The admitted value types are:

- `boolean`: a JSON boolean;
- `count`: a nonnegative JSON integer no greater than `2^63 - 1`;
- `identifier`: a bounded path-free identifier;
- `sha256`: a lowercase, non-placeholder SHA-256 value;
- `text`: one bounded line of printable ASCII text.

The checker hashes its own schema descriptor and requires the report to bind
that identity. It parses and validates every record rather than trusting a
self-reported success field. It also checks the report's exact payload byte
length and SHA-256 and requires `result` to be `canonical` only after those
checks have succeeded.

## Authority boundary

The accepted authority is `canonical-structure-only`. The required nonclaim
states that the transcript establishes only conformance of the referenced
bytes to this checker-owned record schema and exact evidence binding. It does
not establish semantic correctness, a theorem, machine correspondence,
artifact loading or launch, hardware behavior, performance, or M1
qualification. All roadmap requirements and assurance properties remain
`Open`; this validator does not create an evidence index or qualification
receipt.

The hostile policy test exercises canonical Roadmap and Assurance bindings,
both admission and authentication profiles, Ferric and fe2o3 paths, every
typed value, report and payload replay, source/TCB/path drift, malformed JSON,
symlinks, and authority promotion:

```text
python3 -I proofs/m1/evidence/test-canonical-structure-policy.py FERRIC_REPO
```
