# Guarded Full2303 Request

The explicit full-request worker and parent client are CPU-qualified on MI350.
No full-request GPU execution or numerical acceptance is claimed by this
evidence.

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

## Remaining Gates

Measured launch feasibility must precede a full GPU run. Actual completion must
then pass the independent generated-ID and decoded-byte comparison described
in the [decode-gate inventory](../guarded-mlp-readiness40-causal-layer0-v1/DECODE-GATES.md).
Sustained BF16 target-only performance on the requested 2,048/256 workload is
still unmeasured for this route. These CPU results do not close M0-M7, establish
700 tokens/s, or qualify a community performance demo.
