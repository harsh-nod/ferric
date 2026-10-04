# Historical two-residual capture replay (unfrozen draft)

This data-only CLI checks the actual first and second TP2 residual results in
the retained V227 layer0/position0/token9112 capture. It neither launches GPU
work nor evaluates a model. The old baseline and candidate captures happen to
be byte-identical, but each is checked against independent residual arithmetic;
their mutual parity is not the numerical oracle. Nothing here validates the
new V7 image, upstream O/down partial arithmetic, the MLP, or a complete layer/model.

The draft and its 14 authored pure policy/layout tests have not been imported
or executed by the author. Root owns review, testing, actual bounded CPU replay,
source freezing and publication. The only new files are this README, `run.py`,
and `test_run.py`; no existing reference or capture is changed.

## Reused code and evidence

- The exact retained `p227-prefix-layer-replay-v1/layer_validation.py` (16186 B,
  SHA256 `757c077979b7ee02d4c0e7d222e75292cd9d417927f84ae9a8134344f8e217b9`)
  revalidates the original 22 body buffers plus summary/request, bootstrap,
  actual source/program/upload joins, terminal states, close-only capture,
  and all 28 array extents/hashes/parity rows. The recomputed object must equal
  the exact retained replay receipt's `checked` field. Existing original
  failure status stays `FAILED_UNCHANGED`; no historical runtime audit is rerun.
- The unchanged tested `p228-independent-layer-reference-v1` four-file source
  closure is hash-checked before loading. Its `compare_stages` links actual
  first residual into second skip input and checks both rank outputs exactly.
  Root reported its 18 synthetic tests passed on MI350; this CLI does not rerun
  or manufacture that test receipt. It adds no arithmetic implementation.
- The fixed historical replay completion is 465821 B, SHA256
  `d9958b9a6d37fc707993aa4dd4ae05ca062820df4692bf9ab37d8df7821f8d8a`.
  Both actual 9670656-byte captures must hash to
  `c46324f59a03f3db83e891475261cf527d19b7c1b5a981ce21b7b763e182f52f`.
- The source checkpoint's original `model-00001-of-00005.safetensors` is
  3996250744 B, SHA256
  `31d6a825ae35f11fb85b195b4c42c146c051e446433125a215336abdf95cbf5f`,
  as retained in `p222-model-reference/model_reference.py` (source SHA256
  `a058b58c17446e9fa6bfd1e8e5ae2231fb0727401a609a0d6c02b05c6326c37f`).
  No NumPy, Torch, safetensors library, or model interpreter is imported.

## Original hidden input

The request and both native registrations bind model/bundle identities. Their
actual upload tables bind source rank0/id185 to the full BF16 token embedding
tensor, shape [151936,4096], 1244659712 B, SHA256
`458b4af1d22ed8d7d12235ed5249a4606c7606e35e775e7ea2244ba623b0312e`.
The requested prompt file is rehashed and its first u32 token must be 9112,
matching the pinned bootstrap/input/checked result. The safetensors header is
bounded and parsed as JSON, with exact tensor dtype/shape/span validation.
One streaming pass hashes the entire pinned model shard and embedding tensor,
retaining only token9112's 8192-byte row. This row is the independent expected
original hidden input for both ranks; no observed residual is substituted for it.
Original model-directory and transported shard paths are reported separately.
This authenticates supplied bytes, not GPU execution of the embedding kernel.

## Capture partition

Each rank has fourteen consecutive arrays; rank1 begins at 4835328. Exact
shapes/order come from the unchanged validator and are cross-checked locally.

| Input to residual comparison | Rank0 offset | Bytes |
| --- | ---: | ---: |
| O partial | 4741120 | 16384 |
| First residual | 4757504 | 8192 |
| MLP down partial | 4810752 | 16384 |
| Final hidden | 4827136 | 8192 |

Rank1 adds 4835328 to each offset. All 28 slices are rehashed, including
unconsumed stages. First/second residual inputs are taken from their own
original capture boundary, before shared Partial scratch was reused.

## Root-owned invocation

Example paths below are existing evidence/model inputs, not commands executed
by this proposal. `--evidence-root` must contain the original relative evidence
paths under `/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`.
Relocation is explicit and recorded; no arbitrary original absolute path is opened.

```sh
python3 -B -m unittest -v test_run.py
python3 -B run.py \
  --evidence-root "$E" \
  --validator "$E/p227-prefix-layer-replay-v1/layer_validation.py" \
  --reference-dir "$E/p228-independent-layer-reference-v1" \
  --model-shard "$MODEL/model-00001-of-00005.safetensors" \
  --prompt-tokens "$PROMPT/prompt.u32le" \
  --output "$FRESH_OUTPUT"
```

`$PROMPT` is the retained `evidence/finite-two-forward-v223/p224-prompt-v1`
directory, outside `$E`; its relocated token path is explicit and must match
the original request's full size/hash. Root should use its bounded CPU-only
runner. There is no supervisor,
SSH, process launch, build, or GPU API here. The CLI writes only the requested
fresh result after all checks succeed. Small inputs and helper source files are
fully rehashed afterward; the model shard is fully hashed during its single
streaming read, with inode/size/mtime/ctime checked before/after and after math.
Output remains conditional residual comparison, with V7/MLP/full-layer/model,
runtime/math prerequisite discharge, production, and performance claims false.
