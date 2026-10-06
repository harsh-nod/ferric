# Guarded MLP Shared-Full Currentness

This engineering opt-in shares a freshly read topology within each group
currentness fence. It still checks every device's mutable state and queue
identity. It does not cache topology across fences or remove validation.
Runtime implementation, kernel images, ownership and shutdown rules are
unchanged. The existing conservative mode remains the default.

The new worker flag is
`--engineering-native-guarded-mlp-host-shared-currentness-v1`. The parent
selector is `--observe-guarded-host-shared-currentness`. These select a
distinct report schema; conservative and shared-policy reports cannot be
interchanged. Capture and raw timestamp modes remain excluded.

## Qualified Worker And Checker

The [MI350 worker result](worker-cpu-v1/evidence/complete.json) passes all
607 selected tests with four unchanged ignores, across nine clean phases.
All 611 test names were listed. Six new regressions cover mode separation,
one-time pre-allocation configuration, identity/counter validation and failure
handling. The six integrated changes are actual formatted postimages; all
184 canonical worker source bodies match the qualified source map.

- Original terminal: 1,649,375 bytes, SHA-256
  `0e7f73d69fd2d3efef901f62ee90fac54cfdddbae60661314d222f25aa2ae82a`.
- Worker ELF: 5,881,752 bytes, SHA-256
  `436c16df8cafeefcf291aa2b1c5abe0e488b1bc823d26df781413476e899d55d`.
- Retention archive: 1,769,444 bytes, SHA-256
  `4ecbb598b44dd1e9620cc9a473a57e1d353a8228061cf8631233de3c117edb71`.
- Capsule: 259 members, 258 pins, 50 original raw bodies and 11,712,974
  expanded bytes. External dependency/runtime/cache/ELF bodies were rehashed
  remotely, not all bundled. The bound exporter records the actual deployed
  cache stager as `host-shared-stage-worker-cache-v1.py`.

The [MI350 report checker](checker-cpu-v1/evidence/complete.json) passes
all 15 synthetic tests: eleven host-report tests and four topology tests.
The sole bounded CPU child exited naturally, was reaped and had no remaining
process group. Six unchanged source inputs, seven raw files and the original
terminal are retained. The terminal is 11,402 bytes, SHA-256
`9e35d4b0631661fd7731353b7991c36c3bfb98172b69ab29d582f4a39e4dd717`.

## Qualified Parent

The [MI350 parent result](parent-cpu-v1/evidence/complete.json) passes all
55 phases and 391 selected tests across 47 scopes, with five built host
binaries. Its full 871-name library inventory was listed, not fully run.
The two integrated parent changes are the actual qualified postimages.
All 1,222 canonical Ferric source bodies join the composed parent and worker
qualification maps; authored worker copies from the parent capsule did not
replace the separately qualified worker postimages.

- Original terminal: 3,858,912 bytes, SHA-256
  `96b7c52c2c5f14436d3a4d7aecfd7c864dba6c3c62321a3db60e41565c169c3d`.
- Parent ELF: 13,856,232 bytes, SHA-256
  `d4730670f98140f30b3194c314ee515542f9886093561c9d19bc67979a58562d`.
- Retention archive: 2,860,779 bytes, SHA-256
  `03c68a6a374453dc5c03f49d9d0066d5157be769079c2e9b8e33db778ab1e782`.
- Capsule: 403 members, 402 pins, 279 original raw bodies and 23,547,252
  expanded bytes. All selected process groups exited naturally and were
  reaped; source, dependency, cache and tool postchecks are clean.

## Remaining Gates

A fresh four-forward GPU comparison is pending.
The [conservative observation](../guarded-mlp-model-host-observation-v1/README.md)
is the retained timing baseline. Any future comparison must first join all
four complete payloads and genuine autoregressive histories. Inclusive host
counters are not GPU duration, overlap or sustained tokens/s. Full-model
numerical acceptance, the 2,048/256 benchmark and all issue #42 milestones
remain open.
