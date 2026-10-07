# Setup-Only BF16 Transpose

The MFMA projection setup in `src/tp_execution/projection.rs` retains the
authenticated `NxK` weight shard and produces a separate, exact `KxN` resident
copy. The optimized implementation gathers each source row through the existing
verified `source_row_bytes` interval and stages 32x32 raw-byte tiles in 2 KiB of
stack scratch. Source fetches and destination stores are contiguous within each
tile. At most 32 row references are additionally retained on the stack. No BF16
conversion, rounding, finiteness test or other arithmetic is introduced.

Source section hashes, model/shard identity checks, output allocation bytes,
GPU upload chunks, rank orientation, projection selection and kernel dispatch
geometry are unchanged. The previous temporary heap row is no longer needed.
This is host setup optimization only; it does not change an emitted kernel.

## Host Test

Run only on the designated remote build host, with an owned private target and
at most two build jobs. The full-array test is ignored during ordinary tests
because its largest case contains about 5 GB of array payloads. Reserve at least
6 GiB for the process; this is not a measured peak-RSS bound.

```sh
CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 cargo +1.97.1 test --locked --release \
  --features tp-batch-engineering --lib \
  tp_execution::projection::tests::production_shard_transpose_exactness_and_host_microbenchmark \
  -- --ignored --nocapture
```

The test uses the actual target model's authenticated TP plan for all eight
projection kinds, all ranks at TP1/TP2/TP8, and rank-zero global heads. Each of
the 80 matrix cases is compared in full against the original row-copy/scatter
implementation three times, with alternating call order. The ordinary tests
also cover nonaligned tile tails, every one of the 65,536 BF16 bit patterns,
invalid extents, unchanged rejected destinations, and unchanged projection
selection. No GPU or model inference occurs in these tests.

Each `FerricHostTransposeMicrobenchmarkV1` JSON record contains shape, rank,
repeat, exact output-byte count, baseline/tiled nanoseconds, and an explicit
`model_timing: false`. Timers include the actual shard-row validation and
transpose loops. They exclude source creation, output/row allocation, full-array
comparison, section hashing, upload and inference.

## Observed Helper Result

The 4049ad5 source passed 240 full-array comparisons and strict all-target Clippy
on `mi300x`. Summing the per-case medians of three repeats gives the following
host-only suite results. Each suite contains one matrix for each projection
kind and rank, not all 36 model layers.

| Layout | Cases | Original Helper | Tiled Helper | Helper Speedup |
| --- | ---: | ---: | ---: | ---: |
| TP1 | 8 | 2.6667 s | 0.7903 s | 3.37x |
| TP2 | 15 | 2.6441 s | 0.7866 s | 3.36x |
| TP8 | 57 | 1.9203 s | 0.7205 s | 2.67x |

All 80 per-case median comparisons improved in this run, ranging from 1.54x to
8.58x. The full 151936x4096 head was about 2.18x faster. Earlier direct-tile and
contiguous-store-only experiments were retained separately; the latter
regressed some shapes and is not the selected implementation.

These observations are from a shared host without CPU affinity pinning or an
exclusive CPU reservation. They do not establish model setup duration, TTFT,
TPOT or throughput gains. An integrated model run must measure those separately.
