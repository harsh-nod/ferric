# Task Graph Interleaving Model

This standalone, dependency-free host tool models the scheduler protocol in
[the gfx950 device fixture](../../device/gfx950-task-graph-v1/README.md).
It is intentionally outside Ferric's production Verus workspace. It confers
no artifact, launch, allocation, or proof authority.

Each step advances one worker at a queue snapshot, claim, payload execution,
completion RMW, or successor publication point. The model retains the old
completion value, separate claim/owner state, exact payloads, and per-worker
probe budgets. The numerical reference is an independently expanded scalar
expression, not an execution of the scheduler.

## Checks

- Exactly-once execution of all seven tasks and exact integer results.
- Payload publication before completion and dependencies before consumption.
- A failed ready-bit claim does not accidentally retire a worker.
- At most 7 nonempty probes plus 8 empty probes, within 16 uniform GPU rounds.
- One resident workgroup completes the graph; no peer-residency assumption.
- Stale/zero epochs leave ready/payload/ownership state unchanged and flag 1.
- Malformed ready masks, premature ready tasks, and excessive inputs reject.
- A negative test deliberately replaces the old completion value with a later
  load and demonstrates duplicate join publication after a competing claim.

The exhaustive protocol quotient explores 335,134 states and 16,384 terminal
states. This quotient removes nonretiring empty probes: they mutate no shared
state and can be represented by delaying that worker's next scheduling step.
It uses a one-empty-probe retirement limit and is **not** a count of every
concrete sixteen-round GPU schedule. The concrete eight-empty-probe model is
also checked under 10,000 deterministic generated schedules and adversarial
single-resident/failed-claim cases. No C++/Rust weak-memory formal proof is claimed;
device atomic ordering and the compiler's memory proofs remain separate gates.

## Run

```sh
cargo test --locked --all-targets -- --nocapture
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --all --check
```

These commands run from this directory without GPU access. Device package host
tests instead require the supported binding-only `cargo-fe2o3 fe2o3 test
--locked --all-targets` route; ordinary `cargo test` is not a substitute for it.
