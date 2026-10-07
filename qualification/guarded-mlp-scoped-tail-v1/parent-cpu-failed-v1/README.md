# Original Parent Compilation Failure

The first Tail V4 parent qualification on `mi350` failed after five supervised
phases in 248.881 seconds. The parent test-binary build exited naturally with
Cargo status 101; all five process groups were reaped and absent, and source,
dependency, cache and tool postchecks passed. No selected test or GPU ran, and
no parent product was accepted.

The original [compiler stderr](evidence/parent-lib-list.stderr) reports E0433:
the shared `guarded_mlp_long_sequence_v2_tests.rs` test backend calls
`crate::native_catalog::forward::checked_scoped_tail_bytes`, but the parent
imports that pure sequence module without the worker-only native catalog.
The production byte-checker has separate worker tests; the sequence failure
fixture needs to remain independent of the worker backend.

- [Failed terminal](evidence/failed.json): 3,644,095 bytes,
  `086ec9aa9972605cb2cb6e63239b8dfb44a5e78d2b8b0077301e507e1927c183`.
- Original archive: 2,634,207 bytes,
  `6552cfa2e94228b152bec53726fab6068c9b347c53337ef428fdef370b064353`.

All 189 archive originals are retained: the manifest pins 188 bodies, including
29 raw bodies and the unchanged failure receipt. The complete 1,293-source map
and five formatter transitions are preserved. This README is additional
commentary, not an original run artifact. The failure remains a failure.

The repair will use a self-contained error-injection fixture, preserve the
production byte-checker coverage and run fresh coupled and parent qualifications.
Review also identified the newly shared sequence test as an omitted parent
inventory entry; the retry must derive its exact name and selection from the
source rather than retain the original projected counts. No failed parent
source is integrated or admitted for a native run.
