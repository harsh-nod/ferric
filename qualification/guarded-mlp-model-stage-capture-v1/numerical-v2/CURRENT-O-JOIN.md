# Current Capture and Historical O Replay

An independent data-only audit joined the actual guarded layer-zero capture to
the already executed historical exact O-projection replay. All selected bytes
match. This audit performs no new model dot products, fixed-order simulation,
framework execution, or GPU work.

| Joined Values | Exact Matches | Total |
| --- | ---: | ---: |
| Both BF16 attention shards versus historical capture | 4,096 | 4,096 |
| Both FP32 O partials versus historical capture and fixed-order report | 8,192 | 8,192 |
| Both BF16 first residuals versus historical capture and conditional replay | 8,192 | 8,192 |
| Both directly captured embeddings versus replay operands | 8,192 | 8,192 |

The current terminal is `324b40f7`, its capture envelope is `f28715ef`, and its
payload is `d0dce4f4`. The historical capture archive is `1403889c`, whose
9,670,656-byte capture is `3967969a`. The historical all-row replay is
`869541d4` after decompression. [The join report](current-o-join.json) records full hashes,
source paths, exact contained-body identities, per-rank slice offsets and
hashes, counts, report-field joins and nonclaims.

Both attention shards and both FP32 O partials are byte-identical to the
historical operands and outputs. Each of the 8,192 partial encodings also
equals the old report's `native_f32` and `replay_f32` field. Both first residuals
are byte-identical to their historical counterparts; all 8,192 words equal
the old native, conditional-boundary and replayed residual fields.

The old capture did not retain its embedding. Its residual replay therefore
used the genuine framework embedding conditionally. The current capture does
retain both input embeddings: each is exactly the same 8,192-byte framework
embedding (`68b2cf48`), and every word equals the old report's 4,096 embedding
operands. This supplies the previously unobserved operand for the **current
captured boundary**. It does not retroactively add an embedding observation to
the historical capture.

Consequently, the already reported projection/reduction and residual-boundary
result now has exact matching attention, FP32 partial, embedding and residual
data for this current layer/position/token. The current first-residual
differences at rows 1024 and 3444 agree exactly with the old replay. The BF16
materialized O projection remains derived from partials, not directly captured.
This does not infer the framework's accumulation tree, independently reprove
the current kernel's instruction order, or establish behavior on other inputs.

The audit reauthenticated all 90 current capsule pins and all 34 current and
28 historical capture-part hashes. It parsed the authenticated historical
replay and compared bytes/encodings with standalone standard-library code,
without importing the diagnostic implementation or rereading model weights.
No acceptance threshold, full-model acceptance, performance result or
production authority is introduced. Original evidence remains unchanged.
