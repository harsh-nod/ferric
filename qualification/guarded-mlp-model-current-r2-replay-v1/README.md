# Current Guarded R2 Boundary Replay

The data-only replay on MI350 passes all 20 tests, all 8,192 captured native
final-output encodings and all 4,096 separate framework boundary-control
encodings. It checks layer zero, position zero, input token `9112`, over
4,096 hidden-state rows on each of two ranks. No GPU or framework job was
rerun, and no new numerical threshold was introduced.

## Arithmetic Checked

The replay uses each rank's actual captured dedicated Down FP32 partial and
first-residual BF16 operand. For each row, the unchanged integer reference
performs FP32 RNE `+0 + rank0`, FP32 RNE addition of rank one, BF16 RNE
materialization, FP32 addition of the current rank's residual, then BF16 RNE
narrowing. The partials are not individually narrowed. Every intermediate
and output encoding is retained in [replay.json](attempt-v2/evidence/replay.json).

| Check | Exact Matches | Scope |
| --- | ---: | --- |
| Native final hidden, rank zero | 4096 / 4096 | Actual native operands |
| Native final hidden, rank one | 4096 / 4096 | Actual native operands |
| Framework residual control | 4096 / 4096 | Separate genuine framework operands |

The derived combined Down and replayed final vectors are saved separately;
they are not relabeled as directly captured tensors. The framework control
uses its own already captured materialized Down, residual and final hidden.
It does not substitute framework operands into the native replay.

## Evidence

The [actual successful receipt](attempt-v2/evidence/complete.json) is 8,564
bytes, SHA-256
`12c18ae554980536fae43dbf9d09a4e306844874e3d554c6377ea46ec27f1188`.
It records 20 passing tests, 24 clean input posthashes and 0.951 seconds of
data-only execution. That duration is not a kernel performance measurement.
The [raw test log](attempt-v2/evidence/tests.stderr), seven original output
files, source, two qualified Rust contract bodies and all fourteen original
data bodies are retained. [The input map](attempt-v2/retained-inputs.json)
preserves original paths and records the retained copies without rewriting
any historical receipt or capture.

The parser rechecks all 34 capture parts, same-run identities, offsets,
hashes, finite scalars, metadata, rotary data, healthy Close and the first
observation join. The existing independently checked numerical report and
qualified source map authenticate the selected historical inputs. The full
original GPU/process/library audit is not repeated by this data tool.

## Preserved Test Failure

[Attempt one](attempt-v1/evidence/failed.json) remains failed: nineteen tests
passed, but one negative fixture set an already-BF16 Up part's scalar to
BF16 and incorrectly expected rejection. It stopped before reading the real
numerical inputs. Attempt two changes that fixture to `f32`, names each
mutation and asserts that its serialized bytes actually change. The only
runner changes are the test hash and fresh output namespace. Arithmetic,
capture admission, all twenty test names and real input pins are unchanged.

## Limits And Next Gate

This establishes agreement with the source-contract residual arithmetic on
these captured operands. It does not independently prove machine instruction
ordering, recompute either Down dot product, validate every layer/position,
or establish full-model numerical acceptance. The earlier native/framework
differences remain recorded.

The next diagnostic is a matched-input framework MLP evaluation using the
captured native post-normalization vector, with repeated genuine framework
controls. That can distinguish inherited input differences from local MLP
arithmetic. Sustained 2,048/256 decode, GPU overlap, the equal-work vLLM
comparison, 700 tokens/s and all issue #42 milestones remain open.
