# Bounded Indexed-Atomic Membership

This experiment addresses the [measured guarded-kernel compiler rejection](../guarded-mlp-ranked-graph-work-diagnostic-lowering-v1/README.md)
in `AuthenticatedAtomicAllocationsV1::contains_local`. It changes compiler
analysis, not the GPU kernel's arithmetic or launch configuration.

## Change And Bound

The private local-variable inventory is already sorted and deduplicated by
its constructor before membership queries. The patch replaces its linear
membership scan with an explicit binary search over that same vector. No
cache, bitset, allocation or additional storage is introduced. Construction,
sorting charges, escape checks, source-origin checks, atomic ordering/scope,
statement binding and alias predicates are unchanged.

After an unsuccessful comparison, at most half the interval remains. For
`n > 0`, the worst-case number of comparisons is therefore
`floor(log2(n)) + 1`. The compiler charges this full bound before searching.
The midpoint uses `start + (end - start) / 2` to avoid overflow. Each unequal
comparison strictly shrinks the interval; equal local IDs preserve the old
membership result.

| Inventory size | Previous charge | New upper-bound charge |
| ---: | ---: | ---: |
| 0 | 0 | 0 |
| 1 | 1 | 1 |
| 1,656, from the actual diagnostic | 1,656 | 11 |

These are analytical work charges, not measured timings or GPU speedups.
Empty queries still call `charge(0)`, including its refusal of a preexisting
over-limit counter. Failed charges still leave the inventory's counter
unchanged. The graph-work ceiling stays 3,145,728; all storage, block, edge
and other ceilings remain fixed. Resource admission may change because the
lookup now performs and charges less work, not because a limit is increased.

## Tests And Status

Nine new tests cover 4,608 differential queries over all small sets, boundary
lengths, duplicate/extreme IDs, exact-limit and overflow behavior, independent
inventories, the real constructor and statement binding, and negative guard,
marker and pointer-escape cases. The measured-size fixture makes 4,096 queries
and expects 45,056 work units. The existing exact-budget test is adjusted to
the new comparison bound without removing its rejection assertions.

Full qualification on `mi350` is pending. It preserves all 37 predecessor
phases and adds a focused repeat of the nine membership tests. Successful
completion would require 38 phases, 25 scopes, 2,876 passing executions and
the same 25 historical ignores. No such result is claimed by this source
checkpoint. Fresh binary-loader checks and actual guarded compilation must
follow successful qualification.

[proposal-v1](proposal-v1) contains the reviewed patch and source manifest.
The manifest is
`0e98d81d3b437e6bdb25a1a8a70608cf81a2c03fba13864e0793556e9028e8c8`;
the patch is
`95705322ff130f676e4d462876e77427d1dc110ae1b466fc3f13471d0c675a88`.
It changes two existing source files and adds one test file over the exact
[qualified diagnostic generation](../guarded-mlp-ranked-graph-work-diagnostic-v1/README.md).
The pinned failure attribution concerns that baseline, not a new run.

This does not change the production execution path or establish an HSACO,
GPU, full-model numerical or performance result. All issue #42 milestones
and the 700 tokens/s target remain open.
