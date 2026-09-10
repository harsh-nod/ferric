# M1 Team Progress

Updated: 2026-09-09. This is an implementation checkpoint, not a qualification
receipt. The 33 M1 roadmap gates remain open.

## MI350 And Eight-Rank Work

The September 9 MI350 work is a separate, in-progress extension, not an M1
qualification or an eight-GPU Qwen result. Compiler/runtime changes track
[fe2o3 #274](https://github.com/harsh-nod/fe2o3/issues/274).

| Team | Current Slice | Status |
| --- | --- | --- |
| Core runtime | Explicit gfx950 topology discovery, COV6 loader profile, and a separate checked device-observation token with no execution authority | 420 KFD unit tests, 20 integration tests, and 27 doctests pass, along with scoped strict Clippy. All eight MI350X devices pass repeated no-queue bind/currentness/drop checks without descriptor leaks. All twelve kernels in the genuine gfx950 object pass loader/materialization and both old gfx942 dispatch-path rejection tests. Independent review passes. |
| Compiler integration | Target-bound engineering artifact production and additive AMDGPU compiler-handoff extraction; measured library function targets checked after import selection and before optimization | 518 backend, 17 CLI, 7 finalizer, 16 extractor, and 2 export-simulator tests pass. Four native-worker suites pass, including selected incompatible helper rejection with optional verification disabled. Both target emissions and exact replay pass. Published on fe2o3 main through `a8b016e14`. |
| Ferric kernels | Explicit mutually exclusive gfx942/gfx950 aggregate features over the same twelve kernel bodies; both target helper files bound into the build source closure | 33 aggregate tests per target and 21 historical RMSNorm wrapper tests pass. Updated build/release policy tests pass. Both twelve-kernel emissions pass with the same final native worker; gfx942 bytes are unchanged by the worker fix. |
| Tensor parallelism | Qwen3 1/2/8-rank head/weight partitioning, actual BF16 shard-row copying, and ordered collective readiness epochs | Direct Verus passes 36 obligations with zero errors. All 9 release tests and 8 actual-body mutation checks pass; strict Clippy and independent review pass. Integrated in `4afe852`. |

The combined Ferric host checkpoint at `4afe852` passes 645 engine tests (9
separately gated tests ignored), 38 source-gate tests, formatting, strict
Clippy, and source admission. Its inventory has 172 modules and 8,228 executable
bodies: exactly one new module and 21 directly verified bodies, with no added
unverified bodies. Protected runtime feature resolution remains gfx942-only.

The preceding Ferric pin checkpoint used public fe2o3
`a8b016e14ca8c77c9e7abe4591086f7cab11ce61`, integrated locally in `c092df2`.
All fifteen locked graphs resolve, with no registry-version drift; independently
reviewed dependency records and the rebuilt source-pin adapter binding match.
The final-pin remote run passes 645 engine tests (9 ignored), 38 source-gate
tests, source-gate strict Clippy, formatting, property binding, and full source
admission. Connectivity has recovered. The interrupted engine Clippy log was
recovered with exit status zero; the remaining adapter/source-policy checks,
four isolated allocation tests, and both fresh engineering host release
builds now pass. No local build/test fallback was used.

The new gfx950 engineering object is 103,616 bytes, SHA-256
`2679e59626eee9939412aaf7af6a542c8aeccbe4dd13fb7e6ea3bdcf4f3b8222`,
content ID `431f1e294d5018e0f057d490495921a1983bac0c25e4e900c3f72a05350b3f84`.
It contains twelve Wave64 kernels with COV6 and exact replay; all authority
grants are false. Component provenance remains explicit: Ferric kernel source
`296bb98`, device dependencies `0ea54ed9`, compiler tool images `24e842d70`,
and final native-worker source `a9ea16247`. The matching gfx942 object has
content ID `fa6ccd9130001647932502d13092ca44f7ec78c9580984c96ce28f34710ab670`
and is byte-identical to the pre-worker-fix regression object. Neither object
is relabeled as a final-pin Ferric rebuild or a GPU execution observation.

Compiler-wide Clippy and aggregate-kernel Clippy are not reported as clean:
the existing backend has 344 source-attributed diagnostics, and the aggregate
retains 104 baseline kernel-body diagnostics. The changed compiler CLI/finalizer,
runtime, engine, source gate, and kernel build helpers pass their scoped strict
checks; no broad lint suppression or unrelated kernel rewrite was added.

The follow-on execution batch is in progress against published fe2o3 main
`3546d54d2c4a913f5d079701aed557d0a378bba8`. Its release gfx950 worker passes
433 library tests, a genuine single-device RMSNorm dispatch with exact BF16
output and unchanged guards, and eight concurrent isolated memory/queue
lifecycles with distinct device IDs, live executable hashes, and confirmed
child teardown. The eight-worker lifecycle fixture does not dispatch kernels
and is not a Qwen result.

The new thirteen-root TP crate includes six rank-local kernels and seven
unchanged imported helpers. It passes 30 host/source tests per target; actual
new-artifact emission and numerical checks remain pending. The rank driver,
bounded child-process transport, authenticated model intake, and measurement
command are integrated and undergoing final published-pin checks. The new
sequence cursor has a same-source Verus proof and eight actual-body negative
mutations; its final integrated replay is in progress. Measurements use a
release controller and worker, host-staged FP32 ordered collectives, contiguous
rank-local KV, and token-at-a-time prompt processing. Setup is reported
separately. Full 1/2/8-GPU Qwen validation and timing results are not yet
available. No gfx942 artifact
or device authority may be relabeled as gfx950. Builds/tests stay on `mi300x`;
target-specific MI350 hardware checks use `mi350`. Symmetric memory and MTP
remain deferred.

At the connectivity interruption, all completed agent-owned remote stages and
local worktrees had been removed. The integration worktree remains active.
Root-owned `mi300x` stages `/tmp/fe2-mi350-engineering.B01UxK` (13 GiB at the
last check) and `/tmp/ferric-mi350-integration.2WTV7c` (1.9 GiB at the last
check) still require final evidence recovery and cleanup. No other user's
processes, directories, or networking settings were modified.

## Historical M1 Checkpoint

| Team | Implemented And Checked | Current Work |
| --- | --- | --- |
| Integration and runtime | This checkpoint used published fe2o3 `0ea54ed921cbef5fb2171f4ed7a25b9c1c5e2b71`; all 15 lockfiles, dependency records, and the rebuilt source-pin adapter binding passed independent review without unrelated registry-version drift. Resident storage preparation, explicit production-owner close and recovery, and settlement allocation tests are integrated. All 23 reference unit tests pass. Verified full-block SHA updates and opt-in startup phase diagnostics are integrated. Combined host checks at `a6a3f2e` and the 32-token Qwen comparison pass. | Broaden the numerical workloads and profile remaining token latency. Authenticated serving still requires the protected authority bundle. |
| Kernels | Native Qwen `[N,K]` weight layout correction; K3 paged-KV writes copy active rows instead of scanning every physical page. Generic dynamic GridExclusive projection and singleton-invocation race handling are reviewed and published in fe2o3 main. The matching `0ea54ed9` 12-kernel aggregate emits successfully with exact replay and passes independent review. | Measure K3 with the existing independent reference; investigate matrix-instruction support for the remaining dense projection work. No controlled K3 speedup has been measured. |
| Verification | The complete developer qualifier passed on frozen `bb50d0e`: 1,586 selected Verus queries, zero errors, 694 admitted bodies, actual-body negative mutations, and all listed formatting, strict Clippy, debug/release and all-feature test gates. The receipt's 197 files pass hash validation. This checkpoint excludes the later SHA optimization, diagnostics, and `0ea54ed9` repin. The separate planner repair at `eb219f0` passes the positive plan and dependency rejection cases against `0ea54ed9`. Independent review accepts the new 32-token engineering observation. | The protected M1 qualification gates remain outside the developer receipt, the planner's synthetic policy tests, and this single-prompt hardware check. |
| Documentation | Public Pages site describes architecture, implemented functionality, limitations, and milestone status. | Publish the latest tested integration checkpoint without claiming a new benchmark or closed M1 gate. |

## Qwen Observations

The current Qwen3-8B target-only run completed 32 generated tokens from the
five-token prompt `The capital of France is`. Every generated ID matched both
frozen Hugging Face reference passes and both reference argmax arrays. The
decoded output continues through Paris, Rome, Madrid, Berlin, and Amsterdam.
Observation SHA-256 is
`d894caf042156abf21436c98fa3de7d40af124ba7374baa0b879bf7df582af44`.

| Measured Boundary | Seconds |
| --- | ---: |
| Host setup through KFD binding | 82.938931 |
| Memory initialization, allocation, and upload phase | 163.126609 |
| Cumulative setup | 246.065540 |
| Controller entry to first generated token | 13.442289 |
| Average post-first interval, 31 intervals | 2.769105 |
| Controller through completed teardown | 100.370762 |
| Whole process | 346.46 |

Startup diagnostics use `std::Instant`; controller events use
`CLOCK_MONOTONIC_RAW`. Arithmetic combining those clocks is approximate.
This is one shared-host engineering observation, not numerical or serving
qualification, a controlled K3 speedup, or a vLLM/SGLang comparison. The output's
`r33_tpot_eligible=true` expresses event-count/timing eligibility only; all
authority fields remain `none` or false and `benchmark_comparable=false`.
GPU 3 subsequently returned to its exact pre-run VRAM baseline. The immediate
post-exit counter was transiently higher; its mechanism was not established.

The earlier target-only Qwen3-8B smoke produced:

```text
Prompt: The capital of France is
Output:  Paris. The capital of Italy is Rome
```

That eight-token engineering observation recorded approximately 13.65 seconds
to the first token and 2.77 seconds per subsequent token. It is not serving
qualification or a result comparable to vLLM or SGLang. No K3 speedup has yet
been measured.

An independent offline Hugging Face Qwen3-8B run using the same canonical model
produced exactly the same eight token IDs on two passes. A separate 32-token
reference also repeated identically and preserves that eight-token prefix.
The current Ferric run matches that reference across logical KV positions 16
and 32. These are not physical page boundaries (the KV page size is 256).
These short reference checks do not establish full-model numerical qualification.

The artifact used for this Qwen observation, compiled with `0ea54ed9`, has content ID
`d33933adf7f5dfe0a9aa4aba0cc4cb3909b5aa35f9bfb65db7af9af4a5c5bb40`
and HSACO SHA-256
`33d754aaa10292fa37e974eb004b6c52067a141dcd5b58ed080023c6bf315c2d`.
Its 12-kernel roster and compiler replay pass, with authority explicitly
`none`. Ferric device source is identical between emitted `9392755` and
integrated `eb219f0`; the matching release host is built from `a6a3f2e`.
The previous `42882993` artifact, content ID `528fa128`, is retained as
historical evidence and is not relabeled as the current artifact.

All eight shared GPUs were occupied before the September 8 launch attempt.
On September 9, GPU 3 was idle with almost all memory free, allowing the new
32-token attempt. No other user's process was stopped or modified.

An earlier K3 attempt exceeded a 240-second whole-process limit without a
result. Historical successful release runs took approximately 24-27 minutes
for the full process, but their source checkpoints (`6239bdb`, `f151965`) pin
fe2o3 `1ddcd36b`, before the parallel device-initialization change in `d211c9a`.
That older path copied serially and hashed GPU-mapped memory. Those wall times
are not a baseline for the current `42882993`/`0ea54ed9` initialization path.
The historical executable hashes were not archived, so those figures cannot
support a controlled comparison with the new phase-separated measurement.

A later non-dispatch probe of the `42882993` host and final artifact took
98.23 seconds through artifact admission, CPU preparation, runner binding, KFD
admission/topology, and intentionally failing device selection. It stopped
before `initialize_memory`, so it measured no HBM allocation/upload, dispatch,
tokens, TTFT, or TPOT. The two independent model-authentication passes run in
parallel. Setup is excluded from the controller timing above. The earlier
timeout is not a TTFT measurement or a demonstrated kernel failure.

A separate CPU-only probe of public `0ea54ed9` KFD descriptor constructors
measured 58.133 seconds of hashing for 97.313 GiB of input, plus 10.370 seconds
reading weight files. Repeated weight-descriptor calls model the second hash
payload, not the actual private allocation-time validation. This uses KFD's
registry `sha2 0.11.0`, not Ferric's verified SHA implementation. The helper
opened no KFD/render device and measured no HBM initialization or token timing.
It is a one-run engineering observation, not a startup improvement claim.

Both engineering smoke tools now accept exact opt-in
`FERRIC_M1_ENGINEERING_STARTUP_PHASE_DIAGNOSTICS_V1=1`. Six fixed cumulative
completion phases go to stderr only; stdout observations, admission, and
controller timing remain unchanged. The output explicitly grants no authority
and is not benchmark-comparable. The diagnostics passed 10 speculative and 17
target unit tests (one real-GPU test ignored), 20 source-policy tests, formatting,
strict release Clippy, and independent review before integration.

The verified SHA update preserves the existing streaming specification and
passes whole-crate Verus verification (296 queries, zero errors), 143 release
unit tests, strict Clippy, and independent source review. Five alternating CPU
benchmark matrices per build measured 1.115-1.119x throughput for 1 MiB update
chunks, with a 0.49-1.53% time regression for 63/65-byte chunks. This is not a
measured total-startup improvement. All seven regenerated source/dependency
records remain exact, with 171 modules and 8,207 executable bodies.

The combined `a6a3f2e` host check also passes 10 speculative, 17 target, and 20
source-policy release tests (one real-GPU test ignored), strict release Clippy,
formatting, and both release builds. Source-gate tests pass 36/36, and all seven
fresh records match. The later `eb219f0` changes only the planner and its tests;
its actual-history policy test accepts the 354-slot plan and rejects missing,
extra, moved, and incorrectly pinned promotion dependencies.

The reference package now pins Accelerate 1.14.0 and psutil 7.2.2, the actual
dependencies needed by the pinned Transformers loading path. Its exact-version
and virtual-environment provenance checks include both dependencies. The
existing 27 external lock records were preserved.

The new optimized prepack reproduces canonical bundle ID
`6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b`.

## Remaining Dependencies

- The complete developer receipt binds only `bb50d0e`/tree
  `7f3f23579b0adfac8ad7357fbf1246ab4b05dc95`, not the current tree. Receipt
  SHA-256 is `c09f212e82eace9326a6d0a0e47ff7a898a8901e5e531fa01ea52213b064b54b`;
  its unchanged source closure contains 711 files. The earlier `cb9b5ca` run
  remains an incomplete historical run that hit its 600-second cold-build
  bound. Neither receipt establishes GPU machine-code correctness or serving
  qualification.
- Authenticated serving qualification still requires the protected current
  record, verifier, model, and artifact authority bundle.
- Kernel performance work still includes matrix instructions for dense
  projections, attention, normalization, and logits reductions. Source analysis
  found 545 dispatch packets per target decode token; this is not a measured
  attribution of GPU time.
- Physical radix prefix reuse belongs to M2. Symmetric memory and MTP remain
  deferred.

All builds and tests run on `mi300x`. Completed temporary build directories and
integrated worktrees are removed; retained evidence and branches identify the
work without keeping duplicate build trees. Compiler/runtime changes belong
in fe2o3; kernels and inference remain in Ferric.
