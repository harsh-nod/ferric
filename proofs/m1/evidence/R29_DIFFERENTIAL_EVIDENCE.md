# M1 R29 Differential Intake

`validate-r29-differential-evidence.py` authenticates one complete target-only
M1 r29 differential run without granting qualification or closure authority.
The format is an additive Ferric-owned intake boundary. It is not registered as
a production evidence-index kind and does not change `M1_REQUIREMENTS.json`.

## Fixed intake layout

The input is an owner-private nonsymlink directory with exactly this layout:

```text
acceptance-policy.json
acceptance.json
captures/
  KIND.capture.bundle/{logits.bf16le,output.json,runner.json,tokens.u32le}
comparison.bundle/
  records.json
  raw/CASE-ID.differential.raw.json
intake.json
pairs.json
plan.json
policy-review.json
references/
  KIND.reference.bundle/{logits.bf16le,output.json,runner.json,tokens.u32le}
```

`KIND` is each of the exact seven source-controlled differential buckets in
canonical order. Every JSON file must be canonical pretty-printed ASCII with one
trailing newline. Every file must be a stable single-link regular file and every
path component must be a directory rather than a symlink. Root components are
opened one at a time with `O_NOFOLLOW`; all nested directories and files remain
descriptor-held and are rebound by name, roster, metadata, and exact bytes after
the complete intake has been checked.

`intake.json` uses format
`FERRIC-M1-R29-DIFFERENTIAL-EVIDENCE-INTAKE-V1`. It binds ordered fe2o3 and
Ferric commits, trees, bases, and source closures; the exact compiler,
compiler-configuration, runtime ABI, runtime contract, target contract,
qualification protocol, benchmark, and reference identities; and the ordered
compiler, hardware, and runtime TCB identities. The four executable/protocol
identities and both source closures must match the exact differential plan.

`policy-review.json` uses format
`FERRIC-M1-R29-DIFFERENTIAL-POLICY-REVIEW-V1`. It binds the exact acceptance
policy SHA-256 and one externally declared reviewer and review identity. Its
fixed independence status is `not-validated-by-ferric`: the declaration records
the handoff but is not proof of the reviewer's identity, authority, or
independence.

## Checked relationships

The intake validator independently rehashes every file and requires:

- the exact seven-case plan, target, source path, identity roster, and policy;
- one Ferric capture and one reference output for every case, with exact shapes,
  encodings, sizes, payload hashes, and byte-identical runner transcripts;
- production capture transcript authority, target, plan, environment,
  executable, protocol, input, workload, device, program, exact generated-plan
  declaration, per-bucket decode context plan, and payload bindings, including
  row identities recomputed from the held Ferric logits bytes;
- an exact `FERRIC-M1-DIFFERENTIAL-PAIRS-V2` manifest whose companions name and
  hash those held bundles;
- exact raw records and observations whose metrics and identities match the
  acceptance cases; and
- one policy-conforming acceptance result whose plan, pairs, policy, thresholds,
  comparisons, output manifests, payloads, and runner identities all agree.

The validator does not duplicate the Rust BF16 streaming comparison. It binds
the complete inputs and result of that comparison and preserves this limitation
in the report nonclaim.

## Producer and replay

Publish an absent output bundle under an existing owner-private parent:

```text
python3 -I proofs/m1-qualification/produce-r29-differential-evidence.py \
  INTAKE-ROOT OUTPUT-BUNDLE
```

The producer writes `roster.json` and `report.json` into a synchronized sibling
staging directory and uses `renameat2(RENAME_NOREPLACE)` through the held parent
descriptor. It binds the staging name to the held directory before the rename,
then reopens the final name and requires the same inode, exact two-file roster,
single-link file identities, and byte-exact output before returning. It refuses
a preexisting output and preserves substituted names for inspection.

Recompute the complete intake and require byte-identical output with:

```text
python3 -I proofs/m1/evidence/validate-r29-differential-evidence.py \
  validate INTAKE-ROOT OUTPUT-BUNDLE
```

The source-paired producer uses the same parser as the validator, so this replay
is not independent validation. It detects input/output substitution and schema
drift; an outside organization must still perform the independent validation
required by the M1 evidence profile.

## Authority boundary

The report authority is `r29-differential-intake-authentication-only`, status is
`partial-non-evidence`, hardware claim is `external-identities-only`, and the
three fields `independent_validation`, `qualification_evidence`, and
`r29_closed` are fixed to false. Acceptance does not attest observation truth,
hardware behavior, source provenance, reviewer independence, compiler/runtime
correctness, numerical correctness, qualification, r29 closure, or M1 closure.

Run the canonical and hostile policy suite with:

```text
python3 -I -B proofs/m1/evidence/test-r29-differential-evidence-policy.py
```
