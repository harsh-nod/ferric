# Ordered Segment GPU Comparison: Harness Qualification

The V3 comparison supervisor passed 174 named synthetic policy tests on
MI350, with no failures, errors or skips. Separate deployment-selector and
input-assembler suites passed 16 and 13 tests respectively. These Python
tests did not execute a native worker or launch a GPU kernel.

## Actual Evidence

| Scope | Passed | Evidence |
| --- | ---: | --- |
| Frozen V3 supervisor and comparison validators | 174 | [Completion](pure/complete.json), [named transcript](pure/tests.log) |
| Runtime deployment selector V3 | 16 | [Primary tool observation](primary/adapter-tests.json) |
| Input assembler V3 | 13 | [Primary tool observation](primary/adapter-tests.json) |

All 33 supervisor source bodies were unchanged before and after the run.
The adapter observation records successful tool calls and unchanged source
hashes; it is a primary-agent summary, not a supervisor receipt or a retained
full named transcript. The [publication ledger](result.json) authenticates
55 copied source/evidence bodies. No executable, model or tensor bodies are
included, and publication did not import or rerun the tested modules.

The supervisor's actual completion SHA256 is
`000fb1158483b2a0255d85743da2c9c35e59fd2e6ea5d5c7a3dbc54401dfa4d5`.
The frozen V3 manifest SHA256 is
`054b11e3eae9389dad6dc42f216b2ccfc81115436340cab572fda7f93d8d4d69`.

## Comparison Contract

The planned pair runs shared-full and ordered routes sequentially from the
same newly qualified CPU build. It requires identical model, kernel images,
prompt, device identities and actual autoregressive input histories. Each
arm permits one native attempt with no retry, checks all terminal states,
requires naturally exited/reaped children and exact-empty MI350 GPU process
audits. The ASROCK monitoring exception is not admitted on MI350.

The ordered validator handles the distinct 241,096-byte control format and
144 terminal states per forward. The four 606,976-byte captured payloads
must be read and compared as actual bytes; matching hashes or empty reader
return values do not substitute for payload comparison. Equality establishes
cross-route repeatability only, not independent model accuracy.

The pair preserves the existing process, storage and time bounds. Results
must separate setup/configuration/Close from forward wall time. Ordered
segment waits overlap across ranks and do not provide individual kernel
timings. First-use scratch allocation, fixed run order and warmed caches
remain confounders; this is not a statistically established speedup or
sustained tokens/s measurement.

## Preserved Failure

The preceding V2 suite ran 174 tests but failed with one error: a missing
linked handoff artifact was indexed before its roster was checked, raising
`KeyError` rather than the expected validation refusal. The other 173
methods reported success. Its [failed receipt](failed-v2/pure/failed.json),
[transcript](failed-v2/pure/tests.log) and affected frozen source bodies
remain published as failure evidence.

V3 validates the exact six-artifact dictionary before dereferencing it.
The regression now deterministically checks every missing artifact as well
as malformed containers. The test-method count remains 174. The failed V2
outcome is not relabeled and the earlier package is not overwritten.

## Remaining Gates

The [separate Rust CPU qualification](../projection-ordered-segment-cpu-v1/README.md)
passed 1,848 test executions with seven historical ignores. That does not
replace fresh executable audits, reviewed request assembly or actual GPU
execution. Use the published V3 deployment and assembly adapters, not the
earlier V2 selectors.

The new ordered GPU pair has not completed at this checkpoint. Independent
numerical acceptance, sustained Qwen3-8B BF16 target-only single-request
2,048/256 decoding, 700 tokens/s and all issue #42 milestones remain open.
