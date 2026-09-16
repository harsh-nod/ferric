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
