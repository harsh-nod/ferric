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
The actual kernel's post-compaction counts have not yet been measured.

The patch replaces two files and adds nine focused regression tests in one
new module. Three inherited layout expectations change; their actual bounds
and race checks remain. Source review and authenticated patch reconstruction
passed. Full CPU qualification has started on `mi350`; no result is claimed
yet. Fresh loader checks and guarded lowering must follow a passing suite.

Manifest: `a996f2ecae0ce3a29e091ecc50f23ef081e57feba005b621856cd55ce6889428`.
Patch: `c00bedb6983ed9fca9565ffd3ee1bb7be330b936d851806ffb174f73322517c7`.

This checkpoint is source-only. It does not establish guarded HSACO output,
GPU execution, model numerics, or performance. Passing one resource gate does
not imply passing the independent edge, fact, alias, or other proof gates.
All issue #42 milestones and the 700 tokens/s target remain open.
