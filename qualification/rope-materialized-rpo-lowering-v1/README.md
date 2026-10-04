# RoPE/RPO Checked-Lowering Attempt

The qualified reverse-postorder compiler cleared the earlier semantic SSA
partial-move storage refusal on ASROCK. Checked Rust lowering produced a
4,078,537-byte inert handoff, and the actual compiler replay passed. The next
stage, the finalizer's actual-inert-join test, failed at the unchanged
canonical-kernel work budget. No HSACO was emitted or admitted.

| Stage | Actual Result |
| --- | --- |
| Fixture metadata | Passed |
| Checked lowering | Passed; semantic MIR, neutral KIR, target KIR and handoff retained |
| Actual compiler replay | Passed; 15 roots and 8 unresolved runtime requirements |
| Actual finalizer inert join | Failed; work budget exceeded |
| Emission, extraction, metadata, ELF and ISA checks | Not attempted |

The [failing test output](raw/actual-inert-join-stdout) reports
`CanonicalKernelIrWorkLimitV1 { actual: 1084825160, limit: 1073741824 }`.
The reported work is the value at refusal, not a measurement of the work
needed to finish. No budget was raised or proof check removed. The checked
handoff SHA256 is
`ad4b31ee88efa54702dc8dff13e1e3c331c296734f7cdcfa52e8249ed9fa37fc`.

The finalizer test exited with code 101. Its process group was absent after
reaping; the enclosing owner exited naturally with code 1 and no forced
cleanup. All source/input postchecks passed. The [inner failure](lowering/failed.json),
[owner failure](owner/failed.json), raw command/results, tested fixture and
controller are retained with the [publication ledger](result.json).

The [compiler's CPU2700 qualification](../fe2o3-partial-move-rpo-v1/README.md),
[finalizer's CPU190 qualification](../fe2o3-partial-move-rpo-finalizer-v1/README.md)
and [candidate's CPU33 qualification](../rope-materialized-cpu-v2/README.md)
remain prerequisites, not substitutes for the failed actual-capture test.
Large compiler artifacts and snapshots are hash-linked in the retained
courier, not copied into Git. Publication rechecks retained data only.

Next: investigate canonical-kernel analysis cost while preserving semantics
and resource checks, then repeat actual finalization and image review before
any native attempt. Numerical acceptance, full-model correctness, sustained
2,048/256 decoding and 700 tokens/s remain unproven.
