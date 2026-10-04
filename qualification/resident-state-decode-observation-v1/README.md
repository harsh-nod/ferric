# Resident-State Worker Deployment and Observation

Engineering checkpoint, 2026-10-04. The new controller passed **116 CPU policy
tests** on MI350, with no failures, errors or skips. The build-host data export
also completed, preserving the original parent and prior worker deployment as
separate prerequisites. The subsequent [GPU comparison](tf4/README.md) passes:
all 152 tensors are bitwise unchanged and 576 fewer group checks per forward
are measured. This remains an engineering result, not sustained throughput.

## Verified Scope

| Check | Result | Limitation |
| --- | --- | --- |
| Controller and deployment tests | 116 passed in 7.972 seconds | Synthetic policy tests, not GPU or arithmetic qualification |
| Source postchecks | All 25 package files unchanged | Exact frozen controller generation only |
| New worker CPU provenance | 522 passed, four ignored | Separate [runtime checkpoint](../resident-state-fence-consolidation-v1/README.md) |
| Actual data export | Replayed 37 input pins, 88 raw records and 17 command phases | No GPU launch |
| Local and MI350 transfer verification | 128 artifact references, 118 files, 35,277,302 bytes rehashed | Runtime audits remain separate |
| New-worker GPU comparison | 152 tensors bitwise equal; 2,304 fewer group checks across four forwards | One host-only diagnostic pair, not a qualified speedup |

The deployment verifier reconstructs source identity from the original archives
and sequential 13-file host-policy, three-file group-fence and eight-file
resident-state overlays. It checks actual test names/outcomes, compiler
environment, command bounds, natural exits, unchanged source inventories and
the selected worker artifact. Differing Git labels are not substituted for
byte-preimage checks.

The successor intake requires explicit original-parent, prior-worker and
new-worker provenance. Only the worker changes; V7 image selection remains a
separate checked step. The original process ownership, one-attempt limit,
three pre-audits, three post-audits, Close/reaping and resource limits are
unchanged. New tests reject stale worker reviews and altered parent, image,
source, command and test-result identities. The extra 32 tests are included in
116, not added to that total.

## Evidence

- [Actual CPU test receipt](pure/complete.json) and [test log](pure/tests.log)
- [Frozen package](source/manifest.json), [bounded runner](run_pure.py),
  [intake contract](source/INTAKE.md) and [execution contract](source/RUN.md)
- [Actual exported deployment](deployment.json) and [CPU-only root review](cpu-review.json)

The worker is 4,780,024 bytes, SHA256
`79d2b50a080a39300d02648c2844a398907ac4f89a41abff01c720ac58d42430`.
Its deployment receipt SHA256 is
`49f3a097e85d8d5020ac5ca4d7a962eb281900ff58c0f8b84510dbb51af1adbc`.
Runtime binaries and object bodies are retained in the task-owned evidence
store, not committed to Git. Original package prose describes its source-only
draft state; the receipts above record subsequent execution.

## Remaining Gates

Receiver verification and fresh executable/library audits passed before the
single GPU attempt. The same-V7 TF4 comparison checked all 152 tensor rows and
four tokens, actual operation counts, device audits and complete lifecycle.
The predicted 576 fewer full group checks per forward are now measured.
Full-model numerical acceptance, sustained 2,048/256 performance, production
admission and the 700 tok/s target remain open. No main or tutorial-site branch
is changed by this engineering checkpoint.
