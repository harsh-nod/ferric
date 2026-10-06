# Combined State Lowering Controller

Status: **the actual combined-state compile fails at descriptor ownership
admission**. Its 25 controller fixtures pass on MI350, but the kernel crate
has no successful HSACO or GPU result.

## Actual Compile

[Attempt v3](attempt-v3/evidence/failed.json) uses the passing 39-test CPU
candidate, its passing offline vendor preparation and the independently
qualified DAG compiler/tool closure. The
[raw diagnostic](attempt-v3/evidence/compile.stderr) reports:

```text
production descriptor evidence has an internal formal alias obligation not discharged by Rust ownership mismatch
```

This is a compiler refusal, not an observed GPU race. The compile exits
naturally with status one after 363.190239 seconds; whole-controller time is
366.924882 seconds. The child is reaped, the process group is absent, sources
remain unchanged and postchecks are clean. There is no timeout, forced cleanup
or emitted artifact. These times are not kernel latency.

The [closed capsule](attempt-v3/retention-manifest.json) retains 13 members,
12 content pins and all ten raw records. The failed receipt is 19,077 bytes,
SHA-256 `d072928318581d8f492d296dfa4cc4a1f42865fbd3a7d95fdac01dddac7e5ab0`.
The archive is 679,889 bytes, SHA-256
`b3c6b033c5a580bf39c9eef4b11286e93652e7ddd4d64dee1e98705139d6c466`.

The diagnostic does not identify a root or parameter pair. A source-level
investigation points to formal alias derivation treating every atomic access,
including load-only accesses, as a writer. That is a diagnosis to test, not
measured per-root evidence or permission to bypass descriptor ownership checks.

The [atomic-load alias proposal](../guarded-mlp-atomic-load-alias-v1/README.md)
addresses that classification without relaxing the descriptor gate. Its source
is reviewed and formatted; fresh compiler qualification is still required.

## Controller Fixtures

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

The fixtures establish controller behavior only. The actual compile remains
failed; there is no successful HSACO, GPU, model-numerical or performance
result. All issue #42 milestones and the 700 tokens/s target remain open.
