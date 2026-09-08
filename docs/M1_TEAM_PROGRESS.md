# M1 Team Progress

Updated: 2026-09-08. This is an implementation checkpoint, not a qualification
receipt. The 33 M1 roadmap gates remain open.

| Team | Implemented And Checked | Current Work |
| --- | --- | --- |
| Integration and runtime | Active fe2o3 dependencies pinned to public main `d9f6bbcd089fb4bf7980f807e6550b13daed9178`; resident storage preparation, explicit production-owner close and recovery, and settlement allocation tests integrated. Reference model-loading dependencies are pinned and checked; all 23 reference unit tests pass. | Integrate the reviewed compiler fix and rerun Qwen against the independent reference. |
| Kernels | Native Qwen `[N,K]` weight layout correction; K3 paged-KV write changes from a full physical-page scan to copying active rows. The integrated K3 series passes independent review, d9f structural compiler checks, and cross-page multi-sequence indexing tests. | Hardware artifact emission exposed missing dynamic GridExclusive projection and singleton-invocation race analysis in fe2o3. The compiler team is implementing and testing those generic capabilities. |
| Verification | Resident same-shape settlement test measures zero allocations/reallocations for one round and 16 repeated rounds; positive control allocates. Engine library: 636 passed, 9 ignored. Engine doctests: 171 passed. Source-gate unit tests: 36 passed. All ten positive proof packages passed at the cb9b5ca checkpoint. | Complete component quality gates, then rerun the full qualifier with an explicit longer timeout on the updated compiler pin. No complete receipt has been produced. |
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
