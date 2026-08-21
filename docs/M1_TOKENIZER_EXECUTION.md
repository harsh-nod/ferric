# M1 Tokenizer Execution

Ferric executes the tokenizer only through `AuthenticatedTokenizer`, whose
private seal is constructed after the complete pinned `tokenizer.json` stream
has passed its byte identity and closed-schema checks. The authority retains
the exact 151,643-entry base vocabulary and all 151,387 ordered merge pairs;
there is no descriptor-only tokenizer constructor.

The implemented encode domain is deliberately narrower than Hugging Face's
general text domain:

- input is valid Rust UTF-8 and every input byte must be ASCII;
- NFC is therefore the identity operation;
- the fixed Qwen3 split expression is evaluated with closed ASCII letter,
  number, whitespace, contraction, punctuation, and line-ending classes;
- every split is mapped through the pinned GPT-2 ByteLevel alphabet;
- BPE repeatedly applies the lowest-ranked adjacent merge, with no dropout,
  unknown token, prefix, suffix, byte fallback, or merge bypass; and
- the 26 exact added tokens use leftmost-longest matching, then their admitted
  ID order as the final tie break.

The first 14 added tokens have their authenticated `special` bit set. Encoding
rejects these strings by default and requires `SpecialTokenEncodePolicy::Allow`
to emit their exact IDs. The remaining 12 added tokens are recognized under
either policy. Decode either preserves all added-token content or omits only
the 14 special tokens. Decode returns bytes because an arbitrary base-token
slice is not necessarily valid UTF-8.

Each call validates caller bounds against fixed hard ceilings: 32 KiB input,
8,192 token IDs, 128 KiB decoded output, and 16,777,216 charged operations.
All counters and output growth are checked. Exhausting any limit fails instead
of truncating or selecting a fallback tokenizer.

This slice does not claim full Hugging Face tokenizer behavioral equivalence.
In particular, it rejects every non-ASCII input rather than approximating NFC,
Unicode property classes, regex case folding, or offset behavior. It does not
apply the chat template, add implicit special tokens, report normalized source
offsets, stream text, or establish a Verus theorem for tokenizer execution.
