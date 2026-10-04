# Genuine Layer-0 Comparison Draft

Four source files, fourteen authored synthetic tests, not executed by the
author. This is a data-only comparison library, not another launcher or proof
framework. Root owns actual receipt authentication, imports, tests, execution,
retention and publication. No future framework capture SHA is invented.

## Immediate Comparison

Call `compare_retained(framework_pin, original_pin, native_pin, read, diagnostics)`.
`read(FilePin)` returns the exact bytes, optionally using an explicitly recorded
transport mapping. The library independently checks every supplied FilePin's
extent and SHA. It returns `(result, framework_values)` and does not write files.
The caller rechecks all consumed files and retains its own source/input ledger.

The first input is the new `capture.json` FilePin from the successful, naturally
exited/reaped outer `ferric-p228-layer0-framework-launch-complete-v1.reference`.
Root must authenticate the outer result, audit results, execution result and
source package before passing this inner pin. An inner report alone is not
proof of completed process lifetime. The module deliberately returns
`receipt_authentication=false`; it does not claim to repeat the supervisor.

The second input is the original genuine TF4 framework `reference.json`:

```
E/framework-rearm-v224-v1/reference/reference.json
E/framework-rearm-v224-v1/reference/pass1-pos0.bf16
E/framework-rearm-v224-v1/reference/pass2-pos0.bf16
```

These three original files need to be extracted/transported from the retained
framework payload archive if not already present. The original and new model
file names, byte extents and SHA digests must agree; inode/stat differences do
not change checkpoint identity. Both original position-zero complete payloads
are read and validated, and their first 8192 bytes must agree.

The third input is the actual current Down2 native complete:

```
E/prefix-down2-clock-tf4-shared-full-currentness-gpu-v228-v1/complete.json
```

Root retained its actual 963187-byte SHA
`00e1af86b1a61894797dc1eeb3202a113d62bd3d11c8069ba6299d63f8875073`.
The library follows its request and `retained_native/observation-0.bin` pins,
requires the actual V7 prefix and Down2 selected images, and compares the first
8192 bytes of that genuine 606976-byte payload. The actual whole payload SHA
is `27f35fd0e06ee8d0eaabf67a01999ab3d9d1bc3d5fa2241a48f56cb46345aaf2`;
its layer-0 slice SHA is
`faa56202578a3d3497bbe779137736439957e473775bd6f677dbce466a9d6979`.
The current native payload contains layer outputs, not intermediate prefix/MLP
stages. No intermediate values are inferred from its final layer output.

Native transport requires exactly these five files (original paths remain in
the records; the caller's manifest maps them to transported bytes):

| Original suffix | Bytes | SHA256 |
| --- | ---: | --- |
| `E/prefix-down2-clock-tf4-shared-full-currentness-gpu-v228-v1/complete.json` | 963187 | `00e1af86b1a61894797dc1eeb3202a113d62bd3d11c8069ba6299d63f8875073` |
| `E/prefix-down2-clock-tf4-inputs-v228-v1/request.json` | 8114 | `a413a006b377e2a4fe9bfc4decd9aebe7e3a933a5ddd154e1953132af1d401ec` |
| `E/prefix-down2-clock-tf4-shared-full-currentness-gpu-v228-v1/native/observation-0.bin` | 606976 | `27f35fd0e06ee8d0eaabf67a01999ab3d9d1bc3d5fa2241a48f56cb46345aaf2` |
| `/home/harmenon/ferric-asrock-42/evidence/finite-two-forward-v223/p224-prompt-v1/prompt.u32le` | 8192 | `2e81cc60562760f972450ef9205a4e764fa3cce6289d85754e7dfbd7de4d4d02` |
| `/home/harmenon/ferric-asrock-42/evidence/finite-two-forward-v223/p224-prompt-v1/prompt-manifest.json` | 21318 | `30d047aafbd7de2b94647154680b0b9d9fb75aae2fd90b081afe0de595a88600` |

The real request has `prompt` wire pins, not an `input_tokens` member. The
reader authenticates the complete original 2048-token bytes and manifest,
checks first-four selection, then joins `checked.structural` mode, tokens,
positions, generations and the position-zero output token to the actual
payload's argmax. No prompt text or native ELF/image bytes are consumed here;
their execution qualification remains the caller's authenticated receipt.

`diagnostics.py` is the byte-identical existing P224 helper, SHA
`38cee89882d13dce81a992a64d4931c52a6f5b0b2dc77f5cddf1fc7cfe37ccaf`.
Use `load_diagnostics(read, pin)` to authenticate its source before loading.
The copied helper's pure metrics are unchanged: exact words, absolute error,
RMSE, relative L2 and BF16 word distance. They define no acceptance threshold.

## Historical Internal Mapping

`historical_views(framework_values, full_capture, physical_page, diagnostics)`
is a separate optional hypothesis-localization step. The caller first reuses
the retained historical V227 replay validator and authenticates its original
token9112 input, old images, 28-array 9670656-byte capture, and physical page
from the actual input record. The complete capture SHA is
`c46324f59a03f3db83e891475261cf527d19b7c1b5a981ce21b7b763e182f52f`.
This does not relabel old image data as current V7/Down2 internal evidence.

Each rank owns contiguous Q rows 2048, K/V rows 512, and MLP rows 6144.
QKV comparison concatenates those three rank slices, not half of the packed
global QKV vector. Query and current K compare rotated tensors. At position
zero, the cache slot begins at `physical_page * 16 * 1024` bytes. The code does
not assume identity pages or the standalone prefix fixture's shuffled page.
Attention maps to the rank's 2048-word contiguous head range. Native
`activation` maps to genuine framework `product`, not its separate `silu`.
Norm/residual/hidden vectors are replicated full4096-word tensors.

FP32 O/down rank-local partials are deliberately excluded: a rank-local partial
is not the framework's narrowed full projection output. No synthetic sum or
residual is substituted into the genuine framework chain. Historical results
are cumulative chain differences, not independently isolated operator errors.

## Actual Source Audit

The installed `modeling_qwen3.py`, 51968 bytes, SHA
`704c914530530a1acb0b443add1f520404e3ac2c28c0ab7e16f80f86cfe8ccb2`,
confirms the hooks and shapes: RMSNorm at lines71-76; separate genuine SiLU and
product at lines93-95; head reshape/norm/transpose at lines214-222; attention
flatten and O projection at lines251-252; actual BF16 residual additions at
lines300 and306. The rotary module returns BF16 cos/sin at line346. At one token
and position zero, flattening the captured Q/K/V head dimension preserves order.

The native V7 fixture `row-reciprocal-checked-probe-v228-v7/fixture/src/lib.rs`
SHA `d5c2ee3ca4f619080a8b544bc4e47b8ae4a55642ba2eb84448c8d32e91056c37`
uses fixed wave reduction plus sqrt/exact reciprocal and dual BF16 narrowing
at lines36-96. The framework uses FP32 `torch.rsqrt` and its own reduction order.
Bitwise agreement is not guaranteed by the source contract. Native RoPE uses
FP32 products before final narrowing; the framework materializes BF16 products.
This reader checks actual position-zero cos1/sin+0 and does not generalize to
later positions. Native SiLU keeps its intermediate in FP32 before multiplying
up, while the framework exposes a BF16 SiLU result first. Capturing this
boundary is useful; no mismatch is hidden by a new fitted tolerance.

The retained standalone `genuine-pos0` V7 prefix case uses token785, not9112.
Its intermediates are not valid same-input comparands here. New current-native
internal evidence requires a genuine token9112 prefix/layer capture, not a
conditional replay or renamed old fixture.

All results remain diagnostic, with no numerical/full-model acceptance,
performance, calibration or production claim. Equality of the new and old
framework layer0 output is reported before any native comparison is interpreted.
