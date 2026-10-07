# Position 5: Exact Two-Row Head Diagnostic

The observed native/reference token disagreement remains: native selected 9112,
while the reference selected 2. This bounded diagnostic explains the final
rounding boundary for those two tokens. It does not establish full-model
numerical correctness or remove the earlier 39/40 token-match limitation.

The [original CPU receipt](output/complete.json) passed all
[12 synthetic tests](output/tests.stderr), with no error or postcheck error.
Its 15,728 original bytes have SHA-256
`fb88929815dbfdec7a66cfb87aaa8ae7fe5567c5e059d77d431c8e28053c2597`.
The [source README](README.md), [source manifest](source-manifest.json),
and original input bodies are retained unchanged.

## Four Exact Dots

Each row below is a 4,096-term exact-real dot product of original BF16
checkpoint weights and one captured BF16 final-normalized input. Integer
products and sums use units of `2^-266`; no accumulation tree is approximated.
The same two original LM-head rows are used on both inputs.

| Captured input | Token | Exact-real dot | Nearest BF16, ties to even | Observed BF16 |
| --- | ---: | ---: | ---: | ---: |
| Native | 2 | 19.68111478225910104811191558837890625 | 19.625 | 19.625 |
| Native | 9112 | 19.742494957710732705891132354736328125 | 19.75 | 19.75 |
| Reference | 2 | 19.6147026113976608030498027801513671875 | 19.625 | 19.625 |
| Reference | 9112 | 19.6302838144474662840366363525390625 | 19.625 | 19.625 |

All four observed values equal exact-dot round-to-nearest-even on their own
input. The reference's two exact dots are unequal, with 9112 larger, but both
round to 19.625. The resulting BF16 tie selects the lower token ID, 2. On the
native input, token 9112 crosses the 19.6875 midpoint between 19.625 and 19.75;
token 2 stays below that midpoint.

| Captured input | Exact margin, token 2 minus token 9112 | Token 9112 minus the 19.6875 midpoint |
| --- | ---: | ---: |
| Native | -0.061380175451631657779216766357421875 | 0.054994957710732705891132354736328125 |
| Reference | -0.0155812030498054809868335723876953125 | -0.0572161855525337159633636474609375 |

For token 2, the midpoint distances are
`-0.00638521774089895188808441162109375` on the native input and
`-0.0727973886023391969501972198486328125` on the reference input.
Changing only the captured input changes the exact two-row margin by
`-0.0457989724018261767923831939697265625`.

## Interpretation and Limits

This is evidence against incorrect final rounding as the explanation for these
four observed values. The head inputs already differ: their respective 8,192-byte
SHA-256 values are
`234c448c444debeefa286567f35abdc99ad6b54c27135a3f2a22126366db65e9`
and `bd72437a0771ce8b97b0cf9dd9205aeb8a74d4317211ad04825400a532b128b6`.
With authenticated common head weights, that input difference suffices to
explain the change in this two-token rounding outcome.

The diagnostic does **not** identify the upstream operation responsible for
those different inputs. It does not emulate MFMA or framework accumulation,
verify every vocabulary row, prove the internal head accumulation exact, or
infer a framework reduction tree. No tolerance or numerical acceptance rule is
introduced. The [earlier position-5 comparison](../comparison-v1/output/complete.json)
and [native evidence](../gpu-v1/manifest.json) retain their original scope.

The original controller streamed and authenticated the full 1,244,659,840-byte
checkpoint shard before and after the calculation, read both 8,192-byte rows
from the same stable file descriptor, and joined the original model index and
selected native/reference payloads. Retention preserves that receipt and its
metadata; it does not reread or duplicate the checkpoint, reauthenticate the
GPU-resident weight upload, or rerun the model. This was CPU-only data analysis,
not a GPU execution, throughput measurement, or performance claim.
