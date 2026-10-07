# Position-5 Numerical Investigation

The [original 40-position comparison](../guarded-mlp-readiness40-v1/README.md#independent-framework-comparison)
found one argmax disagreement: at prompt position 5, reference token 2 versus
Ferric token 9112. Neither original run retained that position's full logits.
This diagnostic changes the four selected captures from `[0,15,16,39]` to
`[0,5,16,39]`, keeping all 40 authentic prompt inputs, all 36 layers, independent
KV histories, zero generated tokens, arithmetic and resource limits unchanged.

## Tested Native Route

Both executables compiled and passed their CPU gates on `mi350`:

| Gate | Actual Result | Scope |
|---|---|---|
| Worker | 663 passed, four unchanged ignores | Full ordinary suite, nine clean phases; includes executable entry tests |
| Parent | 433 passed | 51 selected scopes and 60 clean phases; not the entire 909-test inventory |
| Native evidence checker | 17 passed | Synthetic framing, capture, profile and lifecycle checks |
| Position-5 comparator | Seven passed | Synthetic history parity, BF16 ties and diagnostic calculations |

Original evidence: [worker](worker-cpu-v1/evidence/complete.json),
[parent](parent-cpu-v1/evidence/complete.json),
[checker](checker-cpu-v1/evidence/complete.json) and
[comparator](diagnostic-cpu-v1/evidence/complete.json).
Every previous selected test outcome is preserved. The worker adds five
passing cases; the parent adds five selected cases.

The ten worker and four parent replacements are integrated. All 195 worker
source bodies and all 1,237 composed Ferric bodies match their tested maps.
The [worker](worker-source-integration.json) and
[parent](parent-source-integration.json) integration records distinguish the
worker's seven actual formatting changes from the parent's preformat shared
inputs. No dependency lock, runtime policy or kernel image is changed.

The original readiness entry remains unchanged. The explicit parent flag
`--observe-guarded-readiness40-position5` requires its own request and worker
profile; cross-profile input is rejected before native setup.

The [native run](gpu-v1/readiness/complete.json) now passes on `mi350`: one
attempt, 40 full-model forwards, all 11 supervised phases, healthy Close and
clean owned-process/device postchecks. The qualified checker revalidates its
four original captures. Both banks reach generation 20; allocation counts
remain bounded at 787/783 pages at Close. No generated tokens are requested.
The [native manifest](gpu-v1/manifest.json) retains 81 original bodies,
including all 70 raw evidence files, without changing the original receipt.

## Independent Reference

The new reference completed on SSH host `mi350` (`gfx950`). Its
[owner receipt](reference-v1/complete.json) records 20 passing pure tests,
17 clean supervised phases, six all-eight-GPU idle observations and removal
of the task-owned container. The [framework receipt](reference-v1/output/complete.json)
records two fresh-cache passes, totaling 80 full-model forwards. All 40 records
and eight captured payloads repeat exactly between these two passes.

The original reference is retained separately. Neither reference consumes
Ferric intermediate tensors. Model, prompt, BF16 parameters, FP32 RoPE buffers,
math-only SDPA, implementation sources and package versions remain unchanged.

The [manifest](reference-v1/manifest.json) preserves 122 original files plus
the data-only retention tool. Its verification rechecks the original receipts,
raw process lifecycle, selected payload hashes and exact repeat gate. Model
shards and installed package bodies are not copied into this capsule.

## Captured Result

The [authenticated comparison](comparison-v1/output/complete.json) passes
on MI350. It rechecks seven original evidence archives, both native lifecycle
admissions, both independent reference repeat gates and the tested diagnostic
functions. All 40 input/output/logit records match each implementation's
original run; common captures at positions 0, 16 and 39 are byte-identical.
Changing capture selection therefore did not change those recorded results.

At position 5, input token 271, the top two logits are:

| Token | Independent BF16 Reference | Ferric BF16 | Difference |
|---|---:|---:|---:|
| 2 | 19.625 | 19.625 | 0 |
| 9112 | 19.625 | 19.75 | +0.125 |

The reference ties these tokens and selects the lower token ID, 2. Ferric
places token 9112 one BF16 representable step higher. This explains the argmax
switch, not the arithmetic cause of the difference and not its acceptability.

| Position-5 Tensor | Exact BF16 Words | Relative L2 | Maximum Absolute Difference |
|---|---:|---:|---:|
| Layer 0 hidden | 4,082 / 4,096 | 0.008694% | 0.00048828125 |
| Layer 35 hidden | 586 / 4,096 | 0.627841% | 8.0 |
| Final norm | 562 / 4,096 | 1.021266% | 1.5 |
| Logits | 25,042 / 151,936 | 0.777883% | 0.1875 |

All 38 tensor comparisons and exact integer BF16 top-two diagnostics are in
the original comparison receipt. The first nonidentical captured tensor is
layer 0's output. This alone cannot locate the first differing operation or
distinguish an implementation bug from floating-point association effects.
The [comparison manifest](comparison-v1/manifest.json) retains the original
receipt and source bodies; all 17 recorded runtime inputs were rehashed.

## Remaining Gates

Next, isolate arithmetic differences at operation boundaries, establish
independent numerical acceptance, execute the full bounded request and then
measure sustained matched-workload performance. The observed tie does not
justify changing argmax, rounding away a discrepancy or inventing a tolerance.

This is not numerical acceptance, generated-token agreement, a throughput
benchmark or the full 2,048-token-prompt/256-generated-token workload. No
tolerance is inferred from this diagnostic. All issue #42 milestones remain
open, and the 700 tokens/s target is not demonstrated.
