# Qwen3-8B Currentness Ablation Demo

This is a retained-evidence demo of Qwen3-8B BF16 tensor parallelism across two
gfx950 GPUs on `mi350`. It shows a bounded host-overhead reduction, not a chat
service, accepted full-model decode or a 700 tokens/s result. The measured
workload is 40 authentic prompt forwards through all 36 layers, with **zero
generated tokens**.

## Show Now

Open the published [Census ablation report](../qualification/guarded-mlp-scoped-capacity-census-v1/matched-timing-report-v1/README.md).
Bank V2 is the control; Census V3 additionally moves two zero-add allocation
preflights inside each existing warm-layer currentness window. Both cases use
the same parent and worker binaries. Arithmetic and capacity limits are
unchanged. Full entry/exit checks remain, but temporal equivalence to repeated
full discovery is explicitly not claimed.

| Parent wall interval | Bank V2 (s) | Census V3 (s) | Observed change |
| --- | ---: | ---: | ---: |
| First use, positions 0-1 | 25.999121 | 26.194714 | +0.752305% |
| Warm, positions 2-39 | 109.162388 | 72.475772 | -33.607378% |
| Complete parent, including setup and shutdown | 480.325641 | 445.273315 | -7.297617% |

These are overlapping intervals from one ordered pair, not additive gains,
repeated samples or confidence intervals. Parent wall time includes host checks
and waiting; it is not GPU kernel time, overlap evidence or token throughput.

## Demo Sequence

1. State the workload and open the [original pair record](../qualification/guarded-mlp-scoped-capacity-census-v1/matched-timing-gpu-v1/README.md).
   Each case completed one attempt and 11 naturally retired supervised phases,
   with healthy Close and retained process/device postchecks.
2. Show the [per-position frame waits](../qualification/guarded-mlp-scoped-capacity-census-v1/matched-timing-report-v1/waits.svg)
   and [parent/forward groups](../qualification/guarded-mlp-scoped-capacity-census-v1/matched-timing-report-v1/groups.svg).
   Include the first-use regression and complete-parent result, not only the
   warm interval. Use the [category table](../qualification/guarded-mlp-scoped-capacity-census-v1/matched-timing-report-v1/table.md)
   and [integer group CSV](../qualification/guarded-mlp-scoped-capacity-census-v1/matched-timing-report-v1/forward-groups.csv)
   when inspecting the numbers.
3. Show the parity boundary: all 40 semantic records and four complete
   606,976-byte observation payloads at positions 0/5/16/39 match each other
   and the historical baseline. Capture files also contain control/timing
   fields; whole-file byte equality is not claimed. Same-side parity is not
   independent framework acceptance.
4. Optionally run the exact [data-only reproduction commands](../qualification/guarded-mlp-scoped-capacity-census-v1/matched-timing-report-v1/README.md#reproduce-the-report)
   on Linux with Python 3. They authenticate the published original archive,
   retain a fresh original-only capsule and regenerate the report without
   running the model or GPU. Do not pass the commentary-augmented published
   capsule directly to the strict reporter. [Provenance](../qualification/guarded-mlp-scoped-capacity-census-v1/matched-timing-report-v1/provenance.json)
   binds the report source, verifier and original archive.
5. Finish with the open gates below and the [current progress record](GFX950_FINITE_PREFIX_PROGRESS_V1.md).
   This demo requires no new native launch or benchmark run.

## Tail V4 Follow-Up

The [Tail proposal and qualification record](../qualification/guarded-mlp-scoped-tail-v1/README.md)
are a separate next step, not part of the measured Census result.

| Gate | Status at this draft |
| --- | --- |
| Runtime/worker CPU qualification | Passed: 1,180 runtime tests plus 8 ignores; 820 worker tests plus 4 ignores; 27 phases |
| Strict data checker | Passed: 114 synthetic tests |
| Parent CPU qualification | Not qualified: first attempt failed during compilation at phase 5; no V4 GPU run |
| Fresh same-binary Census V3 / Tail V4 native pair | Pending; no result or gain predicted |
| Tail report and independently checked publication | Pending actual pair |

The parent failure is a shared test's backend-specific module reference. The
[test-only repair passed a fresh complete runtime/worker qualification](../qualification/guarded-mlp-scoped-tail-v1/cpu-attempt-v2/README.md)
and is integrated; the fresh parent retry remains pending.
CPU tests and synthetic admission do not establish native correctness or
performance. Tail is a distinct Readiness40 route, not Full2303 activation.
Keep pending entries until original receipts and actual comparisons are
retained; publish failures and regressions without substituting projections.

## Still Open

The [independent position-5 diagnostic](../qualification/guarded-mlp-readiness40-position5-v1/README.md#captured-result)
still differs: the framework ties tokens 2 and 9112 at 19.625 and selects 2;
Ferric records 9112 at 19.75 and token 2 at 19.625. This is a prompt-position
diagnostic, not an own-generated-token result. The currentness ablation neither
fixes it nor changes numerical acceptance thresholds.

Native full-workload feasibility, actual long-page/KV transitions and own-output
history remain to be demonstrated within the existing one-hour abort bound.
The target is single-request, BF16, target-only decoding with 2,048 prompt tokens
and 256 generated tokens, including the unchanged exact independent 256-ID/raw
decoded-byte gate. Full CPU and reference qualifications are prerequisites,
not native acceptance. The 700 tokens/s target remains unmet.

Repeated equal-work performance, GPU timing/overlap and an equivalent NVIDIA
comparison are not established here. TP2 does not satisfy the issue's original
single-GPU matrix. **Issue #42 M0-M7 all remain OPEN.** See the
[performance protocol](GFX950_DECODE_PERFORMANCE_V1.md) for workload and
measurement boundaries.
