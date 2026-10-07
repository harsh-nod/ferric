# Retained Performance Experiments

These source snapshots preserve distinct experiments that cannot safely share
one compiled module or one artifact identity. They are outside the workspace
and do not change default execution. Their historical qualification scopes
are recorded in `docs/performance/`, not upgraded by publication.

- `native-gate-up-a009`: reconstructible exact CPU-qualified source closure.
- `paired-down`: paired kernel schedule, distinct from measured native down.
- `prefetch4-endpoint-loador`: authored, unformatted, untested kernel proposal.
- `ordinary-splitk`: the formatted 20-file ordinary-controller overlay and its
  harness. The retained cohort passed 91 tests but failed strict Clippy; seven
  roles were deferred. It is not the native down worker. The separate unqualified
  lint repair is not substituted here. The full baseline archive identity is
  `302b74237a3909d750913b4c08f779295b377a438371cb96c50e38829fe7bc6a`.
- `gate-up-decode-diagnostic`: historical diagnostics tied to the frozen gate/up
  source. Absolute paths in its private Cargo manifest must not be rebound to
  the down-only adapter and represented as the same qualified experiment.
- `component-harnesses`: source-only component and packet-timing tools. Their
  external images, custody data, and run inputs remain required. They are not
  interchangeable with full-model or matched HTTP benchmarks.

`source-imports.json` records exact imported source hashes. Generated binaries,
transport archives, raw logs, model files, SSH material, and retirement helpers
are excluded. Historical paths and scripts are retained for provenance; they
are not portable launch authorization. Source preservation is not a successful
fresh build, a performance win, production qualification, or a combined-kernel
measurement.
