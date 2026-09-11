# TP1 Diagnostic Acceptance

`adapters/m1-engineering-execution-v1/tools/check_tp_numerical_diagnostic.py`
checks a separately archived numerical-capture run. It never produces an
accepted performance comparison or ledger input. Every successful output has
`performance_qualified: false`, even when every fixed-reference token matches.

Use the original zero-exit case, globally idle before/after snapshots, unchanged
raw JSONL, the capture directory, and externally frozen identity inputs. The
expectation object uses schema `FerricTpNumericalDiagnosticExpectationV1` and
requires controller/worker/HSACO/manifest/handoff SHA-256, workload/reference
SHA-256, all eight physical GPU IDs in their original order, prefix-cache and
pruning booleans, the exact seven-key performance profile, collective, and
`selection: {batch_ordinal, layer, role}`. The selection role is the exact Rust
name, for example `Query`. The legacy host collective expectation is `null`.

This checker supports the fixed four-request TP1 workload with 16-row/16-token
chunks, baseline or MFMA projection, baseline attention, and optional operational
currentness checks. Admission caching, sequencing, rollover, runtime profiling,
wide kernels and replica cohorts are outside its explicit scope.

```bash
python3 -B adapters/m1-engineering-execution-v1/tools/check_tp_numerical_diagnostic.py \
  --run-dir /absolute/archived/case --capture /absolute/archived/case/capture \
  --workload /absolute/workload.json --reference /absolute/reference.json \
  --expect /absolute/externally-frozen-expectation.json \
  --manifest-sha256 EXPECTED_MANIFEST_SHA256 --output /absolute/new-diagnostic.json
```

Exit 0 means capture custody and the unchanged strict fixed-reference comparison
both passed. Exit 2 writes a diagnostic-only report with valid capture custody
but failed exact-reference parity. Malformed or incomplete evidence raises an
error without publishing a successful diagnostic receipt. Output creation is
exclusive. The manifest digest must be bound externally to the completed run,
not selected from an unrelated untrusted manifest.

The checker verifies complete observed-token causality, schemas, request and
batch identity, cache bookkeeping, dispatch counts, clock consistency, clean
close, and both physical idle snapshots. Observed tokens are used only to verify
that the next decode input and final request agree with actual committed output;
they are never substituted into the fixed reference. Decoded text/bytes must be
self-consistent here, and the unchanged reference checker separately requires
the exact pinned reference text and token IDs.

Capture binding checks the Closed receipt, manifest bytes/hash/cap, all payload
hashes and exact extents, selected scheduler/pool batch, execution-row mapping,
projection role/kernel/shape/layout, descriptor widths, and head rows against
published output. Full finite BF16 logits are independently rechecked for
lowest-ID argmax, top16, watched values and selected LM weight row IDs. The
frozen offline replayer additionally verifies full MFMA transpose identity and
its sampled actual-operand diagnostics. A sampled difference from serial FP32
does not relax token parity and is not by itself a capture-custody failure.

For the final strict-reference call only, an in-memory copy removes the exact
validated Setup and Closed `numerical_capture` objects and restores the exact
normal numerical-status literal. This normalization is listed in every report.
No raw file is changed, no normalized trace/report is saved, no checker constant
is patched, and every returned timing metric is discarded. The production
performance checker and ledger remain unchanged and reject diagnostic traces.

The tool pins its dependency checker and replayer source hashes. A future source
update requires explicit review, tests, and a newly pinned diagnostic checker;
historical evidence is not silently reinterpreted.
