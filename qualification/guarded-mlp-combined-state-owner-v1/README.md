# Combined State Owner Proposal

Status: independently reviewed source only. This is not installed in the
canonical runtime, built, CPU-qualified or GPU-tested. It does not change
Ferric's inference path. All issue #42 milestones remain open.

The [proposal](proposal-v1/README.md) adds a private owner for a genuine
2,208-byte allocation containing 552 atomic words. It preserves the old
2,192-byte owner and exposes only the exact private regions needed by the
MLP prefix, the combined-state validator and read-only guard consumers.
It does not spoof the old state token or relax compiler alias checks.

Fourteen new tests cover allocation extent, atomic storage, allowed pointer
regions, generation transitions and poison behavior. They are authored test
cases, not passing results. The proposal also extends the existing queue-first
Close selection fixture for this new allocation kind.

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
