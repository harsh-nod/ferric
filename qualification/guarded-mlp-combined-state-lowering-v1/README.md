# Combined State Lowering Controller

Status: **25 controller fixtures pass on MI350**. The new combined-state
kernels have not yet passed an actual guarded compile or run on the GPU.

[Controller tests](controller-tests-v1/attempt-v1/evidence/complete.json)
exercise the frozen lowering controller with synthetic inputs: 21 admission
cases and four normalization cases. This includes six new combined-state CPU
and vendor-admission cases. The test process exits naturally with status zero,
is reaped and leaves no process group; sources are unchanged and postchecks
are clean. Whole-controller elapsed time is 6.423984 seconds, not kernel time.

The frozen [lowering controller](controller-tests-v1/attempt-v1/lowering.py)
has deliberately unbound future CPU/vendor pins. Actual compile execution
requires separate observed qualifying evidence and a separately bound body.
The existing DAG-qualified compiler/tool evidence is not regenerated or
replaced by these fixtures.

The closed capsule retains 18 members: 17 pinned files and its
[manifest](controller-tests-v1/attempt-v1/retention-manifest.json), including
seven raw records. The actual receipt is 14,592 bytes, SHA-256
`bb0499a116f2ae5b88c778775a020ef6102b534d8a3711ca19aee0e394a8939e`.
The transferred archive is 65,894 bytes, SHA-256
`d461fbc254b801e7fd5febf3256dff1641125f900b058c5fe6ab10dfca2ce401`.

These are controller tests only. There is no new compiler qualification,
HSACO, GPU, model-numerical or performance result. All issue #42 milestones
remain open.
