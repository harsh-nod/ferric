# Core Result MIR Observation

This 2026-10-05 MI350 diagnostic captures the genuine core `Result::branch`
helper rejected by the unchanged matrix extraction test. **It adds no compiler
admission and does not qualify the provider or a GPU kernel.**

The private source tree is derived from `a057bf3b0e9175f481a31181b19d587d342bf8f3`
with three explicitly pinned diagnostic source bodies. It is not represented
as that exact Git revision. The sources, controller, input manifest and raw
evidence are retained here; Ferric's dependency remains unchanged.

## Observed Result

All six rendering tests passed, including byte limits, UTF-8 boundaries and
incomplete-output labeling. The [failed receipt](evidence/failed.json) records
19 natural, reaped phases with no remaining process groups: the first 18
returned zero, then the unchanged matrix test returned 101. Integrity
postchecks passed. The failure is preserved, not relabeled as a passing test.

The [derived observation](derived-core-result-diagnostic.json) is an exact
4,679-byte slice of the [raw test output](evidence/matrix-extraction-0.stdout).
Its 4,605-byte body reports the genuine nominal Result/ControlFlow identities,
monomorphic signature, local types, source scopes and complete helper MIR:

| Property | Observation |
| --- | ---: |
| Arguments / locals | 1 / 6 |
| Basic blocks / source scopes | 5 / 3 |
| Statements | 8 |
| Calls / cleanup blocks / inlined scopes | 0 / 0 / 0 |

The Ok route transfers its payload into Continue. The Err route constructs
an Err residual and transfers it into Break. The discriminant default is
unreachable. This is evidence for an exact body recognizer, not proof that
all Result helpers or generic payload families may be admitted.

Logging is opt-in for the exact matrix leaf, attempts one selected helper per
process, and caps its complete output at 64 KiB. It does not dump kernel bodies
or model buffers. The original source-safety rejection immediately follows
the logging hook. The run retained the same offline dependencies, two CPU
cores/jobs, 12 GiB address-space limit, storage floors and bounded lifecycle.

The retention manifest pins 105 original files and the separately derived
observation. Next are a closed body matcher with actual-MIR mutation tests,
the real FromResidual/conversion checks, and rerunning both unchanged pipeline
controls. Attention, provider qualification, checked guarded HSACO emission,
GPU/model validation and performance gates remain open, including 700 tokens/s.
