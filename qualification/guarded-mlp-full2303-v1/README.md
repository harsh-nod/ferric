# Guarded Full2303 Request

The explicit full-request worker and parent client are CPU-qualified on MI350.
An independent BF16 framework reference also completed two full 2,048/256
passes on MI350. Full-request Ferric GPU execution and numerical acceptance
remain open.

## Scope

The separate worker selector is
`--engineering-native-guarded-mlp-full2303-v1`. It consumes the authenticated
2,048-token prompt and performs exactly 2,303 forwards through all 36 layers.
The output at position 2,047 is generated token zero; each subsequent input is
the preceding committed output, producing exactly 256 generated tokens.

The native owner, worker transport and parent reconstruct their own histories.
Successful Close requires the completed forward count, matching transcript
digest, matching 256-token history, retired reusable banks, unchanged allocation
census and healthy child termination. Early EOF, an altered history, publication
failure or cancellation cannot produce a successful Close.

This route reuses the existing pure long wire and sequence implementation. It
does not select paired-terminal optimization or extra causal tensor captures.
Readiness40, Position5, causal and AR4 selectors retain their existing behavior.
The separately named one-hour Full abort bound is not evidence that this
workload can finish within it. GPU launch feasibility remains a separate gate.

## Worker CPU Result

The actual MI350 run completed all nine phases in 84.17 seconds. All 694 tests
passed, none failed, and four existing tests remain ignored. The complete
inventory contains 698 names. All 680 prior passing outcomes are preserved;
13 new library tests and one new executable regression passed.

| Target | Passed | Ignored |
| --- | ---: | ---: |
| Worker library | 674 | 4 |
| Worker binary tests | 0 | 0 |
| Executable CLI regressions | 4 | 0 |
| Shared wire | 16 | 0 |
| Total | 694 | 4 |

All source, tool, private-cache, dependency and product postchecks passed.
Every owned phase exited naturally and was reaped. The executable exercised
by the real CLI regressions equals the final built worker. These are CPU tests,
including synthetic full-length streams, not generated model output.

The source map contains 815 unchanged runtime bodies, 203 worker bodies and
two harness bodies. Thirteen worker files were overlaid: seven replacements
and six additions. Eleven have observed rustfmt changes. Canonical integration
uses the actual tested postimages and joins all 203 worker files; it does not
substitute the unformatted source proposal.

`worker-cpu-v1` retains 81 original archive members and a separate retention
record. `worker-source-integration.json` is the original integration plan;
its `patch_applied=false` records the plan's creation, not current repository
state. All planned postimages have subsequently been applied and rehashed.

| Artifact | SHA-256 |
| --- | --- |
| CPU terminal | `0b91b220824f093b0297134ff21e4ddc2dde64fd17909439bb15421fb862e55f` |
| Tested source map | `0fc0a33c61dd0e6f70903bb246654bfeb99095fd515be53c828d8d91f646e7cb` |
| Worker ELF | `4e95b834aba1441b4b6a48085eb1f4fc28f9ee4a3fd04dff04d45c7de82c27f0` |
| Retained archive | `aaf720a12d17b36e63921dc94e04aa915ed19d94f9485b098538dfb366c40403` |

## Parent CPU Result

The actual MI350 parent run completed all 63 phases in 405.70 seconds. All 455
selected tests passed across 53 scopes, with no failures or ignored selections.
All 444 prior selected outcomes are preserved. The eleven additions cover six
parent behaviors, four imported Full wire contracts and the new binary selector.
The full library inventory contains 928 names; this is not a claim that all
928 were selected for execution.

The seven Cargo-selected binaries include the six existing parent products and
`ferric-qwen3-guarded-mlp-full2303-engineering`. Both the existing readiness and
new Full features were enabled. The locked dependency graph is unchanged after
accounting for the new feature and binary. Every phase exited naturally and was
reaped, with clean source, dependency, private-cache, tool and product postchecks.

The new parent entry authenticates the full prompt, uses 144 physical cache
pages, retains all 2,303 compact records and four full captures, independently
reconstructs the 256-token history, and retains decoded output. Its aggregate
evidence limits and the existing generated-output acceptance requirements are
not relaxed. CPU fixtures do not constitute model execution.

The source map contains 1,251 Ferric bodies and three harness bodies. All 203
worker bodies are identical to the separately qualified worker, before and after
parent compilation. Five of the six permitted parent Rust paths changed under
rustfmt. Integration uses the actual tested postimages for all seven parent
files: three replacements and four additions.

`parent-cpu-v1` retains 451 original archive members, including 319 raw bodies,
and a separate retention record. Its executed V2 evidence collector corrects a
formatter-list ordering assumption in V1: duplicate-free exact names are compared
after sorting. The original CPU receipt and source are unchanged. The initial
V1 export refused before archive creation; no test was rerun or relabeled.
`parent-source-integration.json` preserves the original integration plan, whose
`patch_applied=false` describes plan creation; its postimages were subsequently
applied and rehashed.

| Artifact | SHA-256 |
| --- | --- |
| Parent CPU terminal | `67949bbd5a662efc4d56a65df8b9988dbaf56cbd10b0e006981bed7f4fffbf13` |
| Tested parent source map | `2d6613ee1d0267416adcc496d7806e9fdad9317b641a96e900d4b04a5a697df4` |
| Full parent ELF | `1110ecc3ee4c5ff511b0a87170e4a7ce46dcd448f0f3003b548a4fae261eb7a5` |
| Retained parent archive | `f1d7cd2add4ead1cadcb428f366d99a6a9ed990a20bd72d9e43e00e8d45e0338` |

## Full-Request Reference

The independent reference completed on `mi350` with 20 passing pure checks,
17 naturally retired owned command phases, six idle-device observations and
clean source, package, tokenizer and topology postchecks. The private offline
container was removed after exit. Its owner took 78.465 seconds, including
setup and audits; this is not a decode throughput benchmark.

Each of two fresh passes used one actual 2,048-token matrix prefill followed
by 255 one-token calls driven by its own preceding greedy choices. This yields
256 outputs per pass and processes 2,303 positions. The final output is not fed
back. Across both passes there were 512 model calls and 512 generated choices.
No Ferric hidden states, logits, KV cache or generated tokens were supplied.

Both 256-token vectors and decoded byte strings repeated exactly. The reference
uses BF16, deterministic math-only SDPA, FP32 rotary values, no autocast, and
disabled reduced-precision matmul/SDPA reductions. Equal BF16 maxima select the
lowest token ID; EOS does not truncate the requested length.

| Output per pass | Bytes | SHA-256 |
| --- | ---: | --- |
| 256 token IDs, u32 little-endian | 1,024 | `198ad03410c23eeff68c503c721eb3b4235b63d568ecbac0ed2b5728e9bc5221` |
| Decoded bytes | 1,405 | `383ac0bbd750d53433fd263f5a1d3792721bb3d98b757f94f01d00d4ab974909` |

`reference-v1` retains 131 original archive members plus a local retention
record: all source/input files, 85 command records, eight full payloads at
positions 0/2,047/2,048/2,302, compact per-choice records, both output vectors
and both skip-special/preserve-special decoded representations. Position zero
is a diagnostic prefill slice, not a generated token. All 36 actual KV caches
were checked for shape, dtype, device and length at every call.

The exporter and local retainer rechecked both own-token histories, exact repeat,
selected raw payloads, argmaxes and evidence identity. Unselected raw logits,
model shards, package bulk and the 11.4 MB tokenizer are not retained. Their
execution/posthash evidence is authenticated metadata, not local recomputation.
The V1 exporter refused before archive creation because it expected a nonexistent
tokenizer `path` field. V2 checks the producer's actual `stat`/`bytes`/`sha256`
record. Original receipts are unchanged; the model was not rerun for retention.

| Artifact | SHA-256 |
| --- | --- |
| Reference owner terminal | `e862ba7fc32df915c22867063f5cba48e9f776fbf36d2639cba786043bfe685d` |
| Framework terminal | `876678fa2815a109546c2ff01db33fe80c2da66d9d00e0d8bb53e108ed2ed006` |
| Retained archive | `85bf81ee0bac1f743a3d74eae3ae84c8b83397ff0c8e4a81da78b136b65c8aaa` |
| Executed V2 collector | `874dce584128bab37aa63f64abaef9cdebf270e4d0878ed618439b8b0269a71a` |

Reference repeatability does not establish Ferric correctness or a speedup.

## Remaining Gates

Measured launch feasibility must precede a full GPU run. Actual completion must
then pass the independent generated-ID and decoded-byte comparison described
in the [decode-gate inventory](../guarded-mlp-readiness40-causal-layer0-v1/DECODE-GATES.md).
Sustained BF16 target-only performance on the requested 2,048/256 workload is
still unmeasured for this route. These CPU results do not close M0-M7, establish
700 tokens/s, or qualify a community performance demo.
