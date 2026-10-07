# Shared Test Repair Qualification

Fresh CPU qualification on `ssh mi350` passed all 27 phases in 173.070 seconds.
There were 1,180 runtime passes and 820 worker passes, with the eight/four
existing ignored tests unchanged. All prior named outcomes, fifteen scoped-tail
tests, ten facade doctests and eight parser regressions were preserved.
Eleven selected executables and 142 original raw bodies were authenticated.
Every owned phase exited naturally and was reaped; postchecks were clean.

The only Rust change replaces a worker-specific module reference in a shared
test backend with an explicit modeled post-tail validation error. All nine
test names and assertions are unchanged. Runtime and worker production code
are byte-identical to the earlier qualification. The repaired actual postimage
is integrated, with [full canonical source checks](../integration-v2/postcheck.json)
covering 825 runtime and 1,288 Ferric bodies.

The [original terminal](evidence/complete.json) is 2,448,697 bytes, SHA-256
`3c4fe3a6c4942b20ec61fd7ea5c3409526e2167c43d24a62fb8660c34b0e713e`.
The source map is 424,416 bytes, SHA-256
`5de4929b2832eda55a53f99945fc4b64a9fbda28b4d4f8bef07f2e8d5e000d6a`.
The evidence archive is 1,773,668 bytes, SHA-256
`098028e277430fe37d606b04013e9bf7e0876b582b9653ca26fe7d01f83bb36c`.
All 185 original members are retained; the manifest pins the other 184 bodies.
The exporter checked live sources, dependencies, tools and products before
export. Local retention does not claim to rehash remote executable bodies.

The [original parent failure](../parent-cpu-failed-v1/README.md) remains retained
and failed. A fresh parent qualification and same-binary GPU comparison remain
required. This CPU result grants no native, numerical, Full2303 or performance
acceptance. Storage floors, resource limits and deadlines were unchanged.
