# Position-5 Gate/Up Exact-Dot Diagnostic

Source proposal only. No diagnostic, tests, model arithmetic, GPU work or native
rerun has been executed by this author. The primary owns staging and execution
after the active matched GPU pair has retired. This is not a kernel patch or
a new numerical acceptance rule.

## Closed Question

The admitted causal layer-zero comparison retained 204 comparable tensor rows.
At position 5, both native rank postnorm vectors are byte-identical to the
framework's postnorm and actual Gate/Up inputs: 8,192 bytes, SHA256
`62339bad15d35910fe2fe0e8bf4206871c1c656ae5fb5b92291b62df47084a67`.
Exactly seven scalar words differ across the four 6,144-element Gate/Up
partitions. The diagnostic independently rediscovers this complete position-5
difference roster from the pinned raw sidecars:

| Position | Rank | Role | Local Row | Checkpoint Row | Native BF16 | Framework BF16 |
| --- | --- | --- | --- | --- | --- | --- |
| 5 | 0 | Gate | 553 | 553 | bd86 | bd85 |
| 5 | 0 | Up | 2575 | 2575 | badd | badc |
| 5 | 0 | Up | 2603 | 2603 | 3cb2 | 3cb1 |
| 5 | 0 | Up | 2840 | 2840 | ba49 | ba4a |
| 5 | 1 | Gate | 2063 | 8207 | 3c50 | 3c51 |
| 5 | 1 | Up | 1719 | 7863 | b846 | b847 |
| 5 | 1 | Up | 5310 | 11454 | b8cb | b8cc |

For each location, it computes exactly one real 4,096-term dot on the shared
captured input and authenticated original BF16 weight row, then rounds that
real sum once to nearest-even BF16. It reports integer sums in units of
`2^-266`, exact decimals, each observed word's signed distance from the sum,
the observed pair's exact midpoint and the sum's signed distance from it.
It reports whether either observed word equals once-rounded exact arithmetic,
without requiring either side to win.

The retained `head.py` (`9dbe5ff2`) and all twelve `test_head.py` tests
(`fd5b69d1`) are byte-identical to the executed exact-head/QKV oracle.
Finite BF16 operands decode into integer units of `2^-133`; products and sums
are exact integers in units of `2^-266`. This is not an MFMA reduction-tree,
FP32 accumulation, compiler, framework or GPU instruction emulator. No tolerance
is applied, and no numerical threshold is added or relaxed.

## Authentic Inputs And Weights

Eight original input bodies come from the retained
`qualification/guarded-mlp-readiness40-causal-layer0-v1/qkv-exact-v1/inputs`
capsule. They include actual comparison `3f94fe36`, native summary/terminal,
the full native sidecar, the independent reference inner receipt, both repeated
reference sidecars, and the complete upload manifest `d5e66dbf`.
The prior comparison is an explicit admission anchor; this small diagnostic
does not claim to rerun the entire native/reference capsule verifier.

A ninth input, `registration-original.json`, is the unchanged 122,075-byte
`3ecdd5af` original from
`qualification/silu-materialized-native-capture-v1/native/candidate-registration.json`.
Only its parsed `session` and `child_identity` fields are changed in memory to
the actual causal Begin scope. Compact serialization must reproduce the
complete current Begin commitment, 122,073 bytes /
`d45ff2054c5f8b0a99c5f9b449aa97a8dd7463bdd06eb68d27963dee80582880`.
Every other byte of the registration structure, including source-program
commitment, rank/layer roster and role mapping, is thereby joined. The derived
serialization is never substituted for the retained original input or described
as an original captured body.

The current source upload mappings are:

| Rank | Role | Source ID | Shape | Checkpoint Rows |
| --- | --- | --- | --- | --- |
| 0 | Gate | 190 | 6144 x 4096 | 0..6144 |
| 1 | Gate | 191 | 6144 x 4096 | 6144..12288 |
| 0 | Up | 192 | 6144 x 4096 | 0..6144 |
| 1 | Up | 193 | 6144 x 4096 | 6144..12288 |

These are contiguous row partitions, not transposed, column-sharded, interleaved,
or packed-QKV buffers. The two original checkpoint tensors are
`model.layers.0.mlp.gate_proj.weight` and `model.layers.0.mlp.up_proj.weight`,
both BF16 `[12288,4096]`. The authenticated index `f9fdbcb9` maps both to
`model-00001-of-00005.safetensors`, 3,996,250,744 bytes / `31d6a825`.

One ordinary no-follow descriptor reads the bounded safetensors header, validates
the complete indexed shard geometry without holes/overlap/trailers, reads the
seven complete rows, hashes all four 50,331,648-byte rank partitions against
their exact current `source` upload records, then hashes the entire original
shard. File identity/stat are checked before/after. After the dots, this whole
operation repeats and all row bytes, partition metadata and full-shard metadata
must match. Exact selected row hashes are observed outputs of that future run,
not invented source literals. The full original index is also pre/posthashed.

The recorded current selected tiles image is `33320/b0d1766f`. Its pin is
checked in the actual request metadata; this CPU diagnostic neither rehashes
GPU executable bytes nor transfers historical replay ISA identity or emulator
claims to that image. The older registration is used only for a cryptographically
checked current role/weight association.

## Focused Tests

Eight new `GateUpTests` methods join twelve unchanged oracle methods for a
closed twenty-test CPU gate. The new tests cover both 6,144-row boundaries,
rank-one global coordinates, wrong-role and aliased source IDs, transposed
tensor dimensions, index/dtype/extent errors, missing/duplicate uploads,
wrong projection inputs, changed comparison pins, complete seven-word census,
reference repetition, registration/source-program/upload commitments, and
partition hash mutation. Synthetic one-term exact dots check signed midpoint
reporting, finite/extent refusal and nonacceptance flags.

Tests use synthetic bytes only and are not native measurements. The full
original captures are admitted separately after the tests.

## Bounded Execution

Fresh remote root:

`/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-causal-gate-up-exact-v228-v1`

The package is seven source bodies including the source manifest, plus nine
original input bodies under `inputs/`. No checkpoint shard or GPU ELF is copied.
The controller uses the existing authenticated original model path and creates
a fresh `output/`; it refuses reuse. Root invokes:

```text
python3 -B run.py OBSERVED_SOURCE_MANIFEST_SHA256
```

Bounds remain the qualified QKV controller's 240 s wall, 180 s CPU, 512 MiB
address space, 2 MiB file/output, CPU affinity 8/9 and nice 10. All GPU visibility
variables are empty. No subprocess, torch import, GPU API or model forward is
used. The same catchable signal/absolute deadline and source/input posthash
helpers are retained. Raw twenty-test stderr and the original successful or
failed terminal are preserved; a failed run is not promoted or overwritten.

A successful result means the bounded exact-dot question and provenance checks
completed. It does not establish an arithmetic bug, explain the final position-5
argmax, repair the other differing tensors, prove SiLU/Down correctness, admit
full-model inference, or claim a performance improvement. Any kernel change
requires a separately justified source proposal and existing independent gates.
