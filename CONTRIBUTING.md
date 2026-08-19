# Contributing

Ferric accepts only changes that preserve the single final execution path.

- Do not add a legacy, vendor-library, raw-launch, JIT, or unproved fallback.
- Put reusable GPU compiler, kernel, proof, and HSA capabilities in fe2o3.
- State the exact property and last covered boundary for every evidence claim.
- Follow `docs/PROOF_DEVELOPMENT.md` for every proof-required change.
- Add negative mutations for new authority, ownership, proof, and parser gates.
- Keep runtime state deterministic, fixed-capacity where practical, and free of
  ambient model or target discovery.
- Bind correctness and performance evidence to exact source and environment
  identities.

Before submitting a change:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

A correctness-critical change cannot merge with tests alone. Its pull request
must identify the affected assurance properties and either provide the closed,
identity-bound Verus evidence required by `docs/PROOF_DEVELOPMENT.md` or mark
the properties `Contracted`/`Unsupported`. Unsupported evidence cannot satisfy
a proof-required deployment bundle.
