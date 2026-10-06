# Combined State Candidate

Status: experimental; [both CPU qualification attempts failed](../guarded-mlp-combined-state-cpu-v1/README.md). The new
[experimental v2 crate](../../device/qwen3-tp-guarded-mlp-segment-kernels-v2/README.md)
is not connected to inference, and has not passed CPU qualification, guarded
lowering or GPU execution. All issue #42 milestones remain open.

The [latest actual guarded compile](../guarded-mlp-memory-bounds-dag-lowering-v1/README.md)
fails because its two potentially aliasing allocation origins lack relative
base-offset information. This additive candidate accepts a single genuine
552-word atomic allocation, retaining the 548-word state predicate and using
the four-word suffix for guard publication. Writes at 548, 549 and 551 remain
Relaxed; the verdict at 550 is published last with Release. State reads retain
Acquire ordering. The existing v1 crate, 2,192-byte token and profiles are
unchanged, and no compiler proof predicate is relaxed.

The [source manifest](proposal-v1/source-manifest.json) and
[patch](proposal-v1/integration.patch) contain eight additions. The manifest
is 10,515 bytes, SHA-256
`cf98f7028d3697e120193290beb5621b00482ee83f6c7c5c7ba45af007bd8eb6`.
There are 39 authored tests: the 27 inherited tests plus 12 combined-state
tests. The first actual run passes 38 and fails one formatting-sensitive
source-shape assertion after rustfmt. All inherited tests pass. The failure
is retained unchanged; the complete CPU qualification has not passed.

The [second source proposal](proposal-v2/source-manifest.json) makes the
supplementary store assertion formatting-independent, but its test build
fails due to missing explicit `std` imports for `format!` and `String`. No
tests run in the second attempt. This proposal is retained separately and
has not replaced the canonical device crate.

The [typed 2,208-byte owner and suffix-only peer-read capability](../guarded-mlp-combined-state-owner-v1/README.md)
have a separately reviewed source proposal and 14 unexecuted focused tests.
Runtime CPU qualification, the coordinator, artifact ABI admission and GPU
tests remain separate work. Predicted ABI
sizes are not emitted-metadata evidence and grant no load or launch authority.
