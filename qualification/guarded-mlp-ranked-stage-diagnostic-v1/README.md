# Ranked Validation Boundary Diagnostic

Engineering evidence for [Ferric issue #42](https://github.com/harsh-nod/ferric/issues/42),
2026-10-08 UTC. This checkpoint identifies a compiler refusal boundary.
It does not fix that refusal, emit a worker HSACO, or establish GPU/model
correctness or performance. All M0-M7 milestones remain open.

## Result

Both actual worker compilations report the same original lowerer cause,
now qualified by its failing boundary:

```text
ranked projection structural validation failed: stage wave-replay; root 1; body 1:
semantic-to-Kernel-IR correspondence no longer matches its exact owner
```

The exact single-line errors are in [fixed stderr](pair/fixed/compile.stderr)
and [early stderr](pair/early/compile.stderr), line 67. The replay boundary
precedes ranked root projection and CFG compaction in this source generation.
It does not identify which internal replay predicate failed. Root/body IDs
are the observed numeric diagnostic context, not a separately decoded
source-name or helper-identity claim.

| Actual attempt | Exit | Leaf seconds | Whole seconds | Final HSACO |
| --- | ---: | ---: | ---: | --- |
| Fixed-round V15 | 1 | 121.648421269 | 122.598608377 | None |
| Early-stop V15 | 1 | 120.481832295 | 121.422458648 | None |

Both processes exited naturally, were reaped, and left no owned process group.
Neither timed out, received a signal, required forced cleanup, exceeded a
storage limit, or had a source/tool postcheck error. These are successful
lifecycle checks of failed compiles, not successful compiles.

## Source Change

The [three-file patch](correction.patch) is relative to the
[qualified Index-to-U64 transport parent](../guarded-mlp-call-index-transport-v1/README.md).
Its acceptance predicates, traversal order, budgets, worker provider, helper
ABI, call transport, and runtime behavior are unchanged.

- A closed seven-value stage enum identifies finite replay, wave replay,
  finite selection, wave selection, finite source-call checking, generated
  finite effects, or final ranked-candidate custody.
- A new error variant retains the original lowerer error and
  `Error::source()` chain. The legacy variant and display remain unchanged.
- Context contains numeric root/body IDs and a fixed stage label, with no
  IR/module/source dump. A missing root displays `unknown`, not zero.
- Existing root-context propagation fills only a missing root, preserving
  an already present root and the innermost failed body.
- The two replay adapters receive the body ID already selected by the caller;
  they perform no additional selection or replay.

Only new context is bounded by the fixed stage vocabulary and numeric IDs;
this does not claim that every preexisting nested error display is bounded.
The preserved legacy error variant produces one new non-test dead-code
warning. The test build has no new warnings.

## CPU Qualification

All work was authored, built and tested on `ssh mi350`, host
`smci350-rck-g03-b19-03`. The [complete receipt](cpu/complete.json) records
19 clean phases in 276.173908186 seconds:

| Full suite | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Compiler library | 1,339 | 0 | 24 existing |
| Device library (CPU tests) | 359 | 0 | 0 |
| Semantic lowerer integration | 125 | 0 | 0 |
| Lowerer library | 766 | 0 | 0 |

The exact inherited compiler roster and ignored set are preserved, with three
new nonignored tests. Device and both lowerer rosters are exact parent matches.
Original stdout is retained for the [compiler](cpu/compiler-tests.stdout),
[device](cpu/device-tests.stdout), [integration](cpu/lowerer-tests.stdout),
and [lowerer unit](cpu/lowerer-unit-tests.stdout) suites.

The three appended tests check all seven stage labels, maximum IDs, original
and nested error chains, missing-root enrichment, existing-root preservation,
innermost-body preservation, and legacy display compatibility. A genuine
materialized neutral fixture exercises both replay adapters: ordinary valid
roots still return no wave/finite recipe; a foreign root produces the actual
correspondence refusal with the correct context. Wave replay reserves and
restores the owner's storage floor. Synthetic display tests are not evidence
that all seven boundaries were reached by a production worker.

The final backend build occurs only after all four suites pass. Source,
raw-log, tool, Cargo metadata, dependency and artifact checks bind 5,808
project files, 5,812 inputs, 99 raw CPU evidence files, eight tool pins,
six selected ELF products and 2,247 pinned rust-src files. The two actual
worker retries use this backend and the existing qualified 25-test frontend,
not a reconstructed or unqualified compiler.

CPU work retains cores 8-9, nice 10, offline Cargo jobs 2, 40 GiB initial /
38 GiB live free-space floors, 12 GiB address-space and 6 GiB cache limits.
The suite retains 3,600-second whole, 1,800-second leaf, 1,200-second leaf CPU,
and 50-second cleanup bounds. Each worker compile retains 600-second wall/CPU
and 50-second cleanup bounds, 64 MiB streams, and 1 GiB file limits.
Outer Cargo job settings do not prove the frontend's inner Cargo concurrency.

## Evidence

[manifest.json](manifest.json) binds the exported source, receipts and archive.
[originals.tar.gz](originals.tar.gz) contains 152 members, including 151 exact
originals and an original-path manifest, totaling 58,572,711 expanded bytes.
It includes source preimages/postimages, the parent receipt, the controller
and input manifest, all CPU raw evidence, both seven-file worker fixtures,
both complete compile records, and their original semantic/source-map captures.
The archive was read back member by member before publication.

| Object | Bytes | SHA-256 |
| --- | ---: | --- |
| CPU receipt | 5,785,793 | `23427c2873cae75c5900ce12e1b268edb225e6fd4abb4996c9a29a781254aaeb` |
| Backend | 206,472,888 | `7f9b3217055ef4b22aa39032ff2f38daee6fd0df78b598f613fabe5deded676d` |
| Extractor | 205,248 | `1c8342c8c7964c436da49a2303d72cf6c79e4fecfbe910eb1820b1c490676456` |
| Fixed result | 6,452 | `b562a0f4d18b36850eae4983a21c8a20f03b51b4a7c76f7fd32a738cbe24c9e0` |
| Early result | 6,452 | `c5ba2c597d78890248053d01720061edb8ab5edb1aa613ecfd2575d1ce8c5c66` |
| Patch | 16,844 | `da9f2391f15bbc243330151bc31feae67917768af18f85c638b789ad4af01594` |
| Archive | 8,249,513 | `bc30d0bf88ce257325771ce3cc4d4b0bd45e1cfae1b7a4ac69e3aabdbeadb818` |

Capture bytes and source maps are hash-joined, but these V15 captures were not
independently decoded and joined to typed semantic/source identities. No
earlier attempt's numeric IDs or helper names are imported as V15 facts.

## Next Gate

Source review identifies a candidate: wave recipe preflight requires each
operation's local effect summary to be complete; ordinary defined calls do
not have that local summary even when materialization has proven their
interprocedural effects complete and pure. The current diagnostic does not
prove this is the failed internal predicate.

First reproduce that refusal with a genuine retained-wave source fixture.
Any correction must use exact owner/root/callee-bound evidence and retain
refusals for unknown, effectful, recursive or malformed helpers and exhausted
budgets. Independent checks in PLIRON wave bindings, retained custody and
formal archive decoding also need coverage. Do not mark all calls pure,
skip evidence replay, or treat one repaired guard as end-to-end acceptance.

The existing device-completion hold remains unresolved. No GPU operation was
performed for this checkpoint. Qwen3 BF16 2,048/256 numerical acceptance,
the 700 tokens/s target, gfx942/gfx950 native validation, and milestone closure
remain unproved. Main is unchanged; this is an engineering-branch publication.
