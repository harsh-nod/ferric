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

The next gates remain an independent framework comparison, full bounded request execution,
independent numerical acceptance, and then sustained matched-workload timing.
All issue #42 M0-M7 milestones remain open.
