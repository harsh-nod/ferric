# Offline Draft Reference Producer

`tools/draft_reference.py` is a separate Draft06B reference producer. CPU tests
exercise policy and parser logic only, with explicitly synthetic tokens. No
GPU or numerical pass is implied until root launches and validates the actual
producer. Existing target8B references and all performance checkers remain
unchanged. The reference is BF16/SDPA, not an FP32-head reference.

## Frozen Offline Environment

The tool accepts only the exact canonical `Qwen/Qwen3-0.6B` checkpoint revision
`c1899de289a04d12100db370d81485cdf75e47ca`, verifying complete file sizes and
SHA-256 for config, safetensors and both tokenizer files before and after the
forward passes. The header-free tensor payload hash is separate from the
complete safetensors hash. No remote model code or downloads are allowed.

Use the already cached image
`vllm/vllm-openai-rocm@sha256:e0a3b2bd3fe7ec563916c3a5d949898d133458c18d6b2f460c906885cfb32032`,
local image ID
`sha256:c5f9efa2623d9c8b2e483d2a139202e496baf5a04900a4f32907149f41a42dba`.
Override its server entrypoint with Python. It supplies torch
`2.12.0+git6bbd260`, Transformers `5.15.1`, tokenizers `0.22.2`, safetensors
`0.8.0`, and Python3.12. Exact Qwen3 source-file hashes are checked too. This
environment differs from the historical target8B reference and gets its own
producer identity. No container package or shared cache is modified.

## Root-Owned GPU Recipe

Root alone prepares the owned mode0700 output directory, freezes producer
SHA-256/checkpoint/image ID/device/idle expectations, selects one GPU, and runs
a bounded container. Require `--pull never`, `--network none`, a read-only
checkpoint/producer mount, private output, overridden Python entrypoint,
`HF_HUB_OFFLINE=1`, `TRANSFORMERS_OFFLINE=1`, and exactly one visible gfx950 GPU.
Do not use the default `vllm serve` entrypoint. Do not expose a server port.

Inside that wrapper, the producer arguments are:

```bash
python3 -I /producer/draft_reference.py --producer-sha256 "$PRODUCER_SHA256" \
  produce --source /checkpoint --output /result/draft-torch-raw.json \
  --image "$EXACT_IMAGE_REFERENCE" --image-id "$EXACT_LOCAL_IMAGE_ID"
```

Root must require zero container/wrapper status, normal removal/reaping,
expected physical device and clean before/after all-eight GPU idle receipts;
the raw file is insufficient by itself. Image fields are explicitly external
assertions and must be checked by that wrapper against Docker inspection.
An initial 8GiB free-GPU allowance and 600-second timeout are conservative
preflight bounds, not measured memory or performance.

The producer loads only local canonical BF16 Qwen3 weights, requires empty
loading-error information and actual tied embedding/head storage and bytes,
then uses eval/inference mode and explicit SDPA. It tokenizes the raw literal
`The capital of France is` into five IDs without a template/special tokens.
Each repetition is six token-at-a-time cached forwards: five prompt tokens,
then the first selected token. Cache lengths and explicit position IDs are
checked. No EOS stopping, penalties, suppression or generation processor is
used. Both completed repetitions must agree in every retained step, including
all six argmax choices, top-two values/gaps, and both decoded output tokens.

The cached Transformers5 loader returns three empty sets plus an empty error
list; the tool checks those exact types before converting them to empty JSON
arrays. Its independently hashed loading-report source raises conversion
errors before a successful `from_pretrained` return. No nonempty load report,
unknown field or missing tied-weight exception is silently discarded.

Every forward examines all151936 logits for finiteness and computes ascending
lowest-ID argmax independently on CPU after a readback; device argmax must
agree. The actual head dtype must remain BF16. Converting those logits to FP32
for inspection does not undo BF16 rounding. The readbacks are diagnostic work,
not accepted TTFT/TPOT or throughput measurements.

## Separate Identity Adapter

The raw `FerricIndependentDraftTorchObservationV1` contains no Ferric bundle
identity and cannot be mistaken for a Ferric run. Prepare a separate externally
SHA-pinned `FerricDraftReferenceIdentityV1` JSON document from authenticated
canonical model/configuration admission:

```text
schema: FerricDraftReferenceIdentityV1
checkpoint: exact four filename -> {bytes, sha256} records from canonical files
identity:
  model_bundle_id: canonical authenticated bundle identity
  draft_model_id: canonical Draft06B model identity
  draft_config_id: canonical authenticated draft config identity
  draft_weights_sha256: complete header-free tensor payload SHA-256
```

This identity input must not contain tokens or candidate outputs. The adapter
checks all four checkpoint hashes, canonical draft model ID and payload hash;
bundle/config identities remain externally authenticated inputs, and the
Ferric canary independently compares them to its actual admission before
worker spawn. No target model ID may replace the draft model ID.

CPU-only adaptation, after root has accepted the raw wrapper receipt:

```bash
python3 -I /producer/draft_reference.py --producer-sha256 "$PRODUCER_SHA256" \
  adapt --raw /result/draft-torch-raw.json --raw-sha256 "$RAW_SHA256" \
  --identity /result/identity.json --identity-sha256 "$IDENTITY_SHA256" \
  --output /result/draft-reference.json
```

The exact producer is checked before input files are read. Documents are
bounded, closed-schema and reject duplicate keys/nonfinite values. New files
use exclusive creation in an owned0700 parent, mode0600 and file/directory
sync. Any exception is nonzero; if a final sync failed after writing bytes,
root must reject the file through the nonzero wrapper status.

The final small `FerricDraftCanaryReferenceV1` pins its source producer and raw
receipt in its producer label. It copies expected tokens only from the accepted
independent raw reference. Never replace a mismatch with Ferric-observed IDs or
change dtype/backend/prompt after observing the candidate result.

## CPU Gate

Run only on mi300x; tests import no torch or Transformers and launch no GPU:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s tools -p test_draft_reference.py -v
```

The pure policy tests cannot establish actual PyTorch import compatibility,
loading-info API behavior, numerical repeatability or draft token equality.
Those remain explicit root-owned native prerequisites.
