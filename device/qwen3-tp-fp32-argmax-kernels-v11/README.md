# Parallel FP32 Argmax Candidate

This separately admitted, single-root gfx950 candidate replaces only the serial
selection scan when explicitly integrated. It does not change the frozen v8
head, FP32 accumulation, logits, existing artifact admission, or any default.
There is no adapter integration in this crate.

The export is `ferric_qwen3_tp_batch32_wave_argmax_f32_v11`. Its ABI remains
`(&[f32], WriteOnlyDisjointSlice<u32, RowStriped2D<Index1D, 64, 1>>, u32)`:
36 argument bytes, vocabulary 151936, 1 through 32 active rows, and exactly
one 64-lane workgroup per active row. Logits and choices may include inactive
capacity up to 32 rows. The kernel does not read or write that inactive tail.

Each lane scans 2374 coalesced, stride-64 FP32 values in ascending token order.
Three convergent `Gfx950Subgroup::reduce_max_f32::<64>` operations first reject
any nonfinite input anywhere in the row, then find the finite maximum, then
choose the largest exact integer key `151936 - token_id` among equal maxima.
The key range is 1 through 151936, below the exact-integer FP32 limit; zero is
reserved for nonwinning lanes. Lowest token IDs therefore win exact ties,
including either ordering of positive and negative zero. Lane zero alone
publishes the result, after all three reductions. Rejection is per row; a
faulted dispatch must be treated as failed, not as transactional output across
multiple rows.

Host tests use per-lane scan/key macros checked for exact source equivalence
with the expanded device bodies, and separately model reductions; they do not
execute or fabricate subgroup capabilities. Tests cover every lane,
scan boundaries, exact key representation, finite random bit patterns, full
vocabulary, signed zeros, subnormals, FP32 values that narrow to BF16 ties,
nonfinites in every lane, and immutable input/output-tail fixtures. Structural
tests check the closed ABI, convergent source ordering, and output ownership.
These are not native execution, device guard validation, or performance proof.

The dependency is pinned to fe2o3
`21682228486f7186cc3c37ddf165fffc438d8b6a`. Device emission and native qualification
remain separate gates, described in [the probe recipe](tools/README.md).
One wave per row is intentionally bounded and may still limit occupancy at
small row counts. No latency or throughput improvement is claimed before a
matched, correct end-to-end measurement.
