# Ferric M1 Benchmark Suites

These five Ferric-owned CLIs bind the open M1 benchmark paths to deterministic
run plans and fail-closed ingestion of externally collected records:

```text
cargo run --locked -p ferric-m1-benchmarks --bin ferric-m1-SUITE -- describe
cargo run --locked -p ferric-m1-benchmarks --bin ferric-m1-SUITE -- plan INPUT OUTPUT
cargo run --locked -p ferric-m1-benchmarks --bin ferric-m1-SUITE -- validate PLAN RECORDS OUTPUT
```

`SUITE` is `differential`, `adversarial`, `d10`, `speculation`, or `serving`.
Inputs, plans, records, descriptors, and transcripts use canonical
pretty-printed ASCII JSON with one trailing newline. Plans bind the exact
source closures, artifacts, model inputs, environment, protocol, workload,
schedule, and dispatch graph. Record sets bind their plan and immutable raw
companions by SHA-256. Missing, duplicated, reordered, failed, malformed,
undersampled, or identity-substituted observations are rejected.

An accepted ingestion transcript establishes only that the declared records
are structurally complete and identity-bound. It is not an M1 evidence-index
artifact and does not establish numerical correctness, fault safety, D10
performance, speculation speedup, serving competitiveness, hardware behavior,
or qualification. Real MI300X/model/reference/baseline records and the
independent M1 evidence validators remain required. Requirements `m1.r29`
through `m1.r33` remain `Open`.

Before any D10 observations, an external policy owner can freeze the complete
core-kernel experiment policy:

```text
cargo run --locked -p ferric-m1-benchmarks --bin ferric-m1-d10 -- \
  admit-experiment-policy POLICY-ROOT OUTPUT-BUNDLE
```

`POLICY-ROOT` contains exactly `policy.json`, the source-controlled
`protocol.json`, and canonical timing, tuning, execution-order, telemetry,
resource-inspection, calibration, holdout, and regression-reference companions.
The policy covers the exact seven K1-K7 case/family positions and freezes each
external profile, work-unit definition, Ferric implementation, vendor
applicability and implementation, weight, threshold, and companion identity.
It freezes a requested count of exactly 10 untimed warmups followed by 30
recorded samples per case. Calibration and holdout companions carry nonempty,
disjoint canonical member rosters whose digests are recomputed, and the tuning
companion must bind the calibration roster and give Ferric and the vendor equal
tuning budgets.

Ferric supplies no default threshold, weight, vendor mapping, tuning budget, or
work-unit value. All inputs remain held through descriptor-relative rereads and
no-replace publication. `OUTPUT-BUNDLE` contains exactly `admission.json` and
the admitted protocol. Its authority is structural only, its status is
`PARTIAL_NON_EVIDENCE`, and it explicitly admits no observations and cannot
close `m1.r31`. The legacy `plan` and `validate` commands do not consume this
policy and therefore do not enforce its 10/30 counts. Admission records
`observation_counts_enforced=false` and requires a future
policy-SHA-256-bound D10 observation validator before execution conformance can
be claimed.

Safe Rust does not provide an atomic directory-create-and-open primitive. The
publisher therefore adopts only the exact empty `0700`, effective-owner/group
directory opened after `mkdirat` and makes no claim that this inode is the one
created by that call. A failed prepublication attempt removes only output file
names that still identify transaction-created inodes; it retains the staging
directory and any substituted name for inspection.

The differential suite first authenticates the exact seven Ferric capture and
independent reference bundle rosters and writes their canonical output-pair
manifest:

```text
cargo run --locked -p ferric-m1-benchmarks --bin ferric-m1-differential -- \
  write-pairs PLAN CAPTURE-ROOT REFERENCE-ROOT OUTPUT-PAIRS
```

`CAPTURE-ROOT`, `REFERENCE-ROOT`, and `OUTPUT-PAIRS` must be distinct direct
children of one safe parent. The roots must contain exactly the seven canonical
`KIND.capture.bundle` and `KIND.reference.bundle` directories, respectively;
every bundle must contain only `logits.bf16le`, `output.json`, `runner.json`,
and `tokens.u32le`. The command requires the reference copy of each canonical
runner transcript to be byte-identical to the Ferric capture transcript. It
checks every transcript field against the plan and capture protocol, validates
both output manifests and their payload identities, streams every payload
through the full finite-logit, lowest-ID argmax, and shape comparison, and emits
only relative paths without parent traversal. Every Ferric and reference output
manifest is carried as a `{path,bytes,sha256}` companion, as is the runner
transcript, so replacing a manifest after generation changes or invalidates the
pairs artifact rather than preserving its identity.

`OUTPUT-PAIRS` is created without replacement through a synchronized sibling
staging file. The created descriptor and metadata snapshot remain held through
validation, the staging name is rebound to that identity before publication,
and the parent directory is synchronized after the no-replace rename. A failed
pre-publication run removes only its own staging inode and leaves an existing or
substituted caller-owned path untouched. The plan is reread before publication
and must remain byte-identical.

The differential suite can then turn that exact seven-case output-pair manifest
into immutable raw comparison records plus the common benchmark-record envelope:

One differential plan binds `dispatch-graph` to the ordered generated plan
catalog and binds each of its seven case kinds to a separate
`dispatch-graph-KIND` identity. The common plan SHA-256 therefore authenticates
the exact generated graph selected for every output pair without requiring a
different plan document per case.

```text
cargo run --locked -p ferric-m1-benchmarks --bin ferric-m1-differential -- \
  produce PLAN PAIRS OUTPUT-BUNDLE
```

The producer publishes `OUTPUT-BUNDLE` without replacement only after every
file has been written and synchronized in a sibling staging directory. The
bundle contains `records.json` and exactly seven files under `raw/`. Failed
runs remove their owned staging files, so the same absent output path can be
retried without removing or replacing caller-owned data.

`PAIRS` uses `FERRIC-M1-DIFFERENTIAL-PAIRS-V2` and binds one canonical
`FERRIC-M1-DIFFERENTIAL-OUTPUT-V1` manifest for Ferric and one for the
independent reference in every planned case, plus an immutable canonical runner
transcript companion. Inputs are opened descriptor-relatively without following
symlinks, retained through comparison, and checked for metadata drift after
reads. Each output manifest binds its producer, protocol, environment, plan,
case input, workload, and runner transcript identities, and describes
exact-size little-endian BF16 logits
`[rows,151936]` and `u32` greedy-token payloads by path and SHA-256. Rows are
fixed by the seven declared case kinds.

The producer hashes while streaming both full logit payloads, rejects short,
trailing, substituted, or nonfinite data, computes monotonic BF16 ULP distance
with signed zeros equal, and requires each declared token to be the lowest-ID
argmax of its own row. It records comparison counts, maximum ULP distance, and
Ferric/reference token mismatches. It deliberately applies no numerical
tolerance and does not turn the resulting records into qualification evidence.

Numerical acceptance is a separate, fail-closed operation:

```text
cargo run --locked -p ferric-m1-benchmarks --bin ferric-m1-differential -- \
  check-acceptance PLAN PAIRS POLICY
```

`POLICY` is a canonical `FERRIC-M1-DIFFERENTIAL-ACCEPTANCE-POLICY-V1`
artifact whose SHA-256 must already be present as the plan's
`differential-acceptance-policy` identity. It fixes one maximum monotonic BF16
ULP error and maximum Ferric/reference greedy-token mismatch count for every
one of the seven case kinds. Ferric does not supply a default tolerance. The
checker rereads the full identity-bound payloads, requires exact shapes and
encodings, finite logits, and lowest-ID BF16 argmax tokens, then applies only
the plan-admitted per-case thresholds. A missing, substituted, incomplete, or
type-drifted policy fails before comparison. Every result case binds the exact
Ferric and reference manifest and payload SHA-256 identities plus its runner
transcript SHA-256, so metric-preserving output substitution changes the
canonical result identity.

The canonical result has `checked-differential-policy-conformance-only`
authority. It does not establish that the external threshold was independently
reviewed, create qualification evidence, or close `m1.r29`; those remain duties
of the independent M1 evidence and qualification validators.

The Ferric qualification capture binary can generate and revalidate the exact
seven-case input bundle without opening KFD:

```text
ferric-m1-qualification-capture generate-inputs \
  MODEL-SOURCE PREPACKED KERNEL-ARTIFACTS CLOSURE ACCEPTANCE-POLICY \
  REFERENCE-IMPLEMENTATION REFERENCE-PROTOCOL GPU-ID OUTPUT
ferric-m1-qualification-capture validate-inputs \
  MODEL-SOURCE PREPACKED KERNEL-ARTIFACTS CLOSURE ACCEPTANCE-POLICY \
  REFERENCE-IMPLEMENTATION REFERENCE-PROTOCOL GPU-ID INPUT-BUNDLE
```

The bundle contains exactly 20 flat, regular, single-link files: benchmark
input, plan, roster, closure, environment, acceptance policy, and adjacent
workload/token pairs for all seven differential cases. Reference files and the
running capture executable are measured through retained no-follow descriptors.
Every token is derived deterministically from its case kind, lane, and ordinal
and remains below the base vocabulary boundary. Generation publishes through a
synchronized sibling staging directory without replacement and prints a
canonical seven-command capture invocation map. Validation authenticates the
same model, prepack, artifact, closure, policy, executable, and reference inputs
and byte-recomputes the complete bundle before reporting its plan SHA-256.

The adversarial suite has a separate five-case producer:

```text
cargo run --locked -p ferric-m1-benchmarks --bin ferric-m1-adversarial -- \
  produce PLAN EXECUTION OUTPUT-BUNDLE
```

`EXECUTION` uses `FERRIC-M1-ADVERSARIAL-EXECUTION-V1` and binds exactly one
input and workload for each required case to the plan. It also binds the exact
canary layout and fault roster named by the plan. The exhaustion case executes
the public Ferric engine directly and observes transactional `OutOfPages`
rejection plus complete ready-state reclamation. Canary, cancellation,
rollback, and injected physical queue reports remain `reported-unvalidated`
intake because production completion authority is available only after real
queue completion and readback. Each such report has a nonzero
`unexpected-errors` measurement and zero `faults-observed`; it cannot represent
a passing safety observation.

External observation documents bind the exact execution, plan, case input,
workload, fault plan, and one selected fault occurrence. Their canonical
`FERRIC-M1-ADVERSARIAL-RUNNER-TRANSCRIPT-V1` companions repeat those bindings,
join the exact result and derived outcome, and bind the planned benchmark
executable, protocol, and environment identities. Optional hardware identities
are all-or-nothing correlation handles only. The producer copies each checked
transcript, writes a computed raw record, and publishes those files with
`records.json` as one no-replace bundle. These external reports explicitly
carry no hardware claim: the Ferric-owned MI300 harness and independent evidence
validators must still establish snapshot provenance, exact completion, injected
device failure coverage, and typed custody before `m1.r30` can close.

The Ferric engine also exposes one deliberately partial physical cancellation
capture using the already admitted qualification inputs:

```text
cargo run --locked -p ferric-engine --bin ferric-m1-qualification-capture -- \
  capture-r30-cancellation PLAN ROSTER CASE-ID WORKLOAD MODEL-SOURCE \
  PREPACKED-SNAPSHOT KERNEL-ARTIFACTS CLOSURE ENVIRONMENT GPU-UNIQUE-ID \
  OUTPUT-BUNDLE
```

This command accepts only an authenticated target-prefill workload. Immediately
after queue publication it requires every scheduler request to still be
`InFlight`, requests retirement, and requires a reclamation probe to return no
request before waiting for physical completion. It then observes physical queue
completion and readback before recording exact completion settlement,
authenticated nonzero target-page release, and queue release. It publishes
exactly `capture.json` and the canonical
`FERRIC-M1-R30-PARTIAL-PROTOCOL-V1` as `protocol.json`, without replacement.

The bundle authority and status are `ferric-physical-partial-capture-only` and
`partial-non-evidence`. It is not accepted by the adversarial benchmark evidence
intake, supplies no independent validation, covers none of the canary,
exhaustion, rollback, or injected device-fault cases, and cannot close
`m1.r30`.

The serving suite has an additive pre-observation producer for one bounded,
externally declared target-load diagnostic:

```text
cargo run --locked -p ferric-m1-benchmarks --bin ferric-m1-serving -- \
  partial-capture EXPERIMENT OUTPUT-BUNDLE
```

`EXPERIMENT` is canonical `FERRIC-M1-R33-PARTIAL-EXPERIMENT-V1`. It fixes one
case, one externally declared server start, one warmup window, one recorded
window, exact request order and arrival offsets, and the TTFT, ITL, TPOT,
end-to-end, and rate timing boundaries before observation. It binds exact
descriptor-relative `arrivals`, `artifacts`, `baselines`, `environment`,
`model`, `policy`, `tuning`, and `workload` companions by path, byte length, and
SHA-256. Baseline versions, cache and tuning identities, and TTFT/ITL p99 SLOs
must be supplied by external pre-observation inputs; Ferric supplies no default
version, tuning, threshold, or acceptance decision.

The separately collected request-event transcript binds the experiment
SHA-256 and repeats the exact start, window, request, and arrival order. Its
recorded request count must equal the externally declared offered concurrency,
and the raw half-open arrival-to-terminal intervals must realize that exact
peak overlap. This authenticates the declared target load without claiming
server saturation. Ferric recomputes successful input/output/total tokens per
second, all-request and successful-request rates, TTFT/ITL/TPOT/end-to-end
p50/p90/p99, failure counts, and token counts from the raw request events, then
requires the collector's summary to match exactly. Every rate is an integer
milli-unit per second: its declared population is multiplied by `1e12`, divided
by the exact recorded-window duration in nanoseconds, and floored.

Every input remains held through computation and its descriptor-relative name
is rebound immediately before publication. Every output file remains held from
exclusive creation through final post-fsync content, metadata, and name
verification. Symlinks, hard links, aliases, noncanonical JSON, changed
companions, reordered events, invalid timestamps, summary drift, directory or
parent metadata drift, and replacement publication fail closed.

Safe Rust provides no atomic directory-create-and-open operation. Across the
`mkdirat`/open boundary the producer therefore adopts only the exact empty
`0700`, effective-owner/group directory it opens, without claiming that this
inode was created by its `mkdirat` call. Any prepublication failure removes only
file names still bound to retained transaction-created file descriptors. The
adopted staging directory and its name are retained for inspection, including
when the name was substituted before it could be opened.

The producer publishes exactly `capture.json` and the canonical
`FERRIC-M1-R33-PARTIAL-PROTOCOL-V1` as `protocol.json`, without replacement.
Both carry `partial-non-evidence` status: this first slice does not establish a
fresh server launch or server saturation, is not continuous serving, has no
measured vLLM/SGLang comparison or independent validator, and cannot close
`m1.r33`. Existing `describe`, `plan`, and `validate` commands are unchanged.

The policy test uses the distinct `synthetic-policy-fixture-only` authority and
the nonpublishing `check-policy-fixture` command solely to exercise the shared
parsers and mutation rejection. Normal `produce` rejects those fixtures. The
test also publishes one temporary `reported-unvalidated` intake bundle to
regression-test publication and demotion; every external case remains
nonpassing, and the temporary directory is discarded with the test:

```text
python3 -I benches/m1/test-policy.py .
```
