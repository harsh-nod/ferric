# BF16 Lossless Weight-Traffic Feasibility

This is a CPU-only sampling study, not an implemented compression format,
measured compression ratio, GPU bandwidth result, or decode performance claim.
The admitted BF16 weights, GPU kernels, and numerical contracts were unchanged.

## Frozen Sampling Policy

- Target: Qwen/Qwen3-8B, revision
  `b968826d9c46dd6066d109eabc6255188de91218`.
- All 399 tensors were represented. Each large tensor contributed 32 disjoint,
  SHA256-name-seeded, stratified 2 KiB windows. Small tensors were read fully.
- The full embedding matrix was excluded from active decode traffic; one
  complete 8 KiB row (token 785) was included. Subsequent token embeddings differ;
  this row's contribution to total active weight bytes is negligible.
- Payload sampled: 17,205,248 bytes, below the predeclared 32 MiB hard cap.
  Additional safetensors headers totaled 46,056 bytes; the index was 32,878 bytes.
- Active BF16 bytes per dense target forward: 15,136,819,200.
- Runtime limits: one CPU, nice 10, 512 MiB address-space cap, 180-second timeout.
  Observed: 10.47 seconds and 21,776 KiB peak RSS.
- Header/index/window hashes and before/after shard stat identities were retained.
  Full weight files were not rehashed during this bounded study. No raw sample
  values were saved. The private report contains source identities and paths.

## Observations

All aggregate values below weight each tensor by its active BF16 byte count,
not by its sampled byte count.

| Statistic | Bits per weight |
| --- | ---: |
| BF16 symbol entropy, plug-in estimate | 10.4940 |
| Symbol entropy, Miller-Madow diagnostic adjustment | 10.5458 |
| Sign entropy | 0.99994 |
| Fraction entropy (all 7 fraction bits) | 6.97005 |
| Exponent entropy | 2.63726 |
| Sum of separate field entropies | 10.60725 |
| Best sampled per-tensor fixed exponent escape model | 11.29515 |

The fixed exponent model preserves the sign and all seven fraction bits. A
short exponent code covers a contiguous range, with one reserved escape carrying
the complete original eight exponent bits. One-, two-, three-, and four-bit
short codes were evaluated without constructing a packed representation. The
best per-tensor selection uses three-bit codes for 94.01% of active bytes and
four-bit codes for 5.99%; the weighted escape fraction is 2.942%.

Adjacent-exponent mutual information was only 0.02871 bits in the sampled
windows. This plug-in statistic is biased and does not rule out other useful
correlations. No sampled zero or nonfinite values were observed; that does not
assert their absence in the unsampled weights.

## Conditional Traffic Scenarios

The bandwidth values below are declared hypothetical scenarios, not bandwidth
measured by this study. Rates omit all codec overhead, metadata, activations,
KV traffic, compute, synchronization, and latency.

| Representation model | Estimated GB per token | At 6.81 TB/s | At 8 TB/s |
| --- | ---: | ---: | ---: |
| Uncompressed BF16 | 15.1368 | 449.9 tok/s | 528.5 tok/s |
| Symbol entropy plug-in estimate | 9.9279 | 685.9 tok/s | 805.8 tok/s |
| Symbol entropy adjusted diagnostic | 9.9769 | 682.6 tok/s | 801.9 tok/s |
| Fixed exponent escape estimate | 10.6858 | 637.3 tok/s | 748.7 tok/s |

Derivation: estimated bytes/token = active BF16 bytes * estimated bits/16;
weight-only tokens/s = declared bytes/s / estimated bytes/token.

At 700 tokens/s, the fixed exponent estimate alone requires 7.4801 TB/s, or
93.50% of an 8 TB/s bandwidth scenario, before any other work. The corresponding
traffic budgets are 10.2833 bits/weight at 6.81 TB/s and 12.0803 at 8 TB/s.
Thus these samples weaken an unconditional impossibility claim based on raw
BF16 traffic, but do not establish that 700 tokens/s is achievable.

## Limits and Next Gates

Deterministic stratified samples are not independent random draws. No confidence
interval is claimed. Finite-sample symbol entropy is downward biased; the
Miller-Madow adjustment is diagnostic, not a rigorous bound. Marginal entropy
is neither an achieved coding ratio nor a universal bound on all lossless
representations. Unsampled exponent tails can increase escapes.

The fixed exponent estimate omits its nominal per-tensor header and all block
indexes, restart offsets, alignment, padding, and decoding cost. Practical
parallel GEMV needs independently addressable blocks. Variable-length escapes
introduce index/offset overhead and divergent or prefix-scan decode work.
Decompression can increase registers/instructions and reduce effective HBM
bandwidth. Retaining BF16 values bit-for-bit is required; substituting FP8 is
not lossless and is outside this study.

Before treating this as a runtime optimization, independently validate a fully
specified block codec with exact reconstruction of every input BF16 word and
measure actual encoded bytes, including all metadata. Then measure fused
unpack-plus-GEMV against the same dense BF16 arithmetic and source identity,
including end-to-end greedy choices, setup boundaries, and repeated qualified
decode measurements. Existing runtime overhead remains a separate optimization
target; this study did not change production format or admission contracts.

## Retained Evidence

- `study.py`: frozen sampling script, SHA256
  `4d013dcc7f48050062589f0a0a6fc8bd06c4b173bfef3af63858f729a48626b1`.
- `report-private.json`: per-tensor metrics, sample policy/window digests,
  source identities, resource use and caveats; SHA256
  `67a70b52c721cf16446fe96dee7a498b12a54ad0bad8cbbb09bdc2db5fa53322`.
- `study-before.sha256`, `run.status`, `run.stdout`, and `run.stderr` retain the
  pre-run script pin and successful process outcome.
