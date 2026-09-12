# Wave Argmax Submission Canary V1

This additive source experiment builds on Ferric driver source
ad0663303a642a5d5c5b7e300b4a6f3fab5bced3, tree
7f8cc3ce530e5f163aa2f929e12e796676521ddc. That driver already provides the
atomic ordered wave-attention/v11 selector and focused recording/failure
contracts. This canary does not change kernels, KFD/compiler code, the driver,
the default route, prior CLI parsers, or frozen evidence/checkers.

## Closed Interface

The separate `ferric-qwen3-wave-argmax-submission-canary` binary requires the
existing `tp-batch-engineering` feature and all existing attention-canary
arguments. It additionally requires `--submission synchronous|ordered`, and
accepts only `--attention wave` and `--argmax-mode wave-v11`. The new parser
preserves option/value boundaries and forwards remaining arguments through
the unchanged attention parser. Prior parsers still reject `--submission`.

Both modes use new schemas:

- FerricWaveArgmaxSubmissionCanarySetupV1
- FerricWaveArgmaxSubmissionCanaryPrefillV1
- FerricWaveArgmaxSubmissionCanaryObservationV1
- FerricWaveArgmaxSubmissionCanaryClosedV1

Their field sets are unchanged. Setup binds attention=wave, argmax_mode=wave-v11
and the actual runtime_ordered_batches Boolean. There is no open-ended profile
override. The old profile schemas, output fields, error prefixes and selector
branches retain their prior values. Frozen executable identities are not
relabeled as rebuilt binaries.

The parser enables the worker's ordered capability only for SubmissionOrdered.
Before TimingFile creation, reference/model reads or worker launch, the runtime
rejects new-profile mismatches: ordered capability must match the selected
profile; cache admission, operational currentness and rollover must be enabled;
sequences, runtime profiling and shared full currentness must be disabled;
argmax must be wave-v11 and output extent must be8 or128. Invalid options are
not repaired by the runtime.

Both profiles retain preallocation v5/v8/v11 image admission, MFMA, TP1,
capacity32, pruning, device residual reduction, context256,16 pages and
chunk16. Synchronous mode uses the existing wave/v11 selector. Ordered mode
uses the already reviewed atomic selector after wave attention and the v8
head are configured. No late generic ordered setter is used. The workload
schema remains FerricArgmaxCanaryWorkloadV1 with the unchanged8/128 references.
The reservation, commit, packet, retirement, close and failure/quarantine loops
remain shared and unchanged.

## Timing Contract For A Future Checker

No timing instrumentation or label is added or renamed. A separately reviewed
checker must consume the new raw schemas and truthful ordered flag directly;
it must not normalize records or edit either existing checker.

Per completed batch, the ordered path must have the following closed affected
phase roster. Counts below are aggregated across36 layers, not GPU durations.

| Phase | Required spans and transport counts |
| --- | --- |
| attention | attention parent36; each of seven attention_* sibling spans36; dispatch_each360; no IPC records or response-wait spans |
| feed_forward | feed_forward parent36; dispatch_each180; no IPC records or response-wait spans |
| collective_attention | parent36; flush_ordered_batches36; dispatch_zero36; ipc_response_wait72; ordered IPC send/roundtrip36 carrying360 packets; synchronous dispatch send/roundtrip36 carrying36 packets |
| collective_feed_forward | parent36; flush_ordered_batches36; dispatch_zero36; ipc_response_wait72; ordered IPC send/roundtrip36 carrying180 packets; synchronous dispatch send/roundtrip36 carrying36 packets |

`dispatches` belongs only to terminal IPC roundtrips, never send/span rows.
Ordered response payloads remain empty. Preparation spans have rank=None;
IPC and response-wait rows retain rank0. Synchronous mode retains the prior
attention/FFN dispatch and wait roster with no ordered flush/group records.
The exact grouped command order is [10,5] per layer, as already covered by
driver recording tests; aggregate timing alone cannot prove individual root
order. Embedding remains one synchronous dispatch; residuals remain72 total
synchronous dispatches; selected output heads remain three synchronous
dispatches with one four-byte choice readback. Packets remain613 for an
unselected prefill batch and616 for a selected batch, totaling9,219/83,139 for
8/128 outputs. Packet rollover still uses the existing conservative per-batch
reservation and must not be counted as a kernel dispatch.

The seven attention siblings still fit their preparation parent, but no longer
include the deferred GPU execution or worker wait. The collective flush spans
include packet packing, submission and aggregate completion waits. Their
parents also include the synchronous residual. IPC spans overlap the flush
and collective parents. Only explicitly disjoint siblings may be summed;
none of these host spans is a kernel GPU duration. Keep the existing host
prefill-start-through-token-commit timing boundary, excluding setup.

## Validation Plan

Source-only candidate, not yet built or run. Focused tests cover five new
parser methods, three new runtime-profile methods (including20 direct mismatch
cases and no sidecar creation), and a source-policy boundary. Re-run legacy
parser/profile/retirement tests, the six ordered-driver contracts, focused
attention/v11 tests, both frozen workload goldens and admitted-image tests.
Remote formatting, strict Clippy and full host admission require root review
and a separately pinned source snapshot; no local builds or tests.

Only after that gate should a separate wrapper/checker establish native8 and
128 correctness for both modes against the unchanged oracle with fresh exact
controller/worker/image pins, normal close and shared-machine lifecycle
checks. Preserve every result. A later matched measurement must use the new
binary for both modes, not substitute old-cohort values or claim additive
gains. No serving, default, statistical, competitive or M1 qualification is
conferred by this source change or its recording tests.
