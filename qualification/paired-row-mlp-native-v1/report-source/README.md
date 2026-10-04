# Retained Down2 Raw-Clock Comparison

Data-only draft, with twelve authored synthetic policy tests. No imports, tests,
reports or native/GPU jobs have been run by the author. The actual candidate
completion SHA is deliberately an argument; no future result is assumed.

The root-owned invocation on MI350 is:

```sh
python3 -B report.py BASELINE_COMPLETE BASELINE_SHA CANDIDATE_COMPLETE CANDIDATE_SHA NEW_REPORT_DIRECTORY
```

Inputs must be actual canonical retained paths. The baseline completion is fixed
to the genuine 902,162-byte clock TF4 receipt `e34189597dc7db7a7c325040f5381932e84390c1a2cf32055830858cb9278ddb`.
The candidate must be a successful, one-attempt, closed Down2-clock result with
six audits, all four captures and all 152 tensor rows checked. Its request must
select the actual 33,112-byte `65a76f917c12476f4814174ee39d515642a643a0352c6609ee514955a6a97449`
Down2 image. The renderer verifies both complete/request/sidecar/image pins,
unchanged workload and runtime, and all 288 per-case MLP dispatch image bindings.

The reader also checks exact 1172 packet coordinates, [592,580] per-rank totals,
group/queue identity, profile/transcript/child identity, and ordered nonzero tick
endpoints. Zero tick deltas remain in the data. It reuses the retained clock
sample and plot functions from `E/p228-device-clock-report-v1/report.py`, SHA256
`2dff8e703bde0e2370ada6190d635e023f4929ec502a70d8a536979d8a7898d3`.
This is reused data-processing code, not a new runtime qualification framework.

Six fresh outputs are emitted: `comparison.json`, `comparison.md`, separate
baseline/Down2 raw-tick range plots, and separate layer-position heatmaps. Tables
show per-rank/per-stage medians, zero counts, and signed arithmetic differences;
JSON also retains minima/maxima and raw same-device clock-counter differences.
Inclusive host intervals are separately labeled in recorded nanoseconds.

Raw tick values have no established cross-run/device frequency or aligned time
origin. There are no ratios, GPU nanosecond conversions, speedup, overlap,
throughput, SOTA, sustained2048/256, or700tokens/s claims. The renderer relies on
the pinned observer's exact-output result and does not rerun independent
numerical validation. All acceptance and performance authority flags are false.

Runtime bounds are 2GiB address space,120CPU seconds,16MiB per file, no core
dumps,8MiB per input/output, and a fresh output directory. The source, pinned plot
helper and all consumed input bytes are rechecked before and after writing.
There is no subprocess, SSH, compiler, device-open or GPU-launch path. Root owns
pure-test execution, the real data replay, artifact retention and publication.
