# TP1 FP32 Final Head V7

This separate, opt-in engineering image adds three roots beside an unchanged
BF16 base image. It is not protected M1 admission or a serving qualification.
Weights and transformer activations remain BF16. Only the final head stores
FP32 accumulators, and its ascending finite FP32 argmax retains lowest-ID ties.
There is no token-specific production logic.

The two head roots accept `[rows, 151936, 4096, 1, 6]`, rows 1 through 16,
three slices and five u32 scalars (68 explicit bytes), WG64, 9,496 groups.
Scalar weights are row-major NxK; MFMA weights are separately resident KxN.
The MFMA SDK zero-fills inactive rows. The FP32 argmax has two slices and one
u32 scalar (36 explicit bytes), WG64, one group per row. Outputs preserve all
inactive tail rows. Nonfinite active arithmetic traps; none is silently clamped.

The controller explicitly selects `--head-precision fp32-v7` together with
`--fp32-head-artifact DIR`. `bf16-v7-control` loads and records the same separate
image but retains both original BF16 head dispatches and allocates no new
workspace. The candidate retains the original BF16 allocation and adds exactly
9,723,904 bytes of FP32 logits. Both profiles are restricted to TP1, the 16-row
image envelope, baseline or MFMA projections, baseline attention, no dispatch
sequences, numerical capture, wide profile, or replica control. Normal default
CLI output and arithmetic remain unchanged. Additional Setup fields require the
separately pinned v7 comparator; the legacy performance checker rejects them.

## Native Fixture Handoff

All native runs are scheduled by the integration lead. The harness never runs a
GPU unless `--run` and all external identities are supplied. Reserve at least
12 GiB host RAM headroom for full-weight reads and guard checking. This is a
conservative reservation, not measured RSS. There are nine dispatch fixtures:
scalar head, MFMA head, and FP32 argmax, each with 1, 3, and 16 active rows.
Head cases use full vocabulary/hidden dimensions and exactly representable
dyadic arithmetic. Every output element is checked, including tails; every
immutable input byte and both guard regions are reread. Ordinary fixture IDs
exercise BF16-collapsed scores with a distinct FP32 winner. Separate argmax
rows cover exact signed-zero ties and the last vocabulary ID. No nonfinite GPU
fault is deliberately submitted; those rejection checks stay in host tests.

```bash
python3 -I -B /ABS/device/qwen3-tp-fp32-head-kernels-v7/tools/probe.py \
  --run --helper /ABS/proofs/tensor-parallel-kernels-v1/probe.py \
  --worker /ABS/worker --worker-sha256 WORKER_SHA256 \
  --artifact /ABS/observation.hsaco --artifact-sha256 HSACO_SHA256 \
  --device-unique-id PHYSICAL_ID --output /ABS/FRESH_OUTPUT --operational
```

The exact helper SHA is embedded in the harness. Results retain source,
worker/image hashes, inspected metadata, wire records, PID custody and teardown.
Dispatch observations are diagnostic only, not model TTFT/TPOT or throughput.
Only a separately checked complete fixed-token model run can qualify the
candidate's output behavior; this image is not enabled by default.
