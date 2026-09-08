# M1 Team Progress

Updated: 2026-09-08. This is an implementation checkpoint, not a qualification
receipt. The 33 M1 roadmap gates remain open.

| Team | Implemented And Checked | Current Work |
| --- | --- | --- |
| Integration and runtime | Active fe2o3 dependencies pinned to published `0ea54ed921cbef5fb2171f4ed7a25b9c1c5e2b71`; all 15 lockfiles and dependency records are refreshed without unrelated registry-version drift. Resident storage preparation, explicit production-owner close and recovery, and settlement allocation tests are integrated. All 23 reference unit tests pass. Verified full-block SHA updates are integrated at `63e0f7f`. | Complete independent review of the latest fe2o3 cast semantics, bounded worker teardown, and refreshed dependency custody; then emit a matching aggregate. Measure device initialization separately when a shared GPU is available, then run the 32-token Qwen comparison. |
| Kernels | Native Qwen `[N,K]` weight layout correction; K3 paged-KV writes copy active rows instead of scanning every physical page. Generic dynamic GridExclusive projection and singleton-invocation race handling are reviewed and published in fe2o3 main. The `42882993` aggregate emits successfully and passes independent review. | Emit and review an aggregate for the active `0ea54ed9` pin. Measure K3 with the existing independent reference; investigate matrix-instruction support for the remaining dense projection work. No K3 speedup has been measured. |
| Verification | Resident same-shape settlement test measures zero allocations/reallocations for one round and 16 repeated rounds; positive control allocates. Earlier engine library: 636 passed, 9 ignored; doctests: 171 passed. Source-gate unit tests: 36 passed. All ten positive proof packages passed at cb9b5ca. The 9dcbd6e component matrix passed formatting, strict Clippy, debug/release and all-feature suites, and worker behavior tests. Stale S1/K4 policy anchors and the source-pin adapter binding were subsequently repaired and independently reviewed. | Full qualification is running on frozen `bb50d0e`, with a 1,800-second per-proof timeout. That checkpoint excludes the later SHA optimization. No complete receipt has been produced. |
| Documentation | Public Pages site describes architecture, implemented functionality, limitations, and milestone status. | Publish the latest tested integration checkpoint without claiming a new benchmark or closed M1 gate. |

## Qwen Observations

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
It will support a Ferric comparison across logical KV positions 16 and 32;
the 32-token Ferric comparison has not yet run. These short reference checks
do not establish full-model numerical qualification.

The `42882993` compiler/runtime pin and K3 artifact remain the latest emitted
bundle. The active `0ea54ed9` source pin is being validated; a matching artifact
has not yet been emitted.
All eight GPUs on the shared MI300X host became occupied by another user's
training job before launch, so the run is waiting for a GPU window. No other
user's process was stopped or modified.

The final engineering artifact has content ID
`528fa128e398b9aac5f5fa672388b44ff7b7e67932332abbb61d7e9704715d7a`
and HSACO SHA-256
`cf786f800b818a1771c32bd9aa3eb2fe8daf56c625177aa193d6406eab033804`.
Its 12-kernel roster and compiler replay pass, with authority explicitly
`none`. The latest release host admitted this artifact in a pre-model-loading
check; that check intentionally stopped at a missing model snapshot and did
not execute GPU work.

An earlier K3 attempt exceeded a 240-second whole-process limit without a
result. Historical successful release builds needed approximately 24-27
minutes for the full process. A later CPU-only probe of the `42882993` host and
final artifact took 98.23 seconds through artifact admission, CPU preparation,
runner binding, and intentionally failing device selection. It stopped before
`initialize_memory`, so it measured no HBM allocation/upload, dispatch, tokens,
TTFT, or TPOT. The two independent model-authentication passes run in parallel.
Their cost must not be conflated with the unmeasured device-initialization
portion of historical full-process timings. Setup is excluded from the
controller timing above. The earlier timeout is not a TTFT measurement or a
demonstrated kernel failure.

The verified SHA update preserves the existing streaming specification and
passes whole-crate Verus verification (296 queries, zero errors), 143 release
unit tests, strict Clippy, and independent source review. Five alternating CPU
benchmark matrices per build measured 1.115-1.119x throughput for 1 MiB update
chunks, with a 0.49-1.53% time regression for 63/65-byte chunks. This is not a
measured total-startup improvement. All seven regenerated source/dependency
records remain exact, with 171 modules and 8,207 executable bodies.

The reference package now pins Accelerate 1.14.0 and psutil 7.2.2, the actual
dependencies needed by the pinned Transformers loading path. Its exact-version
and virtual-environment provenance checks include both dependencies. The
existing 27 external lock records were preserved.

The new optimized prepack reproduces canonical bundle ID
`6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b`.

## Remaining Dependencies

- The cb9b5ca full qualifier completed 1,586 selected-package verification
  queries with zero errors and 694 admitted executable bodies. It then stopped
  at the 600-second limit while compiling a cold negative-test target; the
  remaining quality gates did not run in that invocation. This is incomplete
  qualification, not a passed receipt or a semantic proof failure.
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
