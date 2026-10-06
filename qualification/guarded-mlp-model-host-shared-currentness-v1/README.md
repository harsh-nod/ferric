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

## Actual GPU Execution And Comparison

The [shared-mode GPU run](gpu-attempt-v1/ar4/complete.json) passed in one
attempt with 11 clean process phases, six idle device snapshots and healthy
Close. All 587 host snapshots and 586 intervals use the required shared
policy. It executes all 36 layers on both ranks for four forwards, producing
`67, 25, 576, 2701` from its own-output history beginning at token `9112`.
All four complete payloads are byte-identical to ordinary guarded AR4 and
to the conservative host-observation run.

- Original terminal: 617,545 bytes, SHA-256
  `d94fd19c98459b82750487467ded4bf0f289f90f8c0dd1140a623d21ac1dc8ce`.
- Retention archive: 3,937,318 bytes, SHA-256
  `0b693a8a792da88a40ec462c9993cf3aa4aca1c5fc454438422c88dfec11b232`.
- Capsule: 97 members, 96 pins, 79 raw records and 7,924,412 expanded bytes.
  The five ordinary-AR4 reference bodies are retained without changing their
  original contents or embedded paths. External dependencies are not bundled.

The [plot, tables and source-level attribution](comparison-v1/README.md)
show observed forward host brackets falling from 12.808-15.384 seconds to
5.598-6.271 seconds. The paired stage matches the predicted count change
exactly: 2016 individual checks per rank become 216 per rank plus 900 group
checks. All 16 comparison tests and 27 input rehashes pass on MI350.

## Remaining Gates

The [conservative observation](../guarded-mlp-model-host-observation-v1/README.md)
is the retained timing baseline. This is one observation per mode, not a
controlled repeated benchmark. Inclusive host counters are not GPU duration,
overlap or sustained tokens/s. Full-model
numerical acceptance, the 2,048/256 benchmark and all issue #42 milestones
remain open.
