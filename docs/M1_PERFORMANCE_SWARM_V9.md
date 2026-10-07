# Performance Swarm V9

Updated: 2026-09-29 UTC. Engineering experiments, not a serving qualification.

## Objective

Reduce Qwen3-8B TTFT and TPOT while preserving a matched comparison. The last
HTTP pair remains September 28: Ferric 888.594 ms TTFT / 55.775 ms TPOT,
vLLM 20.125 ms / 4.230 ms. TP1/C1, 128 input and output tokens, BF16 decoder,
FP32 head, prefix caching and speculation off. No new speedup is claimed here.

## Per-Change Measurement Rule

User requirement: measure after every performance-affecting change, before
accepting it or stacking another change on the measured path. Source completion
and passing correctness tests do not establish a speedup.

1. Freeze the last accepted baseline and the single candidate change, including
   source, compiler/runtime and image identities. Treat compiler/runtime upgrades
   as separate changes; do not silently combine them with a kernel experiment.
2. Pass correctness checks, then run baseline and candidate on mi350 with the
   same model, precision, TP, concurrency, prompt/output lengths, sampling,
   context, prefix/speculation settings and measurement protocol. Keep builds
   on the approved remote build host and retain resource/cleanup checks.
3. Use uninstrumented repeated alternating-order comparisons with equal warmup
   and sample counts. Record raw samples, run order and variability. Diagnostics
   and kernel microbenchmarks inform attribution but do not replace full-model
   measurements or the matched HTTP comparison used for serving claims.
4. Report absolute TTFT, TPOT and output-token rate, plus per-metric deltas.
   Latency improvement is `100 * (baseline - candidate) / baseline`; rate
   improvement is `100 * (candidate - baseline) / baseline`. Label finite-cohort
   rate separately from sustained throughput. Report tradeoffs and inconclusive
   results, including regressions, without selecting only favorable runs.
5. Promote only a correct, repeatably beneficial candidate under the declared
   workload/objective. Regressing or inconclusive changes remain experimental.
   Measure any composed candidate again against the last accepted baseline.

Parallel teams may prepare independent candidates, but measured integration is
one change at a time. If measurement is blocked, record `unmeasured`, the exact
blocker and the next gate; do not assume a gain or advance the measured baseline.
Documentation-only changes need no GPU run. Instrumentation changes require
validation and an overhead comparison before their timings are used to infer
performance; instrumented and uninstrumented results must not be conflated.

### Per-Change Ledger

| Change | Measurement Status | TTFT / TPOT / Rate Delta | Next Gate |
| --- | --- | --- | --- |
| Fixed-shape safe gate/up | Unmeasured; pinned 1a599 SDK-backed host matrices and strict Clippy pass | Not available | Device emission, native correctness, isolated comparison |
| Paired-load K2 prefill | Unmeasured; pinned 1a599 SDK-backed host matrices pass | Not available | Device emission, native correctness, isolated comparison |
| Packed host-timing instrumentation | Correctness validated; overhead unmeasured | No speedup claim | Same-path instrumented/uninstrumented overhead comparison |
| Compiler refresh to `1a5999f6` | Capture producer builds; 99 unique compiler tests and six LLVM worker CTests pass; native unmeasured | Not available | Fresh emission/native correctness, then compiler-only native baseline |

The instrumented existing-path B/A capture below is a diagnostic, not a
measurement of either new kernel or of instrumentation overhead. The accepted
performance baseline remains unchanged.

## Teams

| Team | Implementation | Validation |
| --- | --- | --- |
| Projection | Default-off fixed-shape packed gate/up safe-load crate | 19 standalone host tests; later V10 pinned-SDK matrices pass 21 tests each plus strict Clippy; emission/native pending |
| Prefill | Default-off `prefill-mfma-k2` paired K16 fragment loading with unchanged serial MFMA updates | Seven standalone host tests; later V10 pinned-SDK matrices pass 22 tests each; emission/native pending |
| Measurement | Separate packed host-wall diagnostic and strict sidecar analyzer | Controller builds; 111 tests pass, 11 existing ignores; 14 analyzer and nine native-harness fixtures pass; both native captures pass |
| Integration | Frozen-dependency diagnostic build, resource checks and comparison ownership | Existing four-core/8 GiB/24 GiB CPU profile retained; no local builds or new worktrees |

## Scope

The initial fixed gate/up proposal pinned fe2o3 main
`c508e7a2cc94ea6d60ba3d56f0246a8599bd5898`. V10 separately qualifies its
`1a5999f6` SDK migration; neither source pin grants native qualification.
Safe indexing, intermediate finite checks, arithmetic order,
Wave64 reduction and rounding remain intact. Index tests cover all 25,165,824
admitted weight coordinates, but cannot establish the generated ISA's speed.

Prefill K2 is selected only by `prefill-mfma-k2`; the original projection source
and normal adapter selection remain unchanged. Loading two adjacent K16
fragments before consuming them is a scheduling hypothesis, not evidence of
overlap. Device emission, register/spill inspection, full-buffer parity and
same-composition native ABBA are still required.

The new `--packed-host-timing` diagnostic uses existing controller and worker
wall timers. It has a distinct profile, excludes other instrumentation, and
limits execution to 256 physical batches. Ordinary benchmarks remain
uninstrumented. The diagnostic-only rebuild reuses the exact Cargo-selected
historical dependencies, including the original prefill library and 807 worker
API. Test and production dependencies are selected separately, with unwind and
abort panic strategies respectively. This does not qualify the complete current
workspace or adopt latest compiler/runtime binaries.

The first controller test attempt failed because a staged ABI fixture was
missing. Its failed record is retained. The corrected attempt includes that
fixture and the exact test-profile dependency set. Remote formatting initially
reported whitespace differences; the corrected source passes formatting and
fresh controller tests/build. A prefill source-selection assertion initially
counted two unrelated Wave loops; its corrected MFMA-scoped check and separate
unchanged-Wave check pass. Earlier failures remain retained. Final focused CPU
validation totals 160 passes, with 11 existing controller tests ignored. An
independent audit rechecks the archived source and all selected dependency hashes.

Latest observed main has unchanged relevant runtime trees relative to the prior
snapshot, but substantial compiler changes. No compiler equivalence is inferred.
Core changes remain in fe2o3; kernel and inference changes remain in Ferric.

## Native Host-Wall Capture

September 29 on mi350, one packed B request followed by one baseline A request.
Both produce all 128 exact reference token IDs and decoded bytes, close their
135 batches and 1,973 ordered groups, and exit with clean owned-process teardown.
The supervisor exits zero with no remaining KFD users. Both strict analyses bind
the sidecar to the raw controller transcript. This measures the existing images,
not either new kernel candidate.

| Instrumented Native Metric | Baseline A | Packed B |
| --- | ---: | ---: |
| TTFT (ms) | 901.918 | 877.714 |
| TPOT (ms) | 61.362 | 62.643 |
| Mean decode batch wall (ms) | 61.343 | 62.608 |
| Mean worker publication/completion wall (ms) | 53.659 | 48.459 |
| Mean ordered roundtrip minus worker wall (ms) | 6.814 | 12.557 |
| Mean controller argument packing (ms) | 0.200 | 0.406 |

These are instrumented, single-request native observations, not HTTP results or
a repeatable packed speedup. The worker interval includes GPU execution,
publication, polling and fences; it is not GPU time. Ordered roundtrip minus
worker includes preparation/staging and transport/controller overhead, not pure
IPC. The partition avoids adding overlapping response waits. Batch sums exclude
setup, between-batch gaps and teardown.
Ordered-group phase labels identify flush sites, not homogeneous kernel content;
packing can move mixed work between labels. Per-phase totals must not be used
as isolated attention or feed-forward kernel costs.

Worker intervals account for about 87% of baseline and 77% of packed decode
batch wall time. Packing is below 1% in both. The next diagnostic priority is to
separate GPU execution from polling/fences, and preparation/staging from
transport. Controller argument packing is not the leading measured opportunity.
No causal explanation for run-to-run timing variation is established.

## Retained Evidence

Artifacts live under the local `ferric-perf-swarm-v6/products` evidence directory:

| Archive | SHA256 |
| --- | --- |
| `perf-v9-focused-cpu-custody-a001.tar.gz` | `92aa89843fabe2046dcfa0f6cff132c6de1f6da02fdc8be21909d280a81653c9` |
| `perf-v9-native-diagnostic-custody-a001.tar.gz` | `5ae227904621ad215e5e37a42e617dacb6f6e944cdee009f35de716014f6c9c7` |
| `perf-v9-host-analysis-custody-a001.tar.gz` | `e0c3341b69760baa9bf02fae21c5b7c7c92f357ed2ab10c09c2717962db85f56` |

The final diagnostic controller is 15,467,672 bytes with SHA256
`5741da4b49e51e67177bf009be1e1c743a925b3dc2eeec150720521ca5fb1c68`.
CPU analysis owners also report successful cleanup under the unchanged limits.
Independent native review confirms prompt IDs, output IDs/UTF-8 bytes, recorded
counts, source/result hashes and clean process teardown. Ordered timing records
have no incomplete, active or failed entries.

Owned-stage deletion is separate from process teardown. The initial retirement
and one quiet retry both refused before any deletion: the all-UID no-use census
did not converge within its unchanged eight-round bound amid process churn.
Both refusal records remain under `proposals/perf-v9-host-diagnostic-retirement-r1`.
A separately admitted a003 retirement subsequently passed the unchanged checks,
removed the 149-file diagnostic stage and reclaimed 43,307,008 allocated file
bytes; `/tmp/ferric-opt-v9-host-diagnostic-r1` is verified absent. No borrowed
model or unrelated process was touched. Together with V10 affinity R1 a001
and R2 a002 retirement, all three GPU payload stages are removed and total
reclaim is 97,427,456 allocated file bytes, about 92.9 MiB. R2's first refused
attempt remains failed in its own record.

## Remaining Gates

- Refine worker timing to distinguish GPU execution, polling/fences and
  preparation/staging without weakening runtime validation.
- Qualify the latest compiler/SDK and emit both kernel candidates, then measure
  each independently before composing them.
- Repeat uninstrumented full-model ABBA and matched HTTP trials after a candidate
  passes. Include packing costs, TTFT, TPOT and finite-cohort rate, and keep
  sustained throughput claims separate.
- Numerical-contract relaxation, further fusion and replay optimization remain
  hypotheses. Neither strict arithmetic nor runtime safety checks are weakened.

The old mi300x-2 private stage still has limited headroom and its cache cleanup
remains held by unavailable all-UID process inspection. A fresh bounded stage on
mi300x now has successful unmodified `1a5999f6` backend/extractor and CLI builds.
The CLI ran from 18:59:59 to 19:01:03 UTC, status 0, clean and unsignaled, with
peak sampled RSS 2,217,259,008 bytes. Raw CPU custody a004 SHA256 is
`a616b2f4f21331ce405a2de24cf0a6c22962176f0f9b85298b7106155c493d49`;
compiler binary custody a001 SHA256 is
`1413057b4e8388a5a96ee3a9092cb1a67d29b019ca85a941c81ac296b752abd3`.
The later V10 capture patch is applied: its producer rebuild, 99 unique compiler
tests and all six rebuilt LLVM worker CTests pass. Both kernel SDK-backed host
matrices pass. Vendor a001 fails on missing `dlmalloc`; the exact std-input
transport/merge and offline locked a002 retry subsequently pass. Fresh emission,
native correctness and all new kernel timings remain pending. Raw CPU custody
a005 SHA256 is `358eeac3c4e89a369dab126c75bde08596a40998f5a2ed141d1034e31e81569a`;
the std merge/vendor success is a later root-reported result. Newer observed main
`f2dd128e35d5345cf4b7f6fb3f71480ee483c961` is queued separately, without changing
the pinned comparison. No kernel gain follows from these CPU checks. V10 affinity ABBA
regresses in both orders and is not adopted; see
[Performance Swarm V10](M1_PERFORMANCE_SWARM_V10.md). No cap increase or
cleanup-check bypass is used.
