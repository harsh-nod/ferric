# Core Result Residual MIR Observation

This 2026-10-05 MI350 diagnostic extends the [branch observation](../guarded-mlp-core-result-diagnostic-v1/README.md)
with the genuine core `FromResidual` wrapper reachable from the unchanged
matrix fixture. **It adds no compiler admission and qualifies no provider,
GPU kernel, model result or performance target.**

The private source tree derives from `a057bf3b0e9175f481a31181b19d587d342bf8f3`
with five separately pinned diagnostic source bodies. It is not represented
as that exact Git revision. Ferric's dependency and production path are unchanged.

## Observed Result

The diagnostic compiler built and all ten rendering/census tests passed.
The [failed receipt](evidence/failed.json) preserves 19 natural, reaped phases
with no remaining process groups: the first 18 returned zero and the unchanged
matrix test returned 101 at its original source-safety rejection. Source and
dependency integrity postchecks passed. Attention and GPU execution did not run.

The [derived observation](derived-core-result-diagnostic.json) is an exact
8,449-byte slice of the [raw output](evidence/matrix-extraction-0.stdout),
starting at byte 5,148. Its complete 8,375-byte body contains the branch MIR
and one reachable residual wrapper selected from nine collected functions.
Neither the scan nor the match list was truncated; no downstream conversion
body was dumped.

| Property | Branch | FromResidual |
| --- | ---: | ---: |
| Arguments / locals | 1 / 6 | 1 / 6 |
| Basic blocks / source scopes | 5 / 3 | 2 / 2 |
| Statements | 8 | 7 |
| Calls / cleanup blocks / inlined scopes | 0 / 0 / 0 | 1 / 0 / 0 |

The measured residual signature converts
`Result<Infallible, Bf16MatrixViewError>` into `Result<(), KernelError>`.
Its first block reads the discriminant, checks equality to the Err variant,
assumes that condition, moves the error payload, and invokes the actual
`From<Bf16MatrixViewError>` conversion. The second block wraps that converted
error in Err and returns. An exact recognizer must account for the assumption
and retain the conversion call's independent safety checks; this observation
does not authorize arbitrary Result wrappers or converters.

Logging remains opt-in for the exact matrix leaf, one bounded capture per
process, with a shared 64 KiB cap. It scans at most 4,096 already collected
functions and renders at most four matching residual wrappers. No model
buffers or kernel bodies are logged. The original rejection remains intact.
The controller retains the existing offline dependencies, CPU/job limits,
12 GiB address-space limit, storage floors and bounded process lifecycle.

The retention manifest pins 107 original files and the separately derived
observation. Next are the exact branch/residual matchers, actual-MIR mutation
tests and both unchanged pipeline controls. Provider qualification, checked
guarded HSACO emission, GPU/model validation, sustained 2,048/256 decoding and
700 tokens/s remain open. This is diagnostic evidence, not a passing pipeline.
