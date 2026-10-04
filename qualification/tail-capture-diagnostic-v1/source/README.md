# Conditional Tail Diagnostics

Draft only. No imports, tests, real-data replay, GPU work or publication have
been performed for this proposal. There are 22 authored synthetic adapter and
policy tests. Root owns review, source freezing, the bounded CPU execution and
any publication. The existing reader/reference sources are not modified.

## What This Adds

For each of four **retained TF4** positions, compute these separately:

1. P222 `model_reference.dense_norm(helpers, captured_layer35_hidden,
   original_model_norm_weight)` versus the captured final norm.
2. P222 `layer_reference_v2.dense_project(mlp, original_head_NxK,
   captured_final_norm)`, followed by its unchanged `narrow_projection`, versus
   captured logits.

The head intentionally receives **captured final norm**, never our predicted
norm. These are conditional stage diagnostics, not a reference-fed whole tail
or model. The existing BF16 diagnostic comparator reports exact words, maximum
absolute error, RMSE, relative L2 and maximum BF16 steps. Predicted norm/logits
are retained as eight BF16 files. Captured-logit lowest-index argmax must equal
the captured output token; the independently predicted head token may differ
and is reported without inventing an acceptance rule.

`passed: true` means the retained-data diagnostic completed with authenticated
inputs, not that any numerical discrepancy was accepted. Every numerical,
full-model, runtime-premise, production and performance acceptance flag remains
false. `acceptance_threshold` remains null even for exact agreement.

## Authentic Inputs

The fixed observation is
`prefix-independent-decode-tf4-shared-full-currentness-gpu-v228-v1/complete.json`,
SHA256 `db417b2f7d0728577a711aa212564d0f3df783ec33a133fadaf35356ea9c159b`.
Its actual selected prefix is the new V7 image. Its separate tail image remains
112872 bytes, SHA256
`11f53cfe2d18668f191af9f998f48627211aa09a36d5483e516946032ad54d8e`.
Do not confuse the old bootstrap default prefix image with the selected V7
override. Both selected runtime and actual tail image are retained in output.

The frozen `p228-independent-decode-diagnostic-v1/run.py`, SHA256
`259f6f233be23da36eac213bfb5e0c905afa43461461fbb0305efcc45c08d930`,
replays the unchanged closed observation, native captures, natural reap/Close,
selected runtime and six retained audit-leaf checks. The adapter calls only
its read-only `bootstrap`, `loaded` and `replay_observation` helpers. It does
not invoke a native process or the old whole-framework comparison.

P222 is loaded from an explicit canonical directory. Its ten-file manifest
SHA256 is `7c19a46b59c67094447152d3e30c2c9bbe1deedfbef4a20c2a23ed4574b3efe3`.
The whole declared closure is rehashed before import and after replay, including
unchanged P218 helpers and P222 policy/tests. The model helper is
`a058b58c17446e9fa6bfd1e8e5ae2231fb0727401a609a0d6c02b05c6326c37f`;
the FP64 projection helper is
`3facbd495b6fd0dd01856fd389101ed7c7b0af3c28d94171d185d3a28335c60f`.
Bootstrap import aliases are restored after loading authenticated helpers.

The original model directory is normally:

```text
/home/harmenon/ferric-mi350-model-b968-c189-r1.kUjwIKs0/model/target
```

Only the authentic original index and shards 4/5 are read. Exact full-file
hashes, header/index membership, complete shard geometry and payload bounds are
checked by the existing P222 readers before mapping just these two tensors:

| Tensor | Original Shape | Shard | Payload SHA256 |
|---|---|---|---|
| `model.norm.weight` | `[4096]` | 4 | `4f4f6cc0467f0cf8516b154f3ec0748ed03d82ae24d435dfe678075e9a8e2070` |
| `lm_head.weight` | `[151936,4096]` | 5 | `6e46ee56769d64b9a338f350100bdfca5f26a91c3fb5bf479ffcb36c733f7939` |

The old retained `prefix-layer-gpu-v227-v1/native/baseline-uploads.json` and
`baseline-program.json` are reused **only because their complete byte extents
and SHA256 values equal the new TF4 bootstrap records**. No old registration,
session, model/layer acceptance or runtime authority is transferred.

Original source 979 is NxK. Its actual uploaded transpose is KxN, SHA256
`bbeb36637eac1dcf46c74be1241037752551edf73beed03887938b640f1651bb`.
The adapter independently hashes that transpose in at most 16-row chunks,
without allocating a second whole 1.24GB tensor. The authenticated actual final
MFMA dispatch reads transposed source **1488**, not original source 979; this
corrects the earlier memo's dispatch-source shorthand. Its exact scalars are
`[1,151936,4096,2,6]`. Norm uses source 978 and epsilon FP32 bits 897988541.

The existing capture reader validates all 38 arrays in each 606976-byte capture.
This adapter additionally checks these slices agree with its partition:

| Stage | Byte Offset | Bytes |
|---|---:|---:|
| `layer35-hidden` | 286720 | 8192 |
| `final-norm` | 294912 | 8192 |
| `logits` | 303104 | 303872 |

## Root-Owned Run

The package requires NumPy in the chosen existing bounded CPU environment.
No new subprocess/ownership framework is added. Use an ordinary interpreter
without `-O` or nonempty `PYTHONOPTIMIZE`. Parent should retain its exact Python,
NumPy, environment, command, raw output and resource-limit evidence as usual.
Set BLAS/OpenMP thread counts to the existing two-core budget before starting
Python; this adapter does not alter the execution environment.

```sh
/usr/bin/python3 -B -m unittest discover -v -s PROPOSAL -p test_run.py
/usr/bin/python3 -B PROPOSAL/run.py \
  --p222-directory AUTHENTIC_P222_DIRECTORY \
  --model-directory /home/harmenon/ferric-mi350-model-b968-c189-r1.kUjwIKs0/model/target \
  --output NEW_CANONICAL_OUTPUT_DIRECTORY
```

Defaults use the existing MI350 `finite-resident-integration-v220` evidence
root for the frozen reader, fixed TF4 observation, old identical uploads and
source program. Their file paths can be supplied explicitly with
`--diagnostic-reader`, `--observation`, `--uploads`, `--program`; exact content
pins remain fixed. The complete observation replay still requires its original
retained E paths, not just a copied JSON. The P222 path is explicit because this
package does not assume its deployment namespace.

The read-only head mapping consumes about 1.24GB of address space; original
shard 4 is streamed for authentication, not mapped in full. Dense projection
uses P222's 64-row FP64 chunks. The transpose has a bounded approximately
4.9MB copy buffer. Rehashes of two original shards at both ends are intentional.
The existing 12GiB address-space envelope is not relaxed. Root must choose and
retain a CPU deadline; no elapsed-time prediction or successful run is claimed.
On failure, no `complete.json` is written, and already written source snapshots
or predicted outputs remain evidence, not a successful receipt. After success,
`complete.json` records all source, capture, original-file/payload, upload,
transpose and predicted-output pins plus pre/post checks.

## Tests And Remaining Gap

The 22 authored tests cover ordered and chunk-independent NxK/KxN hashing,
noncontiguous logical layout, invalid shapes/dtypes, strict digest bytes,
upload source/extent/transpose joins, duplicate uploads, new-bootstrap identity,
actual final dispatch source/scalars, original index selection, immediate-input
routing, expected output geometry, nonfinite/truncated/mispartitioned captures,
propagated reference failure, token diagnostics and false acceptance flags.
Synthetic adapters deliberately do not claim to test authentic file replay or
the unchanged P222 arithmetic. The real retained-data invocation is the
integration gate; optional existing P222 arithmetic tests remain unchanged.

P222 norm uses independent FP64 summation/sqrt and BF16 boundaries, not the
actual tail's serial FP32 sum/sqrt/division sequence. Head uses FP64 dots, then
an explicit FP32 boundary and BF16 RNE, not the actual 256-update K16 MFMA
accumulation. Any hard tolerance still requires a separate actual-image-specific
error contract covering rounding, denormals, sqrt/division and BF16 conversion.
The prefix/MLP reduction bound is not reused. New V7 reciprocal qualification
does not qualify the unchanged tail normalization reciprocal. This diagnostic
does not establish preceding-layer, whole-model, or sustained-decode correctness.
