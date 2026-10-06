# Independent O-Replay Data Audit

The reviewer completed local read-only standard-library data checks on the
actual MI350 report and test receipt. `audit.py` preserves that audit method
as a standalone source artifact; this packaged script has not itself been
executed. It does not import the proposal, rerun a model, invoke a project
test/compiler, start a subprocess, or use a GPU. It prints its result without
writing source or evidence files.

## Actual Inputs

- All-row report: 10,946,336 bytes,
  `869541d4852c6cc1af45164f2a2521b075db30c62ed0487309cf0dd16abaa515`.
- Eighteen-test receipt: 4,605 bytes,
  `701e09f47946c2974c0f79eca0d14e5b1cb21e94ee197955dc12ee796ea075d9`.
- Reviewed source manifest: 4,340 bytes,
  `6655e7b7bd7a7f9b6f2e10aae1b4a5e854a0cf2b39481b237f420366f69a6827`.

The script also pins both pre-existing local capture archives. Thirty of the
report's 32 consumed bodies were independently rehashed from local proposal,
codegen, or retained capture bytes. The remaining model config/index pins join
the authenticated genuine framework capture manifest; those two bodies were
not locally reread. The roughly 4 GB safetensors shard was not locally rehashed,
and its dot products were not independently recomputed by this reviewer.

All 8,192 native FP32 partial encodings and both 4,096-word native residuals
join the original binary capture. Both genuine framework passes join, including
byte-identical immediate attention operands, projection, embedding, and first
residual. Registered O-buffer identities and extents join actual upload hashes;
the captured prefix image joins the pinned reviewed historical image. This
local audit does not separately deinterleave model weights a second time.

## Independent Arithmetic Checks

Every row was checked using a separate integer scalar decoder and binary search
over finite FP32/BF16 encodings for nearest-even rounding, rather than the
replay's bit-length/shift rounding function. The checks cover all reported
exact-distance and midpoint numerators, ideal BF16 encodings, FP32 rank combine,
BF16 materialization before residual addition, and aggregate flags/counts.

| Observation | Verified Count |
| --- | ---: |
| Captured partials equal fixed-order replay encodings | 8,192 / 8,192 |
| Native derived projection equals once-rounded reported exact dot | 4,095 / 4,096 |
| Framework projection equals once-rounded reported exact dot | 4,093 / 4,096 |
| Native derived projection equals framework projection | 4,093 / 4,096 |
| Captured residual equals replay conditional on framework embedding | 8,192 / 8,192 |
| Captured residual equals framework residual | 8,188 / 8,192 |

Projection differences are exactly rows 1024, 1356, and 3444. Native is the
once-rounded exact-dot value at 1024 and 3444; those are also the two residual
differences on each rank. At 1356, native `b492`, framework `b491`, and ideal
`b490` differ, but both observed residuals remain `3c89`. No semantic defect or
framework accumulation tree is inferred from these observations.

## Conditional Bounds

Let `u = 2^-24` and `gamma_n = n*u/(1-n*u)`. The fixed rank schedule has at most
32 sequential additions followed by six XOR stages. With the report's observed
finite intermediates, zero rounded products, and zero subnormal product/sum
counts, gamma38 is a conservative rank bound. One additional effective FP32
rank-combine addition gives gamma39 for the combined FP32 result; these bounds
do not include final BF16 narrowing.

Using each **reported** exact dot and absolute-product sum, the reviewer checked
the inequalities by integer cross multiplication, not floating-point tolerance:

```text
abs(FP32_value - reported_exact_dot) * (2^24 - n)
    <= n * reported_sum_of_absolute_products
```

All 8,192 rank inequalities and 4,096 combined inequalities hold. Maximum ratios
of observed error to this conservative bound are 0.01076145842283481 at row 26,
rank 0, and 0.008427481618513085 at combined row 26. These are conditional
consistency checks on the retained exact-dot report, not independently
recomputed model dots, a blanket acceptance criterion, or a full-model proof.

The audit preserves the original limitations: historical layer 0/token 9112/
position 0 only; native materialized projection is derived from partials;
native input embedding was not directly captured; residual replay is conditional
on genuine framework embedding. Numerical acceptance, production authority,
model correctness, performance, and sustained-workload claims remain false.
