# Ferric project site

This directory deploys a dependency-free static GitHub Pages site. Project
status is kept in [`data/project.js`](data/project.js); curated, strictly
checked performance observations live in
[`data/performance.js`](data/performance.js). The harness pins Playwright for
real browser checks. Run all checks on the designated remote build host, not
locally, and remove the private stage after archiving evidence.

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
