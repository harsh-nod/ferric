# Combined State Owner Proposal

Status: **CPU qualification passes on MI350** in an isolated source workspace.
This is not installed in the canonical runtime or GPU-tested. It does not
change Ferric's inference path. All issue #42 milestones remain open.

The [proposal](proposal-v1/README.md) adds a private owner for a genuine
2,208-byte allocation containing 552 atomic words. It preserves the old
2,192-byte owner and exposes only the exact private regions needed by the
MLP prefix, the combined-state validator and read-only guard consumers.
It does not spoof the old state token or relax compiler alias checks.

Fourteen new tests cover allocation extent, atomic storage, allowed pointer
regions, generation transitions and poison behavior. All pass in the full
suite and in focused repeats. The proposal also extends the existing queue-first
Close selection fixture for this new allocation kind.

## Actual CPU Qualification

[Attempt v1](attempt-v1/evidence/complete.json) passes all nine phases. The
full KFD run reports 1,008 passes, zero failures and three unchanged ignores;
focused owner and atomic-memory runs add eight and six passing executions.
The full run preserves all 997 old named outcomes and adds the 14 new tests.
This is 1,022 passing executions, not 1,022 unique tests.

All children exit naturally with status zero, are reaped and leave no process
groups. The complete 790-row source map is unchanged, as are dependency and
input checks; there are no postcheck errors. Whole-controller elapsed time is
59.558565 seconds. This is CPU qualification time, not GPU or model latency.

The [closed retained capsule](attempt-v1/retention-manifest.json) contains
73 members, 72 content pins and all 49 raw records. Selected test executables
are represented by metadata only; their binary bodies are not retained.
The receipt is 1,147,625 bytes, SHA-256
`e1d8582b90344c8cd52925b88bd849b248ea242964a543d1c96b52f91f86d37e`.
The archive is 1,049,376 bytes, SHA-256
`d278e56e05cca138fe7520d5bfaa2e66b62507843f965d2d8df9cb40ec04f3b6`.

## Remaining Integration

The [source manifest](proposal-v1/source-manifest.json) is 6,686 bytes, SHA-256
`323ef39a6c6ec1d9c91c179a3e28e6d48bc5f56e99e786df52ff30b68f8dd311`.
It pins seven postimages: three modified runtime files and four new private
module/test files. Existing preimages match dependency revision
`5a500d63c29b78f8788356b20dfcbb5c41ec20c9`.

The paired coordinator is still required. Private unsafe completion/rearm
methods explicitly require both submitted batches, all ten actual completion
signals, both current guards and quiescence; the owner alone proves none of
these. A physical peer mapping covers the allocation, while typed argument
checks restrict admitted peer reads to the guard suffix. This is not hardware
suballocation isolation.

See the [test plan](proposal-v1/TEST-PLAN.md) and
[integration map](proposal-v1/INTEGRATION-MAP.md) for remaining CPU, ABI,
coordinator, GPU and inference gates.
