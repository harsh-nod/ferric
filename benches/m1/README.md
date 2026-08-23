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

The differential suite can also turn an exact seven-case output-pair manifest
into immutable raw comparison records plus the common benchmark-record
envelope:

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

`PAIRS` uses `FERRIC-M1-DIFFERENTIAL-PAIRS-V1` and names one canonical
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

The policy test uses the distinct `synthetic-policy-fixture-only` authority and
the nonpublishing `check-policy-fixture` command solely to exercise the shared
parsers and mutation rejection. Normal `produce` rejects those fixtures. The
test also publishes one temporary `reported-unvalidated` intake bundle to
regression-test publication and demotion; every external case remains
nonpassing, and the temporary directory is discarded with the test:

```text
python3 -I benches/m1/test-policy.py .
```
