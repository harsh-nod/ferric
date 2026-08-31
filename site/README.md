# Ferric project site

This directory is a dependency-free GitHub Pages site. Project status is kept
in one structured source: [`data/project.js`](data/project.js).

When implementation or qualification state changes:

1. Update `updated`, `readiness`, `validation`, `latestObservation`, and
   `recentProgress` in `data/project.js`.
2. Keep diagnostic, hardware-observed, and qualified claims distinct.
3. Run the local checks from the repository root:

   ```sh
   node --check site/data/project.js
   node --check site/app.js
   node site/validate.mjs
   python3 -m http.server 4173 --directory site
   ```

The deployment workflow runs the same syntax and structured-data checks before
publishing only this directory. `validate.mjs` also rejects unknown status
states, malformed source references, duplicate transitions, missing render
targets, and missing local assets. No generated framework output or package
installation is required.
