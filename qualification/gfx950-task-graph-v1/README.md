# Finite Task-Graph Qualification Harness

This directory provides an independent exact-integer DAG reference and an
engineering-only dispatch harness for the [device source](../../device/gfx950-task-graph-v1/README.md).
The source's current checked compiler frontier is pipeline convergence; no
scheduler HSACO or GPU result is claimed yet.

```sh
python3 -B -m unittest -v test_reference
bash build.sh WORK_ROOT FRESH_BUILD_DIRECTORY
bash verify.sh PROBE WORKER HSACO SOURCE PRIVATE_SELECTOR NEW_EVIDENCE_DIRECTORY ramp
```

Only run `verify.sh` after successful checked compilation, offline metadata
inspection, and shared-host device allocation checks. Do not bypass the
compiler frontier by substituting hand-written machine code or IR.

The intended device test reuses fifteen guarded allocations for four valid
epochs and a fifth stale-epoch rejection. It checks exact seven-task payloads,
ready/done/claim masks, owner fields, immutable inputs, guards, completion,
and worker cleanup. The report counts actual workgroup owners and crossing
dependency edges; launching two workgroups does not by itself demonstrate
cross-workgroup communication. Worker host timing is not GPU event time.
The CPU model and synthetic evidence mutations establish only their tested
properties, not GPU execution, production authority, or model performance.
