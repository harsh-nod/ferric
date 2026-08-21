# M1 Positive Theorem Foundations

`REQUIRED_FOUNDATIONS` is the finite
`FERRIC-M1-POSITIVE-THEOREMS-V1` roster paired with the existing M1 hostile
foundation mutations. Each row binds a still-Open assurance property and path
to an exact package, source, crate-root Verus module, function, and directly
verified compiler path in `VERIFIED_MODULES`. The two speculative composition
rows deliberately select the same atomic body under distinct Open obligations:
publication-to-KV accepted-count agreement and immutable-preflight failure
framing.

`run-same-source.sh` accepts a clean committed Ferric worktree and the exact
pinned Verus release. It records the source commit, tree, complete M1 source
closure, Verus binary and closure, coverage manifest, registry, and runner. It
requires one ordinary Cargo check to pass and invokes each selected function
with `--no-cheating --output-json`. Every result binds ordered theorem metadata,
the shared compile transcript, the complete Verus transcript, an independently
derived structured summary, exact sizes, hashes, and exit statuses. The pinned
Verus output schema does not emit a `success` field. The checker rejects such a
field as schema drift and derives `RESULT=success` only from exit status zero,
exactly one selected verified query, zero `errors`, both encountered-error flags
false, selected function details without unresolved proof notes, and exact tool
identity. Other dependency detail keys are non-authoritative context, not extra
proof claims. The summary also binds the complete transcript SHA-256; it does
not attribute the derived result label to upstream Verus.

For an evidence binding, the selected roster must be exactly all rows assigned
to that assurance property and path. A one-row smoke run is admissible only for
a property/path pair whose registry product contains exactly one row.

## Non-Claims

The registry and runner are `ExistingFoundation` inputs, not an M1 evidence
index, qualification receipt, or path implementation. Even an authenticated
positive result establishes only a selected source-level function under the
pinned compiler and explicit TCB. It does not close an assurance property, path
obligation, or ROADMAP item, and adds no artifact, machine, hardware, numerical,
load, launch, side-channel, or performance authority.
