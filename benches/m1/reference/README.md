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
