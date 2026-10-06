# Atomic Load Alias Tool Audit

Status: **34 controller fixtures pass on MI350; actual binary audit pending**.
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

The next gate is the fresh fourteen-check audit of the actual final compiler
backend and extractor, retaining the other five tools unchanged. No GPU,
model-numerical, performance or issue #42 milestone acceptance is claimed.
