# CFG Compaction Tool Audit

The [compaction-qualified compiler](../guarded-mlp-ranked-cfg-compaction-v1/README.md)
passes all 14 actual binary-loader checks on `mi350` in 2.510559 seconds.
The backend and extractor roles are redeployed from that generation's final
Cargo products; five other tools remain unchanged. The extractor bytes are
identical to the preceding generation, while the backend changes.

The [fixed loader fixtures](controller-tests-v1/attempt-v1) pass all 26 named
tests in 50.563649 seconds, with unchanged source bytes and clean postchecks.
They exercise producer identity, exact source overlay, inherited test outcomes,
resource limits, artifact provenance, prior failure evidence, and negative
admission cases. Synthetic fixtures are distinct from the actual loader audit.

The [actual audit](attempt-v1) retains the readelf/ldd outputs, commands,
process records, input manifest, producer joins and selected historical
receipts. All children exit naturally with status zero, are reaped, and leave
no process group. No timeout, forced cleanup or postcheck error occurs.

Receipt SHA-256:
`141f7f4f1c7b31fdb936853fc41b807acbf6054116543d9062f274a6e4790725`.
The 3,897,231-byte archive SHA-256 is
`747a4107f2475495cd303039473704a2da089e074822aebfa18eccc61d469caa`.
Its 97 members contain 96 pinned bodies and 72 raw files totaling 23,443,513
body bytes. This is a selected evidence capsule, not a copy of every binary
or every transitive dependency body. The full input readset is recorded.

This audit inspects loaders; it does not compile a kernel or grant HSACO,
GPU, model numerical, or performance acceptance. The separate
[guarded attempt](../guarded-mlp-ranked-cfg-compaction-lowering-v1/README.md)
uses these checked tools. All issue #42 milestones remain open.
