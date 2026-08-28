# M1 Qualification Receipt Evidence

`proofs/m1/evidence/validate-qualification-receipt.py` implements protocol
`ferric.m1-validator.qualification-receipt.v1`. Its reviewed source SHA-256 is
`cf2f043001815f06220dbf03a8131a3931c01b4c8b96681ed07e626374b36612`.
The production evidence-index checker owns the path, protocol, and source pin;
an evidence index cannot select or substitute an executable.

## Canonical layout

All 33 roadmap closure rows must reference one `QualificationReceipt`
artifact. For artifact `<artifact-id>`, the accepted files are canonical,
pretty-printed ASCII JSON with one trailing newline:

```text
artifacts/<artifact-id>.qualification-receipt.json
qualification-transcripts/<artifact-id>.json
```

The checker supplies the receipt artifact identity, complete evidence index,
exact Ferric and fe2o3 repository paths, source roster, TCB roster, and
requirements identity in a canonical invocation context. The validator
accepts no path, protocol, status, roster, or validator override from the
receipt.

The receipt authenticates canonical digests of the complete evidence-binding,
obligation, path-resolution, source, TCB, requirements, and trusted-validator
rosters. It names every source closure and its file count, commit, tree, and
SHA-256. It also binds the complete artifact roster except for its own size and
SHA-256, which are supplied and checked by the checker invocation. This
single-record self-exclusion avoids an impossible recursive self-hash; the
receipt still names its exact artifact ID, kind, and canonical path.

Before invoking any trusted producer or test-only callback, the checker runs
the positive-theorem and negative-mutation foundation registry checkers from
the exact Ferric source closure. Every Assurance theorem or mutation binding
must use a Ferric path and select an exact checked row for the same property
and path through both its `.result` filename and `THEOREM=` or `MUTATION=`
record. This is a reachability preflight only; the source-pinned theorem and
mutation validators still authenticate the complete run and its row identities.

## Independent closure checks

The validator independently requires:

- exactly 33 roadmap rows at `Closed` and 17 assurance rows at their manifest
  statuses (`Proved`, `Validated`, or `Unsupported`), in manifest order;
- the exact statement, assurance-dependency, path, profile, evidence-kind,
  proof, mutation, independent-validator, unsupported-rationale, and TCB
  rosters implied by the canonical requirements manifest;
- exact evidence-kind binding-class filtering, with theorem, mutation, and
  unsupported rationale restricted to Assurance rows and TCB reports fulfilled
  only by the global TCB roster;
- every artifact to be a unique, stable, single-link regular file below the
  nonsymlink evidence root with its exact size and SHA-256;
- the exact clean `HEAD` commit and tree for Ferric and fe2o3, and source
  closure bytes equal to every non-generated path in each committed tree; and
- all checker-owned validators to have non-null source pins that match the
  regular files in the exact Ferric source closure.

Repository requirements and path records must still be `Open`. Closure
statuses occur only in the external evidence index and receipt. Unknown,
missing, duplicate, reordered, status-weakened, status-promoted, reused, or
unreferenced records fail closed.

## Qualification transcript

The immutable companion uses format
`FERRIC-M1-QUALIFICATION-TRANSCRIPT-V1` and protocol
`ferric.m1.qualification.v1`. It binds a non-placeholder run UUID, positive UTC
run interval, exact source and TCB identities, the evidence-index roster,
trusted-validator roster, and these seven passing gates:

```text
evidence-index
hardware
performance
proof
quality
source-closure
validators
```

Each gate has a distinct command identity and output identity and names the
complete checker-derived artifact and evidence-binding roster for its role.
The transcript rejects a failed, skipped, partial, reordered, duplicated, or
self-reported gate. Its fixed target is one `AMD Instinct MI300X` at
`gfx942:xnack-`; it records exact ROCm, amdgpu driver, firmware, device, host,
Cargo, Rust, Verus, Python, evidence-index checker, and receipt-validator
identities. Tool, environment, target, gate, source, TCB, validator, and index
roster digests compose into the qualification-run identity.

A receipt replayed against another artifact, index, run, source commit, tree,
closure, target, tool, environment, TCB, or validator set is rejected. A
byte-identical receipt used with the byte-identical qualified context denotes
the same qualification identity; preventing reuse of that same identity across
an external release ledger is outside this stateless validator.

## Authority boundary

Acceptance grants `m1-qualification-receipt-only` authority for the exact
checker-supplied closure. This source-only validator does not generate a
receipt, execute a quality, proof, GPU, or performance workload, create an
evidence index, or edit requirements. This repository contains no production
M1 receipt or evidence index, and every in-repository M1 implementation state
remains `Open`.

The hostile synthetic policy is:

```text
python3 -I proofs/m1/evidence/test-qualification-receipt-policy.py FERRIC_REPO
```
