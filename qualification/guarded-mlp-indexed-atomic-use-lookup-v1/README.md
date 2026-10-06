# Bounded Indexed-Atomic Statement Lookup

This compiler experiment addresses the [measured statement-validation work charge](../guarded-mlp-indexed-atomic-dead-cast-census-lowering-v1/README.md).
It changes compiler analysis, not GPU arithmetic or launch geometry.

## Change And Bound

Authenticated atomic uses are already appended in ascending block/statement
order, at most once per statement. The new lookup searches that existing
private inventory with binary search. No sort, auxiliary index, cache or new
scratch allocation is added. It preserves the old linear search's first-match
behavior, including duplicate keys in synthetic fixtures.

For N uses, each lookup charges zero comparisons when empty, otherwise
floor(log2(N))+1 before searching. The measured 552-entry inventory therefore
has a 10-comparison bound rather than a 552-row scan. This is an analytical
compiler-work bound, not a measured speedup or a prediction of successful
kernel compilation. The graph-work ceiling remains 3,145,728; resource
admission may change because the analysis does less charged work.

Statement validation retains the exact address, access kind, ordering and
scope checks. Source authentication, bounds, pointer escape, allocation and
alias predicates are unchanged. Other inventory queries remain unchanged.

## Tests And Status

The [source proposal](proposal-v1) changes two existing files and adds one
test file. Nine authored tests cover differential pointer identity against
the old linear search, 6,400 small-set queries, boundary sizes, duplicate and
extreme keys, exact work limits, overflow, failure counter preservation,
independent inventories and unchanged semantic refusals. Another fixture
uses 8,192 queries over 552 uses and asserts an 81,920-unit charge.

Manifest: `aeac5c853c40ec768e1b5add4a2decf6338d4a8a15c44abe3bc9962eab000f0e`.
Patch: `8f171dfdce9aea4ea039fe4a14286c152c0030e2db4e4a668dd7784a1b97068d`.

## MI350 Qualification

The [fresh qualification](attempt-v1) passed all 40 phases across 27 test
scopes in 661.607834 seconds on `mi350`.

| Executed Scope | Passed | Ignored |
| --- | ---: | ---: |
| Complete compiler library | 1,293 | 24 |
| Complete Pliron library | 1,507 | 1 |
| Focused repeats and extraction controls | 112 | 0 |
| Total test executions | 2,912 | 25 |

Totals include repeated executions, not just unique tests. All children
exited naturally with status zero, were reaped and left no process group.
Source and dependency checks stayed unchanged; postchecks were clean, with
no timeout or forced cleanup. The historical ignores remain explicit.

The retained archive contains 226 members, 225 pinned bodies and 204 raw
files totaling 32,527,172 body bytes. Receipt SHA-256:
`e6d2f3d59d8dff63520be146a8e74cc79ead0baec29ea655e2c369232fa1dea9`.
The 5,359,779-byte archive SHA-256 is
`4384e6c344cd7913f02e2ecb9dd1ff39ef2a51d5dca39b3d85005e300d43b9c0`.

The [loader fixtures](../guarded-mlp-indexed-atomic-use-lookup-tool-audit-v1/README.md)
pass 26 tests. The [lowering fixtures](../guarded-mlp-indexed-atomic-use-lookup-lowering-v1/README.md)
pass 19 tests after repairing a production adapter's stale schema literal;
the failed first attempt is retained. Actual tool inspection and guarded
lowering remain separate gates. No HSACO, GPU, model numerical or performance
result is claimed by this CPU qualification.
All issue #42 milestones and the 700 tokens/s target remain open.
