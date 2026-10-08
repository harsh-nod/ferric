# Readiness40 Forward-Phase Diagnostic

This opt-in extension attributes host time inside the actual guarded forward
sequence. It changes neither kernel arithmetic nor currentness policy. The
CPU checkpoint does not establish a new GPU measurement, Full2303 correctness,
GPU overlap, a speedup or the 700 tokens/s target.

## Native Measurement

The [actual GPU diagnostic and phase report](native-gpu-v1/README.md) now pass
their original-data checks. One MI350 TP2 attempt completed 40 prompt positions
through all 36 layers in 445.407950 seconds, with zero generated tokens.
All 40 semantic completion records and four captured numeric payloads match
the historical Ferric baseline; each run's own transcript chain validates.
Export and retain revalidation preserve all 169
original archive members.

Warm positions 2-39 contain 68.043397297 seconds of worker forward-body time.
The layer phase accounts for 59.301173639 seconds (87.151988%), including
19.614708133 seconds of nested measured callbacks. The remaining layer work
is not separately measured GPU compute. The report separates first-use rows,
disjoint phases and nested callbacks, and keeps differently bounded parent
intervals separate. No matched speedup, GPU overlap or Full2303 acceptance is
established.

## Why Add It

The preceding [native currentness report](../guarded-mlp-currentness-duration-v1/report-v1/README.md)
attributes 21.263499761 seconds of disjoint bank/layer/tail intervals within
68.571861417 seconds of warm parent-forward wall time. The remaining
47.308361656 seconds is unprofiled. These historical numbers motivated this
diagnostic; they are not measurements of the new code.

## Measurement Boundaries

The existing `engineering-currentness-duration-diagnostics` feature enables
this extension only for the explicitly selected diagnostic Readiness40 Tail
route. Default builds and other routes do not call the phase clock or sink.
Ten shared timestamps delimit nine adjacent intervals:

| Phase | Included Work |
| --- | --- |
| Input | Initial deadline checks and transcript/request admission |
| Metadata | Existing metadata operation and its following deadline check |
| Embedding | Embedding operation and its following deadline check |
| Bank | Bank operation and its following deadline check |
| Layers | All 36 layers and their existing per-layer deadline checks |
| Tail | Final normalization/head/argmax operation and deadline check |
| Frame | Control/frame construction, hashing, validation and deadline check |
| Fence | Existing fence and deadline check |
| Commit | Backend commit, transcript advancement and final deadline checks |

Each row's integer phase sum must equal its whole-forward body duration.
Rows cover positions 0-39; the first two old callback rows remain unmeasured.
The existing bank guarded-body, layer callback subtotal and tail callback
subtotal must fit inside their corresponding new intervals. These nested
callback measurements must not be added to the new phase totals.

The body excludes Driver construction, outer request parsing and response
serialization/flush, and final diagnostic collection/encoding. It is neither
whole parent wall time nor whole worker-request wall time. Host elapsed time
can include GPU waiting, host checks and scheduling; it is not GPU kernel time.

## Publication And Failure

The original policy and currentness codecs are unchanged. A third canonical
LF-terminated record, `FerricReadiness40ForwardPhaseDurationsV1`, authenticates
the exact LF-inclusive second record and the policy/session/worker/transcript.
The parent still authenticates the entire original stderr file. The combined
cap remains 69,632 bytes; the third record has its own 32,768-byte cap.

The Sequence guard stays armed until the final timestamp and typed row have
been accepted. A late failure poisons the sequence and prevents successful
publication; already committed state is not rolled back. The original 42
backend effects and 46 deadline checks retain their ordering. All three records
are validated and encoded after healthy Close before the first stderr write.
I/O failures propagate; partial external writes are not claimed to be atomic.

The new record claims host timing only. Its GPU timing, numerical acceptance,
performance and execution-authority flags are false. Each row and the sum of
all 40 body durations remain bounded by one hour; no deadline is extended.

## Qualification

All source is authored and formatted on `mi350`. Fresh private Cargo caches
and targets use locked, offline builds, CPUs 8-9, nice 10, two Cargo jobs and
hidden GPUs. The 40/38 GiB free-space floors, owned process-group supervision,
whole-run bound and existing per-command bounds are retained. Local work is
limited to evidence verification, integration and publication.

| Configuration | Passed | Ignored | Supervised Steps | Status |
| --- | ---: | ---: | ---: | --- |
| Worker, diagnostic | 885 | 4 | 8 | Passed |
| Worker, default | 838 | 4 | 8 | Passed |
| Parent, diagnostic | 622 | 0 | 69 | Passed, 60 selected scopes |
| Parent, default | 584 | 0 | 67 | Passed, 58 selected scopes |

The diagnostic build adds 32 worker tests and 27 parent tests, including the
shared codec and real Sequence tests. Default builds add no tests. The parent
results are selected CPU scopes, not its entire test suite. The four worker
ignores are the same hardware tests as before. All 829 runtime bodies are
unchanged; their previous qualification is preserved, not claimed as rerun.

The first staging attempt refused before Cargo because a helper-loop variable
shadowed the predecessor hash. Its original controller and inputs are retained,
alongside an explicitly labeled root-authored refusal audit, not raw stderr.
The fresh retry derives that pin directly from the authenticated receipt bytes.
This was a qualification-script failure, not a Rust test or GPU failure.

## Original Evidence

The complete original byte capsules are the archives below. The source capsule
contains the 15-file overlay, authenticated preimages, formatting evidence and
the pre-qualification proposal; its `compiled: false` records when it was
created, not the later qualification result. The CPU archives retain the actual
terminal receipts, named results, commands, source maps and product hashes.
No ELF, model weight or private Cargo cache is distributed in these archives.

- [Source and formatting capsule](source-v1.tar.gz)
- [Original staging refusal](worker-diagnostic-cpu-v1.tar.gz)
- [Instrumented worker qualification](worker-diagnostic-cpu-v2.tar.gz)
- [Default worker qualification](worker-default-cpu-v2.tar.gz)
- [Instrumented parent qualification](parent-diagnostic-cpu-v2.tar.gz)
- [Default parent qualification](parent-default-cpu-v2.tar.gz)

The [checkpoint manifest](manifest.json) binds the archive bytes, terminal
receipts, test totals, source proposal and integrated Rust postimages. Each CPU
archive also contains an `export-manifest.json` covering its exact original
members. The source capsule's documentation drafts are historical drafts;
this README and the linked current progress record describe the completed
qualification.

The recorded controllers are historical MI350 recipes with explicit host and
source bindings, not portable install scripts or public GPU-launch entry points.
The existing [community demo](../../docs/GFX950_COMMUNITY_DEMO_V1.md) uses the
separate retained-evidence report and requires no new model launch.

## Next Native Gate

The [new three-record checker](checker-cpu-v1/README.md) now passes all 146
tests on MI350: the unchanged 126-test suite plus 20 new tests, across two
naturally retired supervised processes. Its original evidence and separate
source/data review are retained. These synthetic checks are not GPU execution.

The [new native-admission workflow qualification](native-admission-cpu-v1/README.md)
also passes all 227 tests in 112.153889 seconds: 171 unchanged legacy tests,
29 CPU-admission tests, 12 three-record retention tests and 15 preparation tests.
All four supervised children exited naturally and were reaped with absent
process groups. The 146 checker tests and 227 workflow tests overlap in their
inherited cases; they are not 373 distinct tests.

The new tests exercise the real admission/preparation entry points against
authenticated MI350 CPU receipts, source maps and ELF bytes, plus synthetic
retention success/failure/absence cases. They preserve whole-original stderr
and all existing two-record tests. The complete 389-member workflow archive
is retained without duplicating its 364 source/fixture bodies in this directory.
Data-only native request preparation has separately passed. Native GPU results
remain separate evidence and are not established by this CPU checkpoint.
Shortening stderr, fabricating predecessor fields or relabeling old measurements
remains invalid.

Full2303 GPU execution, exact independent 256 generated IDs and raw decoded
bytes, sustained throughput and issue #42 M0-M7 remain open.
