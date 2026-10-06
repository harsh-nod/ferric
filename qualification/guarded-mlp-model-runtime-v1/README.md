# Guarded Model Runtime Bring-Up

The [parent CPU gate](../guarded-mlp-parent-cpu-v1/README.md) and
[worker CPU gate](../guarded-mlp-worker-cpu-v1/README.md) pass independently.
The guarded model now completes both four-forward teacher-forced and genuine
autoregressive runs on MI350. The first native run's outer controller remains
failed; its log-parser error and the separate corrective checks are recorded
below. Model numerical acceptance remains open. The
[independent numerical diagnostic](numerical-v1/README.md) matches all eight
selected tokens but finds differences in every complete tensor slice;
logit relative-L2 errors range from 0.30% to 1.17%.

## Autoregressive Execution

The [fresh V3 AR4 run](attempt-v3-ar4/ar4/complete.json) passes its complete
outer controller in one native attempt, with no retry. Starting from token
`9112`, it produces `67, 25, 576, 2701`, feeding each output into the next
forward. Both ranks traverse all 36 layers at each position, exercise local
bank generations `1, 1, 2, 2`, and finish with a healthy Close.

All 1,213,952 captured BF16 words are finite and all four argmax selections
are independently recomputed from the full retained logits. Eleven supervised
leaves exit naturally; both parent and worker groups are reaped without forced
cleanup. Three before and three after snapshots show all eight GPUs idle.
The [retained capsule](attempt-v3-ar4/manifest.json) includes the exact nine input files
and 77 raw case files. Terminal SHA-256:
`edf05cf2dd19a9934dab9762b328ea3e6160cf01c15606a3df3299ce967e4991`.

The parent, worker, model, prompt and kernel images are unchanged from TF4.
Only the mode, fresh session/output path and corrected outer parser differ.
The whole-controller duration of 411.563075 seconds is not a GPU timing or
throughput result. The [separate framework comparison](numerical-v1/README.md)
reports all tensor differences without introducing an acceptance tolerance.

## Teacher-Forced Execution

The [original V2 receipt](attempt-v2-tf4/tf4/failed.json) records one native
attempt with all four forwards completed. The retained captures contain
144 layer observations, 288 rank guard records and 1,213,952 finite BF16
values. Inputs `9112, 2190, 3772, 220` produce checked lowest-index argmax
tokens `67, 198, 25, 16`; both banks reach local generations `1, 1, 2, 2`.
All eleven supervised phases exit naturally, owned processes are reaped,
and all six surrounding process snapshots show idle GPUs.

The outer controller rejects the worker announcement because its old parser
recognizes `finite engineering`, but this parent emits `finite guarded`.
The independently recorded parent/worker ancestry contains the matching
worker PID. The [complete original inputs and capture](attempt-v2-tf4/README.md)
preserve the failed result, not an edited passing receipt.

The [six corrective CPU tests](announcement-cpu-v1/evidence/complete.json)
pass on MI350 with unchanged sources and clean process postchecks.
They require the exact guarded marker, matching PID/PGID, and two authentic
owned-process records, rejecting missing, duplicate, malformed or conflicting
announcements and ancestry. This is a parser test, not another GPU run.
Receipt SHA-256:
`170ba440cdf33cd88fc55c2ddff709624fd819d3d461d54daad239a719efdf5a`.

The [separate V3 data-only revalidation](revalidation-v3/complete.json) now
passes on MI350. It rehashes 127 inputs, rechecks all 76 original raw bodies,
all eleven natural process leaves and six recorded idle snapshots, and
validates the exact two-process ancestry. Three JSON round-trip regressions
also pass, preserving 64-bit integers and rejecting changed values or types.
This result closes the previously skipped observation checks without rerunning
native code or changing the failed original receipt. Its SHA-256 is
`d0551f310a57f63dfe98af8c887b5996752e3b62128a3087c4a03849082e6a3f`.

The original whole-controller duration of 412.754826 seconds includes setup
and audits and is not a decode-throughput measurement.

Two data-only revalidation failures are retained separately. The
[first reader](revalidation-v1/failed.json) incorrectly required a single
hard link for every input, rejecting Cargo's ordinary hard-linked executable
products. The [second reader](revalidation-v2/failed.json) completed payload
validation but compared Python frontier tuples with JSON arrays directly.
The serialized observations are identical. Neither failure launched native
code, changed the original GPU evidence, or established numerical acceptance.
The passing V3 reader retains both prior failures and fixes only those reader
assumptions; GPU images, model arithmetic and observation rules are unchanged.

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
