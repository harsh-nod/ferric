# Scoped Allocation Census Qualification

This opt-in experiment reduces repeated host-side discovery during a warm
guarded-MLP layer. The runtime and worker are integrated after successful
coupled CPU qualification on MI350. Parent integration, strict data admission
and a fresh GPU comparison remain separate gates.

The selected path moves two zero-add allocation preflights inside the existing
closed layer operation. It retains all four capacity-fence boundaries as
ordered rank-local checks, independently reconstructs full allocation accounting,
compares complete before/after snapshots, and checks actual owner counts against
the worker's private ledger before commit. The ledger values are reject-only
assertions, not allocation authority. Failure or unwind remains terminal.

Only warm positions 2 through 39 of the explicit Readiness40Position5 census
selector use this path. First use, metadata, final-forward, ledger-commit,
setup and Close checks stay unchanged, as do allocation limits and BF16 math.
Ordinary, previous scoped, bank-scoped and Full2303 routes stay unchanged.
Moving checks changes their observation timing; temporal equivalence to repeated
full discovery is not claimed.

- [Coupled CPU results and original evidence](cpu-v1/README.md).
- [Exact tested-source integration postcheck](integration-v1/postcheck.json).
- [Previous bank-rearm GPU comparison](../guarded-mlp-scoped-bank-rearm-v1/matched-timing-report-v1/README.md).

All 26 CPU phases passed: 1,165 runtime tests with eight existing ignores,
779 worker tests with four existing ignores, nine focused runtime scopes,
ten selected facade doctests and eight parser regressions. The 12 runtime and
13 worker tested Rust files were integrated, with a 2,100-body source postcheck.
No GPU run or timing improvement for this new census path is claimed.

The matching fe2o3 runtime commit is
[`a820d383a3`](https://github.com/harsh-nod/fe2o3/commit/a820d383a3676f9a9002a15323347a7526e71ded).

This is not independent numerical acceptance, generated-token throughput,
GPU overlap, Full2303 launch admission or attainment of 700 tokens/s.
All issue #42 M0-M7 milestones remain open.
