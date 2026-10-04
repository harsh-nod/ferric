# Linked RoPE AR4 Native Observation

The newly emitted gfx950 prefix image ran end to end on MI350 through
`ssh mi350`. Four own-output autoregressive forwards completed in one
attempt without retries. All 152 tensor slices, 576 terminal state records,
control chains, Close5 and EOF passed structural validation. All seven owned
process leaves exited naturally and were reaped; all six surrounding
process/topology audits passed. This is a native execution observation,
not independent numerical acceptance or production admission.

## Controlled Change

The existing CPU1037 parent and worker binaries, model and prompt inputs,
device IDs, SiLU image, corrected projection image and initial copy images
were unchanged. The request changed only the selected prefix image, fresh
session and output directory. The image is 54,344-byte SHA-256
`29fd58e7b09fee003ed31660e6201c20a7eb5da4b873d0b4d954993f7018f2f8`.
Its separate retained compiler producer and qualified indexed consumer are
documented in the [checked-emission checkpoint](../rope-indexed-checked-emission-v1/README.md).

| Position | Actual Input | Own Output Argmax |
| ---: | ---: | ---: |
| 0 | 9112 | 67 |
| 1 | 67 | 25 |
| 2 | 25 | 576 |
| 3 | 576 | 2701 |

Each next input is the previous checked output. The supervisor did not
require the old native or framework token sequence. Equal output tokens do
not establish tensor accuracy.

## Evidence And Limits

[result.json](result.json) joins the exact request, runtime and image
provenance, original completion, owned leaves, audit records and structural
validator result. The existing [74-test supervisor qualification](../rope-indexed-ar4-supervisor-v1/README.md)
and prior CPU1037 runtime reviews are referenced without claiming that their
tests or runtime inspection tools were rerun by publication.

The controller took 391.571 seconds including setup, transfers, control and
audits. This is not device time or sustained throughput. Binary tensor
captures are retained privately outside Git; published hashes identify them.
No weight or executable bodies are added by this checkpoint.

Next is an independent comparison of the four captured forwards and tensor
slices against the genuine repeated framework reference. Numerical
acceptance, the single-request BF16 2,048-token prompt / 256-token decode
workload, and the 700 tokens/s target remain open. All issue #42 M0-M7
acceptance milestones remain open.
