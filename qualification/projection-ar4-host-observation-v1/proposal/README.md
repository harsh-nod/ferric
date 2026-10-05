# Projection AR4 Host Observation Proposal

Source-only successor to the actual CPU1037 Ferric sources. No imports,
formatting, builds, tests, GPU execution, live-source edits, or qualified
performance outcome were produced by the author. Exact before/after identities
and every replacement/addition are in source-manifest.json and changes.patch.
Root owns formatting, CPU qualification, integration, audits and native execution.

## Entry Points

- Parent bin: `ferric-qwen3-finite-projection-residual-decode-host-engineering`.
- Parent argv: `--request /absolute/request.json --allow-unauthenticated-machine-code --observe-projection-host`.
- Request: unchanged `FerricFiniteProjectionResidualDecodeRequestV1`; this
  entry requires `decode.mode = "autoregressive"` through the existing Rust enum.
- Worker selector: `--engineering-native-projection-residual-decode-host-v1`,
  followed by the existing opt-in/devices/timeout/mode arguments and
  `--host-sidecar /absolute/new/path.json`. Only `--mode autoregressive` passes.
- Sidecar: sibling of the evidence directory,
  `<evidence-name>-projection-host-observation.json`.
- Sidecar schema: `FerricProjectionResidualDecodeHostObservationV1`.
- Parent stdout schema: `FerricFiniteProjectionResidualDecodeHostDiagnosticV1`;
  includes the actual projection observation, sidecar FilePin, parent FilePin
  and request projection hash. The inner evidence complete.json retains the
  actual projection-native schema, not a fabricated legacy host report.

The full projection bootstrap, selected residual image, actual worker digest,
AR4 profile, source/upload identities, four completions and final transcript
are bound. Forward/Control/payload/Close serialization and the AR4 input/output
recurrence are unchanged. Original plain TF4/AR4, layer capture, legacy host,
policy and device routes retain their selector/argv and wire behavior.

## Measurement Contract

The new route enables the existing KFD observational API on a fresh ordinary
group before setup. It never enables operational currentness, legacy profiling,
admission caching, shared-currentness policy or raw device timestamps. No KFD,
kernel, image ABI, queue, lifetime or safety policy is edited.

Seven existing-shaped snapshots and six intervals retain setup, each forward,
and before-Close cumulative/delta host counters. Per-rank counters include full
currentness, admission, prepare/publication/wait, poll count and guarded
read/write counts/bytes/elapsed time. Shared fence counters remain distinct.
All counter scopes are inclusive and may overlap; they are not additive GPU time.

`forward_host_ns[4]` brackets Owner.run, including metadata/state I/O, dispatch
validation/wait and hidden/logit readback. `serialization_host_ns[4]` begins
after the post-run observer sample and ends at the completion callback after
observation assembly, hashing, Control/payload encoding and response pipe writes;
pipe blocking is included. Each serialization duration is bounded by the
following snapshot interval, which can also include parent/request gaps.
`close_host_ns` brackets consuming Owner.close; no post-consumption KFD counter
snapshot is invented. It excludes final report serialization/file sync and EOF.

These measurements distinguish cold setup, enclosing forward and host control
costs. They do not grant a performance comparison, steady-state throughput,
calibrated device duration, numerical correctness or full2048/256 acceptance.
All such claim flags remain false. Failure prevents successful sidecar
publication; protocol errors retain the original terminal owner poisoning.
Parent reads the sidecar only after healthy Close, EOF and owned child reap.
The report is new-file-only and at most64KiB; the existing aggregate8MiB
diagnostic bound and stdout64KiB bound remain.

## Scope and Tests

Seventeen source bodies: eleven replacements and six additions. Existing-file
diffs are limited to registrations, crate-private helper visibility, one optional
observer in the projection backend, an isolated parent diagnostic variant and
four added parent tests. No original tests are removed or renamed.

Eighteen new test declarations imply26 compiled executions: eight shared report
tests compile in both libraries, five worker CLI/recorder tests, four parent
binding/selector tests and one new parent-bin test. These are authored counts,
not execution results. Full-cohort arithmetic is1037+26=1063. The scoped qualifier
can retain historical runtime208 without rerunning it and run worker518/4 plus
parent337, totaling855/4; its actual named inventories and historical ignores
must be checked, not assumed from this arithmetic.
The existing full worker suite and parent-client selector cover their additions;
add the new shared-report selector and the new parent binary list/test/build.

Focused selectors for root's existing bounded CPU qualifier:

```text
cargo test --offline --locked --manifest-path adapters/tp-peer-finite-engineering-worker-v1/Cargo.toml --lib projection_host_
cargo test --offline --locked --manifest-path adapters/m1-engineering-execution-v1/Cargo.toml --features tp-batch-engineering --lib projection_host_
cargo test --offline --locked --manifest-path adapters/m1-engineering-execution-v1/Cargo.toml --features tp-batch-engineering --bin ferric-qwen3-finite-projection-residual-decode-host-engineering
```

These selectors do not replace the previous regression cohort, Cargo artifact
checks, exact source/dependency snapshots, stable toolchain, process ownership,
disk/target bounds or actual pre/post GPU audits. All source formatting must
produce a separately retained formatted-source receipt before compilation.
No executable or future receipt hashes are predicted here.
