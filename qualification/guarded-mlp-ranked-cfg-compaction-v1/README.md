# Guarded CFG Compaction

The [qualified diagnostic compiler](../guarded-mlp-ranked-cfg-expansion-diagnostic-v1/README.md)
reports 2,778 projected blocks for the MLP state guard, exceeding the unchanged
2,048-block limit. The semantic body has 1,121 blocks.

The [reviewed source proposal](proposal-v1/README.md) shares one terminal trap
among eligible guarded accesses and fuses each successful access with its
continuation. Eligibility requires trap-on-failure and no live induction
variables. Bounds predicates, atomic effects and ordering, source identities,
wave synchronization, and all structural and work limits remain unchanged.
For `g > 0` eligible accesses this saves `2*g - 1` blocks and `g` raw edges.
The [actual guarded attempt](../guarded-mlp-ranked-cfg-compaction-lowering-v1/README.md)
now renders 1,675 blocks, down from 2,778, but fails the independent
1,024-block structural-identity limit before emitting an artifact.

The patch replaces two files and adds nine focused regression tests in one
new module. Three inherited layout expectations change; their actual bounds
and race checks remain. Source review and authenticated patch reconstruction
passed. Full CPU qualification on `mi350` also passed, as recorded below.
The 26/19 loader/lowering fixtures and 14 actual loader checks also pass.
Guarded lowering remains a separate, currently failed gate.

Manifest: `a996f2ecae0ce3a29e091ecc50f23ef081e57feba005b621856cd55ce6889428`.
Patch: `c00bedb6983ed9fca9565ffd3ee1bb7be330b936d851806ffb174f73322517c7`.

## MI350 Qualification

The [fresh CPU attempt](attempt-v1) passed all 43 phases across 30 test scopes
in 649.356388 seconds.

| Executed Scope | Passed | Ignored |
| --- | ---: | ---: |
| Complete compiler library | 1,314 | 24 |
| Complete Pliron library | 1,507 | 1 |
| Focused repeats and extraction controls | 133 | 0 |
| Total test executions | 2,954 | 25 |

Totals include repeated executions, not only unique tests. All children exited
naturally with status zero, were reaped, and left no process group. Source and
dependency checks remained unchanged, with no postcheck errors, timeout, or
forced cleanup. The full map contains 5,804 compiler sources and two harness
files; only the reviewed three-row source overlay differs from the baseline.

The archive retains 243 members, 242 pinned bodies, and 219 raw files totaling
34,228,269 body bytes. CPU receipt SHA-256:
`439d4b5bf10a1fe9f44d02fa1cbd28b03eec9601bb161806909b2f073beb9018`.
The 5,587,557-byte archive SHA-256 is
`576a2463ee1fbb67291c25b9dba1fddfe51675bdc889a57fa2f8cc581838028b`.
The pinned prior failure and stderr are included and verified independently
of their historical controller flags. They measure the old graph. The linked
guarded attempt separately retains the compacted graph and its new refusal.

This compiler qualification does not establish guarded HSACO output,
GPU execution, model numerics, or performance. Passing one resource gate does
not imply passing the independent edge, fact, alias, or other proof gates.
All issue #42 milestones and the 700 tokens/s target remain open.
