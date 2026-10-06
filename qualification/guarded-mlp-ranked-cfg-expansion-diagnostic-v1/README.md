# Projected CFG Expansion Diagnostic

The [qualified borrow-lookup compiler's guarded attempt](../guarded-mlp-indexed-atomic-borrow-lookup-lowering-v1/README.md)
failed with a generic ranked block-limit message. Qualified-source inspection
locates that message at the projected CFG expansion gate, but the raw failure
does not identify the failing root or the expanded count.

This [diagnostic change](proposal-v1/README.md) reuses the existing bounded
CFG error record to report both the computed projected count and declared
semantic count, with the existing function and root context. It preserves
the strict 2,048-block check, counting arithmetic, work charges, earlier
refusals and successful paths. The two earlier CFG diagnostic formats remain
unchanged. It does not raise any capacity or change kernel arithmetic.

The patch replaces four source files and adds three regression fixtures:
the exact and next projected-block boundaries, multi-switch expansion and
function identity, and bounded escaped root/export formatting. All existing
tests remain. Source review and patch/hash checks have passed.

Manifest: `1fe62f3e0f9a892f211cd9b0f75db9eac93d4cfec7c320b46ef55e17a7330de4`.
Patch: `0150812f662f0cc7d9b9adcdc0f5280c28b386c8c38d0f99327bbe1d312b3202`.

## MI350 Qualification

The [fresh qualification](attempt-v1) passed all 42 phases across 29 test
scopes on `mi350` in 665.234699 seconds.

| Executed Scope | Passed | Ignored |
| --- | ---: | ---: |
| Complete compiler library | 1,305 | 24 |
| Complete Pliron library | 1,507 | 1 |
| Focused repeats and extraction controls | 124 | 0 |
| Total test executions | 2,936 | 25 |

Totals include repeated executions, not only unique tests. All children
exited naturally with status zero, were reaped and left no process group.
Sources and dependencies stayed unchanged; postchecks were clean, with no
timeout or forced cleanup. Historical ignores remain explicitly recorded.
The source map has 5,803 source files and two harness files; the four-file
overlay matches the reviewed proposal. The final seven Cargo product records
come from the recorded builds, not an inferred preceding generation.

The retained archive contains 237 members, 236 pinned bodies and 214 raw
files totaling 34,266,888 body bytes. Receipt SHA-256:
`bcf85b67b2b2d2a80f9b2e4a0fa990b038d848eafc0d8117d321dbb059efdf81`.
The 5,572,642-byte archive SHA-256 is
`68360e331f43dc4d8260335897db9447dc7490f6071c3f1f1e96d0783848445d`.

The [loader fixtures](../guarded-mlp-ranked-cfg-expansion-diagnostic-tool-audit-v1/README.md)
pass 26 tests; the [lowering fixtures](../guarded-mlp-ranked-cfg-expansion-diagnostic-lowering-v1/README.md)
pass 19, and all 14 actual binary-loader checks pass. The subsequent guarded
compile fails with a structured diagnosis: the MLP state guard has 2,778
projected blocks against the unchanged 2,048 limit, from 1,121 semantic
blocks. The linked lowering checkpoint retains the complete failed attempt.
This diagnostic-only qualification does not change admission or grant HSACO, GPU, independent
model numerical or performance acceptance. All issue #42 milestones and
the 700 tokens/s target remain open.
