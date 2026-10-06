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

Full CPU qualification is running on `mi350`. The new nine-test cohort covers
chains, diamonds, duplicate edges, cycles, unreachable cycles, malformed
graphs, exact budget boundaries, failed-cache publication and independent
proof instances. An independent transitive-closure reference checks every
directed graph with one through three nodes, totaling 1,570 block queries.
The large-chain cases cover 1,121 and 2,048 blocks.

The controller retains every existing full-suite and focused recipe, and adds
a focused repeat of the nine new tests. No completed result for this source
generation is claimed here yet. Loader inspection, actual guarded gfx950
lowering, GPU execution, independent model numerics and sustained BF16
2,048/256 decode remain separate checks. All issue #42 milestones and the
700 tokens/s target remain open.
