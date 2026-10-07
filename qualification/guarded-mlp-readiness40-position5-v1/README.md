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
profile; cross-profile input is rejected before native setup. The native run
is pending and is not implied by the CPU results.

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

## Remaining Gates

Native position-5 capture and the cross-run comparison are still pending.
Before diagnosing the discrepancy, the comparison must confirm that all 40
input/output/logit records match each implementation's original run and that
the common captured payloads at positions 0, 16 and 39 are byte-identical.
Only then can the new position-5 tensors explain the logit margin and localize
where differences first appear.

This is not numerical acceptance, generated-token agreement, a throughput
benchmark or the full 2,048-token-prompt/256-generated-token workload. No
tolerance is inferred from this diagnostic. All issue #42 milestones remain
open, and the 700 tokens/s target is not demonstrated.
