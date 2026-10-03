# Finite Qwen3 Prefix Progress

This is an engineering checkpoint for [issue #42](https://github.com/harsh-nod/ferric/issues/42),
recorded on 2026-10-03. It is not a production admission, a sustained decode
benchmark, or a claim that the 700 tokens/s target has been reached. All issue #42 M0-M7
milestones remain open. The checkpoint is being published incrementally on an
engineering branch; it does not change the production execution path.

## Observed Results

| Check | Observed result | Boundary |
| --- | --- | --- |
| Corrected finite parent/worker CPU suite on `mi350-2` | 605 passed, 4 ignored; 34 owned commands | Includes 24 tests for the opt-in host-observation extension; no GPU execution |
| Historical KFD host-observation suite on `mi350-2` | 1,776 selected tests passed | Overlapping selections, including 15 new tests; separate source generation |
| Four teacher-forced forwards on `mi350` | 152 retained native tensor rows bitwise equal to the prior native route | All 36 layers, TP2, finite engineering images; not an independent full-model numerical bound |
| Four autoregressive forwards on `mi350` | 152 retained native tensor rows bitwise equal to the prior native route | Own-token history; no 2,048/256 workload or sustained throughput measurement |
| Six prefix-stage numerical cases | All 24 rank/profile rows satisfy the declared conditional norm/QKV bounds | Independent conditional arithmetic check of retained captures, not a new GPU run |

Both four-forward runs completed their Close protocol, reaped their owned
processes, and passed their six surrounding device-state audits. The CPU suite
also completed with no forced cleanup. The newer instrumented parent/worker
binaries have **not** yet completed their GPU observation run; do not attribute
the earlier four-forward GPU results to those binaries.

The six numerical cases cover genuine positions 0 and 4, plus patterned
positions 15, 16, 2,047 and 2,048. They check first normalization, QKV projection,
head normalization, split-half RoPE and exact current-value append. The checks
are conditional on captured preceding-stage values and the declared floating
point premises: at most one FP32 ULP for the square-root sequence and correctly
rounded FP32 division. A universal proof of these instruction sequences is
still open. The candidate cap and numerical policy were not loosened to obtain
these results. The older attention/output-projection check remains separate.

## Changes And Retained Failures

The finite route uses an explicit bounded parent/worker protocol, typed
generation and bank checks, and ownership-bound shutdown. Host observation is
opt-in and keeps operational optimizations off. Its nested counters measure
host activity, not GPU elapsed time, kernel overlap, or tokens/s.

A parent compile failure exposed a `u32` image-size versus `u64` request-size
comparison. The correction widens the image size with `u64::from`, and adds
regressions rejecting oversized requests with identical low 32 bits. The
corrected 605-test run is a new result; the failed run is retained.

Portable-deployment tests also exposed aliasing in a cached test fixture.
The test-only correction deep-copies the fixture and adds a mutation-isolation
regression: all 23 tests passed. The production reader and exporter were
unchanged. The earlier 22-test run remains a failure, not a passing result.

## Evidence Identities

These are SHA-256 identities of retained receipts, not links to publicly
downloadable artifacts. Large logs, captures, model weights, compiler caches
and runtime binaries are deliberately excluded from Git. A receipt digest
alone is not a self-contained reproducer or proof.

| Retained receipt | SHA-256 |
| --- | --- |
| Corrected CPU605 completion | `f9b4da95d00cb62a3eadb33d86ee942d0bdf0ecb5c559650161846f4dcd081e8` |
| Corrected CPU605 owner | `dde383021440a721bd2c41bff614ea1156a3b36c6fa48cd2ad27c95337af7cb2` |
| Historical KFD completion | `e6cdd55c6a2dda4dafdd25a4c8d5e3670312c912cdbc1f20258a6e93db48e603` |
| Teacher-forced four-forward completion | `56f72c0c8c76e7c2c3f00ca767d5a7b53c66ba4b2c17589059565c2c2532fb8b` |
| Autoregressive four-forward completion | `0553862c7e65b74020028421b3bb668ff6eaf0080ba4a1bc405587707a403d77` |
| Numerical case: genuine position 0 | `a1990e1f8153aa8df23a8db175e293bff7d17b0f3c7fc181b223158df9a41586` |
| Numerical case: genuine position 4 | `81d9b3b17d7763b5c9d40db023c6ba1cf0eb61c61d429d1f1214fbef55475adb` |
| Numerical case: patterned position 15 | `fa7795a3ff7968c212600e2ba30dca7c39b4ccf29ac2b075d42a18377b66fa9e` |
| Numerical case: patterned position 16 | `39ae8374a10ea306e0f1426c87e9c3184f04de250e9f82695973158ad941e1e4` |
| Numerical case: patterned position 2,047 | `7cb4e5ee27e423b96f2eae3fbb2518e14da4f546066cdb08bf5d251f2df32fca` |
| Numerical case: patterned position 2,048 | `676d7e878105972365b15c9aba5f5361825a7761e04f761178487271fae491de` |
| Portable test-only correction, 23 tests | `0481e077defafed91c2272984b3d5d1da5d97cb9da9431ba307760000698de8a` |

## Next Gates

1. Publish the exact tested Ferric source closure with explicit sibling fe2o3
   dependencies, without importing unrelated worktree changes.
2. Deploy and audit the instrumented binaries, then run the bounded host
   observation on `mi350`. Use it to locate overhead before changing execution.
3. Close independent numerical obligations and validate the longer resident
   request, multiple requests, KV lifetime and cleanup boundaries.
4. Qualify the full target workload: single-request Qwen3-8B, BF16,
   target-only decoding, 2,048 prompt tokens and 256 generated tokens. Report
   post-first-token throughput as `255 / (last_delivery - first_delivery)`.
5. Run equal-work baselines and paired optimization ablations. Host-inclusive
   diagnostics are not GPU overlap graphs or a qualified vLLM comparison.

The selected TP2 experiments do not satisfy the issue's original single-GPU
matrix. No M0-M7 milestone is closed by this checkpoint. Assurance properties
without closed, identity-bound proof remain `Contracted` or `Unsupported`, as
required by [the proof policy](PROOF_DEVELOPMENT.md).
