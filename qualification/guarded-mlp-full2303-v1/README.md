# Guarded Full2303 Request

The explicit full-request worker is CPU-qualified on MI350. The parent client
is under qualification. No full-request GPU execution or numerical acceptance
is claimed by this evidence.

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

## Remaining Gates

Parent compilation and regression tests must join this exact worker source.
Measured launch feasibility must precede a full GPU run. Actual completion must
then pass the independent generated-ID and decoded-byte comparison described
in the [decode-gate inventory](../guarded-mlp-readiness40-causal-layer0-v1/DECODE-GATES.md).
Sustained BF16 target-only performance on the requested 2,048/256 workload is
still unmeasured for this route. This CPU result does not close M0-M7, establish
700 tokens/s, or qualify a community performance demo.
