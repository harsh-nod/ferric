# Ferric generated Qwen3 gfx942 runner declarations

This crate is the checked-in output of Ferric's deterministic M1 runner
declaration generator. It contains the exact target-then-draft B3 plan order,
operation offsets and counts, and a request-independent schema for logical
scalar inputs. The workspace compiles this file, and `ferric-build` tests that
regeneration produces byte-for-byte identical source. The roadmap-facing
`generated/qwen3_m1.rs` file is a second byte-exact publication of that same
renderer output, not an independently maintained template. Regenerate or
validate both copies from the repository root with:

```console
cargo run -p ferric-build --bin ferric-m1-generate-runner --locked -- .
cargo run -p ferric-build --bin ferric-m1-generate-runner --locked -- --check .
```

The declarations are inert. They contain no addresses, allocations, device
objects, queues, packets, artifacts, loaders, launch operations, completion
handling, graph-refinement proof, hardware observation, performance result,
or qualification evidence. They do not close an M1 roadmap item.

`validate_generated_runner_input` is the crate's verified fail-closed handoff.
It selects one exact generated plan position, checks that an operation lies in
that plan's declared range, checks the complete logical patch schema, compares
16 identity roles with a separately supplied exact expectation, and retains
the accepted input without exposing an execution operation. The expectation
is data supplied by a future authenticated integration; this crate does not
authenticate its identity bytes or grant physical execution authority.

The declaration's generated-source identity binds a canonical two-file source
closure: the byte-exact rendered `src/lib.rs`, plus the path, exact byte length,
and SHA-256 digest of `src/validation.rs`. The source-closure validator checks
both files, and declaration/publication retain that closure identity. Digest
equality is not a proof of SHA-256 collision resistance.
