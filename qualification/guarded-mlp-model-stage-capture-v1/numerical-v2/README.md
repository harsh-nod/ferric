# Guarded Layer-Zero Numerical Diagnostic

The actual MI350 data-only comparison completed successfully. This is a numerical
diagnostic, not numerical acceptance, full-model acceptance, or a performance
result. It introduces no threshold and performs no GPU or framework rerun.

The [original report](complete.json) is 176,352 bytes with SHA-256
`0a8905b7bf2990cff03a8113b17a70c53d08fd1a80f00cfcdc2e1360d98c0b0c`.
All eight in-process regression tests passed; 200 logical input pins were
posthashed, including 73 original framework bodies accessed through explicit
physical aliases. There were no execution or postcheck errors. The report's
acceptance, performance, production-authority, and native-rerun flags remain false.

## Comparison Scope

This compares generation 1, position 0, layer 0, input token 9112. Both directly
captured rank embeddings exactly equal the genuine framework embedding. The
framework executed its own complete chain twice; all 33 stage bodies agree
between those two passes. The candidate is the actual four-forward guarded AR4
capture, with observed outputs `67, 25, 576, 2701` and a healthy Close.

The candidate terminal is 97,034 bytes / `324b40f7dcee931c856aec8b353bd35176e02570290aca5376c0075cfdf3be12`.
Its child-stderr envelope is 911,296 bytes / `f28715ef219057de6d425f4de7f090d7f5f79e6867be4b8197bf3c80f7b8ded4`.
The enclosed 34-part capture is 256,136 bytes / `d0dce4f4632717fbf1fefbcca85e2d0f83ff4728951559accd1de232c35c938f`.
Ordinary model admission and the reviewed additional capture checker were
replayed before comparison. The first request, actual physical cache-page
offsets, metadata, rotary bytes, all part hashes, and both final-hidden parts
were joined to the same native attempt.

The table shows complete BF16 stage/rank rows. Relative L2 is shown as a
percentage; the original report stores the unscaled ratio. Values below are
rounded for display, while the original JSON retains all computed values.

| Stage | Rank | Exact Words | Relative L2 (%) | Max Absolute Error | Max BF16 Steps |
| --- | ---: | ---: | ---: | ---: | ---: |
| Input | 0 | 4096/4096 | 0 | 0 | 0 |
| Input | 1 | 4096/4096 | 0 | 0 | 0 |
| Input normalized | 0 | 4096/4096 | 0 | 0 | 0 |
| Input normalized | 1 | 4096/4096 | 0 | 0 | 0 |
| Raw QKV | 0 | 3071/3072 | 0.0000195960 | 4.76837e-7 | 1 |
| Raw QKV | 1 | 3072/3072 | 0 | 0 | 0 |
| Query | 0 | 2047/2048 | 0.0000217379 | 1.52588e-5 | 1 |
| Query | 1 | 2048/2048 | 0 | 0 | 0 |
| Current key | 0 | 512/512 | 0 | 0 | 0 |
| Current key | 1 | 512/512 | 0 | 0 | 0 |
| Current value | 0 | 512/512 | 0 | 0 | 0 |
| Current value | 1 | 512/512 | 0 | 0 | 0 |
| Attention | 0 | 2048/2048 | 0 | 0 | 0 |
| Attention | 1 | 2048/2048 | 0 | 0 | 0 |
| First residual | 0 | 4094/4096 | 0.00543881 | 0.000244141 | 1 |
| First residual | 1 | 4094/4096 | 0.00543881 | 0.000244141 | 1 |
| Post normalized | 0 | 4094/4096 | 0.00757015 | 0.000488281 | 2 |
| Post normalized | 1 | 4094/4096 | 0.00757015 | 0.000488281 | 2 |
| Gate | 0 | 5911/6144 | 0.0401032 | 0.001953125 | 128 |
| Gate | 1 | 5958/6144 | 0.0364174 | 0.001953125 | 28137 |
| Up | 0 | 5927/6144 | 0.0396060 | 0.001953125 | 151 |
| Up | 1 | 5904/6144 | 0.0450010 | 0.00390625 | 50 |
| Activation product | 0 | 5766/6144 | 0.0434769 | 0.0009765625 | 149 |
| Activation product | 1 | 5777/6144 | 0.0221035 | 0.00048828125 | 27420 |
| Final hidden | 0 | 3807/4096 | 0.0591548 | 0.00390625 | 128 |
| Final hidden | 1 | 3807/4096 | 0.0591548 | 0.00390625 | 128 |

Twelve of the 26 complete rows are byte-exact. The six QKV component rows are
subdivisions of the two raw-QKV rows, not six additional independent stages:

| QKV Component | Rank | Exact Words | Relative L2 (%) | Max Absolute Error | Max BF16 Steps |
| --- | ---: | ---: | ---: | ---: | ---: |
| Q projection shard | 0 | 2047/2048 | 0.0000236585 | 4.76837e-7 | 1 |
| K projection shard | 0 | 512/512 | 0 | 0 | 0 |
| V projection shard | 0 | 512/512 | 0 | 0 | 0 |
| Q projection shard | 1 | 2048/2048 | 0 | 0 | 0 |
| K projection shard | 1 | 512/512 | 0 | 0 | 0 |
| V projection shard | 1 | 512/512 | 0 | 0 | 0 |

## Interpretation and Limits

The first observed nonexact stage is rank-0 raw QKV, specifically one Q word
differing by one BF16 representable step. Rank-0 query also differs in one word,
but both ranks' first attention outputs are byte-exact. This ordering does not
establish that the Q difference causes later differences.

Both first-residual ranks differ at only indices 1024 and 3444. At index 1024,
framework `-0.04248046875` becomes candidate `-0.042236328125`; at index 3444,
framework `-0.004119873046875` becomes candidate `-0.00408935546875`.
Post-normalized values differ at the same two indices, so downstream MLP
comparisons do not have identical intermediate inputs. These are cumulative
chain differences, not isolated operator-error measurements.

The large rank-1 gate and activation step counts occur at the same local index
4251. Gate changes from framework `-7.3015689849853516e-6` (BF16 `0xb6f5`) to
candidate `+7.271766662597656e-6` (`0x36f4`): 28,137 ordered BF16 steps, but an
absolute difference of only `1.4573335647583008e-5`. Activation product changes
from `-1.0654330253601074e-6` (`0xb58f`) to `+1.0505318641662598e-6` (`0x358d`):
27,420 steps and absolute difference `2.115964889526367e-6`. The metric counts
representable encodings across zero, where many small magnitudes exist; it is
not an absolute-error or relative-error percentage. Neither maximum-step
location is the maximum-absolute-error location. This explains the metric's
large value without establishing the computational cause of the sign change.

Four captured FP32 rank partials, two O projection and two dedicated guarded
Down projection outputs, are authenticated but excluded from direct comparison
to full BF16 framework outputs. No rank-partial sum, cast, or reduction order is
silently substituted. Isolating O/residual or MLP arithmetic requires a separate
same-intermediate-input replay or an explicitly matched reduction analysis.

The [current-to-historical O byte audit](CURRENT-O-JOIN.md) now joins both
attention shards, all 8,192 FP32 O partials and all 8,192 first-residual words
to the previously executed exact replay. Both directly captured embeddings
also match its operands. This supplies the formerly unobserved embedding for
the current boundary; it does not add a historical observation or perform a
new model-dot computation. MLP differences still require matched-input analysis.

## Evidence and Independent Audit

The [numerical runner](source/run.py), eight helper/test sources, original report, alias map,
root-owned packer, and all 73 original framework bodies are selected for this
retention. GPU inputs are retained separately in `../gpu-attempt-v1`. The 200
input entries are metadata and posthash evidence; this selection does not bundle
all external ELF, shared-library, model, or dependency bodies.

The [alias map](framework/framework-aliases.json) is 35,122 bytes /
`f69fff0120cd8b25cf8350b246631a88e504de387b685c1b7c92f181d1d0bd3f`.
It retains each original logical file pin and records the physical staged pin
separately. Its exact 73 bodies total 651,037 bytes; neither paths within original
receipts nor their contents were rewritten. The new regression cases verify
exact expected pins, no unknown-path fallback, extra-file refusal, posthash
mutation refusal, and symlink refusal in addition to the six inherited adapter
tests.

An independent standard-library data audit rehashed all 90 pinned GPU capsule
bodies, every capture part, and all 66 framework BF16 bodies. It recomputed all
26 primary and six component metrics directly from those bytes, without
importing the diagnostic implementation. Every reported content hash, exact-word
count, maximum absolute error, RMSE, relative L2, and BF16-step count matched
exactly. This verifies the diagnostic calculation, not an acceptance threshold.
