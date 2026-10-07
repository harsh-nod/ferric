# MFMA projection diagnostics

This opt-in tool diagnoses the frozen TP1 exact-reference failure without
changing any production kernel, arithmetic selector, reference token, or
acceptance threshold. Kernel/inference code remains Ferric-owned. The current
source dependency cutoff is public fe2o3 `4fbc0a34`; a supplied frozen image
retains its original compiler and SDK provenance.

## What it checks

`mfma_differential.py` runs both the frozen scalar and MFMA roots in the same
full-v3 image with identical activation/weight bits. It reuses SHA-pinned
existing worker/ABI/guard helpers. Scalar weights are row-major `[n,k]`; MFMA
weights are separately transposed to `[k,n]`. Every transposed element is
checked before dispatch. Input/output guards, read-only operands, inactive
output tails, worker ownership, exact image identity and clean close remain
required. Raw active outputs and per-case receipts are retained.

The `tp1` suite has 15 cases / 30 dispatches. It covers Q, K/V, gate/up,
attention-output partial and down partial shapes: columns 1024/4096/12288,
reduction widths 4096/12288. Each shape has:

- Sixteen one-hot rows selecting tile edges and reduction tails. Every output
  must equal its exact source weight, for both scalar and MFMA.
- Three dense mixed-sign/mantissa rows, including finite normal magnitudes
  across 16 exponent fields.
- One cancellation-heavy row, explicitly distinguishing accumulation order
  from exact-product FP64 summation.

The `shapes` suite extends the same cases to TP2/TP8: 45 cases / 90 dispatches.
All six actual partial reduction widths are covered. K/V and gate/up share
math but retain their distinct production shape categories. The unsharded
151936-column output head is deliberately excluded from this bounded suite;
there is no claim that it or full-model numerical propagation is covered.
Each fixture holds at most 96 MiB of weight payload per orientation; allow
at least 2 GiB of host headroom for copies, worker transfer, and references.
This is a reservation guideline, not measured process RSS.

All outputs are compared between implementations. Dense/cancellation CPU
references cover eleven columns across tile boundaries and matrix ends, for
every active row. Scalar output must match ascending FP32 product/add order.
MFMA is compared to that order, FP64 product sums, and a separately labeled
chunk16 once-rounded hypothesis. The hypothesis is not an assertion about
hardware accumulation and is not an acceptance condition. The
`mfma_matches_fp64_via_fp32` bit comparison explicitly rounds the FP64 sum to
FP32 before BF16 narrowing; it does not claim direct FP64-to-BF16 rounding.
Absolute errors are still measured against the un-narrowed FP64 product sum.

The coarse forward-error bound is a necessary diagnostic check only. Passing
it is not proof of layout correctness, scalar parity, token parity, model
quality, or numerical safety for arbitrary inputs. Ill-conditioned sums can
have large bounds. A more accurate dot product can still change BF16 results
and model tokens. Fixed reference tokens are never relaxed.

## Invocation

All host checks run on the dedicated private `mi300x` stage. This command
cannot create a GPU worker:

```sh
python3 -m unittest discover -s device/qwen3-tp-perf-kernels-v3/tools \
  -p 'test_mfma_differential.py' -v
python3 device/qwen3-tp-perf-kernels-v3/tools/mfma_differential.py --self-test
```

Only the integration lead runs native probes on an idle, explicitly selected
`mi350` device, with an absolute new output directory and held worker/image
hashes. The image must contain all four named scalar/MFMA projection roots;
the frozen full-v3 image `8c81d3fe...` satisfies that requirement. A closed
wave-only image does not. The legacy single-device worker ABI is unchanged.

```sh
python3 device/qwen3-tp-perf-kernels-v3/tools/mfma_differential.py \
  --run --suite tp1 --helper proofs/tensor-parallel-kernels-v1/probe.py \
  --worker /absolute/frozen-worker --worker-sha256 "$WORKER_SHA256" \
  --artifact /absolute/full-v3.hsaco --artifact-sha256 "$IMAGE_SHA256" \
  --device-unique-id "$DEVICE_UNIQUE_ID" --output /absolute/new-diagnostic-run
```

External pre/post idle snapshots and invocation/source/image receipts must
be retained by that lead. Per-dispatch elapsed times are diagnostic records,
not TTFT, TPOT, serving throughput, or an accepted performance improvement.

## Interpretation

The archived TP1 MFMA-only and MFMA/pruning/residual cases produce
`[9856,374]` (Germany) instead of the frozen `[17689,374]` (Spain) for
`seed-prefix`. The baseline and baseline/pruning/residual controls pass.
The mismatch is therefore present without pruning or device residual, but
the underlying cause is not yet established. Earlier synthetic MFMA fixtures
and TP8 model pairs passing do not establish TP1 accuracy.

If full layout sentinels fail, investigate operand/store mapping before any
model run. If they pass but dense order changes remain, next capture matched
real activation/weight operands and per-layer outputs, plus final-logit
margin under identical inputs. Do not choose a production arithmetic change
or fallback merely to make a single reference pass.
