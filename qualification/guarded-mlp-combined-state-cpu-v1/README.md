# Combined State CPU Qualification

Status: **failed** on MI350. This is a host-side qualification of the
experimental v2 kernel crate, not a GPU compile or execution result.

[Attempt v1](attempt-v1/evidence/failed.json) completed eight phases before
the ninth phase, `lib-tests`, exited naturally with status 101. The raw
[test output](attempt-v1/evidence/lib-tests.stdout) reports 38 passing tests,
one failure and no ignores. The controller leaves `tests: null` because the
qualification failed; that original receipt is preserved unchanged.

The failing test is
`combined_source_contract_has_one_unsplit_argument_and_literal_store_offsets`.
It searches for a single-line Release-store spelling in `guard.rs`, but the
required formatting phase splits that invocation across lines. Host checking,
test compilation and both inventories pass. All 27 inherited tests and 11 of
the 12 new tests pass; the source-shape assertion fails. A formatting-independent
test repair and fresh qualification are required before accepting this crate.

All nine children exit naturally, are reaped and leave no process groups.
There are no timeouts, forced cleanup or postcheck errors. Tested sources and
dependencies are unchanged after execution. The final host-build phase did not
run. No GPU, HSACO, model-numerical, production or performance claim is made.

## Retained Evidence

The closed capsule contains 65 members: 64 content-pinned files and its
[retention manifest](attempt-v1/retention-manifest.json). It retains 51 raw
records and 11 selected source/controller/lock bodies, with the full 5,308-row
source inventory. Dependency source bodies and host binary bodies are not
included.

- Failed receipt: 122,686 bytes, SHA-256
  `17ae87ee9eaf720fba25df765fdee2942fb30866a073ff63c4cdedca93a75760`.
- Archive: 2,204,250 bytes, SHA-256
  `a52c78f53c980ad6d11c66aa36aab78a06b4076c1980f34e0d1dd1ad2279dfea`.
- Original proposal: [combined-state source](../guarded-mlp-combined-state-v1/proposal-v1/source-manifest.json).

All issue #42 M0-M7 milestones and the 700 tokens/s target remain open.
