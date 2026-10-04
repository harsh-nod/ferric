# State-Bank Worker Deployment and Observation

Engineering checkpoint, 2026-10-04. The successor controller passed **144 CPU
policy tests** on `mi350`, with no failures, errors or skips. Build-host export
of the new worker also passed. This does not report a new GPU result, numerical
acceptance, or a performance improvement.

## Verified Scope

| Check | Actual result | Limitation |
| --- | --- | --- |
| Controller, deployment and failure-policy tests | 144 passed in 7.999 seconds | Synthetic policy tests, not GPU arithmetic |
| Frozen source postchecks | All 29 package files unchanged | This exact controller generation |
| New worker build | 553 passed, four unchanged ignored | Separate [paired-source checkpoint](../state-bank-batch-v1/README.md) |
| Build-host deployment export | Source, named test, command and Cargo-artifact replay passed | No GPU launch |
| Local and MI350 transfer integrity | 129 artifact references, 115 files, 14,377,883 bytes verified on both | Executable/library and live GPU audits are separate |

The deployment adds only the new worker and its evidence. It retains explicit
CPU522, CPU475 and CPU633 predecessor manifests and the original parent and
historical image. V7 image selection remains a separate verified prerequisite.
The new verifier reconstructs the full prior source inventory, applies exactly
the ten paired-source changes, checks all 21 actual bounded command phases,
recomputes named test outcomes, and derives the worker identity from the
actual Cargo artifact. A matching declared test count alone is insufficient.

The successor GPU runner preserves six V7 GPU/numerical prerequisite pairs,
three pre-audits, three post-audits, one native attempt, resource limits, and
Close/reaping. Its source and tests retain the original ownership and cleanup
behavior. The new deployment helpers are themselves included in the frozen
package; an earlier package cannot authenticate unlisted helper source.

## Evidence

- [Actual 144-test receipt](pure/complete.json) and [test log](pure/tests.log)
- [Frozen package](source/manifest.json) and [bounded test runner](run_pure.py)
- [Exported deployment manifest](deployment.json) and [CPU-only root review](cpu-review.json)
- [Intake contract](source/INTAKE.md) and [execution contract](source/RUN.md)

The deployment receipt SHA256 is
`b36de470ab814fd6e76cd57f3e0da68efe1ee36137ebea50794344edf09c9104`.
The new worker is 4,797,496 bytes, SHA256
`6901a1319352d0401e452d616efa50d2de08c40943f50cb054d89f3f26d7fd52`.
Binary/object bodies are retained separately, not committed to Git. Source
prose describes author-time status; the actual receipts record completed work.

The completed CPU553 source copy and dependency artifacts were retired on
the build host only after full local source/raw/binary retention. The original
selected worker, 109 raw records, source archives and overlays remain at their
pinned paths. No shared dependency cache or another user's files were removed.

## Remaining Gates

Receiver byte verification passed. Fresh executable/library review and the
same-V7 TF4 GPU comparison remain required. That comparison must preserve all 152 tensors,
four complete captures and tokens, measure the predicted 572 fewer group
checks per forward, and exercise reused banks on forwards three and four.
Independent full-model numerical acceptance, sustained BF16 Qwen3-8B
2,048/256 throughput, production admission, and the 700 tok/s target remain
open. No `main` or tutorial-site branch is changed by this checkpoint.
