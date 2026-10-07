# Causal Layer-Zero Diagnostic

The worker, corrected parent source and independent data checker are CPU-qualified on MI350.
Native causal capture and its authenticated cross-framework comparison have also completed.
The native argmax agreement remains 39/40. This is diagnostic instrumentation,
not numerical acceptance or a performance route.

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
passed the entire selected parent qualification before integration, as recorded
below. The failed V1 evidence remains unchanged.

`checker-cpu-v1` retains the original MI350 data-checker evidence. All 29 tests
passed, including the 12 new causal capture/parity tests. The owned test process
exited naturally and all source/tool postchecks passed. These are synthetic
data tests, not native execution or numerical acceptance.

| Artifact | SHA-256 |
| --- | --- |
| Failed parent terminal | `060c05edc7ec35703e54c8120fa630553fc7c263205e2b45fdddb0c03a69225b` |
| Checker terminal | `99054dc3eaf5d7455bf6cf1c9eb8b82db947f1ba27194f5b03f4238684faec27` |

## Corrected Parent Result

`parent-cpu-attempt-v2` retains 436 original archive members and a separate
retention record. The MI350 retry completed all 60 phases in 402.10 seconds:
444 selected tests passed, with zero failures or ignored selections. All 438
prior selected tests and six new tests passed. The library inventory has 918
names; this run does not claim execution of the entire library suite.

All six selected Cargo products, the default-feature check and postchecks
passed. The original failed V1 terminal is preserved as history, not a successful
baseline. The corrected file-reading regression exercises the same file-backed
validator used by the parent after Close.

Canonical integration uses the five actual tested parent postimages, including
two new files. `parent-source-integration-v2.json` records the complete 1,241-file
Ferric composition with the separately qualified 197 worker files. The worker's
eight formatting-only transitions between the parent and worker compilations
are explicit; the two source trees are not described as byte-identical.

| Artifact | SHA-256 |
| --- | --- |
| Parent V2 terminal | `f2c161753e64f9f2b8b4049ae75cac13332b7eb14217eaad247c8c2e9e13d4ea` |
| Parent V2 source map | `54b516e6cd3d95973c725923d69d1fd7b9e6fa4ff7b3d4f73272282ed3ba2ae2` |
| Readiness parent ELF | `a8ada7572717e931bd9149bcab6d043780d3e8513fb5cd3282d8f3c36092b8aa` |

## Independent Reference Result

`reference-v1` retains 143 original archive members plus a separate local
retention record. On MI350, the independent framework completed two fresh
40-forward passes through all 36 Qwen3-8B layers, with zero generated tokens.
The owner completed in 55.22 seconds, including its test, container and idle
checks; this duration is not decode throughput. Its 20 existing pure tests and
seven new observer tests passed. The container exited and was removed cleanly.

Each pass captures 33 layer-zero stages at positions 0..5 in its actual growing
KV cache. All captured stage bytes repeat across the two passes. Instrumentation
also preserves all 80 historical reference records and all eight selected full
payloads at positions 0, 5, 16 and 39. Export and local retention independently
rechecked those bytes, producer/consumer joins and unchanged cache prefixes.
No native intermediates or historical payloads were supplied as model inputs.

| Artifact | SHA-256 |
| --- | --- |
| Reference owner terminal | `2d4687d2fae7045a572054c5f46018c859e62a30f5b63c08a7f6060adbd7046c` |
| Reference inner terminal | `5ebd12d258fc9c76190b4ec4c7270d9732d15e635c7944d966bd50ffc81fa027` |
| Retained archive | `aab3837ab2d3d7fcff0d69424246272f3969c1f0a827a2bf853f770bc2d204bf` |

This establishes repeatability and unchanged reference behavior, not agreement
with Ferric. The separate native and comparison results follow below.

## Comparison Tests And Launch Preflight

`comparison-cpu-v1` contains the 14 original files from the MI350 comparison
test run. All ten named tests passed, covering rank slicing, KV cache layout,
finite encodings, propagated differences and deliberately noncomparable
projection partials. The process exited naturally and all postchecks passed.
Its terminal is `fb9c982ac824eab32f8b1d35f9ee5ab54eb3eaba5feda38e7efb13aa8dc7e981`;
the original archive is 28,080 bytes with SHA-256
`4d0eff0a66f7d43cf2e6b2e1bbb2460f167bb7cea8b78c6633cfe2242e937bd4`.
These are synthetic comparison tests, not results from actual native captures.

`native-preflight-v2` preserves ten original deployment/preparation files from
an unsuccessful launcher invocation. It exited with
`RuntimeError: actual readiness CPU/ELF bindings remain pending`: the root
agent supplied path-bearing pins where this API requires only `bytes` and
`sha256`. Admission refused them before case-directory creation, owned child
launch or GPU work. There is no successful native receipt for this attempt.
The unchanged ten-file archive is 43,361 bytes with SHA-256
`f5ab79a5ae62865f3b0c3d5575206ec857ebda975a291631f900771aff07f5e3`.

The corrected V3 deployment uses a fresh directory, compact CPU/product pins
and a fresh request. Worker/parent binaries, validators, kernel images and
acceptance checks are unchanged.

## Native Capture Result

`gpu-v3` retains all 157 original archive members. The MI350 run completed forty
genuine forwards through all 36 layers, zero generated tokens, healthy Close,
and all eleven owned phases with clean postchecks. All forty semantic records,
logit hashes and observation hashes match the prior Position5 run. All four
606,976-byte ordinary capture payloads also match byte-for-byte.

The six additional snapshots contain 204 parts. Their cache prefixes and the
selected final hidden outputs pass the required joins. Independent retention
and audit rechecked the raw files, parent/worker ownership and admission pins.
The owner took 944.92 seconds with diagnostic readbacks; this is not a decode
throughput measurement.

| Artifact | SHA-256 |
| --- | --- |
| Native V3 terminal | `00aa0447a23f57c6d9bb2ed39cd4e5b2b9da692f15bceb33107ac3822f95516f` |
| Native V3 archive | `6d346693b1024978d9c445d945ee2e01cace16187e5251932ef9ae52f3d06c0f` |

## Cross-Framework Findings

`comparison-v1` retains the six executed source files and original terminal.
The CPU-only comparison on MI350 authenticated the native, repeated reference
and passing comparator-test archives before inspecting their captures. All nine
input posthashes passed. A separate data audit recomputed every metric from the
original captured bytes, without importing the comparator implementation.

Of 204 comparable stage/rank rows, 132 are bit-exact and 72 differ. Of the
differing rows, 23 have identical captured prerequisites and 49 already have
different inputs. Another 36 rows are deliberately noncomparable: native rank
partials are not equivalent to framework materialized full projections.

| Observation at layer zero | Result | Interpretation |
| --- | --- | --- |
| Embeddings, input normalization, V projections and used K/V caches | Exact at all six positions and both ranks | These captured boundaries agree |
| Q/K projections | Five differing scalars across six positions | Differences occur with identical normalized inputs |
| First difference, position 0 / rank 0 / Q row 168 | One BF16 step; absolute difference `4.76837158203125e-7` | Earliest observed difference, not proof of downstream causation |
| Position 0 attention outputs | Exact | Its Q difference cannot explain the position-zero output-projection/residual difference |
| Position 0 first residual, both ranks | Two differing words per rank with identical captured attention and residual inputs | Isolates the output-projection/TP-reduction/residual composite, not one operation within it |
| Rank 1 attention at positions 2, 3, 4, 5 | Respectively 2, 2, 1, 1 differing words with matching captured inputs | Additional independent operation-level differences |
| Final layer-zero hidden at positions 0 through 5 | Respectively 289, 14, 649, 510, 10, 14 differing words per rank | Downstream differences, not isolated operation tests |

The full-model argmax ledger still matches at 39 of 40 positions. At position 5,
Ferric selects token 9112 with BF16 logit 19.75; token 2 has logit 19.625.
The reference ties both at 19.625 and selects token 2. The previous exact-dot
head diagnosis showed that those selected head outputs correctly round their
respective inputs, so this investigation remains upstream of the head.

| Artifact | SHA-256 |
| --- | --- |
| Comparison terminal | `3f94fe36125e3cbbf92e33caca6049b94fd33963956ef1477bf304dcec6bf38f` |
| Seven-member comparison archive | `99d3f7ee62c5d107a6d6b6df193d532eb41685387a4f7793cb0391deff29bb98` |

The comparison's passing status means authenticated inputs and successful
diagnostic processing. It does not mean numerical acceptance. In particular,
the first Q difference has not been shown to cause the position-five argmax
difference, and matching inputs alone do not identify an arithmetic bug.

## Exact Q/K Diagnostic

The subsequent [five-dot diagnostic](qkv-exact-v1/RESULTS.md) passed all 18 tests
on MI350 and rehashed the original 4 GB shard before and after use. Its original
checkpoint rows and both packed-QKV hashes join the current native upload
manifest. All five dots use the same captured normalized inputs on both sides.

Ferric matches once-rounded exact BF16 at the first three differing scalars
(Q rows 168, 3950 and 1462 at positions 0, 1 and 2); the framework matches at
the remaining two (Q row 10 at position 3 and K row 609 at position 5).
None is an exact midpoint tie. This is not an accumulation-order emulator or
a full-projection exactness claim. It rules out treating framework agreement
alone as evidence that a numerical change improves these selected results.

## Position-Zero Residual Diagnosis

The [current O replay join](current-o-replay-join-v1/README.md) reuses the
previously executed exact all-row O-projection reference. Full-byte joins
cover the current attention operands, both FP32 partial arrays, embeddings,
residuals, both framework passes, and current checkpoint/upload commitments.
No new model dot or GPU run was needed. An independent data peer rechecked
the original artifacts, stored exact-sum rounding and residual boundaries.

At the two differing residual rows, 1024 and 3444, the native-derived BF16
projection equals the once-rounded exact dot; the framework projection differs.
Both sides' residuals reproduce their own projection-plus-embedding boundary.
The useful counterexample is row 1356: neither projection is the nearest exact
dot, yet both residuals match. The current and historical kernel images are
different, so this data join does not transfer historical ISA qualification.

These results explain this observed residual boundary without requiring a
missing BF16 materialization or changing outputs to force framework agreement.
They do not establish a causal path to the position-five argmax difference.

## Attention Conditional Check

The [attention gate](attention-conditional-v1/RESULTS.md) passed on MI350 with
eight passing tests and clean postchecks. All 12 native states and all 12
framework states were compared against their own captured Q/K/V using the
unchanged historical dense reference and conditional attention policy.
All 49,152 output words passed; all four position-zero rank/side controls were
exact. This covers the first six positions of layer zero, not the whole model.

After reference narrowing, 24,572 native words match exactly and four are
tolerated; 24,573 framework words match exactly and three are tolerated.
Maximum encoding distances are four and ten steps, respectively. The existing
policy also has an absolute cancellation bound, so these counts do not imply
a failed component check. Separate 80/160-digit calculations agree on all six
selected rounded words but are not rigorous rounding proofs or pass criteria.

## Remaining Gates

Both native and framework instrumentation preserve their own prior histories
and selected payloads. Exact Q/K dots, the position-zero residual join and the
conditional attention check have now been completed. No arithmetic change or
tolerance relaxation follows from these comparisons alone.

The 39/40 argmax result is a prompt-position diagnostic, not a declared 40/40
acceptance threshold. Both runs consume the same authentic prompt tokens and
generate zero tokens; the position-five prediction is not fed to position six.
Independent numerical acceptance remains unestablished, rather than a claimed
failure against a tensor tolerance this diagnostic never declared. Actual
generated-token agreement and full-model numerical acceptance need their own
explicit gates.

The next runtime work is a separately selected guarded Full2303 transport and
native owner for the actual 2,048-prompt/256-generated request. Existing
Readiness40 and four-forward autoregressive entry points remain unchanged.
Source/CPU qualification, safe launch feasibility and actual long-workload
completion are distinct gates; a new profile is not itself a passing GPU run.

Full 2,048-prompt/256-generated megakernel correctness, sustained decode
throughput, the 700 tokens/s target, and issue #42 milestones remain open.
