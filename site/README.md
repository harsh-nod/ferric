# Ferric project site

This directory deploys a dependency-free static GitHub Pages site. Project
status is kept in [`data/project.js`](data/project.js); curated, strictly
checked performance observations live in
[`data/performance.js`](data/performance.js). The harness pins Playwright for
real browser checks. Run all checks on the designated remote build host, not
locally, and remove the private stage after archiving evidence.

## September 14 Selected Numerical And Latest Compiler Checkpoint

Exact `4ac1250/ccfd` capture, independent canonical Qwen3-8B BF16/SDPA reference,
and comparison each exit 0 for `prefill-s1-t128.001`. This is one generated
128-token input, not the earlier English or speculative padded prompt. All
151,936 logits are finite; both select token 198 with identical top-10 token
ordering. The rows are **not byte-identical**: maximum BF16 ULP error 31,121,
maximum absolute error 0.0703125, RMSE 0.01524191977257529, cosine similarity
0.9999202347605426, 20,968 exact BF16 matches and 619 opposite nonzero signs.
The reference repeats twice byte-identically within one model load, not two
fresh launches. This is one of seven planned R29 cases, with no reviewed
tolerance acceptance, full R29 comparison, qualification or benchmark result.
The retained archive SHA-256 is
`816f162e5a2d3229239dc305e8658f8c570a269b44a077787d916030ec60335c`.

Separately, latest published fe2o3 `4f6f65ce` and private Ferric `8113e231`
pass the changed kernel-ir/analysis full suites, 543 backend tests and all
37 aggregate host tests. Pristine release tools emit/replay the canonical
gfx942:xnack- V6 image. Structural ELF inspection confirms 12 kernel entries
and 12 matching descriptors, not policy descriptor-table order. The image is
103,616 bytes, SHA-256
`6f77d6813e6a2c9fd20c8b50eaffe0feb4435f00ac65192e9574a3637b6e5284`.
It differs from ccfd despite unchanged kernel bodies and has **not been GPU or
numerically tested**. Engineering authority is none; all publication/load/launch
grants remain false. Installed workers retain their actual older producer
identities. Latest evidence SHA-256 is
`6208de3c1329e000c5393739b600d6e04b49b4b20dff4fb8dfc5443a8e65d3cb`.

All 33 M1 gates remain open. No TTFT, TPOT or throughput values are added;
`performance.js` is unchanged. Latest combined host validation is separate from
the historical bb2/ccfd host and d094 coverage records retained below.

### Retained Native And Compiler Checkpoint

Exact `bb2/ccfd` target-only execution on mi300x completes the same five-token
raw prompt with four IDs `[12095,13,576,6722]`, text ` Paris. The capital`,
hardware completion and exit 0. These match the first four IDs from the
independently frozen historical BF16/SDPA reference. This one-prompt token-prefix
spot check is not an R29 logit comparison, general numerical validation,
speculative catch-up, R33 qualification or serving performance. Its retained
archive SHA-256 is `2a9b13d81098a878ad1f299a7f4f999c368c16aca1a7f906cd9dc283f1e9dffc`.

The immediate archived after-state still reports 56,838,090,752 VRAM bytes,
zero selected KFD-process memory and busy 0. A later independent direct sysfs
check observes return to the exact 298,647,552-byte baseline and busy 0. This
delayed observation is separate; the original archive is not rewritten. No
device reset or foreign-process termination was performed.

The separate `6651f2d/c6b4050` target-only native smoke on mi300x passes with
the earlier `f17/c6` image: prompt `The capital of France is` (five token IDs)
produces one token, ID 12095, text ` Paris`. Hardware completion is observed,
the process exits 0 and selected-device memory returns to baseline. Its archive
SHA-256 is `0dc8ca2a724bde837750bae565a4224feae03e6f88fde50913ed35c58199999d`.
Authority is none; benchmark comparability, worker-v3 authentication,
compiler-origin authentication and current-publication selection remain false.
This is not general numerical parity, speculative catch-up, serving,
performance or M1 qualification. Raw controller offsets are excluded from
TTFT/TPOT and the performance data.

Separately, published fe2o3 `ccfd43d6` passes 1,021 Pliron library tests,
58 integration tests, 13 doctests and 543 backend tests. Exact private Ferric
`bb2b012` passes 37 aggregate host tests. Pristine release compiler tools emit
the canonical `gfx942:xnack-` code-object V6 engineering HSACO with exact replay.
Independent structured ELF inspection confirms 12 kernel entries and 12 matching
descriptors. The image is 103,872 bytes, SHA-256
`46335b09a921b33ed66392e415ce3d2348dd63042242c0387d5a58362ba3c84c`.
These are structural symbol-set checks, not descriptor-table ordering, numerical
execution or native qualification. Engineering authority is none; publication,
load and launch grants remain false.
This newer image differs from c6 and has its own separate four-token native
observation. The c6 result is not relabeled as latest-compiler hardware evidence.

The isolated same-source `829/852` comparison resolves the earlier uncertainty:
49 assertions, one false-to-true proof gain, zero true-to-false losses. The paged
`committed_tokens + query_token` obligation at `bb97`, line 373 was already
unproved, not a regression introduced by `852`. The new compiler bounds unsigned
literal subtraction only below the exact authenticated checked-success edge;
all producer assertions and limits remain mandatory. The complete aggregate now
passes. Earlier resource stops and rejected extractions remain separately retained.

Exact `bb2/ccfd` passes all 71 host and metadata phases. Spec passes 123 tests;
engine passes 697 library tests (nine ignored), binary suites 11/2/75/2 (two
ignored), and 171 doctests. Checker passes 81 library, 27 integration (three
ignored), and six doc tests with normal host features `[]`. Adapter passes
109 tests (one ignored), 37 policies and seven host numerical tests; owner
passes 16 tests and three policies. All five scoped strict Clippy checks pass.
All 32 locked graphs, 38 source-gate tests, 31 verifier policies, six source-pin
policies and three dependency inventory comparisons pass. The target-smoke,
speculative-smoke and R29-capture release binaries are built and source-bound,
not GPU-qualified by those builds.

Final `d094/ccfd` source coverage also matches the committed manifest exactly:
173 modules, 8,435 bodies, 723 verified labels and 7,712 unverified identities.
All 722 prior verified records are unchanged. The added helper label is backed
by its scoped Verus run with six executed callees and seven rejected executable
mutations, not whole-crate or physical catch-up verification. The engine
preflight and full physical catch-up chain remain unverified. Host tests and
binaries remain at bb2; d094 adds documentation and reviewed inventories only.

No new TTFT/TPOT, throughput, general numerical parity or serving qualification
is claimed. All 33 M1 gates remain open, and `performance.js` remains byte-identical.

### Retained Earlier Host Cohorts

Authenticated resident K4/K8/K16 catch-up is integrated. It consumes the missing
last accepted draft candidate, advances draft KV by one, emits zero served
tokens, and restores the original speculative queue shape. Twenty focused
host tests passed on the earlier `829` cohort. The exact `d55/852` engine gate
passes 697 library tests (nine ignored), binary suites of 11, 2, 75 and 2 tests
(two ignored in the 75-test suite), 171 doctests, and strict engine Clippy.
The current adapter passes 109 tests (one ignored) and 37 policies; the owner
passes 16 tests and three policies. Strict adapter and owner Clippy also passes.
The exact `d55/852` release generator also passes `--check`: generated runner
source matches the current renderer, not a produced native runner or image.
No warmed allocation-free catch-up or native serving qualification is claimed.

Full canonical target and draft prepack and reopen verification both pass on
the exact `bed11/829` CLI. These producer results are not relabeled as `852`;
they are model-data checks, not kernel execution, model parity or latency data.
Actual source coverage matches 173 modules and 8,433 executable identities:
181 new pending-Verus identities, 7,711 unverified identities in total, and
722 unchanged existing verified labels. This does not establish new physical
proofs. The separate `852` gate passes 32 locked graphs, 38 source-gate tests,
31 verifier policies, six source-pin policies and three exact dependency
inventory comparisons. All 33 M1 gates remain open; performance data is
unchanged.

### Earlier Diagnostic And Getter Checkpoints

The following cohorts retain their original source identities and historical
pending states. They are not relabeled by the current checkpoint above.

The follow-up records published fe2o3 diagnostic `ae441`, rebased onto `836afb284`,
and its separate backend/lineage/analysis/IR host suites. Actual aggregate
extraction attributed the earlier divisor failure to paged GQA coordinates.
Private Ferric `6da025e` uses literal coordinates and passes 35 host tests,
including actual AST equivalence for 14 profiles. Actual gfx942 extraction then
passes the divisor check but rejects an overflow assertion at `bb81`, source
fingerprint `09ab2715abb3`, line 378. Optimized MIR attributes the guarded
`query_position + 1`; this is not a verified compiler fix or a produced image.

Published fe2o3 `b750` exposes the generation retained by the owned completed-read
request. Its 43 service-host, 45 service-qualification and three pure KFD tests
pass; private Ferric `7c3f9729` pins it, with combined engine validation pending.
Pinned Verus verifies only the selected `kv_physical` module at `bd2/e6`:
4 verified, 0 errors, exact 190-file distribution closure before and after.
That source-settlement result is not whole-crate or physical catch-up proof.
Catch-up composition, bindings, coordinator, typed KV, maintenance readback and
preclock storage are implemented locally; resident dispatch is still under
integration. Continuing full acceptance remains fail-closed. No warmed catch-up
allocation claim, image, GPU run or performance observation follows.

The earlier checkpoint below remains source-bound, not revalidated by these
newer results:

The additive `residentCheckpoint` distinguishes private authenticated singleton
K4/K8/K16 selection from native qualification. It records separate host snapshots,
the real owner-builder identity repair, and the public fe2o3 resource-accounting
fix. The 13 focused compiler regressions are a subset of the 1,006 passing Pliron
tests, not additional tests. The old dc8 resource-limit failure is retained
separately. At eaa, CLI/backend builds and 33 gfx942 aggregate host tests pass,
but actual aggregate emission rejects a deterministic-control divisor without
static nonzero evidence. Kernel attribution is unknown. Exit 1 produces no
image, replay or GPU result.
Newer upstream `e6cfa668` was observed afterward; migration and revalidation are
pending. All compiler build, test and emission results stay bound to `eaa057ac`.

All 33 M1 gates remain open. There is no new native
K8/K16 result, GPU measurement or TTFT/TPOT/throughput claim. Every historical
project object, including the prior V15 checkpoint, and all of `performance.js`
remain unchanged. `validate-resident-checkpoint.mjs` rejects scope, source, count,
catch-up and qualification mutations. The earlier live-C1 evidence validator
checks this new object before removing it for the original historical comparison.

## September 12 Matched Cell

The new `matched128` section is pending root review and Pages-only publication.
It records the first admitted Ferric and vLLM Qwen3-8B cell: one MI350X GPU, TP1,
BF16 weights/decoder with an explicitly selected FP32 output head, context 8192,
128 input/output tokens, concurrency one, ten warmups and thirty measured
requests per engine. Speculation and prefix caching are off. Ferric is
substantially slower. The table is descriptive, single-start and closed-loop;
it does not qualify sustained serving, stock defaults, a repeated primary suite,
confidence intervals, token ITL or a competitive win. SGLang startup failures
remain without a metric, never zero throughput. The output-head operand policy
is explicit: Ferric's MFMA head and vLLM use BF16 operands with FP32
accumulation/output, while the independent reference converts head operands to
FP32. These implementation differences were declared before engine outputs.

`latestReadiness` and `pagedDraftReference` separately record the independent
BF16/SDPA-body FP32-head draft reference, one of eight planned native paged
canary cases, the private proposal candidate's passing host gate, and the exact
82703c6 combined host gate (568 invocations, 22 image skips, seven doctests and
31 Python tests). They do not claim a complete native matrix, new Verus proof
or speculative serving. All prior scoped objects and `performance.js` remain
unchanged; their no-baseline/no-paged-model flags describe their earlier cutoff.

`validate-matched128.mjs` rejects metric, precision, identity and qualification
mutations. Its optional evidence route binds every displayed metric to the
root-reviewed pair replay and the independent draft captions to raw, adapted and
clean-lifecycle receipts. It is not a replacement model or performance qualifier.

```sh
node validate-matched128.mjs /private/matched-evidence
```

## Earlier Checkpoints

The recovery checkpoint was published at Pages-only commit `4200318`. The prior
`competitivenessSprint` snapshot in `data/project.js` is preserved verbatim and records private integrated JSONL
and loopback HTTP source, scoped host/protocol tests, v8 native fixtures and
separate exact-reference model canaries at budgets 16/32 (actual maxima 16/17,
n=1 each, effectively flat rates). The real HTTP smoke has four sequential
requests/nine outputs with prefix caching off, exact token bytes and usage,
clean teardown and all-eight-GPU idle checks. No endpoint remains active;
the smoke is not concurrent or sustained-load qualification and provides no
HTTP performance claim. Shared peer-currentness results remain native-only;
the frozen-511 two-per-mode admission-cache canary remains separate. Native
fixtures alone are not model evidence. Its then-unimplemented larger-KV flag
is historical, not the current implementation status. Sequential baseline launches
were approved; no matched result was available at that earlier cutoff.
No serving-comparison or competitive-win claim follows from the canary.

The preserved historical `competitivenessFollowup` records the exact-reference wave/v8
canary (two observations per attention mode, four requests/eight outputs per
run), both the performance-rejected 1-ms-backoff experiment and the separate
unpublished 50-us-cap follow-up, host/native v9 physical-capacity extension,
published ordered-batch native checks, and optional authenticated draft intake.
Neither v9 nor ordered native fixtures establish model or 32-request serving
qualification. Separately, the actual 36-GiB v9 allocation and legacy control
both pass the tiny eight-output model reference, n=1 each, at most 16 observed
rows; these remain frozen 3e3 observations, not long-context, 32-request or
speed claims. Draft intake is not speculative execution. The logical context
limit remains 8,192 tokens; 36 GiB refers only to the maximum TP1 KV payload, excluding model
weights and workspaces. The old BF16 wave failures and all `performance.js`
observations remain unchanged.

`validate-competitiveness-followup.mjs` enforces the closed new snapshot,
rejects scope/promotion mutations, rehashes twelve separate canary reports and
raw traces, and recomputes each paired mean independently. It also checks the
nine v9 fixtures, two bounded model-allocation reports and two ordered-native receipts, their root reviews, and scoped
host test receipts. All validation remains remote-only:

```sh
node validate-competitiveness-followup.mjs /private/compete-evidence
```

`validate-competitiveness.mjs` validates the closed snapshot and rejects 30
scope/promotion mutations. Its optional evidence route, run only on mi300x,
rehashes the root-reviewed native/host receipts and all four exact-reference
cache reports, then recomputes the caption's per-mode mean changes. It also
rehashes both v8 model reports/raw traces and checks actual row maxima and
rates, plus HTTP receipt/controller events/clients/teardown without deriving
HTTP performance metrics:

```sh
node validate-competitiveness.mjs /private/compete-evidence /private/serving-evidence
```

Include the separately hashed source-gate `check.log` and
`metadata-summary.json` under `compete-evidence/source-gate/`. The 38 source
tests, 31 verifier-policy tests, 28 metadata configurations and coverage counts
describe compiler-rooted dependency/source checks, not new Verus or GPU proof.

This route does not replace the original native or token checkers. Historical
`performance.js` values, rejected cases and frozen source pins are unchanged.
The 6f6 host receipt is explicitly historical. Neither the earlier f85 ordered
checkpoint nor current public 216822284 relabels frozen 3e3 emissions/wave models
or 511 head/cache measurements. The polling branches remain unpublished.

The additive `competitivenessRecovery` preserves both older data objects and
all of `performance.js`. It records three distinct ordered cohorts, each with
two observations per mode on the same four-request/eight-output logical-tick
canary: the original -35.40% serial-relative regression; +85.60% recovery against
the old regressed ordered worker; and +22.19% against a fresh same-new-worker
serial control. Serial controls vary substantially. None is a steady-state
HTTP result, confidence interval, default adoption or framework win.

Standalone Draft06B matches an independent offline PyTorch reference for six
choices/two final outputs. Its controller was built at 346d588, not the later
exact 656edb2 CPU aggregate. The frozen baseline image retains compiler/SDK 3546
provenance. The separate closed14 v10 image emits on compiler 216822284 and
passes 36 native full-buffer fixtures; it does not establish paged draft model
execution, speculative serving or draft performance. The later role-checked
consumed-input driver at 58ed5c5 seals completed externally supplied, untrusted
proposal inputs and last-proposal catch-up. Its separate host gate passes 526
invocations, 19 ignores, six doctests and one exact-v10 image admission check.
Recording outputs are synthetic; automatic proposals, paged native model
matching and complete speculative serving remain unqualified. The 656 aggregate has
516 test invocations, not a deduplicated count of unique tests; its 18 explicit
ignores and prior failed attempts are retained. No new Verus claim follows.

Continuous V3 tooling has no drain between adjacent windows, retains failures,
and separates completion-window usage from arrival cohorts. The 90-test
collector and 109-test paired-series gates are CPU-only. Descriptive bootstrap
intervals require at least three fresh start pairs; this is not evidence of
stationarity, stable tails or equal framework tuning. The approved sequential
baseline launches had no matched performance result at that earlier cutoff.
The independent target reference passed two repeated 128-input/128-output
runs with a BF16 eager decoder and explicit FP32-head operands/output. It is
not the stock-BF16-head cell, and that reference-only checkpoint was not itself
a Ferric matching result or a baseline serving run.
Its raw, adapted reference and clean-container/idle wrapper receipts are separately pinned.

`validate-competitiveness-recovery.mjs` checks the closed new snapshot, negative
scope mutations, all twelve independently pinned ordered reports/raw traces,
per-mode means, standalone reference/trace custody, native36 output/guard
receipts, consumed-input driver host gate, independent target128 reference,
exact 656 aggregate and the two V3 CPU logs:

```sh
node validate-competitiveness-recovery.mjs /private/recovery-evidence
```

When implementation or qualification state changes:

1. Update `updated`, `readiness`, `validation`, `latestObservation`, and
   `recentProgress` in `data/project.js`.
2. Keep diagnostic, hardware-observed, and qualified claims distinct.
   Update performance values only from current, externally pinned correctness
   reports and ledgers. Keep n=1 ablations separate from repeated profiles,
   never pool request identities, exclude rejected numerical runs, and retain
   the exact controller/worker/artifact/ledger identities. The historical
   ablation queue contains five accepted n=1 profiles; both wave model profiles
   are rejected.
   Keep the later matched MFMA pair separate, including its startup regression.
   Initial serial-peer TP2/TP8 observations had no matched host controls; later
   source-matched controls use deliberately distinct worker executables and
   show large regressions on the peer path. The CPU
   transpose-helper benchmark is not a model latency or throughput result.
3. Install the pinned browser harness and run both the structural and rendered
   checks:

   ```sh
   cd site
   npm ci
   npx playwright install chromium
   npm test
   FERRIC_EXHAUSTIVE_WIDTHS=1 npm test
   npm run stage -- /tmp/ferric-pages-artifact
   ```

The deployment workflow requires the exhaustive Chromium sweep in addition to
the syntax and structured-data checks before publishing. `validate.mjs` rejects schema
drift, stale current-dependency claims, unknown status states, malformed source
references, duplicate transitions, missing render targets, and missing local
assets. `render-validate.mjs` checks populated visible output without horizontal
overflow at the desktop, breakpoint-edge, and mobile widths; its exhaustive mode
checks every width from 320px through 1440px. `stage-artifact.mjs` creates and
validates the seven-file static deployment roster so `node_modules`, dependency
metadata, documentation, and test-only files cannot enter the Pages artifact.
`validate-performance.mjs` checks the closed performance schema and seventy-two
negative mutations, including incorrect repetition counts, throughput windows,
runtime profiles, ledger identities, and cancelled-request TPOT. A Pages-only
publication must not include Ferric implementation, model files, or raw logs.
`validate-performance-evidence.mjs` additionally compares every new model metric,
request latency, binary/report pin and helper aggregate against externally
SHA-pinned local evidence inputs. Run it remotely with the measurement archive
and transpose archive paths, the previous public performance file and replica
archive path; these inputs are not part of the Pages artifact. Replica cohorts
use a distinct 64-output workload and common RAW release clock, not summed
instance rates or the earlier eight-output window. Publish the per-instance
row policy, actual loaded weight bytes and every request's separate admission
TTFT, release-to-first-token and seven-gap TPOT.

The legacy MFMA data key `ledgerCanonicalId` retains its historical name and
value, but its source field `ledger_sha256` hashes the ledger generator script,
not a canonical report. Render it as `ledger generator source SHA-256`.
The separate ledger-file SHA-256 and actual canonical replica-expectation
digests keep their existing meanings and labels.

Keep the later wide16/32 row-policy pair, source-matched peer controls and
full-model transpose observation separate. Wide row and chunk budgets change
together; request00 TTFT regresses. Peer worker executables differ deliberately
on the same core source. The transpose production change is setup-only, so
observed decode-rate differences are not attributed to that change.

The two-repetition MFMA and TP1 residual tables reuse their published R1
observations plus one new run per profile. Preserve those R1 tables and pins;
do not count them twice. Means and ranges remain per profile and request, with
n=2 nearest-rank p50/p95 equal to the minimum/maximum, not stable tails. The
single MFMA-plus-pruning observation is a distinct cumulative profile and does
not show an additive pruning gain.

Current-controller compatibility is separate from the earlier paired timings.
The rebuilt worker is byte-identical to the earlier binary, and emitted images
retain their original provenance. The cached TP8 capacity-32 smoke observes
only 17 rows. Current TP1 MFMA-only and cumulative MFMA/pruning/residual fail
the exact seed reference and have no accepted timings. Baseline projection
plus pruning/residual passes, with n=1 metrics from its pinned ledger; two
flags change, so no isolated gain follows. Preserve the completed per-case
receipts and the separate outer SSH255 anomaly. The failure is observed
without pruning or device residual, but its numerical cause is not proven.
## Argmax Route Checkpoint

The additive `routeCheckpoint` and `routeReadiness` fields record the private
`d9a2705` correction's exact host gate, the permanently rejected `f662d54`
native attempt, and frozen `f65` overlapping host-counter diagnostics.
The additive `argmaxNative` record binds the corrected six-run native diagnostic
to the unchanged summary, manifest, reducer, checker and controller identities.
Full128 has one run per mode; short8 ABBA has two. Displayed gains use ratios of
arithmetic means, with individual short-pair variability retained. These are
instrumented native controller-wall measurements, not HTTP or GPU durations,
confidence-qualified estimates, a competitive ranking or a new default.
Existing `nativeFollowup`, `matched128`, and historical `performance.js`
records retain their original source and measurement scope.

`node validate-argmax-checkpoint.mjs EVIDENCE_DIR` additionally checks the
five pinned raw receipts/traces. Evidence is retained outside the published
site. Its mutation checks also run in the normal static validator.

`node validate-argmax-native.mjs EVIDENCE_DIR` checks the pinned summary,
manifest, plan and reducer plus all 60 raw files in the six manifest-named
directories. It independently recomputes displayed means and paired reductions,
checks successful reference/cleanup receipts and preserves overlapping host
scope exclusions. The earlier `f662d54` rejection contributes no result or timing.

## Attention Checkpoint

The additive `attentionCheckpoint` and `attentionReadiness` records bind one
`809af24` native host-attribution diagnostic and the separate eight-case
`c26c1f4` finite TP1 fixture pass. The seven sibling groups are disjoint, but
the parent and IPC spans overlap. None is an isolated GPU duration or a
measured optimization gain. The fixtures compare exact complete BF16 bytes,
immutable inputs, masks and tails against controlled finite expectations;
they do not qualify arbitrary softmax inputs, model performance or new Verus
proofs. Their old v5 image retains its original fe2o3 `3e74` provenance.

Run `node validate-attention-checkpoint.mjs EVIDENCE_DIR` on the approved remote
CPU stage to authenticate the reporter, raw timing/trace/receipt and native
fixture probe/report/replay/receipt/prelaunch files. The validator recomputes
each displayed host-span total from the pinned raw timing records and checks
all eight complete native buffer/guard rosters. Evidence stays outside the
site artifact. Mutation checks are part of the normal static validator.

The new combined attention/argmax route's exact ed112 host gate passes 42
required commands across 43 actual attempts, with 685 passed test invocations
and 34 ignored. The nested-TMP failed attempt and earlier Clippy failure remain
separately retained; the unchanged-source gate used a warm Cargo target.
The validator authenticates `composition-HOST-GATE.md` and its archived failure
identities. The same source's four native model qualifications now pass exact
IDs and UTF-8: baseline/wave attention, each at 8 and 128 outputs, with fixed
wave-v11 argmax. These are correctness diagnostics, not comparison samples.
The validator authenticates the qualification note, both equivalent forty-file
hash rosters, all raw files, original references, fixed profile and normal
retirement/close/idle receipts. No qualification timing becomes a metric.
The separate full128 ABBA cohort is now admitted as descriptive native host
diagnostics only, with two runs per attention mode. Its forty raw files are
bound by the frozen manifest and size ledger, then connected to the unchanged
reducer replay. The validator independently recomputes per-run timings from
the original observation offsets, arithmetic means, min/max ranges and paired
ratios. Output rate is the arithmetic mean of per-run rates, not a pooled rate.
Wave's substantial observed variation prevents a stable-gain claim. No HTTP,
GPU-duration, confidence, competitive, default-promotion or M1 claim follows;
qualification timings and older cohorts are excluded. Existing
`matched128`, `routeCheckpoint`, `routeReadiness`, `argmaxNative` and
`performance.js` data are immutable historical observations in this update.
