# Performance Swarm V4

Started 2026-09-18 at user request. This tracker separates implementation,
remote CPU validation, native correctness, and measured performance.
The implementation is retained privately at Ferric
`1fc45a523c2d794373885ae54a628c36afe84547` (tree
`6054e7420cb4f7ba1464fbb296dc47bc62ebe0d4`).
The historical comparison and its limitations remain in
[V3](M1_PERFORMANCE_SWARM_V3.md); no competitive result is claimed here.

| Team | Implementation | Validation | Performance |
| --- | --- | --- | --- |
| Runtime | Borrowed ordered/sequence payload validation and immutable batch-scoped preparation; published to fe2o3 main | Independent review; rebased revision passes 494 CPU tests, one hardware test ignored; strict Clippy and release worker build pass | Unmeasured |
| Target kernels | Opt-in composition of existing V14 attention and V15 RMSNorm, with individual controls | Final 5ed pin: formatting, strict Clippy, four composition tests, 41 binary tests (four ignored), and release build pass; all four native arms match the reference | Combined TPOT 84.55 ms versus baseline 135.85 ms, descriptive cohort only |
| Serving | Direct aggregate kernarg packing with transactional rollback | Two real-V8-metadata CPU tests pass, including byte identity, malformed arguments, rollback and ordered transactions | Unmeasured |
| Integration | Current fe2o3 migration, bounded remote builds, native comparison | 30 locked resolutions updated; 48 source-gate and 39 final-pin adapter source-policy tests pass; corrected native runner/supervisor 10 + 12 CPU tests pass | Completed native cohort; independent raw replay and six audit tests pass |

The full adapter release library suite also passes on the final 5ed pin:
367 passed, zero failed, six ignored. The aggregate protected-release policy's
positive and negative CPU fixtures pass. This is policy testing, not a protected
compiler invocation or release qualification. Obsolete owned debug build products
were removed after the final tests, reclaiming 2,298,860 KiB on mi300x-2.

The first frozen upstream baseline was fe2o3
`e5d92f8fbfec5676f610842ae1f56942a87d269d`. Before publication, both generic
runtime commits were rebased onto `c384de2193c77a5f58533a12492372e542032263`
and revalidated on mi300x-2. Main now contains
`5ed3840a90db3f03a2cded9becffc0459b737f36`; both commits include `[skip ci]`,
and the post-push Actions query returned no runs. The original dirty/conflicted
fe2o3 worktree was not modified. The separate clean runtime worktree has been
removed after retaining the source archive and Git history. Kernel and engine
work stays in Ferric. No implementation is published from Ferric.

Ferric's final-pin migration keeps the separate draft-V16 and property-binder
dependency pins unchanged. Independent diff review confirms that the 30 locks
and generated dependency records change only the exact fe2o3/Pliron revisions,
with no registry, feature, target, edge or authority-boundary drift. The runtime
TCB is byte-identical. Coverage regeneration exposed six pre-existing teardown methods
already admitted as `pending-verus` but absent from `VERIFIED_MODULES`. A bounded
comparison confirms that the source and admission list are unchanged and that
only those six `unverified=` rows were restored. This is an inventory repair,
not new proof coverage or authenticated qualification. All six existing
synthetic behavioral-harness patches apply to the new fe2o3 tree; the full
synthetic harness has not been rerun by that patch-application check.

Builds and CPU tests run on mi300x-2 only, with one CPU/job, a 10-GiB private
stage cap, 512-MiB reserve, and existing host free-space/RAM floors. GPU runs
are serialized on mi350 after fresh admission; passive idle checks are not a
reservation. No local or GitHub builds are used. Owned temporary stages are
removed after retaining source, logs, binaries needed for reproduction, and
measurement receipts.

The kernel experiment has four same-image modes: existing C1, V14 only,
V15 only, and both candidates. It does not change defaults or emit new device
code; retained V14/V15 images keep their historical producer identities.
The initial probe measures sequential controller-ingress latency, not HTTP
latency or sustained serving goodput. Its independently reviewed runner and
pidfd supervisor have passed remote fake-controller tests. The first native
attempt stopped before submitting any request: the runner's target-model hash
contained a one-byte transcription error. The authenticated setup matched the
canonical Rust identity. The corrected runner additionally checks the retained
real setup envelope and rejects the old typo and other identity mutations.
The failed attempt is retained with clean owned-process teardown, at archive
SHA256 `fcbe10574c2efe4b14c3e9df3ecd0f3894133feea5624a2e113d98dbe0b75646`.
A new stage passed fresh admission and completed the corrected four-arm probe.

The final-source controller is
`f5621d2e0bc62a43c1a8d76773d42619f1a0f84cc3da00f54c088a08f430405d`,
with runtime worker
`f68e42197f5f91854f7ec15c92b2fe1eee29a5620a6d313751e64f0af9598ebc`.
The semicolon-only Clippy correction changes the executable hash; the retry
uses this rebuilt binary, and the failed first attempt keeps its original
`56604ad5` attribution. Neither attempt re-emits the historical kernel images.

## Native Results

The completed R2 archive is
`c1cd3b9448e05076cc935801f82ad99b5b48c3d664ad758628860e50b9f953d6`,
retained at `.codex-tmp/ferric-performance-swarm-v4/native-attempt-r2.tar.gz`.
All 16 requests match the independent 128-token reference and decoded UTF8.
Each arm has one excluded warmup and three measured requests. The workload is
Qwen3-8B BF16 weights/decoder with the FP32-v8 output head, TP1/C1,
context8192, 128 input/128 output tokens, prefix caching and speculation
disabled. All arms load the same five historical device images.

| Mode | Mean TTFT (ms) | Mean TPOT (ms) | Measured-Window Output Rate (tokens/s) |
| --- | ---: | ---: | ---: |
| Baseline | 2662.658047 | 135.847603 | 6.427112 |
| V14 query-hoist | 2653.687446 | 134.744319 | 6.475596 |
| V15 RMSNorm | 2274.876606 | 86.422164 | 9.659803 |
| Combined | 2280.814642 | 84.545915 | 9.832207 |

These are controller-ingress wall timings, not HTTP or GPU durations. The output
rate uses the measured ingress span, including gaps between measured requests;
it is not an arithmetic mean of per-request rates or sustained serving goodput.
Fixed arm order and one start per mode do not establish stable gains or tail
latencies. Combined has approximately 38% lower TPOT and 53% higher rate in this
cohort. V14 alone changes TPOT by less than 1%; the main observed difference is
the V15 normalization route. Runtime and packing improvements are common to
all four arms and have no isolated before/after measurement here.

All controller closes and the outer supervisor complete normally, with no
surviving owned KFD worker. A subsequent passive observation finds all eight
GPUs at zero utilization and 297,766,912 bytes VRAM each. This is not a host
reservation or a general teardown proof. Independent CPU replay passes all
2,048 generated IDs, decoded output, commands, batch counts, timing arithmetic,
warmup exclusion, input bindings and cleanup records. Six audit tests pass,
including positive replay and deliberate token, command, metric and cleanup
mutations. The auditor does not independently rehash the model files: their
post-run equality is checked by the source-bound native runner. The completed
GPU stages and redundant remote archives were removed after verified retention;
the existing model directory is untouched. No vendor ranking, TP8 result,
default promotion, new Verus proof or serving qualification follows.

The interrupted debug library run and unnecessary all-binary integration-test
build are retained as interrupted runs, not passes. The 39 adapter source-policy
tests were compiled directly from their unchanged test source with the pinned
TOML dependency, avoiding unrelated executable builds. The initial release
library and packing runs used the frozen e5d dependency graph. The separately
rerun final-pin release suite passes 367 tests with six ignored; earlier evidence
is not relabeled as that rerun.

Ordered preparation now performs one boundary fence instead of one fence per
dispatch in its CPU-only phase. All GPU-visible staging/publication and
completion fences remain. The changed `dispatch_prepare_ns` extent excludes
those amortized fences; its isolated delta is not a performance claim.

## Retention And Publication

The final CPU archive is retained locally as
`.codex-tmp/ferric-performance-swarm-v4/cpu-retained-v4-r2.tar.gz`:
212,402,070 bytes, SHA256
`02812d6b11f334a37197e5bac28a66bccc34955d6dcac61ea1fcdf8e21e2e866`.
Its checksum passes after transfer. It retains source snapshots, binaries,
validation logs, native inputs, dependency review and Pages QA evidence.
All 1,134 non-site/non-doc implementation files match private commit `1fc45a52`.
The first archive attempt's directory-change warning is retained as an archival
failure, not a build/test failure or a successful retention receipt.

The [project site](https://harsh-nod.github.io/ferric/) is updated through
static-only commit `329a86b4bab2b7fc4c8585437f395c295c8cbe44` on
`pages/prebuilt`. Remote structural/negative checks, all widths 320..1440,
eight named viewports and 64 screenshots pass. Deploy-only workflow
[35394680140](https://github.com/harsh-nod/ferric/actions/runs/35394680140)
succeeds without GitHub builds or tests. All seven live files are byte-equal
to the validated artifact, totaling 789,721 bytes. The deployment receipt is
retained separately because it postdates the CPU archive. The clean temporary
Pages publishing worktree is removed; no private implementation is pushed.

Final CPU-stage removal is deferred. Three read-only cleanup attempts stop at
an inaccessible same-UID process; the diagnostic retry identifies the host's
`systemd --user` manager, not a Ferric worker. Noninteractive sudo is unavailable.
The exclusive build lock and completed job receipts pass, but the complete
no-use census cannot be established, so no deletion check is bypassed and no
process is signaled. Approximately 6.2 GiB remains at
`mi300x-2:/tmp/ferric-perf-v4.Dju0tBoo`, with all required evidence already
retained locally. The cleanup script and diagnostic failure are retained for
a later authorized cleanup. Both native stages and temporary runtime/Pages
worktrees are already removed; model files and unrelated worktrees are untouched.
