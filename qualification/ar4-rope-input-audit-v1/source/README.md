# Actual AR4 RoPE Input Audit

Source-only data replay, with eleven authored synthetic tests. Neither file has
been imported or executed by its author. Root owns qualification and the actual
retained-data run. This helper loads no model, generates no trigonometric table,
opens no GPU, and imports no earlier tested controller.

An earlier read-only agent diagnostic found 512/512 BF16 coefficient-boundary
matches across the four actual AR4 requests. That calculation is not a
root-qualified result. The helper independently derives the counts from pinned
input words; it does not hard-code equality as success. A difference is retained
in the output rather than converted into acceptance or hidden by a tolerance.

## Exact Inputs

All paths below are relative to
`E=/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`.
The complete original paths, extents and SHA256 values are enforced by `FIXED`.

| Relative Path | Bytes | SHA256 |
| --- | ---: | --- |
| `rope-framework-reference-v228-v1/complete.json` | 330074 | `f5c47aabf7a307a19f9fe54fa37187f8548ceab4ac92eeeac2638fc9cc4589b4` |
| `projection-ar4-framework-reference-v228-v1/reference.json` | 201239 | `00952244362ad51d241d179f741ae5ae61ff8acfcb3fd160b9dc64ce3b5f699e` |
| `projection-ar4-framework-inputs-v228-v1/native-case/complete.json` | 1054985 | `15938580d218f855883a589c819d532bbf941a02c4677cb29928f7bf7106d1cb` |
| `projection-ar4-framework-inputs-v228-v1/native/request-0.json` | 1865 | `dfdfef97a872fcb7622998489f219b61cff5a9d4dbe4c27c1955b7ab0c6066ce` |
| `projection-ar4-framework-inputs-v228-v1/native/request-1.json` | 2400 | `04cd581b9b4c7b75f1bb988e0f6c9f8e5bff828b412347ebbdfad9d665429c00` |
| `projection-ar4-framework-inputs-v228-v1/native/request-2.json` | 2404 | `bba7e200de72e40866022407e2781769a5c48b75b0021e0947bdf850386e75c4` |
| `projection-ar4-framework-inputs-v228-v1/native/request-3.json` | 2405 | `0acf40630457e988af9f6e003def15f8f5c3562756616deaa03cac0964b3533e` |

The RoPE complete body is also retained byte-exact at
`F/qualification/rope-framework-reference-v1/complete.json`. The other six
bodies retain those relative paths under the root-exported Windows archive
directory `W/projection-ar4-framework-evidence-v228-v1`, where
`W=/mnt/c/Users/harmenon/ferric-session-evidence/20261004`.
The reference has an additional flat retained copy at
`L/projection-ar4-framework-reference-v228-v1.json`.
These local copies are transport sources, not rewritten input identities.

## Computation and Scope

For positions 0, 1, 2 and 3, the actual request contains 64 cosine FP32 words
followed by 64 sine FP32 words. The retained framework tables have two equal
halves per coefficient vector. The helper checks these layouts and the actual
request ID/generation/position, original-to-retained request pins, native token
metadata and the previously recorded reference/native structural join.

Each finite FP32 word is narrowed by integer round-to-nearest-even. Signed zero
is preserved; nonfinite inputs and overflow to infinity are refused. Exact
BF16 counts and differing indices are reported, plus separately labeled raw
FP32 comparisons against the retained libm model and framework companion.
No floating-point trigonometry is recomputed. Input and source bodies are
rehashed before and after replay. `completed` means this data replay completed;
`all_bf16_coefficients_equal` reports the measured comparison independently.

Position-zero residual differences remain. Four-position coefficient equality
does not explain every error source, validate a new RoPE image, replay native or
framework ownership, establish GPU trigonometric behavior, or cover actual
positions 2047/2048/2303. It grants no numerical acceptance, full-model,
production, performance, or sustained 2048-prompt/256-decode claim.

## Root Invocation

Transport only `run.py` and `test_run.py` into
`E/p228-ar4-rope-input-audit-v1`; the seven inputs above must already be present
at their exact original paths. Tests are a separate eleven-case unittest run
under root's existing CPU-only limits and retained primary observation. They
do not produce or imply a native supervisor receipt.

The actual data-only command is:

```sh
env -u PYTHONPATH -u PYTHONHOME -u PYTHONOPTIMIZE \
  HIP_VISIBLE_DEVICES= ROCR_VISIBLE_DEVICES= CUDA_VISIBLE_DEVICES= \
  timeout 150s taskset -c 8,9 nice -n 10 python3 -B \
  "$E/p228-ar4-rope-input-audit-v1/run.py" \
  ar4-rope-input-audit-v228-v1 RUN_SHA256 TEST_SHA256
```

Pass the actual frozen source digests, not placeholders. The script requires
ASROCK UID9661, CPU8/9, nice10, hidden GPUs and ordinary unoptimized Python `-B`;
it sets 2 GiB address-space, 120 CPU-second and 16 MiB file-size limits without
raising stricter hard limits. Every input is at most 2 MiB. It refuses an
existing output directory and writes only `E/LABEL/complete.json`, then rehashes
the written result. No subprocess or remote command is started by the helper.

Synthetic coverage: signed zero and finite exact words; signed ties-to-even;
subnormal boundaries; invalid/nonfinite/overflow words; four-table splitting;
raw differences that disappear at BF16; material differences that remain;
extent/half-duplication refusal; missing/duplicate/incorrect positions;
generation and boolean-identity refusal; duplicate/nonfinite JSON refusal.
