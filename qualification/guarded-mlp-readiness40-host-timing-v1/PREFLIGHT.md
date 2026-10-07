# Resource Preflight

The first launcher invocation on MI350, at 2026-10-07 07:40 UTC, exited with
`RuntimeError: free-space floor` from `resources(initial)` before creating the
case directory, admitting the CPU products, or spawning a parent/worker.
It used the final launcher SHA-256
`a5598c1ac6babb7e7d7cec84885e90766ce42669d3746eb113d2a1e621e632ee`.

A subsequent read-only inspection confirmed that the case directory was absent
and observed 42,611,990,528 free bytes, below the unchanged 40 GiB admission
floor (42,949,672,960 bytes). Shared-host space had declined after our preceding
cleanup. This is a launcher preflight refusal, not a GPU or numerical failure;
no native terminal exists for this invocation.

This note records the observed command result; it is not a machine-generated
native receipt. The original sources and prepared inputs remain unchanged.
Any explicit launch retry follows additional task-owned scratch cleanup and
must pass the same resource and idle-GPU admission checks. No result from a
later attempt may replace this preflight history.
