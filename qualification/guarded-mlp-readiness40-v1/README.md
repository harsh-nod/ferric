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

Native execution is pending. A successful attempt must establish all 40
authentic prompt inputs, the fixed page map, bank generations through 20,
queue progression, the exact chained completion transcript, bounded stream
accounting, healthy Close, and clean owned-process/device postchecks.

The independent checker must inspect the four retained control/payload bodies,
finite BF16 values and lowest-index argmax. It does not independently compute
model outputs or inspect the 36 unretained payloads. A separate framework
reference and full-workload numerical gate are still required.

The next gates remain native readiness, full bounded request execution,
independent numerical acceptance, and then sustained matched-workload timing.
All issue #42 M0-M7 milestones remain open.
