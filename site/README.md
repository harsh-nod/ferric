# Ferric project site

This directory deploys a dependency-free static GitHub Pages site. Project
status is kept in one structured source: [`data/project.js`](data/project.js),
and the validation harness pins Playwright for real browser checks.

When implementation or qualification state changes:

1. Update `updated`, `readiness`, `validation`, `latestObservation`, and
   `recentProgress` in `data/project.js`.
2. Keep diagnostic, hardware-observed, and qualified claims distinct.
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
validates the six-file static deployment roster so `node_modules`, dependency
metadata, documentation, and test-only files cannot enter the Pages artifact.
