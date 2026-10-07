# Current Position-0 O/First-Residual Join

This is a read-only data audit of the actual causal Readiness40 position-0
boundary. No model dot, framework call, GPU workload, project import, or test
suite was executed for this join. The original successful exact O replay is
reused only after full-byte operand, partial, output, upload and model joins.
No canonical source or original evidence was changed.

## Finding

Both ranks differ from the genuine framework first residual at exactly global
rows **1024 and 3444**. At both rows the native-derived O projection is the
nearest-even BF16 value of the already executed exact real 4096-term dot.
The framework projection is one BF16 encoding step away. The captured residuals
on each side reproduce that side's materialized-projection-plus-embedding
boundary exactly. These differences do not establish that native is wrong.

| Global row | Exact dot to BF16 | Derived native O | Captured framework O | Native R1, both ranks | Framework R1 |
| ---: | --- | --- | --- | --- | --- |
| 1024 | `bc9d` | `bc9d` | `bc9e` | `bd2d` | `bd2e` |
| 3444 | `3b88` | `3b88` | `3b87` | `bb86` | `bb87` |

Values are encodings, not decimal numbers. The projection on the native side
is derived from captured FP32 partials, not directly captured. Current native
input embeddings are directly captured and byte-identical to the framework;
the old replay's formerly conditional embedding assumption is therefore closed
for this current boundary.

Row 1356 is a useful control retained in the JSON: exact/native/framework
projection encodings are `b490` / `b492` / `b491`, but all produce residual
`3c89`. Native is not uniformly nearest to the exact dot. A projection mismatch
is also not sufficient to imply a residual mismatch.

## Closed Data Joins

The full path, byte extent and SHA256 of every directly read artifact, every
selected contained body, and every selected stage slice are in
`current-o-join.json`. The following are the primary custody anchors:

| Artifact | Bytes | SHA256 |
| --- | ---: | --- |
| Current native archive | 5614887 | `6d346693b1024978d9c445d945ee2e01cace16187e5251932ef9ae52f3d06c0f` |
| Current native terminal | 278903 | `00aa0447a23f57c6d9bb2ed39cd4e5b2b9da692f15bceb33107ac3822f95516f` |
| Current native summary | 21934 | `5a414a72e6d0e4b76031d57de909f954c7cd0853a39888b5f3a167fe6e94d8f4` |
| Current framework archive | 9927249 | `aab3837ab2d3d7fcff0d69424246272f3969c1f0a827a2bf853f770bc2d204bf` |
| Current framework inner | 285346 | `5ebd12d258fc9c76190b4ec4c7270d9732d15e635c7944d966bd50ffc81fa027` |
| Historical replay gzip | 1946667 | `ffb00e00d8783d694a4f0379e23a9d96252944371696304c4807638c716dddbf` |
| Historical replay decompressed | 10946336 | `869541d4852c6cc1af45164f2a2521b075db30c62ed0487309cf0dd16abaa515` |
| Historical uploads, also current Begin commitment | 178103 | `d5e66dbf7e3abfec463424addb6735f9a3a5b2d50653d56689ee79d01404da47` |

The current original bodies are retained under:

```
/home/harsh/ferric-p227-integration/qualification/guarded-mlp-readiness40-causal-layer0-v1/gpu-v3/
/home/harsh/ferric-p227-integration/qualification/guarded-mlp-readiness40-causal-layer0-v1/reference-v1/
```

The previous executed report and exact source are retained under
`/home/harsh/ferric-p227-integration/qualification/o-projection-exact-replay-v1/`.
Its original registration/upload manifests remain under
`qualification/silu-materialized-native-capture-v1/native/`.
Archive paths and original framework pass-body paths are explicit in the JSON;
no historical path or receipt has been rewritten.

- Both current attention halves equal the historical original native capture
  and concatenate to the genuine framework attention operand.
- All **8192 current FP32 O partial encodings** and **8192 current R1 BF16
  encodings** join the original historical capture and every replay-report row.
- All **8192 current embedding encodings** join the genuine framework and the
  replay's residual operand. Both current framework passes' attention,
  projection, embedding and R1 arrays equal all eight historical raw bodies.
- An independent integer binary-search nearest-neighbor oracle rechecked all
  4096 stored exact-dot BF16 roundings, 4096 current TP-sum projections, 8192
  native residual encodings and 4096 framework residual encodings. It consumed
  stored exact sums and current captured operands; it did not recompute dots.
- Every directly read local artifact was rehashed after the audit.

## Weight Custody

The original replay streamed and rehashed the complete original shard before
and after the full all-row computation:

```
/home/harmenon/ferric-mi350-model-b968-c189-r1.kUjwIKs0/model/target/model-00001-of-00005.safetensors
3996250744 / 31d6a825ae35f11fb85b195b4c42c146c051e446433125a215336abdf95cbf5f
```

That identity also equals the current genuine reference's model source record.
This local audit does not claim a fresh read of the remote 4 GB shard.

The tensor is `model.layers.0.self_attn.o_proj.weight`, BF16 `[4096,4096]`,
without bias. Its safetensors relative byte interval is
`[1555054848,1588609280)`, after the 8-byte length and authenticated 9328-byte
header. Each full row is 8192 bytes; the absolute row offsets are
1563452792 (row 1024) and 1583277432 (row 3444).

The prior executed replay hashed all 4096 deinterleaved row halves against
these actual upload records, each 16777216 bytes:

- Rank 0: source ID 200, columns `[0,2048)`,
  `695a205027c01cc6bd7238b76d8ca2e503f508f4be7d6ad7f6dfe427a0c2fb8e`.
- Rank 1: source ID 201, columns `[2048,4096)`,
  `79bca2f8b04191dd004a3248160b1e97848d5b1613b72bfed63ef8a944ec4b4c`.

The current Begin upload commitment equals the entire original upload manifest,
not a reconstructed selection. Its source-program commitment also equals the
old registration's 871211-byte program,
`eb8607aae21d2774b188c3477e0b03a0b924dcd75638b18624707e543b743392`.

The current registration is not claimed byte-identical to the historical one.
As a data-only check, changing only `session` and `child_identity` in the old
parsed registration and serializing with its original compact key order yields
exactly the current Begin commitment: 122073 bytes /
`d45ff2054c5f8b0a99c5f9b449aa97a8dd7463bdd06eb68d27963dee80582880`.
Thus every role, source ID, shape and weight mapping remains identical. That
derived serialization was never substituted for an original retained body.

## Exact Method And Association

The historical replay computed every full 4096-term real dot by decoding finite
BF16 operands into integer units of `2^-133`, multiplying them exactly, and
summing in integer units of `2^-266`. It rounded the final real sum once to
nearest-even BF16. This is not an FP32 or framework-tree emulation.

Separately, its fixed-order emulator modeled each 2048-term rank: 64 lanes,
32 terms per lane, individually RNE FP32 multiplication and addition, then six
XOR stages with masks `1,2,4,8,16,32`. All 8192 historical captured rank partials
equaled that emulator. No rounded or subnormal products, or subnormal sums,
were observed on these operands. This does not eliminate addition rounding.

The boundary uses ordered `FP32(FP32(+0 + partial0) + partial1)`, materializes
BF16 projection, widens that value and the BF16 residual input, adds in FP32,
then narrows to BF16. The independent data audit reproduces both ranks' current
residuals exactly by this boundary, and independently reproduces the genuine
framework residual from its own captured projection and embedding.

Captured partials are **not** assumed once-rounded exact rank dots. At row1024,
the rank1 captured partial is `3cd019a2`, versus `3cd019a3` from rounding its
exact rank sum once. Nevertheless both lead to full projection `bc9d`. At
row3444 both captured partials equal once-rounded exact rank sums. At row1356,
rank0's captured `3c35af0c` differs from ideal-rank `3c35af0f`; the native
projection is `b492`, versus full exact `b490`. These distinguish association
effects from a missing residual materialization boundary. They do not prove a
particular framework reduction tree or transfer historical machine semantics
to a different current image.

## Image Roles And Limits

All three pins below are image-byte identities, not interchangeable manifests:

| Role | Bytes | SHA256 |
| --- | ---: | --- |
| Current base/setup prefix, `sequence.begin.prefix_image` | 48584 | `4d0fe835eef3b76bc1b60ca560e6a8028f85bb9ce292bac77ccf6c6da65e6285` |
| Current guarded prefix, `sequence.prefix_image` and `request.prefix_image` | 54344 | `29fd58e7b09fee003ed31660e6201c20a7eb5da4b873d0b4d954993f7018f2f8` |
| Historical replay's source/LLVM/ISA-bound prefix | 53560 | `4885204c8d510122588549107f42d2bc6f180f48fbc4eddd3bb1260e8d6629c5` |

Current guarded prefix path is
`rope-indexed-checked-emission-v228-v1/emitted/artifact.hsaco` below the original
remote evidence root. Parent source reads base setup images separately from
the guarded sequence images. The current partial/output bytes match the prior
replay, but no current/historical image or ISA equivalence is asserted.

For this observed boundary, differing attention operands, differing embedding
operands, differing checkpoint/uploaded O weights, and an omitted modeled BF16
projection materialization are not needed to explain the two R1 differences.
The first Q discrepancy does not propagate into the captured position-0
attention result. The data do not establish a causal path from either R1 word
to the position-5 argmax, a semantic defect, framework accumulator order,
full-model acceptance, a new tolerance, or performance improvement.

No new O dot job is needed. If a separately executed CPU join gate is desired,
it should consume these exact original artifacts and assert the above joins,
without reopening weights or importing project code. Focused negative fixtures
would mutate one current partial, residual or attention word; a registration
weight role/ID; an upload half hash; a reference/model pin; or a replay row ID.
Every mutation must fail custody before deriving a claim. Image inequivalence
must remain an explicit nonclaim rather than an equality requirement.
