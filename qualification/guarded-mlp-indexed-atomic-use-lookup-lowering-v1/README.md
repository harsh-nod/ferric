# Indexed-Atomic Use-Lookup Guarded Lowering

The controller consumes the use-lookup compiler generation and its separately
audited tools. The guarded candidate, vendor, driver, gfx950 target and
optimized-inline normalization remain unchanged. No work or structural ceiling
is raised.

## Controller Fixtures

| Attempt | Result | Seconds |
| --- | --- | ---: |
| [V1](controller-tests-v1/attempt-v1) | Failed: 18 reported successes, one error | 5.815253 |
| [V2](controller-tests-v1/attempt-v2) | 19 passed, no skips or errors | 4.655673 |

V1 exposed a production adapter defect: the current audit-input admission
check still expected the previous census-generation schema. The positive
fixture correctly supplied the new use-lookup schema and was rejected.
V2 changes only that production schema literal and extends existing tests to
assert acceptance of the current schema and rejection of the old one. The
19 test identities, normalization policy and all other admission predicates
are unchanged. This is an adapter repair, not a test-expectation-only change.

Both children exited naturally, were reaped, and left no process group.
Sources stayed unchanged during each run; postchecks were clean. V1's receipt
and seven raw files remain intact. Passing negative tests do not turn V1 into
a successful qualification.

V1 receipt:
`8a51e98c60b7b009afd0563c5cccede86d28090fee86368a88861a46a8927298`.
V1 archive:
`3728021cbebecc7cee8e1bc4a11c8a5f85a6c41ac494df6bd2b3df41b2758f65`.
V2 receipt:
`3b36a8fe7d89081915edd7c39ff89eff461c965b2439865c4f43d21fdae95d0e`.
V2 archive:
`c93ee4e85695113ab7e7f6e95eece5d8a3e33a88e91bd93da1fd364e192803c9`.
Each archive has 16 members, 15 pinned bodies and seven raw files.

## Actual Guarded Compile

The [actual retry](attempt-v1) used the successfully qualified compiler and
the seven freshly inspected tools on `mi350`. It still fails compilation, now
at `AuthenticatedAtomicAllocationsV1::permits_address`,
`indexed_atomic_v1.rs:84:14`, rather than the prior statement-use lookup.

| Measured Graph Work | Units |
| --- | ---: |
| Counter before the rejected charge | 3,145,230 |
| Requested address-inventory scan | 1,656 |
| Requested total | 3,146,886 |
| Unchanged limit | 3,145,728 |

The diagnostic identifies kernel root `ferric_qwen3_mlp_state_guard_v1`,
semantic function 1, with body SHA-256
`72f186b0c49aafc62f29a7b77a5bbca4fc458c507019297c8bea173b272fe70c`.
The charge covers the borrow inventory and projected-place comparisons.
This localizes another repeated query; it is not an alias-analysis verdict.
The next candidate is a bounded lookup preserving the exact place and block
checks, not a work-limit increase.

The compiler exited naturally with status one after 117.361738 seconds and
was reaped, leaving no process group. The whole attempt took 120.870576
seconds. Sources remained unchanged and postchecks were clean; there was no
timeout or forced cleanup. No HSACO or GPU result was produced.

All 13 archive members, 12 pinned bodies and ten raw files are retained,
totaling 4,223,088 body bytes. Receipt SHA-256:
`5a9b97db2af7e9e6f152aa84f632112abb587a27f3ffb6dca14920b0657a53df`.
The 674,001-byte archive SHA-256 is
`9eac7d5b6548378a78955682ec11b2c170a802721fce5fa3a26d6a22460c6b95`.

Independent model numerical and performance acceptance remain unestablished.
All issue #42 milestones and the 700 tokens/s target remain open.
