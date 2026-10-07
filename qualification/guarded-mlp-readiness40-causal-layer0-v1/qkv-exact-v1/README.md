# Five Current QKV Exact Dots

Source-only CPU diagnostic. No tests, model reads, dots or GPU executions have
been performed by its author. The primary runs the bounded script on MI350.

The actual causal comparison is 285,894 bytes / SHA256
`3f94fe36125e3cbbf92e33caca6049b94fd33963956ef1477bf304dcec6bf38f`.
It authenticates both original causal capsules, their same-side parity and the
genuine repeated reference. Its first comparable difference is position 0,
rank 0, Q projection. Direct original-byte inspection finds exactly five QKV
scalar differences across the six captured positions, each with the same full
4,096-word captured normalized input on both sides:

| Position | Rank | Projection | Local Row | Checkpoint Row | Packed Rank Row | Native BF16 | Framework BF16 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | 0 | Q | 168 | 168 | 168 | b8c2 | b8c3 |
| 1 | 1 | Q | 1902 | 3950 | 1902 | b976 | b977 |
| 2 | 0 | Q | 1462 | 1462 | 1462 | 3787 | 3788 |
| 3 | 0 | Q | 10 | 10 | 10 | 35d8 | 35d7 |
| 5 | 1 | K | 97 | 609 | 2145 | 3663 | 3664 |

All V outputs are equal. Position 4 has no QKV difference. These observations
do not establish the cause of the position-5 argmax mismatch, nor make QKV the
only source of divergence. In particular, position-0 attention and embedding
inputs agree while its first residual differs at two words. That separate
O/TP-reduction/residual composite is not diagnosed by this helper.

## Arithmetic and Custody

`head.py` and its twelve tests are byte-identical to the qualified
`model-readiness40-position5-v1/head-exact-v1` originals. The new helper reuses
only their finite BF16 decoder, exact 4,096-term integer dot, exact decimal
formatter and round-to-nearest-even BF16 encoder. Each product has units
2^-266. Five dots require 20,480 exact products. There is no floating-point
dot, native lane-reduction replay, framework GEMM emulation, tolerance or
arithmetic substitution.

The new six tests cover rank/row coordinates, original tensor/index geometry,
five-difference extraction and reference repetition, packed-upload uniqueness,
five-dot reporting and finite/exact-extent refusals. The unchanged twelve
oracle tests include independently structured Fraction checks, cancellation,
ties, subnormals, signed underflow and overflow. The runner requires all
eighteen exact names before admitting actual data.

The native current Begin authenticates the *entire* upload manifest at
178,103 bytes / `d5e66dbf7e3abfec463424addb6735f9a3a5b2d50653d56689ee79d01404da47`.
That is the exact retained historical `candidate-uploads.json`, not a new
serialization or an assumption that historical weights are current. It has
one layer-0 packed-QKV entry for each rank, each 25,165,824 bytes:

- Rank 0: `f1d51970183aef10357a69cd80e138f936e99fbd2ebd9133435b0cab2fb71248`.
- Rank 1: `b2bb9c215888c94f800040e1bc981ecd44062857ba9ce938d99b103aa153881a`.

The native setup source concatenates each rank's Q rows, K rows and V rows,
each with all 4,096 columns. Thus Q has 2,048 rows per rank and K/V have 512
each. The runner independently reads the authentic safetensors header/index,
checks full-shard no-hole/no-overlap geometry and exact tensor shapes, hashes
both concatenations and requires their equality to those current upload pins.
The five row bytes and packed hashes come from the same regular descriptor.
They are not native device-memory readbacks.

The original target shard is `model-00001-of-00005.safetensors`, 3,996,250,744
bytes / `31d6a825ae35f11fb85b195b4c42c146c051e446433125a215336abdf95cbf5f`.
The complete shard is streamed and rehashed before and after the diagnosis;
stable device/inode/type/link-count/owner/size/mtime/ctime stamps, exact row
pins, packed pins and complete model index pin must agree. RAM does not hold
the shard. The model remains at the genuine authenticated source path.

## Staged Inputs

The eight filenames and full pins are literal `INPUTS` in `run.py`. Stage only
these original bytes in the new root's `inputs/` directory:

- `comparison.json`: actual `causal-layer0-stage-comparison-complete.json`.
- `native-terminal.json`: `Q/guarded-mlp-readiness40-causal-layer0-v1/gpu-v3/readiness/complete.json`.
- `native-summary.json`: the same capsule's `readiness/native/complete.json`.
- `native-sidecar.bin`: the same capsule's `readiness/native/child-stderr.bin`.
- `reference-inner.json`: `Q/guarded-mlp-readiness40-causal-layer0-v1/reference-v1/output/complete.json`.
- `reference-sidecar1.bin`: the same output's `pass1-causal-layer0.bin`.
- `reference-sidecar2.bin`: the same output's `pass2-causal-layer0.bin`.
- `uploads.json`: `Q/silu-materialized-native-capture-v1/native/candidate-uploads.json`.

`Q` denotes the canonical qualification directory. The actual source/hash
and full capsule admission belong to the pinned prior comparison; this new
run does not requalify the entire capsules. It rehashes these eight selected
original bodies, joins current Close/Begin and model/bundle, checks both
reference sidecars byte-repeat by stage payload, rediscovers all five QKV
differences from raw words, and joins their full tensor pins to the original
comparison. No unauthenticated extracted input files are accepted.

## Primary Invocation

Stage the seven source files (six bodies plus manifest) under the fresh root:

`/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-causal-qkv-exact-v228-v1`

The primary invokes `/usr/bin/python3 -I -B <root>/run.py <manifest-sha256>`.
The exact manifest SHA is handed off after source peer. The runner requires
the MI350 owner host/UID, affinity 8/9, nice 10 or lower priority, hidden GPUs,
512 MiB address space, 180 CPU seconds, a 240-second continuously rearmed wall
deadline, no core and 2 MiB output-file bounds. It starts no child or model.
`output/` must not exist: there is no retry or overwrite of spent evidence.

The original `tests.stderr` and `complete.json` or `failed.json` are retained.
Hard expiry may leave a failed prefix, never a successful receipt. Source,
selected input and index hashes are checked again before terminal publication.
The terminal includes original row pins/offsets, both packed hashes and full
shard pin, exact dots, observed words, ideal real-dot RNE, signed exact errors
and signed distance to the two observed words' midpoint. Native/framework
matches are independent booleans: neither is assumed to be the oracle.

Matching ideal real-dot rounding is not proof of framework equivalence, a
native implementation bug, full-model correctness or numerical acceptance.
Every acceptance, performance, production and GPU-execution claim stays false.
Elapsed CPU diagnostic time is not GPU performance.
