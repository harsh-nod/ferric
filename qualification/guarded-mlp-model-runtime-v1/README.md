# Guarded Model Runtime Bring-Up

The [parent CPU gate](../guarded-mlp-parent-cpu-v1/README.md) and
[worker CPU gate](../guarded-mlp-worker-cpu-v1/README.md) pass independently.
Native model execution is not yet accepted.

## First Preflight

The [original controller and prepared inputs](preflight-v1/) are retained.
Input preparation authenticated the actual parent and worker executables,
model-bundle identity, prompt files and eight kernel images. It did not run
the model. Its receipt SHA-256 is
`896b22340ebb0794a60ade2b14635bf40f57932c56e5a18824887b93b33b3959`.

The first teacher-forced invocation stopped in `topology()` before any child
launch with `RuntimeError: exact selected gfx950 devices`. This was a harness
bug: it compared KFD's short `gpu_id` handles, 39903 and 22482, with the
runtime's 64-bit `unique_id` values. The actual device properties contain the
expected unique IDs and `gfx_target_version 90500`.

The initial topology check was outside the controller's retained exception
path. Consequently this invocation did **not** write a normal terminal
receipt; the retained directory contains its inputs, not a passing GPU
result. The source was stopped before all supervised leaves, including
the native parent. This paragraph records the observed startup failure and
is not a substitute for a controller-generated completion receipt.

A fresh namespace is required for the correction. The next harness compares
the `unique_id` property, keeps `gpu_id` as a distinct observed handle, and
records topology failures inside its exception path. Separate synthetic
regressions must exercise this distinction before another native invocation.

The [four topology regressions](topology-cpu-v1/evidence/complete.json) now
pass on MI350, with zero failures, errors or skips and clean source/process
postchecks. They cover distinct KFD handles and unique IDs, swapped devices,
wrong architecture and malformed or duplicate properties. Receipt SHA-256:
`0558c970079d48094e96917fa7d71141e2bf1ff1ca9716145cb05a734d2ca8e2`.
These are synthetic parser tests, not GPU model execution.

Four-forward teacher-forced and autoregressive checks are bring-up diagnostics.
They do not constitute the 2,048-token prompt / 256-token generation workload,
an independent model reference, kernel-overlap evidence or a throughput claim.
