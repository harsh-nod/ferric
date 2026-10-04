# Root Inputs for the Down2 Clock Successor

The plan remains closed and uses the same FilePin representation (`path`, `bytes`,
`sha256`). Its schema is `ferric-p228-down2-clock-inputs-v1`; the output label must
be a fresh `prefix-down2-clock-tf4-shared-full-currentness-gpu-v228-vN`.

Exact plan fields:

```text
schema output_label policy baseline parent_cpu worker_cpu parent worker
image_deployment standalone_prepared standalone_cases numericals request
decode_review parent_runtime_review worker_runtime_review supervisor_tests
supervisor_test_sources clock_baseline down2_lowering down2_image down2_review
```

`policy` is only `shared-full-currentness`. The baseline pin is unchanged CPU553
TF4, and the image deployment, prepared prefix inputs, six ordered actual prefix
GPU cases and six conditional numerical receipts must equal that baseline's
prerequisites. `clock_baseline` separately binds the actual passed clock-enabled
TF4 receipt. The new request is the same Clock V2 wrapper and actual clock runtime,
allowing only `decode.tiles_image`, fresh session and new evidence directory to
differ. Worker, model and bootstrap `decode.images.mlp` cannot change.

`worker_cpu` and `parent_cpu` must authenticate the actual completed 669-test and
275-test results pinned in `intake.py`. The latter was bound only after its actual
receipt, source snapshots, build stream and selected ELF were retained locally
and checked. `parent` and `worker` are the transported exact selected Cargo
executables, not source paths or substitute binaries. Both full source snapshots
must match each other for the shared worker subtree, including the V1 raw and V2
clock schemas; only README content is excluded, not its roster membership.

`decode_review` has schema `ferric-p228-down2-clock-engineering-review-v1` and
retains the original explicit root-reviewed bindings and six substantive topics:
source lineage, formal, ISA, coherence, lifecycle and selected device. Its added
`clock_domain_validated` flag must be false, as must all existing numerical,
runtime-premise, production, calibration, alignment, overlap and performance
authority flags. It also binds all four added plan pins. The separate
`down2_review` has schema `ferric-p228-down2-image-engineering-review-v1` and binds
the exact provenance returned by `down2.lowering`, new image/request and unchanged
runtime. All eight runtime requirements remain undischarged. Root must write
substantive new image reviews; this package never creates or marks them accepted.

The actual clock parent/worker runtime reviews are reused byte-for-byte. Their
current platform and library identities are still checked on every admission.
No source rebuild, new runtime binary or altered runtime review is admitted.

Before admission, root freezes the exact 17-file
`ferric-p228-down2-clock-package-v1` manifest and runs the 75-test discovered suite.
The expected pure receipt schema is `ferric-p228-down2-clock-pure-v1`, using a fresh
`down2-clock-gpu-pure-v228-vN` directory and the bounded root runner
`E/run_down2_clock_gpu_pure_p228_v1.py`. Actual source before/after, transcript and
runner pins must all join. No expected future receipt SHA is supplied.

When every prerequisite is actually satisfied, root invokes `run.py PLAN_PATH
PLAN_SHA` through the existing bounded root-owned invocation. It preserves the
existing UID/affinity, native ownership, six audits and fresh-directory checks.
The result schema is `ferric-p228-down2-clock-gpu-v1`; the checked observation uses
`ferric-p228-down2-clock-observation-v1`. Passing reports require 16 samples,
1,172 raw rows, dispatch totals [592, 580], four full captures and all 152 compared
tensor rows equal. No clocks are calibrated and no new numerical authority is
granted. The root invocation's own terminal/reaped status remains root custody;
this package does not invent a top-level supervisor receipt.
