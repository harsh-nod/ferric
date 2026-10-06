# Borrow Lookup Tool Audit

## Fixture Qualification

The bounded MI350 loader-admission fixture run passed all 26 selected methods.
The controller completed in 35.674147 seconds. Its single owned child exited
naturally with code 0, was reaped, and left no process group. Source, input and
tool postchecks were clean; there were no failed, skipped or expected-failure
methods.

The [actual receipt](controller-tests-v1/attempt-v1/evidence/complete.json) is
14,064 bytes with SHA-256
`ab5c802a331e8c4ef1fdebca46c072a3617e87aafb55b14f3b15a14b63f2b59b`.
The original [child observation](controller-tests-v1/attempt-v1/evidence/indexed-atomic-borrow-lookup-loader-tests.stdout)
and [test output](controller-tests-v1/attempt-v1/evidence/indexed-atomic-borrow-lookup-loader-tests.stderr)
retain all 26 exact identities and passing statuses.

The [retention manifest](controller-tests-v1/attempt-v1/retention-manifest.json)
authenticates 14 original bodies, including seven raw records. The archive has
15 ordinary members totaling 242,376 body bytes. The 46,922-byte archive
`guarded-mlp-indexed-atomic-borrow-lookup-loader-tests-evidence-v228-v1.tar.gz`
has SHA-256 `378bdff0c7d313ef8574afc00d95864fc047228d09683a2071d7b3bf49725f59`.

The retained [auditor](controller-tests-v1/attempt-v1/audit_tools.py) and
[fixtures](controller-tests-v1/attempt-v1/test_audit_tools.py) are the reviewed
`70e8e3c7` and `55d52aa7` bodies. They preserve the original producer/driver
checks and add exact borrow-generation source, phase, inventory and final-Cargo
joins. The 26-method roster and resource/lifecycle limits remain consistent
with the preceding use-lookup fixture generation; no tests were dropped.

This section records synthetic fixture evidence, not the actual fourteen-leaf
tool audit. CPU qualification, real tool inspection and guarded compilation
are separate gates. These fixtures do not establish an HSACO, GPU execution,
numerical correctness or performance.

## Actual Tool Inspection

The [fresh deployment](attempt-v1) passed all 14 binary-loader checks on
`mi350` in 2.455048 seconds. Every child exited naturally with status zero,
was reaped and left no process group. Postchecks were clean, without timeout
or forced cleanup. The extractor and backend are joined to the successful
borrow-lookup compiler build; the other five tools retain their prior
identities. The extractor resolves the exact deployed backend.

The archive contains 96 members, 95 pinned bodies and 72 raw records, totaling
22,135,850 body bytes. It includes the historical diagnostic, membership and
dead-cast census receipts consumed by admission. This is a bounded selected
evidence collection, not a copy of every transitive input or binary body;
the producer and preceding checkpoints retain their own qualification data.

Receipt SHA-256:
`c1fe5fa6dd25bb477d2e9a58508ddd2d7cfcae30af3b45d4534d9fe594af9216`.
The 3,690,880-byte archive SHA-256 is
`f57f0c4e150db013fd025b39b15bb0f66c5350ba78da34fe6bb25c0157c4d979`.

This inspection does not compile or execute the guarded kernel. No guarded
HSACO, GPU, independent model numerical or performance acceptance is claimed.
All issue #42 milestones and the 700 tokens/s target remain open.
