# CFG Expansion Diagnostic Guarded Lowering

## Fixture Qualification

The bounded MI350 lowering-admission run passed all 19 selected methods:
15 audit-binding methods and four unchanged normalization methods. It
completed in 7.718621 seconds. The child exited naturally with status zero,
was reaped and left no process group. Source, input and tool postchecks were
clean; there were no failures, skips or expected failures.

The [receipt](controller-tests-v1/attempt-v1/evidence/complete.json) is
12,957 bytes with SHA-256
`217f208452e7b91cfcb4e8e90ee1302fa73dd0d3c020f39b1729ae2dc85d600f`.
The [child observation](controller-tests-v1/attempt-v1/evidence/ranked-cfg-expansion-diagnostic-lowering-tests.stdout)
and [test output](controller-tests-v1/attempt-v1/evidence/ranked-cfg-expansion-diagnostic-lowering-tests.stderr)
retain all exact identities and passing statuses.

The archive contains 16 members, 15 pinned bodies and seven raw records,
totaling 185,400 body bytes. Its 39,911 bytes have SHA-256
`0949d9720baac77f9a680dfc8d9f60a0e09cc54177f5ab1e9dd9f3d9dbacb378`.
The retained [adapter](controller-tests-v1/attempt-v1/lowering.py),
[admission fixtures](controller-tests-v1/attempt-v1/test_lowering.py) and
[normalization fixtures](controller-tests-v1/attempt-v1/test_normalization.py)
are the reviewed `e151f6f5`, `949deb1a` and `6d4d91e2` bodies.

Source review corrected an inherited result flag that incorrectly described
this diagnostic as a graph optimization, and negative fixture mutations
that had become no-ops under the new contract. Those corrections preceded
execution. The suite checks the emitted result flags and rejects preceding
generation provenance; no failed test attempt is being omitted. Test count,
resource bounds and normalized command/output checks remain unchanged.

These synthetic fixtures do not establish actual guarded compilation,
HSACO, GPU execution, alias-analysis acceptance, model numerics or performance.

## Actual Guarded Attempt

The guarded compile failed at the unchanged projected-block gate. Its child
exited naturally with status 1 after 117.460291 seconds; the complete run
took 121.073785 seconds. The child was reaped and left no process group.
Sources remained unchanged and postcheck errors were empty, with no timeout
or forced cleanup. No artifact was produced.

The original [stderr](attempt-v1/evidence/compile.stderr) now reports:

```text
cfg-block-diagnostic-v1 blocks=2778 limit=2048 reason=above-limit
site=projected-cfg-expansion semantic_blocks=1121
kernel_export=ferric_qwen3_mlp_state_guard_v1 phase=root-projection
```

These fields appear on one line in the original; the excerpt is wrapped
here for readability. The function SHA-256 is
`72f186b0c49aafc62f29a7b77a5bbca4fc458c507019297c8bea173b272fe70c`,
with role `KernelRoot`, source `Rust source edb8c73e35fa:17:1` and semantic
root/body 1. This identifies the state guard's 2,778 projected blocks,
730 above the 2,048 limit; 1,121 is the declared semantic block count.
It does not measure the number of guarded accesses or establish alias safety.

The controller does not parse compiler diagnostics. Its predeclared
`actual_failure_block_count_observed=false` and
`actual_failure_caller_identified=false` fields therefore remain unchanged
in the original receipt. The attribution above comes from the separately
pinned stderr, not from rewriting or reinterpreting those receipt fields.

The [failed receipt](attempt-v1/evidence/failed.json) is 15,422 bytes with
SHA-256 `ba277bc9fbf3593b0b797a6bba3e7f8ee4fd073b8e28dc33d245b465d0c09a59`.
The 8,424-byte stderr SHA-256 is
`f806e763a124eb460d47de32531749744f9d24a03cd240115d9f744326ee374a`.
The archive contains 13 members, 12 pinned bodies and all ten raw records,
totaling 4,232,865 body bytes. Its 675,135 bytes have SHA-256
`61c3c752eff4ec02c481f9e95b3184b46b844240cdde29eca28e7bebf8b1f428`.

The next investigation is compaction of generated trapping-control blocks,
without deleting bounds checks, atomic effects or independent capacity
checks. No such optimization is qualified by this diagnostic attempt.
No HSACO, load/launch authority, GPU execution, independent model numerical
acceptance or performance result is established. All issue #42 milestones
and the 700 tokens/s target remain open.
