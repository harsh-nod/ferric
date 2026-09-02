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
   ```

The deployment workflow runs the same syntax, structured-data, and Chromium
checks before publishing only this directory. `validate.mjs` rejects schema
drift, stale current-dependency claims, unknown status states, malformed source
references, duplicate transitions, missing render targets, and missing local
assets. `render-validate.mjs` checks populated visible output without horizontal
overflow at 1440px, 390px, and 320px.
