# M1 Team Progress

Updated: 2026-09-08. This is an implementation checkpoint, not a qualification
receipt. The 33 M1 roadmap gates remain open.

| Team | Implemented And Checked | Current Work |
| --- | --- | --- |
| Integration and runtime | Active fe2o3 dependencies pinned to public main `d9f6bbcd089fb4bf7980f807e6550b13daed9178`; resident storage preparation, explicit production-owner close and recovery, and settlement allocation tests integrated. | Refresh source/dependency records and rerun Qwen using the integrated kernel artifact. |
| Kernels | Native Qwen `[N,K]` weight layout correction; K3 paged-KV write changes from a full physical-page scan to copying active rows. The K3 candidate passes the d9f compiler check and focused host/device tests. | Resolve the independent review's global workgroup guard finding, validate cross-page multi-sequence indexing, and emit a fresh gfx942 artifact. |
| Verification | Resident same-shape settlement test measures zero allocations/reallocations for one round and 16 repeated rounds; positive control allocates. Engine library: 636 passed, 9 ignored. Engine doctests: 171 passed. | Source-gate integration checks, independent K3 review, and final combined-tree qualification. |
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

The new optimized prepack reproduces canonical bundle ID
`6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b`.

## Remaining Dependencies

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
