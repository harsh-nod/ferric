# Projection-Residual Decode Supervisor Qualification

Two separate CPU-only suites passed on `mi350` for the opt-in four-forward
projection-residual route. Neither suite launched a model executable or GPU
kernel.

| Suite | Passed | Errors / Failures / Skips |
| --- | ---: | ---: |
| Supervisor, structural validator and input admission | 35 | 0 / 0 / 0 |
| Qualified-executable runtime audit selector | 9 | 0 / 0 / 0 |

Both runs used two CPU cores, low priority, hidden GPUs and bounded memory,
CPU time and file sizes. Source snapshots were unchanged. Exact transcripts,
source files and executed wrappers are retained here; [result.json](result.json)
records the identities and separate suite inventories.

## Coverage

The supervisor retains one-attempt execution, three pre-run and three post-run
device/process audits, owned process tracking, natural Close/reaping and failure
retention. Tests cover malformed files, extra files, partial captures, parser
errors, failed audits, changed inputs, forged process identities, resource caps
and forced cleanup.

The new validator checks the wrapped teacher-forced request, separate image
profile, all four request/completion generations, 576 terminal state records,
four 606,976-byte payloads, 152 tensor slices, finite values, lowest-index argmax,
transcript and Close ID 5. It does not require equality with old native output
because the corrected projection boundary intentionally changes arithmetic.

The runtime selector checks the actual new CPU completion, selected Cargo
artifact, source snapshots, binary hash, executable mode and ELF format before
handing the executable to the existing dependency auditor.

## Limits

These are synthetic policy tests, not hardware results. The runtime dependency
audits and actual GPU attempt are separate evidence. Full-model numerical
acceptance, independent framework comparison and sustained performance remain
open. The final target is still single-request Qwen3-8B BF16 target-only decode
with a 2,048-token prompt and 256 generated tokens.

The source snapshot's original README records its unexecuted draft state.
This page and the authenticated test records describe the subsequent actual
qualification, without rewriting that frozen source snapshot.
