# M1 Team Progress

Updated: 2026-09-09. This is an implementation checkpoint, not a qualification
receipt. The 33 M1 roadmap gates remain open.

| Team | Implemented And Checked | Current Work |
| --- | --- | --- |
| Integration and runtime | Active fe2o3 dependencies pinned to published `0ea54ed921cbef5fb2171f4ed7a25b9c1c5e2b71`; all 15 lockfiles, dependency records, and the rebuilt source-pin adapter binding pass independent review without unrelated registry-version drift. Resident storage preparation, explicit production-owner close and recovery, and settlement allocation tests are integrated. All 23 reference unit tests pass. Verified full-block SHA updates and opt-in startup phase diagnostics are integrated. Combined host checks at `a6a3f2e` and the 32-token Qwen comparison pass. | Broaden the numerical workloads and profile remaining token latency. Authenticated serving still requires the protected authority bundle. |
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

The current engineering artifact, compiled with `0ea54ed9`, has content ID
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
