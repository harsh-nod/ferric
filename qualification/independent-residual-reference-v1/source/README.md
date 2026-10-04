# Independent TP2 residual reference (unfrozen draft)

This is the first small component of an independent layer reference, not an
MLP, full-layer, model, runtime, or GPU qualification. It checks only supplied
FP32 partials and BF16 residual/output captures. No imports, tests, builds, or
GPU execution have been performed by this proposal's author. The 18 authored
pure tests and source must be reviewed and actually run by root before any
passing-result claim. No dependency packages are needed.

## Exact source contract

Both residual phases select
`ferric_qwen3_tp_peer_tp2_ordered_residual_bf16_v18`, not the general TP8 V4
kernel. For each of 4096 elements the source performs:

```text
first = RN_f32(+0 + partial_rank0)
sum   = RN_f32(first + partial_rank1)
value = RN_f32(sum + exact_f32(residual_bf16))
out   = RN_bf16(value)
```

All input values, FP32 intermediates, and narrowed output must be finite.
The residual is added last, with no BF16 rounding of partials or intermediate
sums. RNE means round-to-nearest, ties-to-even. Signed zero is preserved where
the specified operations preserve it; the explicit initial +0 removes -0 from
an all-zero partial sum. A negative tiny final FP32 value can still narrow to
BF16 -0. The existing p222 helper already implements this exact contract using
integer multiples of 2^-149, RNE, finite checks, and complete 4096-element rows.
It is reused byte-for-byte as `helpers/residual_oracle.py`, with SHA256
`551f89f968cad8f63073ca975346c593a31dad624115a93059b28a328e312ef3` checked before
loading. Its original path is `L/proposals/p222-model-reference/helpers/residual_oracle.py`;
the p226 `fixtures/residual_oracle.py` copy has the same hash. No new arithmetic
implementation or relaxed tolerance is introduced. New code adds only missing
per-stage/two-stage capture comparison and focused integration tests.

The one-layer capture hook already exposes the required data:

| Phase | FP32 partials, each rank | BF16 residual input | BF16 actual output |
| --- | --- | --- | --- |
| `post_attention` | `prefix_capture[rank][6]` | pre-prefix hidden input | `first_residual[rank]` |
| `post_mlp` | `mlp_capture[rank][4]` | `first_residual[rank]` | `final_hidden[rank]` |

Both kernels read shared Partial scratch via `prefix[rank][13]`; the MLP down
output is the same storage as `mlp[rank][9]`. Capture prefix partials before MLP
overwrites them. Both ranks consume identical replicated residual inputs; the
comparison rejects inconsistent replicas. It authenticates no storage identity,
source receipt, image, execution, or rank provenance by itself.

`compare(stage, partials, residuals, outputs)` takes pairs of immutable
little-endian byte strings: each partial is 16384 bytes and each residual/output
is 8192 bytes. It compares all 8192 output BF16 words exactly and returns a
data-only conditional report with hashes of its actual conditioning bytes.
`compare_stages(input_hidden, prefix_partials, first_residual, mlp_partials,
final_hidden)` checks the two boundaries in order, using the actual first
residual as the second skip input. It does not validate the intervening MLP.
The unchanged helper exposes `residual_vector` and `ordered_residual_bits` for
vector/scalar reference use. Invalid input, overflow, extent, stage, replica,
or output mismatches raise `ValueError`; no partial successful report is returned.

At TP2 swapping the two partial rows does not change the mathematical result
under this fixed finite RNE addition contract. The tests make this limitation
explicit: ordered input hashes change, but the expected output does not.
Actual rank/shard identity must be authenticated by the caller, not inferred
from arithmetic success. Conversely, wrong residual placement, early BF16
narrowing, altered partials/residuals, and changed output bits are detectable
when they affect the result; no oracle can detect a numerically inert mutation.

## Inspected source closure

These are review inputs, not newly executed or qualified dependencies. Paths
below are relative to `/home/harsh/ferric-p227-integration` unless marked `L`.
The comparator imports only the hash-pinned helper above; the following Rust
files are inspected source contracts, not runtime Python dependencies. Root must bind the actual
residual artifact/provider review when connecting real captures; source hashes
alone do not prove that an image executes these operations.

- `device/qwen3-tp-peer-tp2-kernels-v18/src/collective.rs`, lines 3-25 and 30-80:
  `dac509f21bdd5f43c5f6334f4d36a9ddb6c3fda2de710c9d4f8936df233541f0`.
  Existing Rust tests at 145/153 distinguish early narrowing and residual order.
- `device/qwen3-tp-peer-tp2-kernels-v18/tests/contract.rs`:
  `abd1d0bb007c9faae6b57e6e1d9c143fdf11e11e635d3bf1b5d305c4bcbff97d`.
- `adapters/tp-peer-finite-engineering-worker-v1/src/resident_artifacts.rs`,
  residual symbol selection at 47:
  `04f3e82eb7b21abf8d6c820b9c598f659d1565ed5ecd15ad95fbf4e79c8bc8aa`.
- `adapters/tp-peer-finite-engineering-worker-v1/src/resident_layer/prefix_tiles_v6.rs`,
  capture order at 56, both residual calls at 241, captures at 286/334:
  `51ca180f410768d088a3ff1c39f1a11705d4b3aedb95190468202fc096cc7b35`.
- `adapters/tp-peer-finite-engineering-worker-v1/src/native_catalog.rs`,
  shared Partial storage at 712-740:
  `67ba5a41268467beaad82f4630474e75d40f479dafd3fcc5f3441480a82a5922`.
- `L/independent-native-profile-cpu-v228-v1/source/fe2o3/crates/fe2o3-device/src/half.rs`,
  BF16 RNE/widening at 185/197:
  `0d7b2193ec67c34919181a1ecefff23d24ee9af45ec567c5a4f57772792f10af`;
  `L=/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216`.

## Tests and next integration

Root can run `python3 -B -m unittest -v test_residual_reference.py` from this
directory in its bounded CPU environment. New tests cover pinned helper loading,
normal/subnormal ties and overflow, nonuniform 4096-element rows, both stage labels, sum-invariant
rank swap, wrong residual/partial/output, signed-zero corruption, nonfinite
values, exact type/extent/rank-pair constraints, first-to-second skip joining,
and first-stage failure preventing the second comparison. Historical arithmetic
tests remain separate; they are not claimed as newly executed. This is a suggested command,
not an executed result or receipt.

Before using real data, root should join these rows to the existing complete
layer captures and original hidden-input pin, preserving capture-before-reuse
and source/image/provider custody. An accepted conditional residual report
does not establish the upstream partial's correctness: the O and MLP down
operators need their own independent checks. All numerical/runtime prerequisite,
GPU provenance, full-layer/model, production, and performance flags stay false.
No gamma tolerance, paired-profile parity requirement, or full-layer acceptance
is introduced here.
