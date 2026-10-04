# CPU522 Resident-State Decode Observation

Unexecuted successor of the frozen `p228-independent-decode-observation-v1`
package. It selects the CPU522 worker while preserving the CPU633 parent,
separately qualified V7 image, Four workload, and all six standalone GPU and
conditional numerical prerequisites. It neither changes Rust sources nor
implements Long2303. No imports, tests, builds, SSH or GPU commands were run
by the author. Root owns package freezing, testing, deployment and execution.

The only controller changes are its result schema and explicit CPU475 prior
deployment fields. All owned-process handling, limits, audit sequencing,
post-audit cleanup, native invocation, capture retention and structural-only
acceptance remain unchanged. The existing observation helper is byte-identical.

Intake additionally authenticates the new resident-state deployment through
`resident_state_portable.deployment`, then independently replays the frozen
CPU475 deployment. Both must bind the same CPU633 base. Parent and historical
image remain unchanged; only the worker changes to the actual CPU522 pin:
4,780,024 bytes, SHA256
`79d2b50a080a39300d02648c2844a398907ac4f89a41abff01c720ac58d42430`.
The V7 image is then selected through the unchanged six-case prerequisite path.

See [INTAKE.md](INTAKE.md) for the new schemas and root inputs and
[RUN.md](RUN.md) for the unchanged execution envelope.

## Root Integration

Stage unchanged members from the frozen original package, plus the exact new
resident-state verifier and its tests/exporter/auditor supplied by the deployment
author. Do not replace historical helpers or loosen their pins. Fill
`PACKAGE_FILES` and `PURE_TESTS` from the final actual source roster/test census.
The already authored root pure runner SHA is pinned in intake; its result is
not assumed. No manifest or successful pure/deployment/runtime/GPU receipt is
invented by this draft.

The original 84 synthetic tests are retained, with their namespace/fixture
adjustments where required. Eight additional tests cover the new lineage,
selection, stale-review and result fields. The deployment author supplies a
separate additional test cohort. All remain unexecuted here. These tests do
not establish GPU coherence, arithmetic correctness or a performance gain.

After actual pure qualification and deployment replay, obtain fresh parent and
worker runtime audits/reviews and a root-authored scoped engineering review.
Run one fresh TF4 attempt. After that controller terminates and all post-audits
finish, independently compare its four captures/152 tensors and output tokens
against the retained same-image CPU475 TF4 control. Inspect the expected 576
fewer group checks per forward separately from timing; this is not measured
in the proposal. Existing full-model discrepancies remain unresolved.

No numerical acceptance, proof authority, speedup, sustained 2,048/256 result
or 700-token/s claim follows from a closed structural observation.
