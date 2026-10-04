# Four-Step RoPE Coefficient Audit

All 512 coefficients in the four actual autoregressive requests match the
retained framework coefficients after BF16 round-to-nearest-even. Eleven
synthetic audit tests passed separately on ASROCK. No model or GPU execution
was needed for this retained-data audit.

| Position | BF16 Exact Words | Raw FP32 Matches to Framework Companion | Raw FP32 Matches to Retained libm Model |
| --- | ---: | ---: | ---: |
| 0 | 128 / 128 | 128 / 128 | 128 / 128 |
| 1 | 128 / 128 | 101 / 128 | 128 / 128 |
| 2 | 128 / 128 | 91 / 128 | 128 / 128 |
| 3 | 128 / 128 | 94 / 128 | 128 / 128 |

Each request provides 64 cosine and 64 sine FP32 words. The audit narrows
those captured words using integer ties-to-even rounding and checks the
framework's duplicated-half layout. Raw FP32 differences disappear at this
BF16 boundary for these four positions. Equality is a measured result, not
an acceptance condition built into the audit.

The [actual completion](complete.json) retains every count, input identity
and empty mismatch list. The [eleven-test observation](pure-root-observation.json)
is primary-agent SSH output, explicitly not a remote supervisor receipt.
The [publication ledger](result.json) rehashes the seven actual input bodies;
it does not rerun coefficient math or the native/framework process lifecycle.

This rules out a BF16 coefficient-table mismatch at positions 0-3. It does
not validate the new RoPE arithmetic image or explain all tensor differences:
position-zero residual differences remain. Actual long-position tables,
GPU trigonometry, independent model acceptance, sustained 2,048/256 decoding
and the 700-token/s target are not established here.
