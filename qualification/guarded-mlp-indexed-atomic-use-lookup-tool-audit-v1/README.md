# Indexed-Atomic Use-Lookup Tool Audit

## Controller Fixtures

[controller-tests-v1/attempt-v1](controller-tests-v1/attempt-v1) passed all
26 synthetic admission tests on `mi350` in 30.894754 seconds. The child exited
naturally with status zero, was reaped, and left no process group. Sources
stayed unchanged and postchecks were clean. No tests were skipped.

Receipt:
`79fd0d19c211593d22961c01953967bf0fffeef61340689df6b0336794178dd0`.
Archive:
`5f202074a34d204a5fdb1bc97b186d9323fbad0b4351e00670e9c72f9fbd5c90`.
The archive contains 15 members, 14 pinned bodies and seven raw files.

## Actual Tool Inspection

The [fresh deployment](attempt-v1) passed all 14 binary-loader checks in
2.344472 seconds on `mi350`. Every child exited naturally with status zero,
was reaped and left no process group. Input postchecks were clean, with no
timeout or forced cleanup. The deployed backend and extractor are joined to
the successful use-lookup compiler qualification; the other five tools retain
their prior identities. The extractor resolves the exact deployed backend.

The evidence archive retains 95 members, 94 pinned bodies and 72 raw files,
totaling 20,779,596 body bytes, including the historical membership and
diagnostic receipts consumed by admission. Receipt SHA-256:
`49e13142c8316431c76a7fecb094e4ec42ce86c962d81be08da9e5aa6f65febd`.
The 3,489,517-byte archive SHA-256 is
`c9e8218d86710c02c3f60657ec0b649566569bc22d2ad903614041a3472f20e5`.

This inspects binary loading and provenance; it does not compile or execute
the guarded kernel. No guarded HSACO, GPU, independent model numerical or
performance acceptance is claimed. All issue #42 milestones remain open.
