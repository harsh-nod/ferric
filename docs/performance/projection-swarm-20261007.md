# Projection Optimization Swarm, October 7

Three parallel source lanes followed the native packet attribution. Builds
and tests ran only on mi300x-2; GPU work ran only on mi350. The changes below
are default-off experiments, not a new serving-performance result.

| Lane | Implemented | Observed Qualification | Remaining |
|---|---|---|---|
| Decode down | Current-worker binding for the split-K8 native688 experiment; fresh three-block ABBA campaign | 24 adapter tests; all 14 native cells independently audited; median TPOT reduced 7.1586% | Matched HTTP integration measurement, then a fresh vendor comparison |
| Checked loads | Explicit fe2o3 gfx950 API groups independent checked scalar loads, retaining original fallbacks | 19 focused tests and seven LLVM assembler fixtures passed; 112 baseline tests passed, 11 explicitly ignored; raw logs retained | Additive physical-profile integration; actual ISA, parity and timings |
| Prefill down | Exact M32/N4096/K12288 control and paired-K16 roots in one opt-in Ferric crate | 18 API/source, 18 full-host default and 19 enabled-feature tests; strict Clippy passed in both full-host configurations | Current emitter/worker qualification, image/ABI/ISA, native output parity and matched component timings |

Checked-load tests exercise all 256 guard masks, invalid-pointer non-access,
overflow and nonzero address offsets, successor/backedge PHIs and conservative
region boundaries. Strict library Clippy reports eight diagnostics in unchanged
code; no lint suppression or unrelated repair is included. Its ordinary-module
API is not wired into the protected physical/anchored emission route.
The compiler implementation and qualification notes are published in fe2o3
`9f891d63335fc3589955870e1fc98634d06fa5b2`; no kernel or inference code is
included there.

The prefill experiment preserves all 768 ascending K16 updates to one FP32
accumulator and the original output layout. Paired source loads alone do not
establish outstanding hardware loads, overlap, acceptable register pressure,
or faster execution. Host tests use exact-SDK path-bound fixtures, not emitted
device kernels. The older BF16 gate/up harness cannot qualify its FP32 output.

## Current-Worker Down Result

The native campaign started around 21:19 UTC with the ordinary, uninstrumented
worker built from fe2o3 `1736eff451d445f1f194abe145af242cf51ec322`. Its worker hash
is `8f764849a5a1c6a23c567f8aefe253de5e6ef2320977a79db7c472ad3417a8b5`.
The fresh plan hash is
`723298e87e8f5e12949267d761986fbd5d3aa6c8c8a90a728c6f93219b19c54a`.
Historical controllers/images remain explicitly identified as ancestry;
old interrupted measurements are not reused.

The original campaign completed in 2,617.68 seconds, despite loss of the SSH
transport. Recovery obtained its accepted session receipt and all original
results; it did not rerun or reconstruct the measurements. All 16 commands
(validation, 14 cells and summary) exited normally, without TERM or KILL.

The independent data-only audit verifies every member of the complete archive,
both frozen plans, current worker and CPU-build ancestry, all raw request
streams, mechanism counters, finite device-admission samples and clean owned
process lifecycles. All 9,472 output token IDs and streamed UTF-8 bytes match the
independent reference. This includes warmups and two separate counter requests.
Latency samples have no counter, packet-timestamp or wait instrumentation.

| Native Ingress Metric | Control A | Split-K8 B | Change |
|---|---:|---:|---:|
| Median TPOT | 47.7079645 ms | 44.292754 ms | 7.1586% lower |
| Median TTFT | 396.691256 ms | 396.514336 ms | 0.0446% lower; effectively flat |
| p95 TPOT | 53.0076885 ms | 51.64735235 ms | 2.5663% lower |
| Finite output rate | 19.46499876 tokens/s | 20.75341466 tokens/s | 6.6191% higher |

Three complete ABBA blocks provide 24 measured requests per arm, with two
excluded warmups per latency cell. The block-mean TPOT reductions are 6.0051%,
6.4648% and 7.4388%. Five of six adjacent pairs improve; the first BA pair
regresses 4.6002%. Both order medians are positive, and all four predeclared
engineering checks pass. The first block is slower in both arms than later
blocks, so absolute times and pooled medians must not be treated as stable
device characteristics or statistical proof.

Both arms use the same ten images, worker, scratch allocation and unchanged
native649 prefill. B replaces the down operation with split-K8 decode,
increasing the decode program from 652 to 688 dispatches. This isolates the
explicit native down selector, not a comparison against ordered64 or a change
to default serving behavior. The workload is Qwen3-8B on one MI350, TP1/C1,
128 input and 128 output tokens, context 8192, greedy generation, BF16 decoder
with configured FP32 head, prefix caching and speculation off.

The original durable output is on mi350 at
`/home/harmenon/ferric-native-down-current-retained-20261007-a001`.
The complete 293,456,887-byte archive has SHA-256
`39d315bc138cfd6ec04984984c742e59c937417d85314e16f67183e44cb6b09d`;
3,084 members were independently verified on the original host. A locally
retained 56-member raw subset is 2,101,365 bytes, SHA-256
`2457bac76f35861419d68084dcf9235b17f1b7f3e9de6fa057e7f1026069c2ae`.
The [independent report](native-down-current-a001-independent-review.json)
has SHA-256 `abb03d83d2c3cb0690534f51346b52342e78ea855677ac2db1645a1f159fbd5e`.
Full-archive remote verification and local subset retention are distinct; a
partial local full-archive transfer is not accepted as complete custody.

All compiler baseline raw logs and both prefill Clippy receipts have also been
recovered. The current emitter rebuild's first offline attempt stopped on a
missing dependency before compilation; a bounded dependency fetch subsequently
passed. Compiler tools, LLVM worker and emitted kernel images require their own
fresh qualification and must not be inferred from host tests.

## Comparison Boundary

The latest complete matched HTTP comparison is still the October 6 Width55c
Qwen3-8B TP1/C1 128-input/128-output workload: Ferric 431.033 ms TTFT and
53.119 ms TPOT versus vLLM 18.969 ms and 4.352 ms. This new within-Ferric
native-ingress gain is not a new HTTP or vLLM comparison, and is not composed
arithmetically into those older HTTP numbers. Ferric has not demonstrated a
vendor win or production promotion.

Qualify each candidate separately, then measure the composed
HTTP path. Keep native-ingress timing separate from HTTP serving results and
keep diagnostic instrumentation out of latency measurements.
