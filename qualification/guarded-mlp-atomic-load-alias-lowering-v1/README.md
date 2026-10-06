# Atomic Load Alias Lowering

Status: **31 controller fixtures pass on MI350; actual checked lowering pending**.
The latest retained guarded compile is still the
[combined-state descriptor refusal](../guarded-mlp-combined-state-lowering-v1/README.md).

The [actual fixture receipt](controller-tests-v1/attempt-v1/evidence/complete.json)
records 31 passing synthetic tests, zero failures or skips, in 31.660490
seconds: 27 lowering tests and four unchanged normalization tests. All 25
historical cases remain, with six new alias-generation cases. The process
exits naturally, is reaped and leaves no process group. Source and tool
postchecks pass under the unchanged 180-second whole-run and 120-second leaf
limits.

New coverage checks the separate current and historical producer contracts,
semantic flags, source and product identity, kernel-IR census, pending evidence,
and malformed or substituted admission data. The auditor fixture is an
authenticated data dependency; its test cases are not included in this suite.
Candidate CPU and vendor gates, command construction, output validation,
resource limits and cleanup rules are unchanged.

The [retention manifest](controller-tests-v1/attempt-v1/retention-manifest.json)
pins 21 bodies in a 22-member capsule, including seven raw files, totaling
700,373 expanded bytes. The receipt is 17,841 bytes, SHA-256
`7aedc64cbcbdce5f58062cfe18d7cea0389f03c4401fc659e2b1be1a0be7e87b`.
The 125,637-byte archive has SHA-256
`f1d31bb3814e396f6a5ed358ad3d336c753d07c2d7c0a8452792d49be0fea06b`.

Actual lowering requires the new
[compiler qualification](../guarded-mlp-atomic-load-alias-v1/README.md) and a
fresh successful [tool audit](../guarded-mlp-atomic-load-alias-tool-audit-v1/README.md).
The fixture controller retains pending actual outcome pins; tests do not
substitute invented receipts for real evidence. No HSACO, GPU, model-numerical,
performance or issue #42 milestone acceptance is claimed.
