# Combined State CPU Qualification

Status: **attempt v3 passes** on MI350. This is a host-side qualification of the
experimental v2 kernel crate, not a GPU compile or execution result.

## Latest Attempt

[Attempt v3](attempt-v3/evidence/complete.json) passes all ten phases and all
39 tests, with zero failures or ignores. The supplementary source assertion
now tolerates rustfmt whitespace while checking the exact four literal stores
in order; its two required `std` imports are explicit. Production kernel
behavior is unchanged by these test repairs.

All children exit naturally with status zero, are reaped and leave no process
groups. Sources and dependencies remain unchanged, with no postcheck errors.
The final host-library build passes. Whole qualification time is 75.840396
seconds, not GPU or model latency.

The passing capsule contains 70 members, 69 content pins and 56 raw records.
Its receipt is 131,476 bytes, SHA-256
`ebdcb5f36dcdadd77614f85b91f0ed510d1ea6a44be407ab18d73a1aa56af696`.
Its archive is 2,218,349 bytes, SHA-256
`822cba63721bb5e475107b13ebc7441fdb4ecef03cd769799fb2e0aac4aee836`.
The [v3 source proposal](../guarded-mlp-combined-state-v1/proposal-v3/source-manifest.json)
and the tested Rust bodies are retained. Guarded gfx950 lowering, GPU execution,
independent model numerics and performance remain separate gates.

## Second Attempt

[Attempt v2](attempt-v2/evidence/failed.json) fails while building the test
binary. The formatting-independent assertion introduced unqualified `format!`
and `String` uses without their explicit `std` imports in this `no_std` crate.
The [raw Cargo diagnostics](attempt-v2/evidence/build-tests.stdout) contain
three missing-macro errors and one `E0425` missing-type error. No tests run in
this attempt. Five phases pass before `build-tests` exits naturally with
status 101; all children are reaped, process groups are absent, sources remain
unchanged and postchecks are clean.

This failed retry does not supersede the first run with a passing result.
The [v2 source proposal](../guarded-mlp-combined-state-v1/proposal-v2/source-manifest.json)
is retained, not installed in the canonical device crate. Its production
bodies match the first attempt's formatted sources. A two-import test repair
and a fresh complete qualification were required; attempt v3 above supplies
that CPU result. Both failed attempts remain unchanged.

The second closed capsule contains 50 members, 49 content pins and 36 raw
records. Its failed receipt is 111,015 bytes, SHA-256
`4d70e61800b8f82ae609583f5de08424ad79f099aad18fd9d8e400c1e67c8f47`.
Its archive is 2,201,568 bytes, SHA-256
`4674ae17f5e15f4334b8200f5ec9cfac4da390d0a5a2bb78d88b7f7c48b0f100`.

## First Attempt

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
test repair and fresh qualification were required; attempt v3 supplies that
CPU result without changing this failed attempt.

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
