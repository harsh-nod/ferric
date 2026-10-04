# SiLU Four-Step Native Decode

The checked BF16 SiLU image completed four full 36-layer forwards on MI350.
One attempt retained all 152 tensor slices, four payloads and 576 terminal
state records. Seven process leaves and six exact-empty device audits passed;
the parent and worker exited naturally with Close/reap checks satisfied, no
retry and no forced cleanup.

This is an engineering execution and structural checkpoint. Inputs were
`[9112, 2190, 3772, 220]` at positions 0-3 in teacher-forced mode; output tokens
were `[67, 198, 25, 16]`. It is not autoregressive or sustained 2,048/256
validation, and matching output tokens alone would not establish numerical
correctness. The [independent comparison](../silu-materialized-decode-comparison-v1/README.md)
records all tensor differences separately.

## Selected Implementation

The request uses the same CPU1022-qualified parent and worker as the previous
corrected-residual decode. Only `decode.tiles_image` selects the
[checked SiLU image](../silu-materialized-lowering-v1/README.md), plus fresh
session/output identifiers. Original bootstrap images, V7 prefix, corrected
projection-residual image, BF16 precision, four inputs and scheduling remain
unchanged. No Rust executable rebuild or compiler bypass is introduced.

The candidate adds BF16 rounding between SiLU and the up product. Its exponential
arithmetic and two-row Down implementation are unchanged. The separate
[layer-zero comparison](../silu-materialized-comparison-v1/README.md) preceded
this attempt, and the [43-test supervisor suite](../silu-materialized-decode-supervisor-v1/README.md)
passed on MI350 before launch.

## Retained Evidence

[complete.json](complete.json), SHA-256
`ad977bca8c02e329ce90d15cc61961a9b82e35ce2488c3a8b07277d1c5414833`,
records the actual run. [result.json](result.json) binds published source,
input assembly, root review, pure-test evidence, native JSON, seven owned
process records and six device snapshots. The publisher rehashes the actual
native bodies and replays the unchanged structural validator and ownership
checks. Tensor/control buffers, ELF binaries and HSACO remain outside Git,
identified by exact pins.

The parent was PID 706975 and the worker PID 714038. Their recorded identities
and process groups were checked without synthesizing missing child identity.
The outer wall time was 392.982 seconds, including model loading, validation
and capture. This is not GPU kernel time, inter-token latency or tokens/s.

Publication does not rerun the controller or runtime audits, rehash local
dynamic-library bodies, or replay every transitive admission input. Full-model
numerical acceptance, production admission and the 700 tokens/s target remain
open.
