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

Actual guarded compilation is a separate gate. No HSACO, GPU, independent
model numerical or performance acceptance is claimed by these fixtures.
All issue #42 milestones and the 700 tokens/s target remain open.
