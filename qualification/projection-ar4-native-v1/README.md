# Four-Step Autoregressive GPU Observation

The rebuilt Ferric parent and worker completed four full 36-layer Qwen3-8B
BF16 forwards on `mi350`. Each next input was the preceding forward's checked
finite, lowest-index argmax. This establishes execution and structural validity,
not independent numerical correctness.

| Position | Input Token | Own Output Argmax |
| ---: | ---: | ---: |
| 0 | 9112 | 67 |
| 1 | 67 | 25 |
| 2 | 25 | 576 |
| 3 | 576 | 2701 |

One attempt completed without retry or forced cleanup. All 152 tensor slices,
four full observation payloads and 576 terminal state records passed structural
validation. Seven process leaves exited naturally and were reaped; three
before and three after audits found the selected devices idle. Close sequence
5 and the actual parent/worker lineage were checked.

The selected V7 prefix, materialized-SiLU and additive projection images are
unchanged. Original bootstrap/copy images remain distinct. The change is
explicit autoregressive recurrence in the CPU-qualified runtime, not new GPU
arithmetic or admission of the still-unqualified RoPE candidate.

## Evidence

- [Actual GPU receipt](complete.json), [validated observations](observation.json) and [generated trajectory](trajectory.md).
- [Publication ledger](result.json), [data-only publisher](publish.py) and [reviewed inputs](inputs/plan.json).
- Fresh [parent](runtime/parent/complete.json) and [worker](runtime/worker/complete.json) loader/library audits.
- Separate [AR4 Rust qualification](../projection-ar4-cpu-v1/README.md) and [52/38 policy-test checkpoint](../projection-ar4-supervisor-v1/README.md).

The publisher rehashed the complete 57-file retained case and replayed the
frozen structural and owned-process validators. Raw tensor/control buffers,
ELFs and weights are not copied into Git. Their retained identities remain in
the ledger. The local publication does not reexecute remote audits or rehash
all transitive inputs, executable bodies, shared libraries or source images.
The selector's twelve-test primary observation is excerpt-only and does not
claim a separate test-source postcheck.

The supervisor's 392.484-second elapsed time includes setup, model loading,
control and auditing. It is not kernel time, sustained throughput or evidence
of overlap. No speedup is claimed.

Independent genuine-autoregressive framework comparison remains pending.
Numerical acceptance, the 2,048-token prompt / 256-token decode workload,
production authority and the 700-token/s objective remain open.
