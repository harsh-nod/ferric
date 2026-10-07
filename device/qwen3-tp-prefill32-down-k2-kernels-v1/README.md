# Paired K16 Prefill32 Down Experiment

Default-off candidate. CPU qualification on mi300x-2 passed 18 API/source tests,
18 full-host default tests and 19 full-host enabled-feature tests on October 7,
2026. The path-bound fixtures retained the exact current SDK source; they do
not qualify Git fetching or an emitted device artifact. Strict all-target Clippy
also passed for the full-host default and enabled configurations. No compiler emission, ISA, device
parity, timing or serving improvement is claimed. The SDK pin
is fe2o3 `1736eff451d445f1f194abe145af242cf51ec322`. Generate the lockfile and
run all qualification remotely; no local build is needed.

The explicit `paired-prefill32-down-k16-r1` feature declares two kernels for
exact Qwen3-8B TP1 prefill down geometry: M32, N4096, K12288, projection tag 2.
Both use the existing transposed row-major BF16 KN weights and emit one
contiguous M32/N4096 FP32 partial output. The grid remains 512 Wave64 workgroups,
one M16/N16 tile per workgroup. There is no extra scratch, layout conversion,
split reduction, dispatch or BF16 narrowing. Existing serving routes and all
V5 sources remain unchanged.

The control is a fixed-shape specialization of V5's ascending K16 loop. The
candidate loads two adjacent A/B fragment pairs before performing their two
ordered updates on the same accumulator. It retains all 768 K16 updates while
reducing loop trips from 768 to 384. This exposes adjacent loads to the current
compiler; it does not establish asynchronous overlap, fewer weight bytes,
wider loads, or an improvement until the emitted ISA and timings are reviewed.
Extra live fragments can increase registers and reduce occupancy.

## Evidence and Scope

- Native packet attribution assigns 22.95% of prefill firmware-window time to
  down projection, compared with 21.28% for gate/up. These are not shader-only
  shares: [attribution](../../docs/performance/native-packet-attribution-20261007.md).
- Earlier paired K16 gate/up had matched, same-image device parity and a
  12.26% raw packet-tick reduction, but essentially flat host timing. It is
  evidence for testing the mechanism on longer K12288, not a prediction of
  model TTFT: [raw ticks](../../docs/performance/prefill-k16ticks-component-native-a001-independent-review.json),
  [uninstrumented latency](../../docs/performance/prefill-k16-component-native-a001-independent-review.json),
  [ISA and registers](../../docs/performance/prefill-k16-matched-emission-a001-independent-review.json).
- The generic 15-root K2 emission was refused by race-analysis storage bounds,
  not by native wrong output. This exact-shape two-root crate is a separate
  artifact, not a qualification of that refused image. All compiler checks
  remain enabled: [refusal](../../docs/performance/prefill-k2-current-host-a002-independent-review.json).
- M32 gate/up tiling duplicated B loads and showed no useful host improvement;
  this candidate does not repeat that approach:
  [M32 result](../../docs/performance/prefill-m32-native-a001-independent-review.json).

## Qualification Handoff

1. On mi300x-2, use the current compiler-compatible toolchain and the bounded
   CPU profile to format and generate `Cargo.lock`, then freeze the complete
   source/lock roster. Run `cargo test --locked --no-default-features` and the
   same command with `--features paired-prefill32-down-k16-r1`; repeat both
   profiles with `cargo clippy --locked --all-targets ... -- -D warnings`.
   The 18 default and 19 enabled tests and both strict Clippy configurations
   have passed in path-bound host fixtures;
   managed emission and native qualification below remain outstanding.
2. Emit both roots together with the current compiler, gfx950, Wave64, xnack
   disabled, normal bounds/race checks and engineering artifact custody. Keep
   exact compiler/SDK/source/image identities. Expected explicit arguments are
   three slice pointer/length pairs followed by five u32 scalars. The old
   analogous ABI was 328 bytes with hidden arguments; inspect, do not assume,
   the actual new metadata. The output element type is FP32, unlike the older
   BF16 gate/up component harness.
3. Inspect symbol-scoped ISA: one/two MFMA updates per control/candidate loop,
   768 total updates, same ordered accumulator, scalar FP32 stores, actual
   load/wait placement, SGPR/VGPR/AGPR use, private/LDS bytes and spills.
   Fail closed on mismatched ABI, rejected analysis or unexpected arithmetic.
4. Adapt the existing `experiments/component-harnesses/prefill-k16` campaign
   in a fresh evidence namespace, retaining its admission, guard/readback,
   ownership, lifecycle and source binding checks. Replace all old pins with
   reviewed actual new artifacts, not a name-only substitution. Use the same
   image and worker for both arms, buffers and exact projection tag 2; input
   bytes are 786432, KN weight bytes 100663296, output bytes 524288, grid 512.
   The old harness hard-codes BF16 output and tags 4/5 and cannot be reused
   unchanged. Adapt numerical fixtures and actual metadata validation first.
5. Validate full FP32 output and both input/output guards before timing. An
   independent exact dyadic fixture can use A[r,k] = (r%8+1)*(k%17-8)/16 and
   B[k,n] = ((3*k+5*n)%19-9)/16. The worst sum-of-magnitudes numerator is
   12288*8*8*9 = 7077888 < 2^24, making every partial sum exactly representable
   at scale 1/256. Add realistic BF16 input/reference cases and confirm the
   preserved accumulation order before model use; dyadic parity alone is not
   a representative accuracy qualification.
6. Run three complete ABBA blocks with identical warmups, instrumentation off
   for latency, then a separate raw-tick campaign. Also compare the specialized
   control against the qualified full-model V5 root before attributing any
   total gain to paired loads. Only after a repeatable component gain should
   Ferric add an explicit serving selector, native token replay, and isolated
   and composed TTFT/TPOT measurements.

Approximate component buffers total 97.25 MiB plus guards and code, with no
additional candidate memory. This is not a build-stage reservation estimate;
current-compiler dependency/build headroom must be checked independently.
The geometry helper is a host model, not pointer admission or launch authority.
