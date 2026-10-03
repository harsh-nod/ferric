# Strict BF16 Wave64 Argmax V22

This additive experimental root selects one token from one active Qwen3-8B BF16
logits row. It keeps the existing slice/slice/rows ABI and fixed capacity of
16 x 151936 input BF16 values and 16 output token IDs. Only `rows = 1` and one
Wave64 workgroup are admitted. The other 15 rows and output slots are inactive.

Each lane scans 2374 disjoint tokens, preserving its earliest maximum. Raw BF16
bits are mapped to ordered nonnegative integers, with zero reserved for invalid
NaN/infinity inputs. Positive and negative zero share one ordering value;
negative values reverse bit order. The integer map avoids hardware floating-point
denormal handling and preserves BF16 subnormals. Three convergent subgroup
reductions find any invalid input, the maximum ordering value, and the maximum
`151936 - token` tie key. All reduction operands are exact FP32 integers. Only
lane zero writes the earliest winner, after finite rejection.

The independent CPU reference widens BF16 bits with `f32::from_bits` and uses
ordinary numeric comparisons. Tests exhaust every BF16 bit pattern for finite
classification and sorted numeric ordering, then compare the actual scan body
against that reference. Further tests cover cross-lane ties, signed zeros,
subnormals, all finite extremes, random inputs, every lane's invalid-value
rejection, inactive NaN padding, read-only inputs, and output-tail preservation.
Source-body equivalence binds host numeric macros to the inline emitted code.

These tests are numerical models, not device execution or evidence of speedup.
Native checked lowering, ordinary runtime admission, guarded GPU correctness,
and a matched strict-BF16 end-to-end performance comparison are separate gates.
No projection, runtime boundary, compiler proof, or existing kernel is changed.
