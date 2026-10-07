# Bounded 40-Position Readiness

This is an engineering checkpoint for [issue #42](https://github.com/harsh-nod/ferric/issues/42),
not production admission or the full Qwen3-8B 2,048/256 workload. No throughput,
independent model numerical acceptance, or 700 tokens/s claim is made.

## Purpose

The explicit readiness route accepts the authentic 2,048-token prompt and the
full 144-page KV mapping, but executes only the first 40 prompt positions. Each
position traverses all 36 layers and both ranks. It generates zero tokens.
This crosses the former forward-38 allocation limit using the qualified
two-bank reusable owner, without raising allocation or stream limits.

The route retains conservative full-currentness checks. It does not select the
separate shared-currentness or paired-read timing experiments. Existing AR4
results are not transferred into numerical or performance claims for this route.

The compact transcript records all 40 transitions. Complete control/payload
captures are selected at positions 0, 15, 16 and 39; the other 36 payloads are
validated by the parent but are not retained for independent byte inspection.
Healthy native Close and process retirement must precede successful publication.
The full 2,303-forward native route remains explicitly refused.

## Actual CPU Results

Both suites ran on SSH host `mi350`, actual host
`smci350-rck-g03-b19-03`, with hidden GPU visibility, private build/cache
directories, two Cargo jobs, and bounded owned-process supervision.

| Gate | Actual Result | Scope |
|---|---|---|
| Worker | 658 passes, four unchanged ignores, nine clean phases | Complete ordinary worker suite; includes real-executable readiness entry |
| Parent | 428 passes, 60 clean phases | 51 selected scopes, not all 905 library inventory tests |
| Independent data checker | 16 passes | Synthetic transcript, capture, framing, and process-lineage tests only |

The worker preserves all 627 previous named outcomes and adds 35 passing tests.
Its actual executable is 6,102,600 bytes, SHA-256
`583030e3cc02b389b3924a34d03e1a52f00638243d45bd2cf24db3bce7b7076c`.
The new parent executable is 13,582,216 bytes, SHA-256
`3ea6325efb597d422635bd6fdc38e0b88333ef8a7b427ada7ae68946da37c1c1`.
Neither result implies GPU execution.

The parent retains its historical locked Git dependency graph; it is not
relabeled as using the standalone worker's newer runtime dependency.
Both source maps, raw test outcomes, formatter transitions, and actual Cargo
artifact records are retained separately. Binary and external dependency
bodies are not bundled; their original hashes are preserved.

Original capsules: [worker](worker-cpu-v1/evidence/complete.json),
[parent](parent-cpu-v1/evidence/complete.json), and
[checker](checker-cpu-v1/evidence/complete.json). Each directory includes its
original controller, raw outcomes, and unchanged terminal receipt.
The tested source is now integrated: all 195 worker bodies and all 1,237
composed Ferric bodies join the actual qualification maps. Composition uses
the worker's 16 separately tested formatter postimages in place of the
parent's unformatted copies. The
[worker](worker-source-integration.json) and
[parent](parent-source-integration.json) integration maps record every
preimage, tested postimage, and substitution. No runtime, lockfile, or kernel
image changes are part of this integration.

## Native Gate

The first native attempt completed all 40 positions on `mi350`, with all 36
layers and both ranks at each position. All 11 supervised phases passed;
the original parent and worker closed naturally, and all six before/after
observations found all eight GPUs idle. No retry or allocation-limit increase
was needed. The qualified checker revalidated the retained original evidence.

| Captured Position | Prompt Token | Bank-Local Generation | Observed Argmax |
|---|---|---|---|
| 0 | 9112 | 1 | 67 |
| 15 | 5269 | 8 | 374 |
| 16 | 13352 | 9 | 389 |
| 39 | 9104 | 20 | 11 |

All 40 compact transitions pass the chained-history, page-map, bank-generation,
queue-progression, framing and healthy-Close checks. The four selected
849,800-byte captures each contain the original 242,824-byte control body and
606,976-byte BF16 payload. Their finite values and lowest-index argmax pass
independent data checks. This does not independently compute model outputs or
inspect the 36 unretained payloads. The argmax values above are observations,
not an assertion of agreement with a framework reference.

Close also checks the actual reusable-allocation ceiling of 787/783 across the
two ranks. The transcript is not a 40-sample allocation census, so this result
must not be plotted as one. The conservative currentness policy is unchanged.

The [original terminal](gpu-v1/readiness/complete.json),
[checked observation](gpu-v1/readiness/observation.json), and
[closed evidence manifest](gpu-v1/manifest.json) preserve all 70 raw bodies,
source and preparation inputs, and original process identities. The terminal
is 169,839 bytes, SHA-256
`65c85efb646a6980be5a036220e5cfc70acd811b656dc5c821c7f67eae3dd131`.
Its 949.65-second outer elapsed time includes setup and checks; it is not
GPU timing or a decode-throughput measurement.

## Independent Framework Comparison

The [independent owner receipt](reference-v1/complete.json) now passes on MI350,
including 20 pure validation tests, 17 naturally completed supervised phases,
six all-eight-GPU idle observations and removal of the task-owned container.
The [framework receipt](reference-v1/output/complete.json) records two fresh-KV
passes through all 36 layers at each of the first 40 authentic prompt positions:
80 full-model forwards, zero generated tokens. It never consumes Ferric
intermediate tensors. All 40 records and the eight selected reference payloads
repeat exactly between passes.

The reference uses the original Qwen3-8B checkpoint, BF16 parameters, preserved
FP32 RoPE buffers, deterministic inference and math-only SDPA. It authenticates
and retokenizes all 2,048 prompt IDs before executing. Its immutable image and
package overlay, implementation sources, loader record and raw lifecycle
evidence are preserved in the [reference manifest](reference-v1/manifest.json).

The [data-only comparison](comparison-v2/output/complete.json) runs on MI350,
passes eight focused tests, revalidates both original runs and the qualified
16-test native checker, and rejoins all prompt, model, bundle and history IDs.
It compares 152 selected tensors and all 40 reported argmax values.

**The numerical gate remains open:** 39 of 40 observed argmax values agree.
At position 5, with input token 271, the reference reports token 2 and Ferric
reports token 9112. Neither run retained position-5 logits or hidden tensors,
so the difference's margin and cause are unknown. These are teacher-forced
prompt predictions, not 40 generated tokens or a model-accuracy benchmark.

| Captured Position | Reference / Ferric Argmax | Logits Relative L2 | Maximum Absolute Logit Difference |
|---|---|---|---|
| 0 | 67 / 67 | 0.3045% | 0.0625 |
| 15 | 374 / 374 | 0.6476% | 0.25 |
| 16 | 389 / 389 | 0.6887% | 0.125 |
| 39 | 11 / 11 | 0.6561% | 0.1875 |

Only one of the 152 selected tensors is bit-identical: position 15's layer-0
hidden state. Both argmax values are recomputed from retained logits at the
four captured positions; elsewhere they are authenticated original records.
No acceptance tolerance is inferred from these errors or token agreement.

The [layer-by-layer plot and raw tables](numerical-report-v1/README.md)
show all 152 tensor comparisons and all 40 argmax records. The renderer passed
ten tests on MI350; the original metric values are preserved, with no new
acceptance or performance claim.

The next diagnostic preserves the full 40-position schedule, arithmetic and
limits, but explicitly captures position 5 in a separately named profile.
Common-position payloads and all 40 input/logit records must first confirm
that changing capture selection did not change execution results.
The [position-5 reference](../guarded-mlp-readiness40-position5-v1/README.md)
has now completed two repeatable fresh-cache passes; its matching native
capture and cross-run numerical diagnosis are still pending.

The next gates remain resolving the numerical discrepancy, full bounded
request execution, independent numerical acceptance, and then sustained
matched-workload timing. All issue #42 M0-M7 milestones remain open.
