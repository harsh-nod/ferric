# Qwen3 Safetensors Schema Fixtures

These fixtures contain metadata only. They do not contain model tensor data.
The `*.header.json` files are canonical semantic JSON fixtures derived with
`jq -c` from the upstream headers; they are not byte-exact upstream headers.
Each file has one repository LF terminator. Tests remove that LF and append
ASCII-space padding to the pinned header length, reconstructing the exact raw
header bytes and checking the raw header SHA-256 below. The index fixture
retains the exact upstream bytes.

Header lengths and header digests cover the JSON-plus-padding region after the
eight-byte little-endian length prefix. Target full-file bytes total
16,381,516,776, while target tensor-data bytes total 16,381,470,720. Draft
full-file bytes are 1,503,300,328, while draft tensor-data bytes are
1,503,264,768.

Both source repositories declare the Apache-2.0 license in their Hugging Face
model metadata.

## Target

- Repository: `Qwen/Qwen3-8B`
- Revision: `b968826d9c46dd6066d109eabc6255188de91218`
- Source: <https://huggingface.co/Qwen/Qwen3-8B/tree/b968826d9c46dd6066d109eabc6255188de91218>
- Index: `model.safetensors.index.json`, 32,878 bytes, SHA-256
  `f9fdbcb91c23971c13ec5d5f2573d2349e8f61f2f049371ec699281748fdb1bc`

| Shard | Full bytes | Full SHA-256 | Header bytes | Header SHA-256 |
| --- | ---: | --- | ---: | --- |
| `model-00001-of-00005.safetensors` | 3,996,250,744 | `31d6a825ae35f11fb85b195b4c42c146c051e446433125a215336abdf95cbf5f` | 9,328 | `979bbeed365485ddaa67a1ed41d0289e15e2f3ba0b3388cb93e42d31f346d1df` |
| `model-00002-of-00005.safetensors` | 3,993,160,032 | `5991236cea6fe21f3d43cab0f0e84448734fbbe0789816202989f2ddc9d18282` | 13,144 | `347ca6985eefe87273139b97dde9c547da3ab59c782a73beac1d498556ac0b45` |
| `model-00003-of-00005.safetensors` | 3,959,604,768 | `c5185c4794be2d8a9784d5753c9922db38df478ce11f9ed0b415b7304d896836` | 12,824 | `2c34e283f27490d335b129053f9c50504a357ba8b67a2e89fcf5d39cdd85f6f4` |
| `model-00004-of-00005.safetensors` | 3,187,841,392 | `b5ee7de71fbf17db3d5704e0c8f2bc7d005ca9e1d7ca2aeb19827b0cfcaa917a` | 10,600 | `3f2742edfb110486c05a06b091280eefb7d2bc05252d1b78f1233c1a813a48e7` |
| `model-00005-of-00005.safetensors` | 1,244,659,840 | `20c2d6366ab85c90786ccdd829cd2b9e7d30ef3b2ebbb998280e7e4014b542ff` | 120 | `205a19ce27198b75abaeedc42f7a64387e19628b7804aff215887b255f1b8dd7` |

## Draft

- Repository: `Qwen/Qwen3-0.6B`
- Revision: `c1899de289a04d12100db370d81485cdf75e47ca`
- Source: <https://huggingface.co/Qwen/Qwen3-0.6B/tree/c1899de289a04d12100db370d81485cdf75e47ca>
- File: `model.safetensors`, 1,503,300,328 bytes, SHA-256
  `f47f71177f32bcd101b7573ec9171e6a57f4f4d31148d38e382306f42996874b`
- Header: 35,552 bytes, SHA-256
  `399d16f500e925c7e923fe05966c6df6862ab64da60916843119e802f1801bca`
