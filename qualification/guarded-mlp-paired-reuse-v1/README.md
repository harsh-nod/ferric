# Retained Paired MLP Session

Status: **private Session implementation compiled and CPU-tested on MI350**.
The prior [nonzero paired GPU diagnostic](../guarded-mlp-paired-native-v1/README.md)
passed one generation. This checkpoint adds retained custody for consecutive
generations; it does **not** claim that GPU rearm/reuse has been tested.

Follow-on evidence: [two consecutive GPU generations now pass](../guarded-mlp-paired-reuse-native-v1/README.md)
with the same owners and payload allocations. The receipt in this directory
remains the original CPU-only checkpoint.

Issue #42 milestones and the single-request Qwen3-8B BF16, 2048-token prompt,
256-token decode, 700 tokens/s target remain open.

## Implementation

The [Session](cpu-attempt-v1/fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_combined_mlp_paired_session_v1.rs)
exclusively borrows the same group, two combined atomic owners, selected kernels
and payload roles. Its private custody moves through:

```text
Ready -> Busy -> Idle(generation, actual terminal snapshots)
Idle  -> Busy -> Ready(next generation)
any failure -> Poisoned
```

`run()` invokes the existing paired R1 -> MLP -> validator -> barrier -> R2
coordinator. A successful run retains a private copy of both terminal snapshots;
the returned Completion is reporting data and cannot authorize rearm. A second
run without rearm, or a second rearm without a run, is terminally refused.

`rearm_next()` derives exactly the next generation internally, with checked
overflow. Before the first atomic reset it calls the existing native completion
gate again. That gate checks both retired batches, all ten acquired-zero
completion signals, current arena/queue identities and counters, faults, both
Completed owners and both exact terminal snapshots under full context fences.
Only then are the two owners reset and their initial states read back.

The aggregate rearm deadline is checked after quiescence, after each reset and
after final paired readback. A partial reset, mismatch or deadline failure
poisons both owners and the group without rollback. Busy or poisoned Session
drop quarantines the group, including during unwinding.

The bounded read/write methods expose only retained PublicVram payload roles,
not owner atomics or signal arenas. Reads require Ready/Idle; writes require
Ready. Transfers retain the existing 4 MiB limit and context fences. Construction
remains unsafe and private: reviewed exact images, valid model-role bindings,
completed producers and coherent payloads are obligations for every run.

After successful rearm only host staging metadata is discarded. Old signal and
kernarg allocations remain owned by the group until queue-first Close, and the
next run prepares fresh arenas. No signal is reset, no allocation freed early,
and no hardware read cursor advanced by software. This is owner/payload reuse,
**not constant-space arena reuse**; sustained model execution still needs explicit
capacity planning or separately qualified arena reclamation.

## Tests And Evidence

The [CPU receipt](cpu-attempt-v1/evidence/complete.json) records 12 naturally
successful phases in 61.328984 seconds on `ssh mi350`. Every child was reaped,
every process group was absent, and there was no timeout, forced cleanup or
postcheck error. No GPU test was executed in this attempt.

| Scope | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Full KFD suite, five targets | 1039 | 0 | 5 |
| New Session tests | 10 | 0 | 0 |
| Existing paired coordinator | 14 | 0 | 0 |
| Combined owner and fixture CPU tests | 14 | 0 | 2 |
| Existing paired native fixture CPU tests | 3 | 0 | 1 |
| Atomic memory | 7 | 0 | 0 |

All 1034 prior named outcomes are preserved. The ten new
[tests](cpu-attempt-v1/fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_combined_mlp_paired_session_v1_tests.rs)
cover before/after-effect failures at every rearm operation, expiry after each
boundary, partial-reset quarantine, invalid timeout, zero/overflow generations,
peer snapshot drift across all 552 words, readback refusals, completion-copy
isolation, phase replay, transfer policy and native pre-GPU refusals.

These exercise the actual orchestration and custody code. The fake backend's
quiescence error is an error-propagation test, not injected device faults or
proof of successful native run/rearm/run composition. Existing arena and atomic
memory tests remain required; the next two-generation GPU diagnostic is separate.

The source map contains 798 entries: 796 source files and two harness files.
The only source changes are the parent module registration and two new Session
files. The baseline, dependencies and tool pins remain unchanged. No-default-features
check and five selected test artifacts are retained by their actual Cargo pins.

The [capsule manifest](cpu-attempt-v1/retention-manifest.json) pins 89 bodies,
including 64 raw evidence files; the archive has 90 members and expands to
5,603,114 bytes. Its 899,085-byte archive SHA-256 is
`f0b0724ae4696e67e94475d7e5754e77619e331c4e082459d51405585d48c402`.
The 1,178,322-byte receipt SHA-256 is
`7c568808706823112657837dd61a3d3271c142a040aa7f6346b251e187bac718`.

## Next Gates

The follow-on GPU diagnostic completed the first gate: generation 1 -> rearm
exactly to 2 -> double Up weights and poison writable payloads -> independent
checks of both generations. Every final output changed, including the expected
generation-2 positive-zero cancellations.

Next integrate a distinct Ferric completion path without the old duplicate R2
callback, qualify model state banks and full-model numerics, and measure the
specified sustained workload against the equivalent vLLM baseline. No throughput,
overlap, model acceptance or production-authority claim follows from this CPU run.
