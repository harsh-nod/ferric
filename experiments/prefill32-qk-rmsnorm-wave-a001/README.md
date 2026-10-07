# Prefill32 Q/K Wave RMSNorm

Source-only, default-off candidate. No active selector, emitted image, native
qualification, token-parity result, or speedup is claimed.

The existing Q/K path uses width128 serial RMSNorm. This candidate reuses V15's
Wave64 reduction with two values per lane, restricted to TP1 prefill32 head-row
counts 256 (K) and 1024 (Q). It preserves the five-slice/four-scalar ABI, empty
auxiliary buffers, exact epsilon, finite checks, and normalization-to-BF16 then
weighting-to-BF16. FP32 summation association changes; neither all-input bitwise
equivalence nor identical near-overflow acceptance is claimed.

The source pins fe2o3 revision `630c01b0bb9b55b0f52084211869af8597f755f5`.
The control must be re-emitted with the same compiler/SDK for comparison. The
historical V15 image is not a matched control. No lockfile has been resolved yet.

## Qualification Order

1. CPU host arithmetic and source/ABI contract tests with `prefill32-qk` enabled.
2. Same-compiler device emission; inspect two input iterations, wave reduction,
   register count and spills. Preserve both exact source and artifact manifests.
3. Component numerical tests at both shapes, including captured real Q/K inputs,
   followed by host-wall ABBA measurement. Host fixtures are not native tolerances.
4. Only after those pass, add a separate prefill32-only selector and admission
   mode. Prove exactly72 kernel-name changes at layer roles4/5 across36layers in
   the649-command/396-slot graph; all arguments and other kernels remain fixed.
   Prefill16 and652-command decode remain unchanged.
5. Exact full-token parity, complete native ABBA, then matched HTTP TTFT/TPOT.

Do not interpret reduced scalar work, packet ticks, or host fixture timing as
device speedup or TTFT improvement.
