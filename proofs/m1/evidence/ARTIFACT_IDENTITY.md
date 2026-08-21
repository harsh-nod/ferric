# M1 Artifact Identity Evidence

`validate-artifact-identity.py` implements
`ferric.m1-validator.artifact-identity.v1`. The production evidence-index
checker owns that path, protocol, and the validator source SHA-256. An index
cannot select another executable.

## Canonical layout

For an evidence artifact named `<artifact-id>`, the report and the bytes it
identifies have fixed locations relative to the evidence-index directory:

```text
artifacts/<artifact-id>.artifact-identity.json
identified-artifacts/<artifact-id>.bin
```

The report is canonical, pretty-printed ASCII JSON with one trailing newline
and no duplicate, missing, or extra fields. It binds the checker-owned evidence
binding, exact still-`Open` roadmap obligation or assurance property, associated
assurance properties, path resolution, evidence profile, requirements SHA-256,
ordered Ferric and fe2o3 commit/tree/source-closure identities, and the complete
ordered compiler/hardware/runtime TCB. The payload declaration is exactly
`M1ImmutablePayload` for `gfx942:xnack-` and records its positive byte size and
SHA-256.

The validator rejects absolute or noncanonical relative paths, traversal,
symlinks in either report or payload paths, replay across bindings, payload
substitution, noncanonical JSON, source/path/TCB drift, status promotion,
self-reported stronger authority, and any schema drift. It opens the payload as
a regular non-symlink file and independently streams its SHA-256 while checking
that its file identity remains stable.

## Authority boundary

Acceptance grants only byte-identity and report-structure authority. The
payload kind is intentionally opaque. Acceptance does not establish semantic
correctness, a theorem, machine refinement, load or launch success, hardware
behavior, performance, or qualification. It creates neither an evidence index
nor a qualification receipt, and it closes no M1 roadmap requirement, assurance
property, or path obligation.

