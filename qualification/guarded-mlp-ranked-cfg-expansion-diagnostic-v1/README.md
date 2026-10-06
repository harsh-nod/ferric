# Projected CFG Expansion Diagnostic

The [qualified borrow-lookup compiler's guarded attempt](../guarded-mlp-indexed-atomic-borrow-lookup-lowering-v1/README.md)
failed with a generic ranked block-limit message. Qualified-source inspection
locates that message at the projected CFG expansion gate, but the raw failure
does not identify the failing root or the expanded count.

This [source-only proposal](proposal-v1/README.md) reuses the existing bounded
CFG error record to report both the computed projected count and declared
semantic count, with the existing function and root context. It preserves
the strict 2,048-block check, counting arithmetic, work charges, earlier
refusals and successful paths. The two earlier CFG diagnostic formats remain
unchanged. It does not raise any capacity or change kernel arithmetic.

The patch replaces four source files and adds three regression fixtures:
the exact and next projected-block boundaries, multi-switch expansion and
function identity, and bounded escaped root/export formatting. All existing
tests remain. Source review and patch/hash checks have passed; fresh compiler
tests, actual loader inspection and diagnostic lowering remain pending.

Manifest: `1fe62f3e0f9a892f211cd9b0f75db9eac93d4cfec7c320b46ef55e17a7330de4`.
Patch: `0150812f662f0cc7d9b9adcdc0f5280c28b386c8c38d0f99327bbe1d312b3202`.

No new CPU, HSACO, GPU, independent model numerical or performance outcome is
claimed. All issue #42 milestones and the 700 tokens/s target remain open.
