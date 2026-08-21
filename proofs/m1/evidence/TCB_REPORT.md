# M1 Trusted-Computing-Base Report Evidence

`validate-tcb-report.py` implements
`ferric.m1-validator.tcb-report.v1`. The production evidence-index checker owns
that path, protocol, and validator source SHA-256. An evidence index cannot
select another executable.

## Canonical layout

For a TCB artifact named `<artifact-id>`, the report has this fixed location
relative to the evidence-index directory:

```text
artifacts/<artifact-id>.tcb-report.json
```

The report is canonical, pretty-printed ASCII JSON with one trailing newline
and no duplicate, missing, or extra fields. The validator accepts one report
for each of the ordered `tcb.compiler`, `tcb.hardware`, and `tcb.runtime`
subjects. Every report repeats the same complete global declaration and binds:

- the exact canonical M1 requirements bytes;
- all 33 roadmap obligations and 17 assurance properties, their still-`Open`
  states, statement hashes, ordered paths, and ordered evidence profiles;
- all 39 path obligations, including availability, repository, source owner,
  and still-`Open` state;
- all seven ordered evidence profiles and their exact evidence-kind rosters;
- the ordered fe2o3 and Ferric commit, tree, base, and source-closure identities;
- the ordered compiler, hardware, and runtime TCB structure;
- fixed Rust, Verus, LLVM/AMDGPU, linker, HSA, driver, firmware, Ferric, fe2o3,
  Python, POSIX host, and single-device `gfx942:xnack-` trust roles; and
- the complete checker-owned validator path, protocol, availability, and
  reviewed source-identity roster.

The validator reads the checker registry as literal Python data with
`ast.literal_eval`; it does not import or execute the checker. A non-null
checker source pin must match the stable bytes at the registered path. A null
pin remains `RequiredFuture`, and its executable path must be absent. Thus a
new file cannot silently widen the validator TCB, while independently reviewed
validator additions compose by updating their checker-owned pin.
The qualification-receipt validator is source-pinned and therefore appears as
an `ExistingFoundation` entry in this roster; the receipt validator separately
requires every roster entry to be source-pinned before qualification can pass.

The three outer TCB `identity_sha256` values are the hashes of the three report
artifacts. They cannot be embedded in the reports without a recursive hash
cycle. Instead, each report binds the complete ordered TCB IDs, kinds, and
artifact IDs; the validator binds its subject report hash to the corresponding
outer TCB identity; and the checker-bound canonical invocation context binds
all three outer identities. This is the exact version-1 identity boundary.

All inputs are bounded. The validator rejects noncanonical or duplicate-key
JSON, absolute paths, traversal, symlinks in any report-path component,
unstable file identity or metadata across a read, artifact size or digest
drift, omission, duplication, reordering, kind or identity substitution,
version or status drift, source or path replay, target drift, and authority or
nonclaim promotion.

## Authority boundary

Acceptance authenticates a declaration of what M1 trusts. The
`qualification-bound-external` entries deliberately record contracted roles,
not measured external binaries or devices. A report does not establish that a
component is installed, that a version string came from a component, or that a
compiler, runtime, driver, firmware, or GPU behaves correctly. It grants no
theorem, machine-refinement, load, launch, hardware, performance, or
qualification authority.

This validator creates neither an evidence index nor a qualification receipt.
It does not change `RequiredFuture` path availability and closes no roadmap
requirement, assurance property, or path obligation. The separate
`docs/M1_TCB.md` path and all M1 implementation states remain `Open`.
