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

The adversarial suite has a separate five-case producer:

```text
cargo run --locked -p ferric-m1-benchmarks --bin ferric-m1-adversarial -- \
  produce PLAN EXECUTION OUTPUT-BUNDLE
```

`EXECUTION` uses `FERRIC-M1-ADVERSARIAL-EXECUTION-V1` and binds exactly one
input and workload for each required case to the plan. It also binds the exact
canary layout and fault roster named by the plan. The exhaustion case executes
the public Ferric engine directly and observes transactional `OutOfPages`
rejection plus complete ready-state reclamation. Canary records compare exact
external before/after byte snapshots. Cancellation, rollback, and injected
physical queue failures require external observation documents and immutable
runner transcripts because production completion authority is available only
after real queue completion and readback.

The adversarial producer copies each authenticated runner transcript, writes a
computed raw record for each case, and publishes those files with `records.json`
as one no-replace bundle. Its external physical observations explicitly carry
no hardware claim: the MI300 harness and independent evidence validators must
still establish snapshot provenance, exact completion, and injected device
failure coverage before `m1.r30` can close.

The policy test uses temporary synthetic values solely to exercise the CLI
protocol and mutation rejection. It never writes benchmark evidence:

```text
python3 -I benches/m1/test-policy.py .
```
