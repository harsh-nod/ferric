# Current Attention Conditional Result

The CPU-only gate completed on MI350 in 0.23 seconds. All eight named tests
passed, with no failures, errors, skips or postcheck errors. All 12 native
and 12 framework attention states passed the unchanged historical conditional
policy against each side's own captured Q/K/V. These are layer-zero states at
positions 0 through 5, with two ranks per position, not a full-context or
all-layer qualification.

| Side | States | Output words | Exact after oracle narrowing | Tolerated differences | Maximum BF16 steps | Conditional result |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Native | 12 | 24,576 | 24,572 | 4 | 4 | Pass |
| Framework | 12 | 24,576 | 24,573 | 3 | 10 | Pass |

All four position-zero rank/side controls are exact copies of their mapped V
values. For later positions, the pre-existing policy permits at most one BF16
step OR absolute error no greater than `5e-5 * per_mapped_KV_head_max_abs_V`.
The reference uses dense FP64 attention followed by FP32 and BF16 RNE. Larger
encoding distances near cancellation can therefore pass the unchanged absolute
bound. The policy was neither fitted nor widened for these observations.

## Higher-Precision Diagnostics

The six native/framework differing scalars also received independent two-pass
Decimal evaluations at 80 and 160 digits, with exact integer QK dots and the
original FP32 scale. Both precisions give the same directly rounded BF16 word
at each scalar:

| Position | Rank | Local output | Native | Framework | 80/160-digit BF16 |
| ---: | ---: | ---: | --- | --- | --- |
| 2 | 1 | 1455 | `bb36` | `bb37` | `bb36` |
| 2 | 1 | 1820 | `b50b` | `b50c` | `b50c` |
| 3 | 1 | 216 | `b96a` | `b969` | `b969` |
| 3 | 1 | 1804 | `b97b` | `b97a` | `b97b` |
| 4 | 1 | 1334 | `b3cf` | `b3d5` | `b3cb` |
| 5 | 1 | 1901 | `b79e` | `b79d` | `b79d` |

These are encodings, not decimal values. The position-four diagnostic matches
neither observed implementation. Agreement between two Decimal precisions is
not an outward interval or a correct-rounding proof. These diagnostics are
separate from, and do not replace, the historical component acceptance policy.

## Evidence And Limits

This directory retains nine original files: seven source/manifest files and
two output files. The eight shared original input bodies are already retained
unchanged at `../qkv-exact-v1/inputs/`; the executed controller reads their
original remote aliases read-only. All 15 source/input hashes passed postchecks.
No model shard was read, model forward repeated or GPU workload launched.

An independent data audit joined all source/input hashes, all 24 state tensor
pins, per-head causal V maxima, four exact position-zero controls, and all
reported Decimal-to-BF16 encodings. It also rechecked the nine retained originals
and raw eight-test stream. It did not recompute attention or import the
diagnostic locally.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Original terminal | 64,562 | `8df71f73d405ec01eae99ac10be2ffba210c758d4cabcf19264acdb266354e2a` |
| Raw test stream | 1,326 | `b1f64e2fdca716f3ea4da7d4cece6640c778c1790185d2e1af345080225c55ea` |
| Nine-member archive | 30,013 | `98be7a3b79b20920761c92cca71169a432b54bed2cf71968bef1649772bad7d2` |

Passing this conditional component check does not prove the current image's
exponential error bound, all possible contexts, full-model numerical acceptance,
generated-token agreement or performance. No kernel arithmetic or tolerance
changed. The source README's original proposal status remains unmodified;
this page records its subsequent execution.
