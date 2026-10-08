# MLP Early-STOP Compilation Investigation

This is an unsuccessful compilation investigation, not a GPU optimization
result. No new HSACO, GPU run, numerical acceptance or speedup is established by
this packet. The existing runtime and its selected MLP image remain unchanged.
Issue #42 milestones, Full2303, independent 256-output acceptance and the
single-request Qwen3-8B BF16 2,048/256, 700 tokens/s target remain open.

## Why Investigate The GPU Worker

The preceding [active-pause experiment](../guarded-mlp-active-pause-v1/README.md)
did not improve warm host latency and was not promoted. More frequent host
polling is not evidence of faster GPU computation.

The actual selected MLP image is 33,320 bytes with SHA-256
`b0d1766fdb0fda17ab09496d1120ba3cee29100692b48fcc5b05cf0db0a8d589`.
Its source calls `WaveMlpTileWorkerV2::run_fixed_rounds`, which executes 512
rounds even after STOP. Each retired round still performs three local LDS
exchanges. The provider explicitly documents that this padding is not
timing-neutral.

The existing `run` entry shares the active task/arithmetic path but exits after
a broadcast STOP and completion of that round's final exchange. It retains the
512-round maximum, empty/contended rounds, global completion checks and error
retirement. This is a source-level optimization hypothesis, not proof of native
progress, numerical equivalence or a useful speedup.

The [provider](provider/wave_mlp_tiles_v2.rs) fixes 64 Wave64 workgroups and
64-row tiles, with stage task counts `[1, 96, 96, 1, 64]`. Gate/Up process rows
serially within each tile; Down already handles two independent rows together.
This establishes limited source-level parallelism, not measured CU occupancy.
The current paired-poll timer includes host currentness work, completion
observation and sleeps; it cannot isolate the MLP device duration.

## Paired Compile Experiments

All authoring and compilation ran on `ssh mi350`. The successful historical
image's compiler/tool paths were absent there. Consequently these experiments
use the available retained engineering compiler, not the historical nine-stage
emission generation. Both arms must be tested; historical image success cannot
be credited as a fresh control compile.

| Attempt | Fixture / Build Change | Fixed Arm | Early-STOP Arm |
| --- | --- | --- | --- |
| V1 | Relocate provider paths; original lock and closure | CLI rejects output-directory basename | Not run |
| V2 | Correct required output-directory basename | Offline dependency resolution fails: lock requests rustix 1.1.5, vendor has 1.1.4 | Not run |
| V3 | Use matching retained vendor lock for both arms | Closure local8 has no admitted origin | Same closure-origin rejection |
| V4 | Pass the named task handler instead of a forwarding closure | Rust inliner ICE on foreign FnMut::call_mut | Same inliner ICE |
| V5 | Use ordinary inline(always) on the named handler | Reachable panic path through atomic_compare_exchange | Same panic-path rejection |

The V3 lock changes dependency versions to the retained vendor's coherent set;
it is not just a package rename relative to V2. Its sole change relative to the
retained replacement lock is the fixture package name. Frozen/offline mode was
never disabled. Both arms use the same lock, tools and provider.

V3 also preserves one pre-launch Python syntax failure caused by shell quoting.
It launched no compiler. The corrected recipe passed syntax and exact-fixture
transition checks before execution. This is separate from the actual V3 Rust
closure-origin failures.

The V4/V5 callback rewrites preserve Rust task semantics, but remain unqualified
for GPU execution. V4's ICE does not establish that the closure gate passed.
V5 avoids that observed ICE in both arms but exposes another closed
collection gate. No rejection was suppressed or relabeled as success.

## Reproduction And Evidence

The [manifest](manifest.json) indexes exact files and original terminal receipts.
The [attempt archive](attempts.tar.gz) retains every prepared fixture/recipe,
original command/start/result/stdout/stderr, and source snapshots from V1-V5.
The earlier rejected and unrun arms are not overwritten.

The [V5 recipe](v5/probe.py) uses the authenticated existing bounded supervisor.
Each compiler leaf has a 600-second wall/CPU limit, 50-second cleanup reserve,
12 GiB address-space limit, two Cargo jobs, CPU affinity 8/9, nice 10, hidden GPU
environment, 6 GiB scratch cap and 40/38 GiB initial/live free-space floors.
The 650-second envelope covers the leaf plus cleanup, not final posthash work.
Source/tool hashes are checked before and after each attempted compile.

`compile_accepted` means only that the checked engineering CLI returned zero
and its process/source checks passed. It would not by itself validate the
emitted image, historical replay, CPU arithmetic, native state or performance.
No attempt in this packet has established those downstream gates.

All eight compiler invocations exited naturally with status 1, were reaped,
left no owned process group, and passed source/tool postchecks. These are
retired unsuccessful compile attempts, not eight passing compiler tests.

## Next Acceptance Gates

1. Resolve the reproduced compiler compatibility failure while retaining all
   source, panic, atomic, uniformity and LDS-lifecycle checks. Keep paired
   fixed/early controls under one explicit, authenticated compiler profile.
   The historical recipe used an inline-MIR hint threshold of 16,384, absent
   from the current driver's two profiles. A distinct default-off profile is
   the next proposed experiment; it is not implemented or tested here.
2. Independently validate new source/provider/compiler provenance and emitted
   image metadata: original symbol, Wave64, 344-byte kernargs, eleven pointer
   arguments, zero private segment and 512-byte LDS.
3. Reuse the unchanged qualified host binaries. A new native request must
   substitute exactly the selected MLP image; base/setup images stay exact.
   Qualify a comparator that permits only the old-to-new MLP image identity
   while retaining all 40 semantic comparisons and four full payload checks.
4. Measure a fresh matched native pair before any promotion or speedup claim.
   Preserve deadlines, currentness, acquired terminal state and owned cleanup.
5. Complete authentic Full2303 and the independent 256-output gate. These
   forty-prompt engineering diagnostics generate zero tokens.

A later 32-row/128-workgroup profile could increase all three projection task
counts, including Down. It requires a distinct checked storage/state/guard
profile, not a grid-only adjustment. Multiwave groups, MFMA, batched prefill and
GPU-resident sequencing are additional work; early STOP alone is not a
demonstrated route to 700 tokens/s.
