# Historical MLP Capture Replay

Draft, not executed or frozen. Root owns review, bounded CPU execution and
publication. This proposal has three files and no copied arithmetic helper.

`run.py` authenticates the retained V227 layer0, position0, token9112 capture,
then calls the unchanged P218 numerical reference for post-attention RMSNorm,
gate projection, up projection, SwiGLU and down projection. It performs all
five checks for two ranks and two historical profiles. Later checks consume
actual captured predecessors; no oracle output replaces a captured input.

The existing historical layer validator, residual replay reader, original
fixture loader and P218 arithmetic are unchanged. No subprocess, GPU launch,
retry, adaptive bound or new tolerance is added. A failure cannot become success
through historical native parity. Only the original historical observation has
paired-parity requirements; this code does not reinterpret that as independent
arithmetic evidence.

## Boundaries

Success means the twenty conditional stage checks pass on historical captured
bytes, under the existing arithmetic premises. It does not establish V7,
all-layer or full-model correctness, tail normalization/head arithmetic,
runtime-premise discharge, production admission or performance. Separate exact
artifact arithmetic review is still required. The previously checked residual
arithmetic is not rerun here. The original historical controller status remains
`FAILED_UNCHANGED`; this new CPU result does not rewrite it.

The fixture's original model-shard/index/extractor pins are retained provenance,
not fresh executions of those tools or a new full-shard rehash. The actual
seven BF16 files are hashed before use by frozen `reference.fixture`, joined
to both profiles' actual registered source uploads, then rehashed after the
comparison. Gate/up are NxK row shards[6144,4096]; down is an NxK column shard
[4096,6144], not a contiguous half of the original down file.

The first-residual, MLP norm, gate, up, activation and FP32 down-partial slices
are derived from the existing authenticated 28-array capture contract. Prefix
`norm` is not `mlp-norm`; down partials stay FP32 and are never narrowed to BF16.
There are no internal MLP captures in the separate TF4 all-layer observation,
so this result must not be attributed to that newer run.

## Exact Dependencies

Set the original existing evidence directories on MI350:

```sh
E=/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220
P218=/home/harmenon/ferric-asrock-42/evidence/resident-layer-tp2-v218
```

This draft consumes these unchanged modules and data:

| File | Bytes | SHA256 |
|---|---:|---|
| tested `p228-residual-capture-replay-v1/run.py` | 14960 | `88f6e66fccb2ea1ef0d6079858a4b6af1d0b609fa50ec3ea02cb21329300021a` |
| `p227-prefix-layer-replay-v1/layer_validation.py` | 16186 | `757c077979b7ee02d4c0e7d222e75292cd9d417927f84ae9a8134344f8e217b9` |
| `$P218/reference/reference.py` | 23782 | `2e41d6e5715cc561bf818fdee794e4ced74a77f93acd4961d8b15f427f196c6e` |
| `$P218/reference/extract.py` | 10473 | `76872f3558d6f90c21b9eee3ec0ba4ebdd79168b620338faa6bdbdd8b78bdf4a` |
| `$P218/reference/policy.json` | 2799 | `9497e55e70a42630d50b74ea33b0856a385d051fb60bc7f23124ee5cbd46e44b` |
| `$P218/reference-fixtures-v1/fixture/manifest.json` | 4230 | `bda8f0697ba29928e3f4f0e130fdb1a69d3f9eb094988ffe900287dda145bcbc` |
| historical selected MLP548 image | 32328 | `ead57f9bb74d47de14ab5b8b84f4728df3ce6e62fac4d8cdf2d4cf0be3e0b1e6` |

The original fixture directory must exist, without path rewriting or symlinks.
It includes `policy.json` identical to the helper policy, `post-norm.bf16`8192
bytes and gate/up/down for each rank, each50331648 bytes. Seven BF16 payloads
total301998080 bytes. The immutable manifest pins every payload. Root confirmed
this original P218 directory is already present on MI350.

The CLI checks the **selected** MLP548 image, not the legacy29656-byte
`images.mlp`. Its original request path is
`/home/harmenon/ferric-asrock-42/evidence/p225-tiles-tf4-runtime-v1/tiles.hsaco`.
An explicitly supplied transported copy is accepted only by the same exact
size and hash, and original versus transported paths remain in the result.

The evidence root supplies `prefix-layer-replay-v227-v1/complete.json` (465821
bytes, SHA`d9958b9a6d37fc707993aa4dd4ae05ca062820df4692bf9ab37d8df7821f8d8a`)
and every file its retained-native roster pins. Both captured files are9670656
bytes, SHA`c46324f59a03f3db83e891475261cf527d19b7c1b5a981ce21b7b763e182f52f`.
Existing reader relocation is allowed only for this evidence-root tree, not for
rewriting the original P218 fixture manifest.

No model-shard argument is needed for these conditional checks. If root needs
to authenticate or recreate the original fixture separately, reuse frozen
`extract.py`. The correct existing MI350 shard path contains `model/target/`:

```
/home/harmenon/ferric-mi350-model-b968-c189-r1.kUjwIKs0/model/target/model-00001-of-00005.safetensors
```

## Root-Owned Run Contract

Use a pinned Python interpreter with NumPy2.2.6. Use the existing bounded
CPU-only runner, with no GPU visibility, one BLAS thread, two-core affinity,
12GiB address-space bound, original deadline/output limits and fresh exclusive
result directory. Do not launch another supervisor from this script. Root
records exact commands, exit status, streams, source/dependency snapshots and
ownership externally. This author has not run any commands below.

Keep `PYTHONOPTIMIZE` unset (even the string`0` is refused), run with`-B`, and
set `OPENBLAS_NUM_THREADS=1`, `OMP_NUM_THREADS=1`, `MKL_NUM_THREADS=1` and
`NUMEXPR_NUM_THREADS=1`. Do not alter the immutable helpers or use an unrelated
top-level `extract` module. The pinned sibling is installed only during the
reference import and the previous module entry is always restored.

First run the original arithmetic suite, then this adapter suite:

```sh
"$PYTHON" -B -m unittest discover -s "$P218/reference" -p test_reference.py -v
"$PYTHON" -B -m unittest discover -s "$PROPOSAL" -p test_run.py -v
```

The original suite contains19 authored test methods. Its unchanged
`test_reference.py` SHA is
`4de379796dc83561809aaf070a1e8f129a72e7a664d8a3b113d0d278280b524d`.
It uses the already-existing original integer oracle at
`/home/harmenon/ferric-asrock-42/evidence/wave-output-lowering-v216/tp2-residual-reference/reference.py`,
4347 bytes, SHA
`551f89f968cad8f63073ca975346c593a31dad624115a93059b28a328e312ef3`.
Unset `P218_INTEGER_ORACLE`, or set it explicitly to this exact pinned file.
There are22 newly authored adapter tests; neither suite was run by this agent.
Synthetic routing tests are not a replacement for the real P218 arithmetic or
the actual retained-capture integration invocation.

After the actual suites pass, root runs:

```sh
"$PYTHON" -B "$PROPOSAL/run.py" \
  --evidence-root "$E" \
  --historical-helper "$HISTORICAL_RESIDUAL_PROPOSAL/run.py" \
  --validator "$HISTORICAL_LAYER_PROPOSAL/layer_validation.py" \
  --reference-dir "$P218/reference" \
  --fixture-dir "$P218/reference-fixtures-v1/fixture" \
  --mlp-image "$ACTUAL_MLP548_IMAGE" \
  --output "$NEW_OUTPUT/mlp.json"
```

`NEW_OUTPUT` already exists and is exclusive to this root-owned run;`mlp.json`
must not exist. The script writes only that result, only after every check and
all retained-input rechecks succeed. Failure yields no accepted result.

Root should inspect four profile/rank records, exactly five named stages each,
the selected image/capture/fixture/upload joins and false nonclaims. Stage
statistics come directly from the frozen reference. Do not invent a result
count or infer success from the authored test inventory.
