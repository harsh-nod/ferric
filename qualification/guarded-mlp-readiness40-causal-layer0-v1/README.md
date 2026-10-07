# Causal Layer-Zero Diagnostic

The worker source and independent data checker are CPU-qualified on MI350.
Native causal capture and its cross-framework comparison have not yet run. This is diagnostic
instrumentation, not numerical acceptance or a performance route.

## Scope

The separate worker selector is
`--engineering-native-guarded-mlp-readiness40-causal-layer0-v1`.
It retains the Position5 wire profile: forty genuine prompt forwards through
all 36 layers, zero generated tokens, and ordinary captures at positions
0, 5, 16 and 39. Six additional layer-zero captures observe positions 0..5
in the same live cache. Kernel/image bytes and arithmetic are unchanged.

Each snapshot contains 34 ordered parts, including the actual growing K/V
prefixes. Reads use the validated physical page mapping and dedicated guarded
Down allocations. The complete R1/MLP/R2 segment remains unsplit. Earlier K/V
rows must remain unchanged; both ranks' final hidden bytes must join the
ordinary checked layer output.

The bounded binary sidecar is published only after all forty forwards and
healthy Close. Incomplete captures, changed cache history, failed reads or
failed Close cannot publish a successful diagnostic. This path does not
enable the warm paired-terminal optimization, and its added readbacks make
its timings unsuitable for performance claims.

## Worker CPU Result

`worker-cpu-v1` retains 77 original archive members and a separate local
retention record. The MI350 run completed all nine phases in 76.43 seconds:
680 tests passed, zero failed, and four remained ignored. All seven new tests
passed, including the real executable's explicit-selector EOF regression.
The five Cargo products and all postchecks passed; the executable used by
the CLI tests equals the final worker product.

The qualified source map contains 815 unchanged runtime files, 197 worker
files, and two harness files. Nine worker paths were overlaid, including two
additions; eight had observed rustfmt changes. Canonical integration uses
the actual tested postimages, not the unformatted proposal. The complete
197-file composition is recorded in `worker-source-integration.json`.

| Artifact | SHA-256 |
| --- | --- |
| CPU terminal | `5c4ffff4b06c7ad1d73c6985c6bcb0193b596ab6618bbda44010347a9b26aa1a` |
| Source map | `00b15b5478ebbeb6fbd00a2de68e24bb6381a6f28a521fc566b95539aa3e35a5` |
| Worker ELF | `44be19f70ff77cbb95651667af4038be4c861502774ae47600aa2da08396494e` |

## Parent Attempt And Checker

`parent-cpu-attempt-v1` preserves the original failed qualification, including
all 297 archive members. It stopped naturally in phase 33: the parent-client
scope passed 228 tests and failed one positive causal-sidecar test. Earlier
selected scopes passed 27 tests. This is not a successful parent qualification;
the original `failed.json` and raw test output remain unchanged.

The failure exposed a byte-retention API error: `FilePin::read(..., false)`
authenticates a file but returns an empty buffer. The new parent consumer
needed the authenticated bytes. The reviewed retry changes that argument,
shares the file-backed validator with the post-Close path, and adds a regression
covering valid contents, mutation, a wrong digest, and a missing file. The retry
must pass the entire selected parent qualification before integration.

`checker-cpu-v1` retains the original MI350 data-checker evidence. All 29 tests
passed, including the 12 new causal capture/parity tests. The owned test process
exited naturally and all source/tool postchecks passed. These are synthetic
data tests, not native execution or numerical acceptance.

| Artifact | SHA-256 |
| --- | --- |
| Failed parent terminal | `060c05edc7ec35703e54c8120fa630553fc7c263205e2b45fdddb0c03a69225b` |
| Checker terminal | `99054dc3eaf5d7455bf6cf1c9eb8b82db947f1ba27194f5b03f4238684faec27` |

## Remaining Gates

The parent suite must pass before the native diagnostic. Both native and
framework instrumentation must preserve their
own prior forty-record histories and selected payloads. Cross-framework
stage comparisons then distinguish same-input operation differences from
upstream propagated differences; native FP32 rank partials are not treated
as framework BF16 full projections.

Full 2,048-prompt/256-generated megakernel correctness, sustained decode
throughput, the 700 tokens/s target, and issue #42 milestones remain open.
