# Native Probe Recipe

This is a recipe, not an executable probe or an approval to launch GPU work.
Use an owned, bounded remote build stage on `mi300x`; use an explicitly approved,
idle GPU only when the integration lead schedules native qualification. Never
fault a shared GPU deliberately. Nonfinite and malformed-launch fault cases
remain host contract tests unless separately authorized in an isolated setting.

## Emission Gate

1. Record Ferric source identity and require fe2o3 revision
   `21682228486f7186cc3c37ddf165fffc438d8b6a` in the compiler, Cargo metadata, and
   generated artifact receipts. Use the existing managed gfx950/Wave64 build
   and measured crate binding, not the host-test binding from `build.rs`.
2. Emit only this crate's closed one-root roster. Admission must independently
   match `compiler_expectation_roster_v11()` and the emitted typed ABI/effects.
   Do not relabel an old v8 image or its receipt. The kernel body must contain
   authenticated subgroup lowering, not a host stub or fabricated lane context.
3. Confirm 36-byte arguments, launch block `[64,1,1]`, grid `[rows,1,1]`, and
   loop bound 2374. Record image, contract, compiler and source hashes, complete
   emission status, and disk cleanup receipts. Stop if emission needs a new
   toolchain stage that exceeds the integration lead's existing space budget.

## Full-Vocabulary Fixtures

Adapt the pinned native fixture helper and admission flow documented by
`../../qwen3-tp-fp32-head32-kernels-v8/tools/probe.py`; do not pass a v8-only
allowlist to this new root. The v11 fixture runner and adapter route are not
implemented by this crate. It needs two buffers and scalar `[rows]`, with
`rows` workgroups. Retain helper/worker/image identity checks, immutable-input
comparison, and guard verification from the existing probe.

For active row counts 1, 2, 16, 17, 31 and 32, compare every output token ID with
an independent ascending finite FP32 scan over all 151936 logits per row:

- The winner in each of 64 lanes at the first, middle and final scan step.
- Equal maxima across lanes and within a lane, especially IDs 63/64 and 127/128.
- Both signed-zero tie orders, uniform negative rows, last-token winner, finite
  extremes and positive/negative subnormals. Device flush-to-zero behavior must
  not silently weaken the exact finite-FP32 contract; fail qualification if it
  differs from the frozen scan.
- FP32 distinctions that BF16 would collapse, and deterministic finite-bit
  randomized vectors. Do not round the reference through BF16.
- Minimum-sized and capacity-32 logits/choices. Fill inactive input rows with
  NaNs and inactive outputs with a bitwise sentinel. Prefix/suffix guards and
  all input bytes must remain unchanged; every active choice must be written.

## Integration And Timing Gate

After native fixture success, the integration lead may add a separately admitted,
explicit opt-in adapter route that replaces only the final v8 argmax dispatch.
Keep head projection mode, FP32 logits, checkpoint, prompt IDs, fixed128 greedy
reference, context, concurrency and all other kernels unchanged. Require the
exact full 128-token reference and output bytes before measuring speed.

Current `output_head` timing aggregates three synchronous host round trips:
final RMSNorm, selected head projection, then argmax. Its maximum is not tagged
with a kernel or ordinal, and the records explicitly report host wall latency,
not GPU duration. Label each head dispatch separately in any diagnostic work.
Only properly attributed GPU timestamps can establish GPU duration; overlapping
host spans must not be added together. Archive matched control/candidate raw
latencies, TTFT, TPOT, throughput, correctness and source identities, including
unsuccessful candidates. Make no speedup claim from CPU fixtures or dispatch
count alone.
