# Dead-Cast Census Tool Audit

The fresh compiler tools from the [qualified census generation](../guarded-mlp-indexed-atomic-dead-cast-census-v1/README.md)
passed all 14 `readelf` and `ldd` checks on `mi350` in 2.447325 seconds.
Every leaf exited naturally with status zero, was reaped, and left no process
group; final input and library postchecks were clean.

Only the extractor/backend provenance rows change in the seven-tool manifest.
Both join the actual final compiler-test-build products; the extractor's bytes
are unchanged. The other five tools remain exact. The selected backend is
`16a9a58baa0c41604a5e71fb75fd24a7494d6f4be054b03df1ccaeaabd039f3f`.
The backend rlib and test executables remain metadata-only provenance, not
deployed or newly rehashed binary bodies in this retained archive.

[attempt-v1](attempt-v1) retains 94 members, 93 pinned bodies and 72 raw files,
including the additional diagnostic-generation receipt actually read by this
audit. The tool receipt is
`3e6f8a2f094f275750b4974973226a5850d76707f9f5857720c289cc0f5d8669`;
the archive is
`3e22659507b3e497485cc28c58c8c9a5e8a4acc0b3ac2edc36f09e1b9f02dccc`.

## Controller Fixtures

| Attempt | Result | Seconds |
| --- | --- | ---: |
| [V1](controller-tests-v1/attempt-v1) | Failed: 25 reported successes, 1 error | 26.263219 |
| [V2](controller-tests-v1/attempt-v2) | 26 passed, no skips or errors | 24.132847 |

V1's synthetic positive case reused the same 14-byte/`a` digest placeholder
for a changed source's preimage and postimage. The production admission check
correctly refused that unchanged overlay. V2 changes only that fixture's
postimage placeholder to 15-byte/`b` and asserts every overlay row differs
from its preimage. The auditor itself is byte-identical. The original receipt
and raw error remain intact; passing negative tests do not turn V1 into a
successful qualification. Both attempts exited naturally, were reaped, and
left no process group, with unchanged sources and clean postchecks.

V1 receipt: `eeae4646bc683f08306892e92b38e30145ebfc73558bfb0140328b22fb706263`.
V2 receipt: `ce918a668b43dc7204aa88a84b4c9caf40a6e76cb11144a7830aff4797a471ac`.

These checks establish compiler-tool provenance and synthetic admission
behavior. They do not establish guarded HSACO emission, GPU execution, model
numerics or performance. All issue #42 milestones remain open.
