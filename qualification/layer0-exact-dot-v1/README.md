# Exact Layer-Zero QKV Audit

An independent integer-only audit ran on `ssh mi350` against the retained
Qwen3-8B layer-zero capture for token 9112 at position zero. **All 6,144 native
QKV outputs equal the once-rounded exact BF16 dot products.** The independent
framework matches 6,143. Its one differing value is not evidence that the
native QKV arithmetic needs correction.

| Projection | Outputs | Native Matches Exact BF16 | Framework Matches Exact BF16 |
| --- | ---: | ---: | ---: |
| Q | 4,096 | 4,096 | 4,095 |
| K | 1,024 | 1,024 | 1,024 |
| V | 1,024 | 1,024 | 1,024 |
| Total | 6,144 | 6,144 | 6,143 |

## Independent Arithmetic

Every finite BF16 value is an integer multiple of `2^-133`. The reference
decodes each operand to that integer, computes all 4,096 products and their
sum exactly using Python integers, then rounds once to BF16 with
round-to-nearest, ties-to-even. The sum uses `2^-266` units. It does not use
PyTorch, BLAS, a floating-point accumulator, or either implementation's
reduction tree. All rows were checked, including those where native and
framework agree.

The known disagreement is Q output row 168, rank 0, head 1, component 40:

| Quantity | Value |
| --- | --- |
| Once-rounded exact BF16 / native | `0xb8c2` |
| Framework BF16 | `0xb8c3` |
| Exact sum, displayed approximately | `-9.274411149817752e-5` |
| Native absolute error, approximately | `2.377028067712672e-7` |
| Framework absolute error, approximately | `2.391343514318578e-7` |
| Adjacent BF16 spacing | `4.76837158203125e-7` |

The exact sum lies only about `7.1577233e-10` from the midpoint between these
BF16 values. Different FP32 reduction orders can affect such a rounding
decision. This audit measures the distances; it does not establish either
implementation's accumulator order or label the framework a semantic bug.
The complete result retains the exact integer sums and distances, not just
these rounded decimal displays.

## Inputs And Verification

The audit uses the [retained SiLU layer-zero native capture](../silu-materialized-native-capture-v1/README.md)
and [genuine repeated framework capture](../layer0-framework-capture-v1/README.md).
Both ranks and both framework passes have byte-identical normalized QKV inputs.
The eight consumed framework payloads repeat exactly. The original model's
index, configuration and safetensors header bind the selected matrices; all
six rank-specific matrix slices match the native upload hashes. The entire
3,996,250,744-byte source shard was streamed and hash-checked before and after
the computation. Source and input postchecks passed.

All **24 tests passed on MI350**: eighteen arithmetic tests against an
independent Fraction-based nearest-neighbor oracle and six geometry/refusal
tests. Coverage includes signed zero, subnormals, cancellation, overflow,
rounding ties, 4,096-term dots, both TP rank boundaries, and incorrect matrix
headers or shard mappings. The recorded test time is not a kernel benchmark.

- [Full 6,144-row result](raw/complete.json), SHA-256 `b78eb10f6d884fffb48f9621081f46680e140e0bcaa7a279bd66f71278b2dba1`.
- [Exact arithmetic](source/exact_bf16.py), [audit and input checks](source/audit.py), and [reproduction instructions](source/README.md).
- [Arithmetic tests](source/test_exact_bf16.py), [mapping tests](source/test_audit.py), and [actual 24-test observation](tests/primary-observation.json).
- [Actual audit command and SSH result](run-observation.json).

Test/run observations are primary-agent SSH records, not remote process-
supervisor receipts. No model weights, raw capture buffers, executable
bodies, new GPU runs, or runtime changes are included in this checkpoint.

## Limits And Next Work

This is a retained position-zero operator diagnostic, not a capture of the
new RoPE image's internal stages. Native FP32 accumulation is not required
to equal a once-rounded exact sum for every possible input. No general
acceptance tolerance is inferred from this case.

O projection, later normalization and attention, accumulated layer error,
and the full BF16 2,048-token prompt / 256-token own-output workload remain
separate checks. The [mixed AR4 comparison](../rope-indexed-ar4-comparison-v1/README.md)
remains unchanged. Next work should distinguish numerical reduction effects
from semantic errors and measure host control overhead before making kernel
latency claims. There is no throughput result or 700 tokens/s claim here;
all issue #42 M0-M7 acceptance gates remain open.
