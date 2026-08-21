# Qwen3 Tokenizer Fixture

`qwen3-tokenizer.json` is the exact shared upstream `tokenizer.json` payload
for both pinned M1 repositories:

- `Qwen/Qwen3-8B` at revision
  `b968826d9c46dd6066d109eabc6255188de91218`;
- `Qwen/Qwen3-0.6B` at revision
  `c1899de289a04d12100db370d81485cdf75e47ca`.

Both immutable revision URLs returned byte-identical files. The fixture is
11,422,654 bytes with SHA-256
`aeb13307a71acd8fe81861d94ad54ab689df773318809eed3cbe794b4492dae4`.
The corresponding Hugging Face LFS object has the same SHA-256 and size. Both
source repositories declare the Apache-2.0 license in their model metadata.

The closed semantic admission records are:

- 151,643 base vocabulary entries, IDs `0..151643`, with semantic SHA-256
  `d42824870d58ccbf38bc6d29b312cc4550c8543f448c45fe644dd041f3eff638`;
- 151,387 ordered two-token BPE merges, with semantic SHA-256
  `1f8c784c660c1659a981d03c46deea0abcbf3fb4f6e85938e27281869890734f`;
- 26 exact added tokens at IDs `151643..151669`.

Semantic records use eight-byte big-endian length-prefixed fields. Vocabulary
records hash the domain `ferric.qwen3-tokenizer-vocab.v1`, then each four-byte
big-endian ID and UTF-8 token in ID order. Merge records hash the domain
`ferric.qwen3-tokenizer-merges.v1`, then each eight-byte big-endian ordinal and
the two UTF-8 token members in order. The upper range endpoints above are
exclusive.

Sources:

- <https://huggingface.co/Qwen/Qwen3-8B/blob/b968826d9c46dd6066d109eabc6255188de91218/tokenizer.json>
- <https://huggingface.co/Qwen/Qwen3-0.6B/blob/c1899de289a04d12100db370d81485cdf75e47ca/tokenizer.json>

This is tokenizer metadata and vocabulary only. It contains no model weights.
