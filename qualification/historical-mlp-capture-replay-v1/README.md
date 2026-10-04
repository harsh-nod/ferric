# Historical MLP Capture Replay

On 2026-10-04, `mi350` passed **41 CPU tests** and then all **20 independent
conditional MLP stage checks** against retained layer-0 GPU captures: five stages,
two TP ranks and two historical profiles. There were no bound violations.
This is an actual arithmetic replay, not just mocked adapter tests or native
implementation parity. It does not execute a new GPU kernel.

| Check | Rank 0, each profile | Rank 1, each profile |
| --- | ---: | ---: |
| Post-attention RMSNorm | 0 violations / 4,096 values | 0 violations / 4,096 values |
| Gate projection | 0 violations / 6,144 values | 0 violations / 6,144 values |
| Up projection | 0 violations / 6,144 values | 0 violations / 6,144 values |
| SwiGLU | 6,144 exact BF16 values | 6,143 exact; one within the existing one-step bound |
| Down projection | 0 violations / 4,096 FP32 partials | 0 violations / 4,096 FP32 partials |
| Maximum down error / bound | 0.00769993 | 0.01481929 |

The two historical profiles have identical captured MLP arrays, but each is
checked independently against its registered weights and actual inputs. Across
both profiles, 24,574 SwiGLU values are exact and two are one BF16 step away.
The largest down-projection absolute error is `3.2758859447312716e-7`.

## What Is Checked

Each stage uses its **captured immediate predecessor**, not a reference-generated
replacement: first residual to RMSNorm, captured norm to gate/up, captured gate/up
to SwiGLU, and captured activation to the FP32 down partial. This avoids hiding
one stage's error inside a later stage. The separate
[residual replay](../historical-residual-capture-replay-v1/README.md) checks the
ordered TP sum and BF16 residual boundaries.

The existing P218 policy is unchanged: width-4,096/depth-71 gate/up bounds,
width-6,144/depth-103 down bounds, two BF16 normalization roundings, and the fixed
SwiGLU one-step bound. No tolerance was chosen after inspecting these captures.
The original seven fixture files are authenticated and joined to the actual
rank-specific source uploads. Gate/up are row-sharded; down is column-sharded.
The selected 32,328-byte MLP548 image is checked separately from the legacy image.

## Scope And Reproduction

This covers historical V227 layer 0, position 0, token 9112 only. It does **not**
validate the V7 prefix image, all 36 layers, final norm/head, full-model decoding,
GPU coherence or throughput. Arithmetic assumptions remain conditional. The
original GPU controller's `FAILED_UNCHANGED` status is preserved; this later
CPU result does not rewrite it. Authentic fixture bytes were rehashed, but the
original model shard was not re-read during this conditional replay.

- [Actual arithmetic result](actual/mlp.json) and [completion](actual/complete.json).
- [41-test transcript](pure/tests.log): 22 adapter tests plus 19 original arithmetic tests.
- [Test completion](pure/complete.json) and source snapshots record unchanged executed bytes.
- [Exact adapter](source/run.py), [tests](source/test_run.py), and [frozen reference](reference/reference.py).
- [CPU runner](run_cpu.py) and [author-stage invocation details](source/README.md).

Root ran tests and replay separately using Python `-B`, NumPy 2.2.6, CPU affinity
8,9, nice 10, single-threaded BLAS, empty GPU visibility, a 2 GiB address-space
limit, 120 CPU seconds, 16 MiB file cap and a 150-second outer timeout. Both
completed successfully and all source postchecks passed. The publisher also
rehashed both full captures and all 24 selected input/output slices locally.
The frozen source README retains its pre-execution notes; the actual receipts
above supersede those notes. Recorded fixture/evidence paths remain prerequisites.
