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
`observation_counts_enforced=false`; the separate policy-SHA-256-bound D10
observation validator is required to check raw observation conformance.

Safe Rust does not provide an atomic directory-create-and-open primitive. The
publisher therefore adopts only the exact empty `0700`, effective-owner/group
directory opened after `mkdirat` and makes no claim that this inode is the one
created by that call. A failed prepublication attempt removes only output file
names that still identify transaction-created inodes; it retains the staging
directory and any substituted name for inspection.

The additive collector runs one exact admitted policy through externally
supplied measurement commands and publishes one canonical raw-observation
bundle without replacement:

```text
cargo run --locked -p ferric-m1-benchmarks --bin ferric-m1-d10 -- \
  collect-policy-observations POLICY-ROOT ADMISSION-BUNDLE \
  COLLECTION-MANIFEST OBSERVATION-BUNDLE
```

`COLLECTION-MANIFEST` is canonical
`FERRIC-M1-D10-COLLECTION-MANIFEST-V1`. It binds the exact collector ELF,
policy, cleared environment snapshot, timeout, K1-K7 case order, admitted
holdout member, per-case expected resource counters, and exact resource and
measurement ELF, argument, configuration, and protocol identities. All paths
are held without symlink traversal and every ELF is executed through its held
descriptor. The environment snapshot digest must equal the policy telemetry
binding, implementation and configuration bytes must match their policy
identities, and reference and resource command protocols must match their
respective policy companions.

Before launching a command, the collector derives the complete deterministic
case, implementation, phase, and sample-ID order and requires its canonical
projection to match the policy execution-order hashes. It runs cases in the
source-controlled K1-K7 order, performs one resource inspection, then runs
`ferric-reference`, `ferric`, and any applicable `vendor` in that order. Each
applicable implementation receives exactly ten warmup requests followed by
thirty recorded requests. Every request is canonical JSON on stdin under an
empty inherited environment. A timeout, signal, nonzero exit, nonempty stderr,
oversized, non-ASCII, malformed, or noncanonical stdout, reported telemetry
error, or resource mismatch fails closed and publishes nothing.

Sample results are canonical `FERRIC-M1-D10-SAMPLE-RESULT-V1` documents. A
warmup has null timing counters; a recorded result has positive raw
`elapsed_ns` and `iterations`. Both carry policy-protocol-bound start/end clock
and temperature readings, the exact environment identity, and an empty
structured error-event roster. Resource results are canonical
`FERRIC-M1-D10-RESOURCE-RESULT-V1` documents with the case, artifact,
inspection protocol, and a nonempty unsigned resource map that must equal the
manifest value whose canonical digest is frozen by the policy. These are
parseable raw schemas, not supplied summaries or generated thresholds.

The additive observation validator then checks that exact admitted policy
against the collector's exact canonical raw-observation bundle:

```text
cargo run --locked -p ferric-m1-benchmarks --bin ferric-m1-d10 -- \
  validate-policy-observations POLICY-ROOT ADMISSION-BUNDLE \
  OBSERVATION-BUNDLE OUTPUT-BUNDLE
```

`ADMISSION-BUNDLE` must contain exactly the original `admission.json` and
policy `protocol.json`. `OBSERVATION-BUNDLE` must contain exactly canonical
`observations.json` in `FERRIC-M1-D10-POLICY-OBSERVATIONS-V2` and the
source-controlled `d10_observation_protocol.json` as `protocol.json`. The
validator holds and repeatedly rebinds those four files plus the original ten
policy-root files. It recomputes the admission and requires every policy,
protocol, admission, companion, case, implementation, profile, work-unit,
resource-policy, tuning, admitted-holdout-member, and regression measurement
roster identity to match.

Every applicable Ferric-reference, Ferric, and vendor stream has exactly 10
ordered untimed warmups and exactly 30 ordered raw timing samples. Each row also
binds the exact command, runner output, successful subprocess outcome, and
parseable telemetry. All applicable streams in one case must use one exact
shared admitted `{id,sha256}` holdout member. An
inapplicable vendor has null identities and member plus an exact empty sample
roster and is excluded from vendor gates and the weighted aggregate. The
execution-order companion binds the canonical per-case projection of each
implementation name, exact holdout member, and ordered sample-ID roster for
both warmup and recorded phases. Submitted summaries are not part of the schema
and therefore cannot act as authority.

For each recorded row the validator recomputes
`floor(work-unit-count * iterations * 1_000_000_000 / elapsed-ns)` in integer
policy work units per second, then computes the exact even-sample rational
median. It applies only the externally frozen regression and applicable-vendor
PPM thresholds. The weighted vendor result is published exactly as the
applicable-case geometric ratio raised to its total policy weight, with the gate
checked by integer cross-products. `num-bigint` is required because valid u64
rates and weighted products exceed `u128`; observation validation limits each
external case weight to 8192 before aggregate arithmetic so the exact seven-case
product and PPM comparison remain under the validator's 8,388,608-bit
representation bound. Admission retains its V1 structural weight domain. This
is a structural computability bound, not a supplied or default weight.

The validator enforces the supplied telemetry and resource schemas, their exact
policy identity bindings, empty error events, physical numeric bounds, and exact
expected/observed resource equality. These checks do not authenticate who
produced the bundle. `OUTPUT-BUNDLE` is published without replacement with
exactly `observations.json`, its `protocol.json`, and recomputed
`validation.json` under descriptor-held file, directory, name, and parent
custody. Its status remains `PARTIAL_NON_EVIDENCE`: it does not validate
external policy values or physical
observation truth, provide independent reproduction or qualification evidence,
or close `m1.r31`.

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

The Ferric engine exposes one opt-in physical adjacent-guard capture for the
target-prefill S1 K7 completion range:

```text
cargo run --locked -p ferric-engine --bin ferric-m1-qualification-capture -- \
  capture-r30-canary MODEL-SOURCE PREPACKED-SNAPSHOT KERNEL-ARTIFACTS \
  CLOSURE ENVIRONMENT GPU-UNIQUE-ID OUTPUT-BUNDLE
```

The command freezes exactly one target-only `PrefillS1T128` lane at context
zero with active length 128 and 128 copies of token ID one encoded as exactly
512 little-endian input bytes. Its canonical workload binds the fixed
`ferric-m1-completion-progress-wait-v2` paced-scan policy rather than a caller poll
budget. Consecutive pending scans are separated by a fixed minimum 10 ms pause;
the pause supplies execution opportunity but does not add scans or change the
checked scan-count termination rule. It allocates its 120-byte K7 completion output
inside initialized host-visible backing laid
out as a 64-byte `0xA5` prefix guard, the zeroed K7 interior, and a 64-byte
`0x5A` suffix guard. The K7 dispatch receives only the checked interior range.
After exact generation completion and queue recycle, Ferric copies the same
enclosing allocation once, validates the retained coordinates and both guards,
and passes only the interior to the existing K7 semantic decoder. A snapshot
copy failure is terminal; a copied layout, guard, or K7 rejection retains the
owned enclosing snapshot without copying it again. Publication occurs only
after normal semantic settlement, exact eight-target-page release accounting,
and queue destruction.

The no-replace bundle contains exactly `capture.json` and canonical
`FERRIC-M1-R30-CANARY-PARTIAL-PROTOCOL-V4` `protocol.json`. Its status is
`partial-non-evidence`. It checks only the adjacent 64-byte guards around one
K7 output for one case. It proves neither K1-K6 bounds nor general
out-of-bounds safety and covers no cancellation, exhaustion, rollback, injected
fault, independent validation, evidence, hardware or numerical correctness,
performance, qualification, `m1.r30`, or M1 closure claim.

The Ferric engine also exposes one deliberately partial fixed-workload physical
cancellation capture:

```text
cargo run --locked -p ferric-engine --bin ferric-m1-qualification-capture -- \
  capture-r30-cancellation MODEL-SOURCE PREPACKED-SNAPSHOT KERNEL-ARTIFACTS \
  CLOSURE ENVIRONMENT GPU-UNIQUE-ID OUTPUT-BUNDLE
```

The command freezes one `Target8B` `PrefillS1T128` lane with context zero,
active length 128, and 128 copies of input token one encoded as exactly 512
little-endian bytes. Its workload binds the fixed
`ferric-m1-completion-progress-wait-v2` policy: at most 8,192 consecutive
completion-signal scans without completed-count progress, with the checked
whole-wait bound derived as `(packet-count+1)*8192`. A fixed minimum 10 ms pause
separates consecutive pending scans without granting any additional scan. It
authenticates the model, artifacts, closure, runner, and exclusive gfx942 device
directly, and measures and binds the environment and executable; no
differential plan, roster, case ID, workload file, or external m1.r29 acceptance
policy is accepted. Immediately after queue publication it requires the
scheduler request to still be
`InFlight`, requests retirement, and requires a reclamation probe to return no
request before waiting for physical completion. This is scheduler retirement,
not GPU work preemption. It then observes physical queue completion and
readback before recording exact one-member completion settlement, eight target
page releases, and queue release. The capture binds the actual checked plan ID
and compact digest. It publishes exactly `capture.json` and canonical
`FERRIC-M1-R30-PARTIAL-PROTOCOL-V5` `protocol.json`, without replacement.

The bundle authority and status are `ferric-physical-partial-capture-only` and
`partial-non-evidence`. It is not accepted by the adversarial benchmark evidence
intake, has `hardware_claim` `none`, supplies no independent validation, covers
none of the canary, exhaustion, rollback, or injected device-fault cases, and
cannot close `m1.r30`.

A separate Ferric-only command captures the strict-prefix rollback case through
the production S1/K4 diagnostic path:

```text
cargo run --locked -p ferric-engine --bin ferric-m1-qualification-capture -- \
  capture-r30-rollback MODEL-SOURCE PREPACKED-SNAPSHOT KERNEL-ARTIFACTS \
  CLOSURE ENVIRONMENT GPU-UNIQUE-ID OUTPUT-BUNDLE
```

The command authenticates the same model, artifacts, runner, closure, and
exclusive gfx942 device as the qualification path. It reads exactly four draft
choices (16 bytes) and five target choices (20 bytes), requires the accepted
draft count to be less than four and equal to their maximal matching prefix,
and checks the emitted draft prefix plus target correction. It snapshots typed
target and draft KV projections after writes are pending, then again immediately
after the single Engine-completion attempt and before page-release accounting.
Both roles must settle to exactly `accepted_draft_tokens + 1` committed and
resident tokens with no pending write. The capture reports the rejected target
and draft suffix token counts.

The continuing member retains one active target page and one active draft page
in the post-completion projection and in closed teardown custody. No retired
pages exist, every release count and `total_released` is zero, and neither page
is described as physically released. The command performs completion, release
accounting, and queue destruction once each, then publishes exactly
`capture.json` and canonical
`FERRIC-M1-R30-ROLLBACK-PARTIAL-PROTOCOL-V1` `protocol.json` without
replacement.

This bundle is `partial-non-evidence` and covers only rollback among the five
required `m1.r30` cases. It grants no physical page or subpage return/reuse
authority and makes no evidence, external or independent validation, hardware
correctness, performance, qualification, `m1.r30`, or M1 closure claim.

The engine also exposes a Ferric-owned physical KV-ledger saturation capture
for the R30 exhaustion case:

```text
cargo run --locked -p ferric-engine --bin ferric-m1-qualification-capture -- \
  capture-r30-exhaustion MODEL-SOURCE PREPACKED-SNAPSHOT KERNEL-ARTIFACTS \
  CLOSURE ENVIRONMENT GPU-UNIQUE-ID OUTPUT-BUNDLE
```

This command authenticates and initializes the exact gfx942 model-memory
allocation owner, admits one production Engine request, leases all 512
request-local target KV pages, and requires a second lease of occupied page zero
to fail exactly with `PageAlreadyLeased` while the complete roster remains
valid. It separately requires the static P512/page-513 index check to fail with
`PageOutOfRange`. It returns all unpublished pages atomically, re-leases page
zero at the next generation, returns it again, and requires exact Engine
retirement and reclamation. The two-file output uses canonical
`FERRIC-M1-R30-EXHAUSTION-PARTIAL-PROTOCOL-V1` and no-replace publication.

The exhaustion bundle remains `partial-non-evidence`: it establishes only
request-local model-memory ledger saturation, not device-memory exhaustion or
pressure. It does not dispatch a kernel or create or pressure a queue. The full
R30 roster now has only one Ferric-owned adjacent-guard readback for one
target-prefill S1 K7 output; it still lacks broader canary coverage and an
admitted physical queue/device fault-injection authority. Those missing runtime
boundaries cannot be replaced by the existing external self-reported intake.

The four Ferric-owned partial bundles can be joined with one canonical external
fault-observation document without conflating their authorities:

```text
ferric-m1-qualification-capture compose-r30-runner \
  CANARY-BUNDLE CANCELLATION-BUNDLE EXHAUSTION-BUNDLE ROLLBACK-BUNDLE \
  FAULT-OBSERVATION OUTPUT-BUNDLE
```

`FAULT-OBSERVATION` must be canonical
`FERRIC-M1-R30-FAULT-OBSERVATION-V1`, use authority
`externally-reported-r30-fault-observation-only`, declare
`hardware_claim: none`, bind its source executable, protocol, environment, and
transcript identities, and contain a nonempty, uniquely named bounded fault
roster. The composer re-admits every physical capture against its exact
checked-in protocol, requires the four captures to bind the same device,
kernel-artifact manifest, program catalog, runner declaration, and GPU unique
ID. It also descriptor-measures the running composer executable and publishes
exactly `runner.json` and canonical
`FERRIC-M1-R30-COMPOSED-RUNNER-PROTOCOL-V5` `protocol.json` without
replacement.

The composed runner remains `partial-non-evidence`. It records four physical
partial authorities and one `reported-unvalidated` external authority, carries
`hardware_claim: none`, and fixes `evidence_case_count` to zero. It cannot turn
reported fault fields into physical observations. A production physical
queue/device fault-injection API, broader canary coverage, and independent
evidence validation remain required before `m1.r30` can close.

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

An additive post-observation checker constructs one self-contained comparison
record from a separately frozen policy and externally collected window
counters:

```text
cargo run --locked -p ferric-m1-benchmarks --bin ferric-m1-serving -- \
  validate-comparison-observations POLICY OBSERVATIONS OUTPUT-RECORD
```

`POLICY` is canonical
`FERRIC-M1-R33-SERVING-COMPARISON-POLICY-V1` and has `pre-observation`
status. It freezes the exact benchmark executable and plan, generated plan,
schedule, workload, arrival trace, output limits, environment, Ferric and
fe2o3 source closures, model, tokenizer, and weights. Its ordered Ferric,
vLLM, and SGLang roster separately freezes each implementation, source,
protocol, configuration, tuning result, version, and tuning-budget identity.
All three tuning-budget identities must be byte-equal. That equality binds one
external declaration; it does not establish that the underlying tuning work or
opportunity was equal. The fe2o3 source closure is likewise an opaque external
input identity only and grants no compiler-correctness authority or ownership
of serving code. The external policy also supplies one positive common p99
SLO; Ferric supplies no default version, tuning choice, budget, or SLO.

`OBSERVATIONS` is canonical
`FERRIC-M1-R33-SERVING-COMPARISON-OBSERVATIONS-V1`. It repeats the exact policy
SHA-256, plan binding, implementation roster, and engine order. Its row roster
is fixed and complete: three ascending server starts, each with ten ordered
warmup windows followed by ten ordered recorded windows. Every row carries a
cyclic Ferric/vLLM/SGLang execution order, passed status, an empty fault roster,
and raw positive duration, successful-request, token, and p99 latency counters
for all three engines. Any missing, duplicated, reordered, failed, faulted,
identity-substituted, or extra summary field fails closed.

The checker derives each integer tokens-per-second sample as
`floor(total_tokens * 1_000_000_000 / duration_ns)`, uses exact rational
medians, selects the baseline with the larger median throughput (vLLM on an
exact tie), checks every engine's median p99 against the externally supplied
common SLO, and computes the floored Ferric-to-fastest-baseline PPM ratio. It
then performs 10,000 paired percentile-bootstrap resamples over the 30 aligned
recorded-window throughput pairs. SplitMix64 is seeded by a domain-separated
SHA-256 binding of the exact policy and observation identities; the 250th and
9,750th nearest-rank estimates form the deterministic 95% interval. The
record fails closed unless its lower bound is at least 950,000 PPM.
`OUTPUT-RECORD` is created without replacement and carries all exact raw rows,
policy bindings, input SHA-256 identities, recomputed summaries, bootstrap
seed identity, and confidence interval; submitted summaries are not in the
input schema and cannot act as authority.
Publication uses an exclusive one-link sibling staging file retained by file
descriptor, rereads and hashes its exact bytes, renames with
`RENAME_NOREPLACE`, synchronizes the parent directory, then rebinds and rereads
the final pathname twice. Concurrent staging or published-name substitution
therefore fails instead of returning success for a different record.

The output remains `PARTIAL_NON_EVIDENCE`. It authenticates declared bytes and
recomputes arithmetic but does not validate external plan or policy choices,
collector aggregation, server freshness, observation truth, hardware or
numerical correctness, independent reproduction, or qualification. Real
externally collected inputs and the independent performance/evidence
validators are still required, and `m1.r33` remains `Open`.

The stronger `m1.r32` post-observation boundary accepts a policy frozen before
measurement, exact externally collected paired counters, and a new output
pathname:

```text
cargo run --locked -p ferric-m1-benchmarks --bin ferric-m1-speculation -- \
  validate-comparison-observations POLICY OBSERVATIONS OUTPUT-RECORD
```

The policy binds the Ferric and fe2o3 source closures; common model, tokenizer,
weight, schedule, environment, and artifact identities; distinct speculative
and target-only config, implementation, protocol, and artifact identities; and
the same Ferric source and version for both modes. It freezes exactly one
eligible speculation holdout and one low-acceptance cell with an admitted
deterministic fallback plan. Each cell carries a canonical workload identity,
an exact case kind whose `S` and `K` geometry must match batch and draft width,
one equal p99 SLO, and forty unique pairing identities.

Observations must contain both modes for ten ordered warmup pairs followed by
thirty ordered recorded pairs in each cell. Each pair must retain the exact
predeclared identity and alternating mode order, pass without faults or failed
requests, and report equal successful-request and total-token work. Accepted
tokens cannot exceed speculative output. Missing, extra, duplicated,
reordered, unpaired, unequal-work, identity-substituted, or summary-bearing
rows fail closed.
The eligible cell must also retain at least one accepted speculative token;
the record reports its exact mean accepted tokens per speculative target
invocation.

The checker derives each integer throughput as
`floor(total_tokens * 1_000_000_000 / duration_ns)` and uses exact reduced
rational medians. The eligible cell must reach 1,100,000 ppm of target-only
throughput while its p99 median remains at or below 1,050,000 ppm. The
low-acceptance deterministic-plan cell must retain at least 950,000 ppm of
target-only throughput. These thresholds are checker-owned constants and the
input cannot submit summaries. Publication is descriptor-held, no-replace,
parent-synchronized, and revalidated by final pathname and bytes.

The resulting `PARTIAL_NON_EVIDENCE` record authenticates declarations and
recomputes arithmetic only. External eligibility and holdout selection,
observations, hardware behavior, numerical correctness, independent
reproduction, and qualification remain unvalidated, so `m1.r32` and M1 remain
`Open` until real hardware records and independent evidence validators pass.

The Ferric-only paired collector produces the observation input for that
checker from separately executed commands:

```text
cargo run --locked -p ferric-m1-benchmarks --bin ferric-m1-speculation -- \
  collect-comparison-observations POLICY COMMAND-PLAN OUTPUT-OBSERVATIONS
```

`COMMAND-PLAN` is canonical
`FERRIC-M1-R32-PAIRED-COMMAND-PLAN-V1` with `pre-execution` status. It repeats
the exact policy SHA-256 and plan, the two policy cells and their workload,
holdout, and deterministic-plan bindings, and both policy implementation
records. It also freezes one canonical absolute benchmark executable and its
policy-bound byte SHA-256, one canonical absolute working directory, a cleared
environment whose canonical identity equals the policy environment SHA-256, a
bounded timeout, and an exact argument vector for each cell and mode. The
collector recomputes each command SHA-256 over the executable, environment,
cell, workload, mode, implementation, and arguments before any execution.

For each cell, the collector executes ten warmup pairs followed by thirty
recorded pairs. Speculative and target-only are distinct sequential child
processes in the alternating order required by the existing checker. The
collector clears the inherited environment and adds reserved
`FERRIC_M1_R32_*` variables containing the exact policy, command, cell, pair,
workload, holdout, fallback-plan, implementation, ordinal, and order bindings.
Each child must emit one bounded canonical
`FERRIC-M1-R32-PAIRED-COMMAND-RESULT-V1` document repeating all bindings and
its raw counters. A timeout, nonzero exit, stderr output, malformed or extra
field, identity drift, failed request, impossible accepted-token count, or
unequal successful-request or token work aborts collection without publishing
an observation file.

The collector copies returned counters; it never constructs, rescales, or
substitutes a duration or latency. After all 160 child executions it builds the
exact `FERRIC-M1-R32-SPECULATION-COMPARISON-OBSERVATIONS-V1` schema, runs the
existing structural observation validator over it, and publishes canonical
JSON through the existing descriptor-held no-replace path. Performance gates
remain checker-owned, so a truthful real run may publish observations that the
subsequent comparison-record command rejects. The command plan and child
reports remain declarations and observations rather than independent evidence;
the collector does not itself close `m1.r32` or M1.

The first `m1.r32` diagnostic slice is likewise Ferric-only and does not alter
the target-only qualification command. Exact target
`SpeculativeS1K4C8192` completion output can opt into two additional coherent
readbacks: four draft choices and five target verification choices. After the
same queue generation completes, the engine binds those copies to the compact
K7 record's request, epoch, plan, and dispatch generation, applies the existing
maximal-prefix checker, compares the first emitted token with the first target
choice for the same pre-round context, settles exact Engine completion, and
accounts KV release. The canonical two-file capture protocol remains
`partial-non-evidence`; the corresponding target token is not a separately
executed target-only queue, and no holdout, performance, hardware-correctness,
qualification, or `m1.r32` closure claim is made.

An exclusive gfx942 operator can opt into that single-round path with:

```text
cargo run --locked -p ferric-engine --bin ferric-m1-qualification-capture -- \
  capture-r32-speculative-k4 MODEL-SOURCE PREPACKED-SNAPSHOT \
  KERNEL-ARTIFACTS CLOSURE ENVIRONMENT GPU-UNIQUE-ID OUTPUT-BUNDLE
```

The command authenticates the existing model snapshot, closure, persisted
kernel artifacts, generated runner declaration, and selected device before
running one physical S1/K4 round. The compact digest is retained through the
checked completion owner. Program catalog, kernel catalog, runner declaration,
and artifact-manifest identities are derived from and cross-checked between the
settled queue and physical runner; the publisher accepts only the resulting
opaque capture. It destroys the settled queue before publishing exactly
`capture.json` and `protocol.json` without replacement. The ordinary
target-only qualification invocation and its diagnostic-off allocation branch
are unchanged.

The ignored hardware smoke is
`configured_mi300x_s1_k4_diagnostic_readback_settles_one_real_round`. It uses
the same artifact, prepacked-weight, and GPU environment variables as the
existing target-only smoke and deliberately publishes no evidence artifact.

The policy test uses the distinct `synthetic-policy-fixture-only` authority and
the nonpublishing `check-policy-fixture` command solely to exercise the shared
parsers and mutation rejection. Normal `produce` rejects those fixtures. The
test also publishes one temporary `reported-unvalidated` intake bundle to
regression-test publication and demotion; every external case remains
nonpassing, and the temporary directory is discarded with the test:

```text
python3 -I benches/m1/test-policy.py .
```
