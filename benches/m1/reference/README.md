# M1 Reference Diagnostics

`run.py` and the engineering reference entrypoints retain their separate
protocols and dependency pins. Engineering observations do not close M1 gates.

## Retained M5 Rankings

`engineering_m5_rank_diagnostic.py` analyzes an existing five-position capture
and its already-produced independent reference. It does not load a model,
import PyTorch, launch a GPU, or rerun inference. Run it on mi300x:

```sh
python3 -I -B engineering_m5_rank_diagnostic.py \
  CAPTURE_DIRECTORY REFERENCE_DIRECTORY COMPARISON_SHA256 NEW_OUTPUT_DIRECTORY
```

The capture directory supplies `capture.json` and `target-logits.bf16`; the
reference directory supplies `comparison.json` and `reference-logits.bf16`.
The expected comparison hash binds the input record. The tool checks linked
capture/logit hashes and reproduces the original metrics before writing
`rank-diagnostic.json` into a new directory. Existing output is never replaced.

The report includes top-five logits, maximum tie counts, both winners' ranks
and signed errors, using lowest-token-ID tie-breaking. It cannot recover
pre-narrowing FP32 values or identify which earlier operation caused a logit
difference. It makes no numerical acceptance or performance claim.

CPU-only regressions, also on mi300x:

```sh
python3 -I -B test_engineering_m5_rank_diagnostic.py
```

## Checkpoint RMSNorm Ablation

The additive `engineering_rmsnorm_ablation.py` uses the pinned reference venv
on mi300x. It authenticates all nine canonical target files (16,392,982,768
bytes), then reads only embedding row 13 and layer-zero input RMSNorm weights.
It never loads the full model. From the repository layout:

```sh
"$REFERENCE_VENV/bin/python" -I -B benches/m1/reference/test_engineering_rmsnorm_ablation.py
"$REFERENCE_VENV/bin/python" -I -B benches/m1/reference/engineering_rmsnorm_ablation.py \
  MODEL_SOURCE NEW_OUTPUT_DIRECTORY
```

The three CPU cases are actual pinned HF RMSNorm, sequential FP32 reduction
with `rsqrt`, and that same reduction with `sqrt` followed by reciprocal. All
cases retain both BF16 rounding boundaries. The fresh output contains raw
inputs, intermediate tensors, outputs, exact bit differences and `ablation.json`.
The checkpoint embedding row is not a captured final hidden state; this test
cannot establish device behavior or explain an end-to-end token mismatch.

## Final-RMS Native And Reference Capture

The separate `engineering_m5_final_rms_reference.py` consumes the opt-in native
`--mfma13 --capture-m5-final-rms DIRECTORY` export. Its exact five-file input
contains the original capture transcript and all five logits rows, plus
`final-rms-capture.json` and the 4,096-element BF16 final-RMS input/output at
target segment 4, active row 3, sequence position 131. The native diagnostic
redirects these workspaces into its owned completed readback allocation.
Default native capture and the original two-file reference are unchanged.

Run only in the pinned reference environment on a separately admitted mi300x
GPU, with the same resource/device controls as the original full-model run:

```sh
"$REFERENCE_VENV/bin/python" -I -B benches/m1/reference/engineering_m5_final_rms_reference.py \
  CAPTURE_DIRECTORY MODEL_SOURCE NEW_OUTPUT_DIRECTORY
```

The reference authenticates the full checkpoint and executes the exact
133-token sequence twice. Hooks capture the final normalization's real
input/output; both intermediate rows and all logits must repeat byte-for-byte.
The new output retains both raw reference rows, all reference logits, and
`comparison.json` with intermediate error metrics and the five token choices.
CPU-only protocol and hook-lifecycle regressions can run independently:

```sh
python3 -I -B benches/m1/reference/test_engineering_m5_final_rms_reference.py
```

The native transcript records the full completed-copy digest, but only the
logits and two selected intermediate rows are exported. Their hashes and
allocation coordinates are checked; the full-copy digest is not independently
reproduced. Neither intermediate comparison nor matching token choices alone
establishes causality, accepts a tolerance, measures performance or closes a
protected M1 gate. Adding the capture path is not evidence that it has run.

## TP1 Final-Stage Comparison

`engineering_tp1_final_stage_reference.py` is a separate diagnostic for the
TP1 command's `--capture-final-stage DIR --capture-positions 4,5` output. It
does not change either frozen M5 reference protocol or its tolerances. Native
execution uses mi350 (`gfx950:xnack-`); independent reference execution still
requires the pinned environment and a separately admitted mi300x GPU
(`gfx942:xnack-`). It does not qualify either target.

```sh
"$REFERENCE_VENV/bin/python" -I -B benches/m1/reference/engineering_tp1_final_stage_reference.py \
  CAPTURE_DIRECTORY WITNESS_DIRECTORY NATIVE_PINS_JSON NATIVE_PINS_SHA256 \
  MODEL_SOURCE NEW_OUTPUT_DIRECTORY
```

The capture has exactly `intent.json`, `manifest.json`, `residual.bf16`,
`normalized.bf16`, and `logits.bf16`. The separate witness directory has exactly
`stdout.txt`, `process.exit`, `wrapper.exit`, `group-probe.exit`, and
`group-after.txt`. Copy the completed native wrapper's `exit.status` bytes to
`wrapper.exit` only after it terminates; the other files retain their original
names and exact bytes. All three statuses must be `0\n`; group absence must be
`leader and group absent\n`. Four stdout records must agree with the capture:
Setup, Measurement, Closed, CaptureReceipt, in that order. These are retained
external assertions, not independently reconstructed process-lifetime proof.

Freeze and retain the native pins and their SHA256 before running the native
campaign. The expected hash must come from that retained campaign, not be
inferred from an untrusted capture. The exact `FerricTpFinalStageNativePinsV1`
object contains `schema`, `authority: "none"`, `qualification: false`,
`benchmark_comparable: false`, and these actual run fields:

- `controller_sha256`, `worker_sha256`, `model_bundle_id`, `artifact_hsaco_id`,
  `artifact_manifest_id`, `artifact_handoff_id`.
- `device_unique_ids` (one nonzero exact u64), `prompt`, `prompt_tokens`,
  `new_tokens`, `capacity`, `repetitions: 1`, `warmup_runs: 0`, `positions`.

The comparator authenticates the canonical target checkpoint and all native
payload extents, hashes, row offsets, identities, consumed tokens and successful
close. It executes the consumed sequence twice with `use_cache=False`, hooks
the final norm once per pass, and projects the selected normalized rows through
the full LM head. Both raw reference passes are retained in six BF16 files,
even if nonfinite or different. `comparison.json` reports per-stage bit, ULP,
absolute and RMS errors, finite argmax choices, and repeat agreement. Numeric
metrics are null for nonfinite rows; no tolerance, cause, parity-pass,
performance or M1-closure claim is made. Structural success is not numerical
success. All model/input identities are rechecked before publication, and an
existing output directory is never replaced. The output parent cannot be a
capture, witness, pins, model root/target/draft, or reference-source directory.

CPU-only fixtures, run on mi300x:

```sh
python3 -I -B benches/m1/reference/test_engineering_tp1_final_stage_reference.py
```

## TP1 Same-Input Final-RMS Diagnostic

`engineering_tp1_rmsnorm_ablation.py` is a CPU-only localization diagnostic for
the authenticated TP1 final-stage capture. It accepts the same capture, witness,
native pins, pins SHA256 and canonical model source as the TP1 comparator. It
does not execute the model or use a GPU.

```sh
"$REFERENCE_VENV/bin/python" -I -B benches/m1/reference/engineering_tp1_rmsnorm_ablation.py \
  CAPTURE_DIRECTORY WITNESS_DIRECTORY NATIVE_PINS_JSON NATIVE_PINS_SHA256 \
  MODEL_SOURCE NEW_OUTPUT_DIRECTORY
```

For each selected native position, the tool decodes the exact held BF16
residual and loads the authenticated canonical `model.norm.weight`. It retains
the native normalized row alongside actual pinned HF RMSNorm and the existing
source-spelled serial-FP32 `sqrt`-then-reciprocal case, including their raw
intermediates, scalar bits and exact BF16 disagreements. The source-spelled case
includes the kernel's BF16 pre-weight rounding boundary.

```sh
"$REFERENCE_VENV/bin/python" -I -B benches/m1/reference/test_engineering_tp1_rmsnorm_ablation.py
```

CPU arithmetic does not establish gfx device or compiler behavior. Neither
agreement nor disagreement admits a tolerance, establishes a cause, supplies a
numerical pass, qualifies a target, measures performance or closes an M1 gate.
Replay inputs, the authenticated weight, and CPU intermediates must be finite;
otherwise the tool refuses publication and the original native capture remains
separate evidence. A nonfinite native normalized observation is retained as raw
evidence and its finite ULP/absolute/RMSE metrics are null because it does not
feed the replay arithmetic.
