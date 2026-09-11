# Ferric project site

This directory deploys a dependency-free static GitHub Pages site. Project
status is kept in [`data/project.js`](data/project.js); curated, strictly
checked performance observations live in
[`data/performance.js`](data/performance.js). The harness pins Playwright for
real browser checks. Run all checks on the designated remote build host, not
locally, and remove the private stage after archiving evidence.

The new recovery checkpoint is unpublished until separately approved. The prior
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
are now explicitly approved; no matched result is available at this cutoff.
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
execution, speculative serving or draft performance. The 656 aggregate has
516 test invocations, not a deduplicated count of unique tests; its 18 explicit
ignores and prior failed attempts are retained. No new Verus claim follows.

Continuous V3 tooling has no drain between adjacent windows, retains failures,
and separates completion-window usage from arrival cohorts. The 90-test
collector and 109-test paired-series gates are CPU-only. Descriptive bootstrap
intervals require at least three fresh start pairs; this is not evidence of
stationarity, stable tails or equal framework tuning. The approved sequential
baseline launches remain without a matched performance result at this cutoff.

`validate-competitiveness-recovery.mjs` checks the closed new snapshot, negative
scope mutations, all twelve independently pinned ordered reports/raw traces,
per-mode means, standalone reference/trace custody, native36 output/guard
receipts, exact 656 aggregate and the two V3 CPU logs:

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
