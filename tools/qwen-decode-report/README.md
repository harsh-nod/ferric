# Decode Observation Reports

CPU-only validation and bounded SVG plots for retained Ferric captures. Python 3
stdlib only; no device opens, worker execution, plotting dependency or downloads.
The package does not issue M1 admission, authenticate a collector, or turn a
correctness fixture into model throughput evidence.

The current native adapter consumes the exact `FerricDraftDecodeProfile*V1`
NDJSON records emitted by `ferric-qwen3-draft-decode-profile`. This is full-model
**Qwen3-0.6B Draft06B bring-up**, not the single-request **Qwen3-8B 700 tokens/s**
goal. It uses frozen independent token/decoded-byte references, not expectations
learned from the measured candidate. No synthetic records ship as benchmark data.

## Commands

Run from the repository root. Outputs must be new directories.

```sh
python3 -B tools/qwen-decode-report/report.py validate --manifest /retained/run.json
python3 -B tools/qwen-decode-report/report.py draft --manifest /retained/run.json --output /retained/report
python3 -B tools/qwen-decode-report/report.py compare --manifest /retained/ablations.json --output /retained/comparison
python3 -B tools/qwen-decode-report/report.py m1-host --manifest /retained/m1-host.json --output /retained/m1-report
python3 -B -m unittest discover -s tools/qwen-decode-report -v
```

`validate` writes nothing. Other commands validate all inputs before creating
their output directory. They do not modify raw captures. Invalid/missing records,
failed parity, changed pins, unknown fields, incomplete runs or failed teardown
reject the report. Existing output directories are never overwritten.

`m1-host` delegates parsing, fixed-workload equivalence and metric calculation to
the existing M1 `host_timing_summary.py` / `performance_ledger.py` unchanged. Its
aggregate host observations are not event endpoints and cannot create a timeline.
Existing M1 path-resolution and manifest conventions still apply. No new
qualification authority is inferred from this plotting adapter.

## Capture Manifest

`FerricDraftDecodeReportManifestV1` has exactly these fields:

| Field | Meaning |
| --- | --- |
| `schema` | `FerricDraftDecodeReportManifestV1` |
| `authority` | `none` |
| `label` | Public, non-private variant label, at most 64 characters |
| `workload_kind` | `full-model-draft06b-greedy` only |
| `files` | Exact file-pin roster below |
| `expect` | Operator's predeclared setup expectations below |
| `trace` | `null`, or the separately pinned optional trace described below |

Each file pin is exactly `{"path": "relative-or-absolute-path", "sha256": "..."}`.
Paths are resolved relative to this manifest. Files must be nonempty regular
files, not symlinks; hashes and before/after file metadata are checked. Duplicate
JSON keys, nonfinite numbers and bool-as-integer fields reject.

`files` must contain all of:

- `capture`: complete, unfiltered profile stdout NDJSON, at most 16 MiB.
- `reference`: exact independent full/tokenwise reference used for every run.
- `controller`, `worker`: actual executable bytes retained by the dispatcher.
- `hsaco`, `artifact_manifest`: exact `observation.hsaco` / `observation.json`
  files whose byte hashes equal the setup record's artifact IDs.
- `handoff`: either a retained file pin `{path, sha256}`, or exactly
  `{"mode": "manifest-identity"}` for the existing engineering CLI output.
  That CLI deletes its private handoff scratch and retains the handoff hash/length
  in the canonical observation manifest. Identity-only mode derives those values
  from the pinned manifest and matches the setup, without claiming handoff bytes
  were retained or revalidated. Retained-file mode also checks byte length/hash
  against the manifest. Never fabricate a handoff file or add sidecars inside the
  closed two-file engineering artifact directory.
- `model_manifest`, `environment`, `plan`, `compiler`: retained model-bundle,
  sanitized environment, predeclared workload and compiler provenance records.

All inputs are bounded to 256 MiB each. The last four records are opaque hash
bindings, not authenticated facts or a replacement for M1 artifact/model checks.
The input model weights themselves are not reread or rehashed by this reporter;
the setup and independent reference must agree on the four exact model identity
fields. Hash matching detects drift, not a dishonest producer or false hardware.

`expect` has exactly:
`model`, `model_revision`, `identity`, `projection`, `prefill`, `head_precision`,
`warmups`, `samples`, `new_tokens`, `runtime_cache_admission`,
`runtime_operational`, `device_unique_id`. The four `identity` fields are
`model_bundle_id`, `draft_model_id`, `draft_config_id`, `draft_weights_sha256`.
Predeclare these before dispatch; filling them from the result afterward does not
establish an independently selected workload. The reporter cannot attest when an
operator created a manifest.

The observation join checks canonical JSON encoding/top-level field order, exact
V10 crate/target/scope, no grants, HSACO byte identity and handoff identity. It is
deliberately not a second descriptor, source-roster or native admission verifier;
the actual runner's unchanged `open_draft32` owns those checks. Reports retain
the handoff evidence mode explicitly alongside the observed identity.

The raw input selector and PID remain private and are used only for consistency.
Reports omit both, input paths and absolute clock origins. Public labels and
optional trace event labels must themselves contain no private identifiers; free
text is not an identity-redaction service.

The adapter validates exact prompt length 5, initially empty KV, reference input
and position schedule, every selected row/choice/cache cursor, all output tokens
and decoded bytes, per-forward 480 dispatches, the whole run budget and final
worker exit. Full prefill forwards `N` times; tokenwise prefill forwards `N+4`.
All pages must be returned after each run. It preserves every warmup and measured
run; retries, record omission, or selecting the fastest subset are not supported.

## Measurement Semantics

The controller clock is `CLOCK_MONOTONIC_RAW`, offsets from one capture origin.
These are host intervals including controller/IPC/queue/waits and checked KV
commit. Per-step observation emission between token completions is included.
They are not GPU event time. Setup, tokenization, retirement and teardown are
outside each reported request window and are separately identified; the whole
controller interval is not end-to-end model throughput.

- TTFT: first checked emitted-token commit minus request start.
- Post-first decode: `(N-1)/(last commit-first commit)`, per request.
- R33 TPOT: integer floor of the same post-first duration divided by `N-1`.
- ITL: each adjacent emitted-token commit gap; prefill steps are not outputs.
- Aggregate output/window: total measured output tokens divided by the interval
  from the first measured request start to the last measured terminal. This
  includes the gaps between requests and is not single-request decode speed.
- Pooled post-first decode: sum of post-first tokens divided by sum of per-request
  post-first durations. This is not an aggregate serving-window rate.

Existing M1 `statistics` and nearest-rank percentile semantics are reused. No
interpolation, tail-confidence assertion or causal significance claim is added.
Warmups are excluded from metrics, retained in host plots. Ten warmups / thirty
samples earns only the CLI's `benchmark-sized-unqualified` label. One worker
lifetime and short output lengths do not establish steady-state serving, the
multiple independent starts required by M1, or performance qualification.

The immutable no-rollover packet cap is 131072. The reporter rechecks
`(warmups+samples)*forwards*480`; for example full-prefill N=4 with 10/30 fits,
whereas N=8 with 10/30 does not. It never silently shortens the requested work.

## Ablations

`FerricDecodeAblationManifestV1` has exactly `schema`, `authority: "none"`,
`baseline` and `variants`. Each of 2..16 variants is
`{"name": "public label", "manifest": {"path": "...", "sha256": "..."}}`.
All referenced raw captures are revalidated, not trusted from cached reports.

Model/revision/identities, prompt, output length and expected decoded result,
context, precision, TP, cache policy, sampling counts, clock and environment hash
must match, as must the private device selector. Full/tokenwise reference files
may differ, but both must independently validate the same outputs. The report
lists changed runtime options and provenance bindings, ratios and descriptive
rates; multiple simultaneous changes are not a single-factor causal experiment.

Trace overhead remains **unmeasured**. This capture has no matched trace-off
switch, so an ablation cannot subtract recording cost or label it zero.

## Optional Timeline Import

No admitted gfx950 device-clock collector is available in this integration.
Default `host-timeline.svg` uses only actual per-forward host endpoints. It never
fills a GPU lane from dispatch counts, aggregate durations or assumed overlap.

Optional `trace` is exactly `{record: pin, collector: pin, clock_evidence: pin}`.
`record` uses `FerricDecodeTraceV1`, with exact fields:
`schema`, `authority: "none"`, `capture_sha256`, `artifact_sha256`, `collector`,
`clocks`, `events`, `dropped_events: 0`.

The collector is `{kind, source_sha256, clock_evidence_sha256, overhead}`.
Kinds are `host-raw-spans`, `device-timestamp-buffer`, `rocprofiler-sdk`; overhead
must be `unmeasured`. Collector and evidence hashes must match retained bytes.
Clock evidence has schema `FerricDecodeClockEvidenceV1`, exact capture/artifact
hashes and the identical clocks roster. A producer name alone is not evidence.

Each clock has `id`, `kind`, `ticks_per_second`, `counter_bits: 64`,
`unwrapped: true`, `max_drift_ppm`, `anchors`. Host clocks use
`controller-monotonic-raw`, 1 GHz, zero drift, no anchors and controller offsets.
Device clocks use `device-counter`, at least two strictly increasing
`{tick, host_offset_ns, max_error_ns}` anchors spanning all event ticks. Declared
frequency/drift/error must agree with those anchors. A 32-bit/truncated or
unbracketed counter is rejected. Calibration is structurally checked, not
authenticated as originating from the physical device.
Each mapped event must also fit the actual reserve-through-commit host interval
for its stated run/output token. Imported traces do not relabel prefill as an
emitted token. Calibration uncertainty remains visible rather than subtracted
from host latency.

Events have exact `id`, `label`, `lane`, `clock`, `run`, `token`, `begin_tick`,
`end_tick`. Limits: 4096 events, 128 lanes, 16 clocks, 24-hour controller window.
Only same-clock, distinct-lane interval union contributes to a reported overlap;
nested spans on one lane are not parallel execution. Cross-domain overlap is
never computed. Whiskers show the declared clock uncertainty. Even an accepted
device trace remains producer-reported evidence, not protected runtime or source
qualification. Do not use unverified rocprof interception for direct-KFD queues.

## Target And Analytical Context

The target is single-request, batch-one Qwen3-8B decode at 700 tokens/s, not
aggregate throughput and not Draft06B. No measured target success is claimed.
The separate BF16 weight-streaming calculation binds revision
`b968826d9c46dd6066d109eabc6255188de91218`: 16381470720 bytes minus the full
embedding plus one row gives 15136819200 bytes/token. The product-optimistic
8 TB/s yields 528.512622 tokens/s. The installed host's operator-reported
6.810 TB/s advertised maximum yields 449.896369 tokens/s (the JSON computes the
unrounded value). Neither is measured sustained bandwidth; the calculation has
no cache credit, omits KV/activation/compute costs and is not a universal bound.
No precision change is made by this tool. Product source:
https://rocm.docs.amd.com/en/latest/reference/gpu-arch/mi350.html

Tests generate synthetic records only in temporary directories, clearly labeled
as CPU tests. These tests validate rejection and arithmetic, not model execution
or any observed performance value.
