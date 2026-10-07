# Scoped Allocation Census Qualification

This opt-in experiment reduces repeated host-side discovery during a warm
guarded-MLP layer. The runtime and worker are integrated after successful
coupled CPU qualification on MI350. The parent is now integrated after its own
qualification, and the strict data checker passes 98 synthetic tests. The fresh
same-binary GPU comparison now passes both modes with complete same-side parity.

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
- [Parent qualification](parent-cpu-v1/README.md) and [source postcheck](parent-integration-v1/postcheck.json).
- [Strict data-checker qualification](checker-cpu-v1/README.md).
- [Final bound report qualification](matched-timing-report-cpu-v1/README.md), 11 passing synthetic tests.
- [Actual GPU pair](matched-timing-gpu-v1/README.md) and [ablation plots and tables](matched-timing-report-v1/README.md).
- [Previous bank-rearm GPU comparison](../guarded-mlp-scoped-bank-rearm-v1/matched-timing-report-v1/README.md).

All 26 CPU phases passed: 1,165 runtime tests with eight existing ignores,
779 worker tests with four existing ignores, nine focused runtime scopes,
ten selected facade doctests and eight parser regressions. The 12 runtime and
13 worker tested Rust files were integrated, with a 2,100-body source postcheck.
The actual MI350 comparison completes all 40 prompt forwards in both modes,
with zero generated tokens, healthy Close and all records/four payloads equal.
Warm-forward parent wall time falls from 109.162 to 72.476 seconds (33.607%),
while total parent time falls from 480.326 to 445.273 seconds (7.298%).
First-use time rises 0.752%. This is one ordered pair, not a repeated benchmark
or generated-token throughput; the report retains each category and regression.

The separate parent run passes 535 selected tests across 57 scopes and all
67 phases in 416.111 seconds, producing seven executables. Its seven tested
Rust files are integrated with a complete 1,279-body source postcheck; all
220 qualified worker bodies remain unchanged. This is not the full parent
library suite. The checker passes all 98 named tests with no skips or failures.

The actual parent and worker Cargo artifact records both specify optimization
level 2, no debuginfo, and enabled debug assertions and overflow checks. Their
`target/debug` paths do not mean unoptimized binaries. Changing those build
settings would require a separate, measured comparison with new artifact IDs.

The matching fe2o3 runtime commit is
[`a820d383a3`](https://github.com/harsh-nod/fe2o3/commit/a820d383a3676f9a9002a15323347a7526e71ded).

This is not independent numerical acceptance, generated-token throughput,
GPU overlap, Full2303 launch admission or attainment of 700 tokens/s.
All issue #42 M0-M7 milestones remain open.
