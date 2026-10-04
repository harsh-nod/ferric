# Independent Profile Numerical CLI

The frozen CLI source passed all 17 synthetic wiring tests on MI350, with no
errors, failures, or skipped cases. This checkpoint publishes the exact source,
test driver, source postchecks, and test transcript. It is not a numerical result
for a real GPU capture.

The CLI is a separate CPU-only step after the GPU observer and its audits finish.
It replays retained custody, binds an explicit arithmetic-assumptions review to
the new V7 source/image/LLVM/ISA evidence, and delegates to the existing
independent per-profile comparator. It adds no GPU launcher, retry, new numerical
tolerance, or paired-bitwise acceptance condition.

The tests cover strict inputs and pins, frozen observer prerequisites, arithmetic
review identities and nonclaims, audit ordering, exact comparator delegation,
explicit mathematical rejection, and fatal custody failures. Their observer and
reference calls are mocked. No real observer replay, numerical reference,
native executable, or GPU is run by this qualification.

## Evidence

- [Qualification result](result.json)
- [Actual 17-test receipt](evidence/complete.json)
- [Named test transcript](evidence/tests.log)
- [Frozen source manifest](package/manifest.json)
- [CLI source](package/run.py) and [tests](package/test_run.py)
- [Actual test driver](test-driver/run_independent_numerical_cli_pure_p228_v1.py)

All three source files and the controller are pinned, and the full before/after
source snapshots match. The frozen package manifest is 664 bytes with SHA256
`bbd9d3e89f0036b06d3080750f01c8003284ed9e1a48570632d27a731a371464`.
The actual pure-test receipt is 2,951 bytes with SHA256
`b712dd14bc46d4736e6615e394369ab90c85d51e8ae699ec114844baabfdda3b`.

The original package README retains its author-time draft wording to preserve
the qualified bytes. This checkpoint's manifest, receipt, and transcript record
the subsequent freeze and actual test execution. External observer/reference
packages remain prerequisites; this is not a standalone runtime bundle.

Real per-profile reference comparisons remain separate evidence. This checkpoint
grants no arithmetic-premise discharge, numerical or full-model acceptance,
production launch authority, performance gain, or throughput claim.
