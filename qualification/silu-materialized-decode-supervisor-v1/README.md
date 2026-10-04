# SiLU Four-Step Decode Supervisor

All 43 CPU policy tests passed on MI350 with no errors, failures or skips.
The suite covers the frozen supervisor that selects the checked BF16 SiLU
image in the existing four-step, 36-layer decode route. It did not launch a
model, GPU kernel or new runtime audit.

## Scope

The previously qualified CPU1022 parent and worker remain unchanged. The
request selects the [checked SiLU image](../silu-materialized-lowering-v1/README.md)
as `decode.tiles_image`; original bootstrap images, V7 prefix, corrected
projection-residual image, BF16 precision and four teacher-forced inputs remain
unchanged. The actual [layer capture](../silu-materialized-native-capture-v1/README.md)
and [independent comparison](../silu-materialized-comparison-v1/README.md) are
prerequisites, not substitutes for a full-model comparison.

The supervisor preserves one attempt, no retries, six idle-device audits,
owned child cleanup, Close/reap checks and the existing resource limits.
Its unchanged tensor validator checks all 152 tensor ranges and four payloads
without requiring equality to the previous native outputs.

| Suite | Passed |
| --- | ---: |
| Tensor and protocol validation | 12 |
| Input and executable selection | 12 |
| Process lifecycle and evidence limits | 11 |
| SiLU image and diagnostic prerequisites | 8 |
| Total | 43 |

New cases reject mismatched CPU/lowering/image generations, changes to original
bootstrap images, reselection of the old Down2 image, wrong layer/reference
evidence and inflated acceptance claims. Dependency aliases are restored on
successful and failed imports, including a preexisting `None` entry.

## Evidence

[pure/complete.json](pure/complete.json), SHA-256
`cd95c4739bfab5f2bc3d01c53218302577d218c2ae9e25ef9fc7ed5d30cb93cf`,
records the actual result and all 43 test names. The transcript and identical
before/after source snapshots are alongside it. Execution used CPU affinity
8/9, nice 10, hidden GPUs, a 120-second CPU cap and 2 GiB address-space cap.

[source](source) contains the exact fourteen-file package and manifest;
[tools](tools) includes the byte-identical external test runner and data-only
publisher. [result.json](result.json) binds the copied files. Publication does
not rerun tests, validate external runtime libraries or launch native execution.

The native four-step attempt and its independent numerical comparison remain
separate gates. These tests establish neither full-model correctness nor
performance for the 2,048-token prompt / 256-token workload or 700 tokens/s target.
