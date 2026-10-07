# M1 Unsupported-Rationale Evidence

`proofs/m1/evidence/validate-unsupported-rationale.py` implements protocol
`ferric.m1-validator.unsupported-rationale.v1`. The evidence-index checker owns
that validator path, protocol, and reviewed source identity.

## Exact nonclaims

The validator accepts only the five planner bindings for the three assurance
properties whose required M1 closure status is `Unsupported`:

- `distribution_preserved` on `m1-tcb` and `speculation-proof`;
- `machine_refined` on `identity-closure` and `m1-tcb`;
- `multi_device_refined` on `m1-tcb`.

Every binding uses profile `nonclaim`, source `source.ferric`, and the complete
compiler, hardware, and runtime TCB. The rationale text is the exact assurance
boundary in `proofs/M1_REQUIREMENTS.json`. The validator fixes each reason code
and ordered excluded-claim roster; the report cannot supply a new unsupported
property, path, rationale, or exclusion.

The report path is fixed by the planner artifact id:

```text
artifacts/<artifact-id>.unsupported-rationale.json
```

It is canonical pretty-printed ASCII JSON with one trailing newline. There is
no companion payload.

## Planner-Bound Producer

After creating the external plan and all three TCB reports, invoke the
Ferric-owned producer for one exact binding:

```text
python3 -I proofs/m1-qualification/produce-unsupported-rationale.py \
  FERRIC_REPO FE2O3_REPO PLAN_DIR binding.NNNNN
```

The producer reauthenticates both exact clean repositories, source closures,
requirements, validator pins, the complete source-pinned plan and work queue,
and all three canonical TCB reports. It holds the private plan and artifact
directories and authenticated input files through completion. Publication uses
exclusive no-follow creation at mode `0600`, file and directory synchronization,
and final exact inode and byte revalidation. A late input or custody failure
removes only the exact report it created, so no failed run leaves a false
completion marker. Existing outputs are never replaced.

Run its focused policy with:

```text
python3 -I proofs/m1-qualification/test-unsupported-rationale-producer-policy.py \
  FERRIC_REPO FE2O3_OBJECT_REPO
```

## Authority Boundary

The accepted authority is `nonclaim-only`, and the required closure status is
only the manifest's exact `Unsupported` boundary. The report grants no theorem,
validation, artifact, load, launch, hardware, performance, or qualification
authority. It does not close an M1 obligation and creates neither an evidence
index nor a qualification receipt.
