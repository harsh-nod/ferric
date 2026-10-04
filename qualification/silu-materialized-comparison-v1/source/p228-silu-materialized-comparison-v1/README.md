# Retained SiLU Candidate Comparison

Source-only draft: 12 authored tests, not executed by the author. Root supplies
the actual native completion and test receipt only after successful execution.
This package neither launches a GPU process nor changes an acceptance tolerance.

## Arithmetic and Scope

`silu.py` reuses the unchanged corrected-residual adapter, exact integer FP32/BF16
oracle, genuine-framework comparison, and SiLU materialization diagnostic. It
requires the same genuine token 9112 at position zero, authenticates both repeated
33-stage framework payloads, and compares the new 28-array capture with the actual
corrected-residual baseline (`4f250305...`). The selected MLP image is the sole
kernel change; the projection-residual image and worker remain unchanged.

The output retains:

- All 22 pre-SwiGLU arrays, byte-equal across the two native captures. KV comparison
  uses each authenticated bootstrap's physical page, after complete cache checks.
- All 24 BF16 framework comparison rows for both candidate and baseline. Earliest
  observable divergence is descriptive, not a causal explanation.
- Four exact conditional residual comparisons, totaling 16,384 words. O uses the
  genuine embedding; Down uses each candidate's own captured first residual.
- The exact 12,288-word framework product control and both native gate/up match
  partitions. A prediction is made only when native and framework gate bits
  match, using captured framework BF16 SiLU and native BF16 up. Native OCML exp
  need not equal framework exp; remaining mismatches are retained.
- Candidate-versus-baseline activation and final-hidden BF16 differences, plus
  separately labeled FP32 Down partial differences. A TP partial is never compared
  with the full framework BF16 projection as though they were the same quantity.

There is no acceptance bound, full-layer/model acceptance, proof that rounding is
the only cause, exp-error measurement, calibrated performance, or production claim.
Exact conditional residual equality does not validate the generating GEMMs.

## Root Invocation

Let `E=/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`.
Freeze a four-file manifest with schema
`ferric-p228-silu-materialized-comparison-package-v1`, `pure_tests: 12`, and exact
`path/bytes/sha256` rows for `silu.py`, `test_silu.py`, `run.py`, and this README.
The manifest's SHA is a caller argument, not a guessed future receipt.

Run under ASROCK UID 9661, affinity 8/9, nice 10, ordinary Python with `-B`, empty
HIP/ROCR/CUDA visibility, and no `PYTHONPATH`, `PYTHONHOME`, or `PYTHONOPTIMIZE`:

```sh
python3 -B "$E/p228-silu-materialized-comparison-v1/run.py" \
  pure ACTUAL_PACKAGE_SHA silu-materialized-comparison-pure-v228-v1

python3 -B "$E/p228-silu-materialized-comparison-v1/run.py" \
  compare ACTUAL_PACKAGE_SHA silu-materialized-comparison-v228-v1 \
  "$E/ACTUAL_TRANSPORT_ADDITIONS.json" ACTUAL_ADDITIONS_SHA \
  "$E/prefix-silu-materialized-capture-gpu-v228-v1/complete.json" ACTUAL_GPU_SHA \
  "$E/silu-materialized-comparison-pure-v228-v1/complete.json" ACTUAL_PURE_SHA
```

Root owns the bounded invocation and terminal/reap receipt. The runner tightens
AS to 2 GiB, CPU time to 180 seconds for pure or 300 seconds for comparison,
file output to 16 MiB and core size to zero. It launches no subprocess. All output
directories are fresh. Pure mode emits exact named results and source snapshots;
actual mode requires that receipt and rechecks the same complete source closure.

## Existing Inputs and Transport

The retained ASROCK evidence already supplies the original reader/controller,
actual corrected comparison `projection-residual-comparison-v228-v1/complete.json`
(`affe711d...`), its eight math/test source pins and 345 consumed original/retained
records, the exact SiLU diagnostic `19ac2cf3...`, and the genuine framework blobs.
The 64 nonidentity mappings from that prior receipt remain unchanged. Additional
mapping entries must be explicit original-path to byte-identical retained FilePin
objects; `{}` is valid when all new bodies retain their original E paths.

Transport the new GPU case, its request/plan/reviews, actual supervisor pure36
receipt and raw sources/log, frozen eleven-file GPU package plus manifest, SiLU
CPU38 and lowering owner/inner evidence, actual image and artifacts, source bodies,
and any other original file selected by the frozen intake's `mlp_evidence` reader.
Do not copy Cargo targets or substitute build-host live platform state.

The runner freshly validates the new seven owned leaves, six historical process
audits, parent selector/stdout/child lineage, actual Close/capture, new image
provenance, and frozen CV's 22 upstream invariants. It rehashes the prior corrected
comparison's full consumed closure, explicitly reusing its already-recorded
ownership result. It also replays the genuine framework's 21 recorded results.
No current GPU/boot audit runs on ASROCK. The output flags distinguish this reuse
from fresh data replay and do not claim every transitive admission input was rerun.

Root must retain `complete.json` and the separately bounded terminal receipt.
Publishing should include both old/new 24-row tables, the conditional residual
rows, SiLU partitions and controls, six changed-stage rows, exact source/test
pins, all mismatches, and the explicit nonclaims. No result is fabricated here.
