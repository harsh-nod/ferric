# Cached Indexed-Atomic DAG Certificate

This isolated compiler experiment reduces repeated acyclicity analysis in
indexed-atomic and static-publication proofs. Its direct baseline is the
[qualified block-cap generation](../guarded-mlp-ranked-cfg-capacity-v1/README.md).
The last actual guarded lowering attempt exhausted the graph-work budget;
that diagnostic did not identify its function or charge site. This experiment
must not be described as repairing that failure until a fresh lowering run
establishes it.

## Algorithm

Previously, each pointer-chain definition requested a fresh reachability
walk to establish that its defining block cannot reach itself. The same
immutable function graph can receive many such queries.

The new implementation lazily runs Kahn's algorithm over all declared blocks,
including unreachable blocks. If the entire graph is acyclic, every block
satisfies the original predicate. A private cache in that graph's proof
instance allows later queries to return the same answer without repeating
the traversal. No cache is shared between functions or proof instances.

If any cycle exists, the original per-definition traversal remains in use.
This matters for an acyclic block that leads into a cycle: it need not itself
be cyclic. Malformed indices fail before cache lookup. A failed initial
construction or query does not publish a cache entry.

| Property | This Experiment |
| --- | --- |
| Graph-work ceiling | 3,145,728, unchanged |
| Block and edge ceilings | 2,048 each, unchanged from the direct baseline |
| Successful whole-DAG construction charge | `5V + 2E` |
| Per-query charge after a successful DAG certificate | 1 |
| Temporary storage | Two arrays of at most `V` machine words each |
| Atomic, alias and source-origin predicates | Unchanged |
| Runtime, kernel source and ABI | Unchanged |

The construction charges for two bounded scratch arrays, two node scans,
one node-processing pass and two edge scans. Arrays are charged before
allocation. Thus a chain queried at every block costs `8V - 2` work units,
including all queries. These are analysis-accounting bounds, not measured
GPU performance or compiler wall-clock speedups.

Resource admission can change even though the Boolean predicate is preserved.
DAG queries can fit the unchanged work budget with less repeated work.
Cyclic inputs incur one certificate attempt plus lookup charges before the
original traversal and may refuse earlier near the budget boundary.

## Qualification

Full CPU qualification passed on `mi350` in 650.860950 seconds. All 36 phases
exited naturally with status zero, were reaped and left no process groups.
Source and dependency checks were clean, with no timeout or forced cleanup.

| Executed Scope | Passed | Ignored |
| --- | ---: | ---: |
| Complete compiler library | 1,257 | 24 |
| Complete Pliron library | 1,507 | 1 |
| Earlier focused controls, repeated | 43 | 0 |
| CFG diagnostic controls, repeated | 8 | 0 |
| Independent-cap controls, repeated | 4 | 0 |
| New DAG certificate controls, repeated | 9 | 0 |
| Atomic, source-safety, matrix and attention controls | 12 | 0 |
| Total test executions | 2,840 | 25 |

Repeated executions are not unique tests. The new nine-test cohort covers
chains, diamonds, duplicate edges, cycles, unreachable cycles, malformed
graphs, exact budget boundaries, failed-cache publication and independent
proof instances. An independent transitive-closure reference checks every
directed graph with one through three nodes, totaling 1,570 block queries.
The large-chain cases cover 1,121 and 2,048 blocks.

The controller retains every existing full-suite and focused recipe, and adds
a focused repeat of the nine new tests. All historical test names and ignore
identities remain present. This qualifies the isolated compiler experiment,
not Ferric's production GPU path.

## Evidence

[attempt-v1](attempt-v1) retains the executed controller, helper, input,
four source postimages, direct baseline lineage, receipt and all 184 raw
records. Its retention manifest pins 206 bodies in 207 archive members,
totaling 33,716,952 body bytes. Receipt SHA-256:
`5441c57333baff468a787e83da5852bc74e3286501b25dabc0be8503b8a1f9cf`.
The 5,546,057-byte archive SHA-256 is
`1968bfb45c779e56236bd47d52fd7cb9cb47c56d7fe79da2595f69d13a0c7dc2`.

Loader inspection, actual guarded gfx950 lowering, GPU execution, independent
model numerics and sustained BF16 2,048/256 decode remain separate checks.
All issue #42 milestones and the 700 tokens/s target remain open.
