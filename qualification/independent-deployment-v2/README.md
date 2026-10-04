# Independent Deployment Qualification V2

This checkpoint packages and authenticates the new reciprocal V7 image and the
new independent-profile native executable for transport to MI350. It is a
data-only qualification, not a GPU run or numerical acceptance.

Actual results: all 30 deployment policy tests passed on MI350 (0.008 seconds),
and receiver verification passed on `smci350-rck-g03-b19-03` (2.087 seconds).
The receiver rechecked the complete transported evidence, frozen package,
metadata, source and artifact joins, and all 24 package/controller/payload pins.
These are host-side validation durations, not GPU timings.

The [result](result.json) binds the frozen reader/exporter, its actual policy-test
receipt, the seven-file exported deployment, and actual receiver-side
verification. The payload carries both complete source-run evidence archives and
their root-verified qualification summaries. Original build-host paths and
transported paths remain distinct; verification never opens build-host tools,
source directories, or targets on MI350.

The reader checks all nine compiler phases, all six native CPU phases, both
original owners and completion receipts, all selected artifacts, and the full
5,783-file native source roster with its exact three-file overlay. Candidate CPU
and compiler/finalizer prerequisite pins are preserved as prior qualification,
not relabelled as a new SourceV5 or MI350 compiler run.

## Actual Artifacts

| Artifact | Bytes | SHA256 |
| --- | ---: | --- |
| V7 gfx950 HSACO | 53,560 | `4885204c8d510122588549107f42d2bc6f180f48fbc4eddd3bb1260e8d6629c5` |
| Independent native executable | 11,759,152 | `017e0d3a5c79b1c64a4dccdb0251e53c8e7f8c17ab47e92bbe1309b40760c607` |
| Deployment manifest | 4,048 | `fb73435a16d3be7d1069a776270e2901b71d31fa352ae3cec352832bbea058aa` |

V1's exporter rejected its command comparison before creating output: it
substituted integer byte counts into argv, while the qualified compiler
controller uses strings. V2 preserves the original string substitution boundary
and adds regressions using the authentic recipe and all nine actual command
records. No compiler, GPU kernel, proof limit, or authority gate was changed.

## Remaining Work

The eight runtime requirements remain open. Artifact-bound runtime/platform,
ISA, arithmetic, selected-device and lifecycle reviews remain separate gates.
Closed finite independent captures are not numerical correctness; independent
reference comparisons still follow execution. This checkpoint claims no
production launch, full-model acceptance, performance gain, or 700 tokens/s
result. The historical baseline bundle remains a separate, unchanged input.
