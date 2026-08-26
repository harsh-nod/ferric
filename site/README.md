# Ferric project site

This directory is a dependency-free GitHub Pages site. Project status is kept
in one structured source: [`data/project.js`](data/project.js).

When implementation or qualification state changes:

1. Update `updated`, `readiness`, `latestObservation`, and `recentProgress` in
   `data/project.js`.
2. Keep diagnostic, hardware-observed, and qualified claims distinct.
3. Run the local checks from the repository root:

   ```sh
   node --check site/data/project.js
   node --check site/app.js
   python3 -m http.server 4173 --directory site
   ```

The deployment workflow publishes only this directory. No generated framework
output or package installation is required.
