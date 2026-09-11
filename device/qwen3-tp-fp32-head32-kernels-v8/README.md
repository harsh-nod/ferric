# TP1 32-Row FP32 Head Candidate

This separately admitted three-root engineering image extends the v7 final
head to allocations with capacity 32. It does not change projection math before
the final head or the frozen v7 profile. The scalar root retains serial FP32
accumulation; the MFMA root retains FP32 accumulators without BF16 narrowing.
Argmax scans finite FP32 values in ascending token order and selects the lowest
token ID for exact ties.

The supported shape is TP1, rows 1 through 32, hidden width 4096, vocabulary
151936, projection tag 6, and Wave64. Scalar weights are `[N, K]`; MFMA weights
are separately transposed `[K, N]`. Head launches use
`ceil(rows / 16) * 9496` workgroups, so rows 17 through 32 execute a genuine
second row tile in the same launch. Checked matrix views zero-fill inactive
MFMA input rows. Typed output stores leave inactive rows untouched.

The closed exports are:

- `ferric_qwen3_tp_batch32_head_bf16_f32_v8`
- `ferric_qwen3_tp_batch32_mfma_head_f32_v8`
- `ferric_qwen3_tp_batch32_argmax_f32_v8`

The host API requires a wide32 driver and separate artifact admission through
`EngineeringTpArtifactV1::open_fp32_head32`. Call
`configure_head_precision_v8(true)` for FP32 or `false` for the explicit BF16
control. The FP32 workspace is exactly 19,447,808 bytes; control adds none.
Baseline and MFMA projection are supported. TP2/TP8, wave attention/projection,
automatic projection selection, dispatch sequences, numerical capture, and
replica benchmarking remain outside this candidate's scope. CLI integration
must reject unsupported combinations before execution.

`tests/contract.rs` checks the closed ABI, unique output ownership for every
row count, bounds, finite checks, and equality between emitted inline scalar
math and the host-tested body. Host numerical tests include real hidden width,
inactive rows, exact ties, and nonfinite traps. Native `tools/probe.py` defines
15 cases: all three roots at rows 1, 16, 17, 31, and 32, with full-vocabulary
analytical references, independent layout checks, immutable inputs, and output
guards/tails. It requires a pinned helper and explicit worker/image identities.
Nonfinite trap tests run only on the host, never by deliberately faulting a
shared GPU.

These are candidate qualification tools, not serving or performance evidence.
Native correctness and complete fixed-reference model checks must pass before
any timing is accepted. Historical v7 receipts must not be relabeled as v8.
