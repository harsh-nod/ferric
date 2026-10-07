# M32 Gate/Up Experiment

Default-off source prototype for Qwen3-8B TP1 prefill gate and up projections.
There is no serving integration or useful measured speedup. Remote host tests,
strict Clippy, engineering emission, ISA inspection and guarded native component
parity pass. The measured prototype remains disabled.

The `prefill32-m2-gate-up-r1` feature selects one fixed gfx950 Wave64 kernel:
32 rows, N12288, K4096, gate/up roles only. Each workgroup computes two M16/N16
tiles with independent FP32 accumulators and the same ascending K16 order as
the existing V5 path. The 768-workgroup grid replaces 1,536 M16 workgroups.
Two adjacent M16 typed output views cover the existing BF16 output allocation;
no output copy, merge kernel or additional weight layout is required.

Both consuming matrix operations receive separately loaded B fragments. This
respects the SDK's non-Copy fragment API. The inspected image duplicates the B
loads: no register reuse or weight-bandwidth reduction is established. Final finite
checks and BF16 rounding remain explicit for each output component.

The host contract rejects partial rows, wrong geometry, padded extents, output
aliases and inexact launches. Host and parsed-source tests cover output ownership,
address mapping, BF16 conversion and arithmetic order. All 24 tests pass in each
default/opt-in profile on mi300x-2, as does strict all-target Clippy. These checks
do not substitute for emitted ABI admission or native numerical tests.

Image `bb164d23d55dec43e55b8d8cce936929a3cc749a29486bdcfff8bfe34ac7d5dc`
uses SDK `55c1a9b6` with the retained historical compiler plus published fixes,
not a rebuilt current-main compiler. Inspection finds the expected 344-byte
argument segment and no reported spills; diagnostic capture is omitted-ineligible.
This README update is not part of the original frozen ten-file emission closure.
Detailed provenance is retained in `docs/M1_PERFORMANCE_SWARM_V14.md`.

The native component campaign passes all 16 guarded outputs after 71 harness
CPU tests and actual-artifact binding. Across three ABBA blocks, V5 versus M32
host medians are 0.469880 versus 0.472145 ms (0.482% slower); worker elapsed
medians are 0.360295 versus 0.3607755 ms (0.133% slower). Only one of six adjacent
pairs favors M32. These are hot-buffer component timings, not shader time or
model TTFT/TPOT; the images also have different compiler histories. Independent
archive, protocol, numerical-reference and cleanup review passes. No client
integration is justified by this result.

Qualification order:

1. Remote formatting, locked default/opt-in tests and strict Clippy on mi300x-2.
2. One-root emission with retained compiler/SDK provenance and exact ABI review.
3. Symbol-scoped ISA inspection for B-load reuse, register use and spills.
4. Guarded native component parity and alternating uninstrumented A/B timing.
5. Only after a repeatable gain, explicit client integration and full-model
   token parity, TTFT/TPOT measurement, then a fresh matched HTTP comparison.

All current decode, partial-row and serving defaults remain unchanged. The
compiler source and KFD runtime belong to fe2o3; this kernel stays in Ferric.
