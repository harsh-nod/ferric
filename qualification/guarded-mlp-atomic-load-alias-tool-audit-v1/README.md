# Atomic Load Alias Tool Audit

Status: **14 actual binary-loader checks and 34 controller fixtures pass on MI350**.
This is distinct from the passing
[compiler CPU qualification](../guarded-mlp-atomic-load-alias-v1/README.md).

The [actual fixture receipt](controller-tests-v1/attempt-v1/evidence/complete.json)
records 34 passing synthetic admission tests, zero failures or skips, in
109.830091 seconds. All 26 historical cases remain; eight new cases exercise
the current alias-refinement generation. The test process exits naturally,
is reaped and leaves no process group. Source and tool postchecks pass under
the unchanged 180-second whole-run and 120-second leaf limits.

The new coverage checks exact source overlays, semantic and inherited flags,
dynamic kernel-IR census, descriptor rejection controls, final Cargo products,
unchanged tools, process cleanup, and pending or mismatched evidence. Historical
DAG admission remains separate and unchanged. The new input contract has 24
keys and preserves the historical tool manifest independently from new products.
Synthetic fixtures are not a binary-loader audit or a kernel compile.

The [retention manifest](controller-tests-v1/attempt-v1/retention-manifest.json)
pins 18 bodies in a 19-member capsule, including seven raw files, totaling
526,209 expanded bytes. The receipt is 17,418 bytes, SHA-256
`d1305b82129c417065ca55ce43af30b0ab7aa96c017e8668a9939be6c9bdda5c`.
The 88,505-byte archive has SHA-256
`8193e9a622dcc899abc29db75c5f159dd9b6e706e80bad0ae6795c0f5f896c7f`.

## Actual Binary Audit

The [actual audit receipt](attempt-v1/evidence/complete.json) records all fourteen
`readelf` and `ldd` checks passing in 2.597166 seconds. All leaves exit naturally,
are reaped and leave no process group; source/input and tool postchecks are clean.
The seven-tool deployment takes its backend and extractor from the new
qualification's final Cargo products. The other five tools are unchanged.

The current backend is 206,535,952 bytes, SHA-256
`8487aca6d2c1f06454fe4c24e3e26c5f77c59cba2f601d91c3dbdcd950ffd261`.
The actual new-run extractor has the same bytes as its predecessor, SHA-256
`13d20c7f4372f873a871b640316cdfa7e720345d3a841d0ac13cfb2a120931a2`;
its current provenance is checked independently, not inferred from that match.
The backend ELF and rlib share one final Cargo record. The rlib is metadata-only
here: it is neither deployed nor passed to `readelf` or `ldd`.

The [retention manifest](attempt-v1/retention-manifest.json) pins 108 bodies in
a 109-member selected capsule, including 72 raw files, totaling 32,466,528
expanded bytes. The original receipt is 278,350 bytes, SHA-256
`3106dceccee940992967c38955fd7f5f3eaf91d10964080206f94ae830963d10`.
The 5,334,768-byte archive has SHA-256
`41b35bd46de28c2c3a8ef509084827b4b51417be49b213c1affc4d99ec94b677`.
No tool binary bodies are included. Current alias-generation evidence and the
historical DAG producer and tool manifest remain separately authenticated.

The [subsequent checked guarded lowering passes](../guarded-mlp-atomic-load-alias-lowering-v1/README.md).
This audit itself invokes no
compiler or GPU and grants no production, load or launch authority. No
model-numerical, performance or issue #42 milestone acceptance is claimed.
