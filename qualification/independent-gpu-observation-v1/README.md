# Independent GPU Observer Policy Qualification

The independent-profile observer passed **81 in-process CPU policy tests on
MI350**, with no failures or skips, on October 3, 2026. The actual unittest
transcript reports 5.328 seconds. This is a test-suite duration, not GPU
latency or model throughput.

| Suite | Passed |
| --- | ---: |
| Preparation, artifact and runtime review | 18 |
| Owned case lifecycle and retained replay | 15 |
| Frozen request and capture validation | 15 |
| Frozen child-process custody | 10 |
| Frozen independent observation adapter | 23 |
| **Total** | **81** |

The complete frozen eighteen-file source package is under `source/`, including
its manifest, unchanged custody/reference helpers and historical controller
preimages. `evidence/` contains the actual test receipt, original source
snapshot, complete named unittest transcript and exact test runner.
`result.json` joins their original FilePins to the published bytes.

The suite tests independent parent schemas, finite capture differences without
a bitwise-parity gate, exact child and leaf identities, refusal of stale
provenance, complete audit replay, failure-path post-audits, and separation
of numerical work from the GPU cleanup interval. Lifecycle/resource helpers
are checked against their frozen preimages. Synthetic process metadata and
file-backed captures in these tests are not real GPU observations.

## Runtime Contract

The controller uses the new independent inspect/execute selectors while
preserving baseline-first profile execution, the original child mode, ten
child sidecars, natural Close, one native attempt and zero retries. The
unchanged owned-process helper tracks and reaps only task-owned descendants.

The limits remain 180 seconds per native/inspection leaf, 30 seconds per audit,
three pre-audits plus three post-audits, 2400 seconds per case, two CPUs,
12 GiB leaf address space, 8 MiB stream/file limits, 32 MiB case evidence and
40/38 GiB initial/ongoing free-space floors. Post-audits run in `finally`,
including after native or custody failure.

The read-only `replay_case` API rechecks all eight retained owned leaves,
six distinct audit records, parent schemas, ten child sidecars and all four
complete capture bodies. A successful GPU observation would certify closed
captures and custody only, not numerical correctness.

## Evidence Boundaries

This publication does **not** run native code or launch a GPU kernel. It does
not claim an actual new-profile capture replay, independent numerical
acceptance, full-prefix/model correctness, production readiness, performance
parity, or 700 tokens/s.

The compiler/image and native-binary qualifications are separate evidence.
Actual deployment and fresh runtime/source/ISA reviews remain prerequisites
for execution. The six selected GPU cases must each produce new authenticated
captures. Only after the GPU controller exits, all post-audits complete and
its owned processes are reaped may a separate bounded CPU leaf compare
baseline and candidate independently against the unchanged references.

`source/README.md` is part of the immutable tested package and records the
authoring-time scope. Its manifest's `tests_executed: false` is a pre-run
declaration, not the actual test result. The subsequently retained
`evidence/complete.json` and this qualification record report the executed
81-test pass without rewriting that frozen manifest.

`publisher.py` verifies the exact manifest, all eighteen source bodies, source
snapshot, full named test census, transcript and retained helper hashes before
publication. It imports or executes none of the observer, native, GPU or test
modules, and refuses a conflicting existing output directory.
