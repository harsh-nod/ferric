# Projection-Residual / MLP Ordered Segment: CPU Qualification

The opt-in two-rank ordered segment passed all 75 bounded qualification phases
on ASROCK. This checkpoint integrates the exact tested Ferric and fe2o3 source
bodies. It is CPU compilation and protocol qualification, not native GPU
execution, independent numerical acceptance or a performance result. All
[issue #42](https://github.com/harsh-nod/ferric/issues/42) milestones remain open.

## Actual Results

| Scope | Passed | Historical ignores |
| --- | ---: | ---: |
| Full fe2o3 runtime library | 922 | 3 |
| Ferric worker library and selected binary tests | 562 | 4 |
| Ferric parent selected suites and binaries | 364 | 0 |
| Total Rust test executions | 1,848 | 7 |

All 75 phases exited naturally, including the final default-feature parent
`cargo check`. Five executables were built: worker, plain decode parent,
default host-observer parent, shared-full parent and ordered parent. The
recorded Cargo profile is optimization level 2, with overflow checks and
debug assertions enabled and no debug information; the `debug/` directory
name does not mean unoptimized code. Seventeen separate CPU-controller
policy tests passed before the run.

The actual [completion record](complete.json) is 624,332 bytes, SHA256
`6ac67053d9b7d5d15772b5e4071a09933f013b6efa27394c31eb2b8279af884f`.
The [publication ledger](result.json) records suite counts, source transitions
and binary identities; exact named inventories are in the completion record
and retained test output. Source, dependency and old-target
postchecks passed. The runtime baseline contains **900**, not 898, named
tests; the 25 additions produce 925 outcomes, including three historical
ignores. Earlier preparation estimates of 1,846 passes were incorrect.

## Implementation

The new parent is
`ferric-qwen3-finite-projection-residual-decode-ordered-host-engineering`.
Its explicit `--observe-projection-residual-mlp-ordered` route enables the
worker's typed ordered-segment path. The default decode route is unchanged.

The corresponding fe2o3 runtime implementation is
[commit abb13a375](https://github.com/harsh-nod/fe2o3/commit/abb13a375472010680ed2949451214e5a57ef38c),
also mirrored to `powderluv/fe2o3` on the runtime engineering branch. Both
repositories use the exact source bodies recorded in this qualification.

Each rank queues projection-residual followed by MLP, with the second packet
waiting for its predecessor. Both ranks publish before host polling. This
combines two dispatch rounds into one and reduces host-coordinated rounds
per layer from four to three. Kernel images and arithmetic are unchanged.
The segment uses one bounded deadline, fixed signal slots, exact terminal
states and currentness checks. Failure poisons the participating contexts
and group; it does not retry, rearm or release uncertain in-flight storage.

A distinct 16 KiB Down-output scratch buffer is allocated once per rank.
Reusing the old output-partial alias would allow a fast rank's MLP to
overwrite data before its peer finished the first residual reduction.
The final residual instead reads the new Down-output pair. There is no
per-layer allocation or host copyback for this buffer. Resource censuses,
wire validation and Close checks account for its ownership.

The ordered control record is 241,096 bytes; the existing captured tensor
payload remains 606,976 bytes. The control format records one inclusive
segment interval instead of inventing independent residual/MLP timings.
Only the four-forward, no-internal-capture observer route is admitted here.

## Failed Attempt And Correction

The first CPU attempt remained failed: its final default-feature check
reported missing feature-gated imports. Two new parent-library exports
lacked `#[cfg(feature = "tp-batch-engineering")]`. The corrected candidate
adds those guards and regression assertions inside the existing ordered
binary test, then reruns the complete 75-phase qualification. It does not
skip the failing check or change kernels, arithmetic or test names.

The original failed receipt and final check output are retained in
[failed-v1](failed-v1/README.md). Earlier successful tests and executable
builds did not make that failed aggregate a qualified build.

## Evidence And Timing Scope

The publication includes all 375 phase records, 60 exact formatted source
overlays, both proposal manifests and patches, the CPU controller and its
input plan. Seven large source/dependency maps were authenticated and are
retained outside Git; their identities remain in `complete.json` and
`result.json`. Executable bodies, build targets and model data are not
committed. This publication does not requalify the compiler.

Read [the counter-scope audit](publication/COUNTER-SCOPE.md) before comparing
this route with shared-full. Ordered publish/wait counters have different
boundaries: staging precedes publish timing, rank waits overlap and include
peer completion, and polling counts only the final slot. Summing them does
not produce a GPU makespan. Compare matched enclosing forward wall times
and actual outputs, disclosing first-use allocation and fixed run order.

Next gates are fresh MI350 executable audits and a same-build shared-full /
ordered GPU comparison. Output repeatability would still not establish
independent model accuracy. Sustained single-request Qwen3-8B BF16
target-only 2,048/256 decoding and the 700 tokens/s target remain unproved.
