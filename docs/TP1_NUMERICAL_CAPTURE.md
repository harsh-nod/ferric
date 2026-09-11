# TP1 Numerical Capture

This opt-in tool diagnoses the fixed-reference MFMA failure. It does not change
kernel arithmetic, acceptable output tokens, weight layouts, or default execution.
Native synthetic fixtures have not established a production defect to fix.

Add all four flags to a separately frozen TP1 batch controller invocation:

```text
--numerical-capture /absolute/private/new-capture
--numerical-batch 2 --numerical-layer 0 --numerical-projection q
```

The role is `q`, `k`, `v`, `o`, `gate`, `up`, or `down`. Batch ordinals are 1..64,
layers 0..35, and selection must fit the requested maximum batches. Only the
16-row TP1 profile with baseline or MFMA projection and baseline attention is
supported. Sequencing, wide profiles, host timing and replica benchmark control
are rejected. Configure capture last: subsequent policy mutations are rejected.

The fresh 0700 directory contains exclusive 0600 files. Total retained bytes,
including the manifest, cannot exceed 224 MiB. The selected projection retains
active BF16 input, original NxK weights, actual KxN MFMA weights when used, and
active BF16/FP32 output. The final head retains normalized input, all active BF16
logits, top16 and watched token IDs 9856/17689, and original LM weight rows for
those selected tokens. Watched IDs have no inferred tokenizer labels.

Scheduler slot/generation/token/position/kind and pool batch identities are
bound inside the existing submission transaction. Execution-row permutation
and publishable rows are checked against that plan, including head pruning.
Finite CPU lowest-ID argmax must equal the GPU choice before the head capture
can complete. Read, identity, cap, argmax and teardown failures never qualify a
capture. Incomplete files are retained for diagnosis, not reported as success.

Require zero process exit, successful owned-worker close, a matching Closed
capture receipt, and SHA-256 verification of the manifest and all files.
`manifest.complete` alone is not run acceptance. A manifest publication/fsync
failure latches failure and has no successful receipt. Every diagnostic timing,
including setup and dispatch time, is excluded from performance measurements.
Use a separate exact-token diagnostic check; do not relax the performance checker.

Offline payload analysis on the build host (no GPU):

```bash
python3 tools/replay_tp_numerical_capture.py \
  --capture /absolute/archived/capture --manifest-sha256 EXPECTED_SHA256 \
  --output /absolute/new-analysis.json
```

The helper hashes every file, checks the full BF16 transpose, and evaluates eight
fixed columns for every active projection row against serial FP32 and FP64
product-sum references. These are numerical diagnostics, not an MFMA instruction
order model, exhaustive projection correctness, fixed-token parity or performance
acceptance. It retains final-logit summaries without inventing decoded labels.
Budget at least 1 GiB host RAM for analysis, excluding the resident model process.

For a source-matched baseline/MFMA capture pair:

```bash
python3 tools/compare_tp_numerical_captures.py \
  --baseline /absolute/baseline/capture --baseline-manifest-sha256 BASELINE_SHA256 \
  --mfma /absolute/mfma/capture --mfma-manifest-sha256 MFMA_SHA256 \
  --output /absolute/new-pair-analysis.json
```

This pins the original replay helper and requires matching executable/image,
model/workload, physical device, scheduler rows and nonprojection policies, with
distinct sessions. It counts all active projection/head input and output bit
differences, checks original weights and shared selected LM weight rows, and
recomputes top16/watch/argmax summaries from every raw head logit. Per-row top
and watched columns are replayed against actual normalized head inputs using
serial FP32 and FP64 product sums. The watch margin is token9856 minus token17689;
neither token receives an inferred decoded label. A serial-reference mismatch
under MFMA is a diagnostic observation, not token parity acceptance or evidence
of a specific instruction-order defect. Full-run exact tokens and custody remain
separate prerequisites; neither analysis tool admits performance measurements.
