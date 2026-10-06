# Atomic Load Alias Lowering

Status: **actual checked lowering and 31 controller fixtures pass on MI350**.
The new backend clears the previous
[combined-state descriptor refusal](../guarded-mlp-combined-state-lowering-v1/README.md)
and emits a gfx950 HSACO. This is not GPU execution or runtime admission.

## Actual Checked Lowering

The [actual receipt](attempt-v1/evidence/complete.json) records a natural zero
exit after 394.288906 seconds, or 399.441433 seconds for the whole controller.
The child is reaped and leaves no process group; no timeout or forced cleanup
occurs. All 5,308 source rows and 308 input pins are unchanged, all 34 checked
Cargo configuration locations remain absent, and postchecks are clean.

The [observation](attempt-v1/artifact/observation.json) records code-object
version 6, `gfx950:xnack-`, and exact-output replay. The retained
[HSACO](attempt-v1/artifact/observation.hsaco) is 28,440 bytes, SHA-256
`de880dcebf79425ccb555c1a2bdca3ff78b926f7b2062d06b918763da4de0f66`,
with both expected exports:

- `ferric_qwen3_mlp_state_guard_v2`
- `ferric_qwen3_tp2_guarded_projection_residual_bf16_v2`

The [retention manifest](attempt-v1/retention-manifest.json) pins 14 bodies in
a 15-member capsule, including ten raw files and both artifact bodies,
totaling 4,480,607 expanded bytes. The receipt is 25,204 bytes, SHA-256
`1053d69036d94db89349b17584819a14f27a824691678730bb4204d283ea93da`.
The 707,862-byte archive has SHA-256
`47e7cc031d3b2082a01e5aba425d630c375b69188dda944b055e6d64d5e8f750`.
The earlier failed compile and its diagnostic remain unchanged.

The observation explicitly grants no publication, load or launch authority.
Image ABI/instruction inspection, private atomic-read runtime admission,
paired GPU lifecycle and independent numerical tests are separate next gates.
All issue #42 milestones and the 700 tokens/s target remain open.

## Controller Fixtures

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

Actual lowering uses the new
[compiler qualification](../guarded-mlp-atomic-load-alias-v1/README.md) and a
fresh successful [tool audit](../guarded-mlp-atomic-load-alias-tool-audit-v1/README.md).
The frozen fixture controller retains pending actual outcome pins; its
synthetic tests do not substitute invented receipts for real evidence.
The separately bound actual controller and outcome are retained above.
No GPU, model-numerical, performance or issue #42 milestone acceptance is claimed.
