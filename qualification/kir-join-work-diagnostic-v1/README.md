# Canonical KIR Join Work Diagnostic

The ASROCK diagnostic localized the retained RoPE handoff's verification-work
refusal to the inert formal layout join. All 190 default finalizer tests
passed, with 15 historical actual-capture tests ignored by the default suite.
The separately selected actual-capture test reproduced the expected failure.
The instrumentation does not change budgets, validation or error propagation.

| Observed Quantity | Value |
| --- | ---: |
| Evidence rows | 318 |
| Basic blocks | 1,019 |
| Counted nodes | 6,096 |
| Accepted work before the layout join | 40,799,816 |
| Pending layout-join charge | 1,044,025,344 |
| Attempted cumulative work at refusal | 1,084,825,160 |
| Unchanged work limit | 1,073,741,824 |
| Live and peak storage at refusal, bytes | 8,539,573 |

The old join prepays `128 * (rows + blocks + 1) * nodes` before repeated
block/value scans. Its pending charge exceeds the remaining work budget by
11,083,336 units. These are abstract verifier work units, not CPU timings or
GPU instructions. The rejected charge was not performed, and the total work
needed to finish is still unknown.

The [actual trace](raw/actual-inert-join-diagnostic-stdout) and
[checkpoint table](checkpoints.md) record the last observed stage as
`layout-join-before`. The handoff SHA256 remains
`ad4b31ee88efa54702dc8dff13e1e3c331c296734f7cdcfa52e8249ed9fa37fc`.
The selected test exited naturally with code 101. All eight process leaves
were reaped; the enclosing diagnostic owner exited naturally with code 0,
without forced cleanup. Source/input postchecks passed.

The [diagnostic result](diagnostic/complete.json), [owner result](owner/complete.json),
three actually formatted Rust bodies, controller, proposal and raw phase
records are retained with the [publication ledger](result.json). Sixteen
controller-policy tests passed separately; their [primary-agent SSH observation](pure/root-observation.json)
is not presented as a remote supervisor receipt. Ten large snapshots were
rehashed remotely and are hash-linked, not copied here. Two large owner
inventories were also rehashed locally and remain in the retained courier;
their identities are in the ledger instead of duplicating them in Git.
No compiler binaries or model tensors are included.

Next is an indexed inert-join implementation that preserves first-definition
lookup order, duplicate-block rejection, ordered evidence checks and resource
accounting. It must pass library regression tests and this same actual handoff
under unchanged limits before fresh emission is attempted.

Diagnostic completion means the failure was reproduced and localized, not
that the inert join passed. No HSACO was emitted, no GPU was run and no
numerical or performance acceptance follows. All issue #42 milestones and
the sustained BF16 2,048/256, single-request 700-token/s target remain open.
