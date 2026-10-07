# Projection Optimization Swarm, October 7

Three parallel source lanes followed the native packet attribution. Builds
and tests ran only on mi300x-2; GPU work ran only on mi350. The changes below
are default-off experiments, not a new serving-performance result.

| Lane | Implemented | Observed Qualification | Remaining |
|---|---|---|---|
| Decode down | Current-worker binding for the split-K8 native688 experiment; fresh three-block ABBA campaign | 24 adapter tests; actual plan validation; both counters and the first A/B latency cells closed | Recover and independently audit the original complete campaign |
| Checked loads | Explicit fe2o3 gfx950 API groups independent checked scalar loads, retaining original fallbacks | 19 focused tests and seven LLVM assembler fixtures passed; 112 baseline tests passed, 11 explicitly ignored | Retain baseline logs; additive physical-profile integration; actual ISA, parity and timings |
| Prefill down | Exact M32/N4096/K12288 control and paired-K16 roots in one opt-in Ferric crate | 18 API/source, 18 full-host default and 19 enabled-feature tests passed | Current emitter/worker qualification, image/ABI/ISA, native output parity and matched component timings |

Checked-load tests exercise all 256 guard masks, invalid-pointer non-access,
overflow and nonzero address offsets, successor/backedge PHIs and conservative
region boundaries. Strict library Clippy reports eight diagnostics in unchanged
code; no lint suppression or unrelated repair is included. Its ordinary-module
API is not wired into the protected physical/anchored emission route.

The prefill experiment preserves all 768 ascending K16 updates to one FP32
accumulator and the original output layout. Paired source loads alone do not
establish outstanding hardware loads, overlap, acceptable register pressure,
or faster execution. Host tests use exact-SDK path-bound fixtures, not emitted
device kernels. The older BF16 gate/up harness cannot qualify its FP32 output.

## Interrupted Observation

The native campaign started around 21:19 UTC with the ordinary, uninstrumented
worker built from fe2o3 `1736eff451d445f1f194abe145af242cf51ec322`. Its worker hash
is `8f764849a5a1c6a23c567f8aefe253de5e6ef2320977a79db7c472ad3417a8b5`.
The fresh plan hash is
`723298e87e8f5e12949267d761986fbd5d3aa6c8c8a90a728c6f93219b19c54a`.
Historical controllers/images remain explicitly identified as ancestry;
old interrupted measurements are not reused.

After a long clock/connection gap, the native SSH transport returned 255
without a terminal receipt. Fresh probes around 22:50 UTC timed out on both
mi350 and mi300x-2. The campaign's 75-minute deadline is not proof that it
finished or cleaned up. Its outcome is **unknown**, not failed or passed.
No new native launch or cleanup was attempted. The latest prefill Clippy
submission is also unconfirmed; the newly drafted emitter helper was not run.

The original durable output is on mi350 at
`/home/harmenon/ferric-native-down-current-retained-20261007-a001`.
Recover its session receipt, archive and manifest, then reconcile boot IDs,
owned process identities and GPU occupancy before any retry. Independently
verify all 14 cells, token replay, full three-block ABBA, unsignaled lifecycle
and unchanged inputs before calculating or publishing a gain.

CPU source and test logs already copied locally remain retained. The compiler
baseline suite's clean outer result was observed, but the later transfer of its
full raw logs did not complete. Do not describe that retention as complete.

## Comparison Boundary

The latest complete matched HTTP comparison is still the October 6 Width55c
Qwen3-8B TP1/C1 128-input/128-output workload: Ferric 431.033 ms TTFT and
53.119 ms TPOT versus vLLM 18.969 ms and 4.352 ms. No new vLLM comparison,
TTFT/TPOT improvement or production promotion is established by this update.

After recovery, qualify each candidate separately, then measure the composed
HTTP path. Keep native-ingress timing separate from HTTP serving results and
keep diagnostic instrumentation out of latency measurements.
