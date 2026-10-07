# Native Packet Attribution, October 7

The exact native649/native652 Qwen3-8B path now has per-packet device timestamp
attribution. The main targets are projections, followed by normalization.
This is **not a speedup or a new comparison with vLLM**.

## Measured Breakdown

Percent of the summed firmware packet windows, not shader-only time:

| Operation | Decode | Prefill32 |
|---|---:|---:|
| Q/K/V projections | 17.96% | 24.23% |
| Attention output projection | 7.29% | 8.13% |
| Gate/up projections | 22.44% | 21.28% |
| Down projection | 16.56% | 22.95% |
| All normalization | 17.30% | 9.66% |
| Attention partial/merge or prefill GQA | 6.83% | 7.18% |
| RoPE | 3.60% | 1.81% |
| Head and argmax | 4.12% | Excluded singletons |
| Remaining operations | 3.91% | 4.75% |

All projections together account for **64.24% of decode** and **76.60% of
prefill**. The role mapping checks the actual symbol at every position in all
36 layers, rather than distinguishing reused projection kernels by name alone.
Exact raw sums and role shares are in the [checkpoint JSON](native-packet-attribution-20261007.json).

## Boundaries

The capture contains 85,400 dispatch intervals in 131 unchanged native program
executions: four prefill32 programs and 127 decode programs. The three separate
prefill head commands remain outside this capture. Every interval is nonzero
and ordered, and every adjacent end/start pair is **exactly equal**. These
firmware intervals partition the observed window; they do not independently
resolve shader execution versus launch, fence, dependency or signal work
inside each interval. Reporting the zero uncovered ticks as "zero GPU idle or
dispatch overhead" would be incorrect. No GPU tick frequency is calibrated or
converted into nanoseconds here.

The previous [wait-observation capture](native-wait-attribution-20261007.md)
already bounded late CPU completion observation to a mean upper tail of
0.123 ms. This capture is consistent with that result: mean completion wait
45.720 ms and mean upper tail 0.124 ms. Its registered span is 51.384 ms, but the
extra profiler preparation, checks and serialization are included in that
span. Do not interpret this as a production regression, or its 5.664 ms outside
wait as normal engine overhead. The earlier wait-only capture had 1.211 ms
outside wait. Neither is a fresh HTTP TTFT/TPOT measurement.

## Qualification

- MI350, TP1/C1, 128 input and 128 output tokens, context 8192, BF16 decoder,
  configured FP32 head, greedy fixed length, prefix cache and speculation off.
- One instrumented request, October 7, 20:41:24-20:43:56 UTC. All 128 token IDs
  and UTF-8 match the independently frozen reference.
- Default-off fe2o3 packet diagnostic published as `1736eff451`, built from
  `179629a` plus the byte-verified source delta. Qualified controller and kernel images
  are unchanged. Same native packet order, fences, publication and polling.
- All 27 focused runtime tests and 42 parser/capture tests pass on mi300x-2,
  along with worker compilation, strict library Clippy, default and ordinary
  engineering feature checks, and release worker build. An overly broad
  native-name test filter was interrupted and is not counted as passed.
- Clean, reaped, unsignaled shutdown; stable inputs/model; paired live process
  endpoints and sampled isolation. These observations do not prove continuous
  isolation or matched power/clock conditions.
- Independent stdlib audit verifies every byte of all 180 retained files,
  token replay, controller/wait binding, all interval/position sums and report
  arithmetic. Archive SHA-256:
  `46896f2639b0a09a67adee1274014a7d62239b0e2202e72e05b2d95de7323000`.

## Next Work

1. Rebind the existing standalone native688 split-K8 down experiment to the
   current uninstrumented runtime and complete all three ABBA blocks. Down is
   16.56% of decode, so its earlier component improvement could matter, but
   the interrupted two-block model estimate remains unqualified. This decode
   candidate does not optimize prefill down.
2. Implement checked-load grouping in fe2o3, preserving each original predicate
   and fallback. Require actual multiple outstanding loads in ISA, register
   pressure/spill checks, component parity and timing before native ABBA.
   Ferric retains the kernels and full-model integration.
3. Prioritize prefill32 MFMA down and gate/up loading, tiling and occupancy.
   These operations account for 44.23% of prefill packet time together. The
   isolated Q/K RMSNorm candidate covers only 4.64%; it cannot close TTFT alone.
4. Measure normalization improvements/fusion after projections. Obtain shader
   entry/exit observations or a qualified device trace before assigning costs
   inside firmware intervals to shader execution versus dispatch/fences.

Each candidate still needs an isolated matched run and then a composed HTTP
run. The latest complete matched HTTP result remains Width55c: Ferric
431.033 ms TTFT / 53.119 ms TPOT versus vLLM 18.969 / 4.352 ms. No new vendor
comparison or improvement is established by this diagnostic.
