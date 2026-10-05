# Shared-Full AR4 Observer CPU Qualification

Authored source-only successor of the executed observer controller
`12898f6b...`. The author has not imported, tested, built or executed this
proposal. Root owns review, policy tests, storage admission and execution.
The source proposal is frozen; this controller remains subject to peer review.

## Exact Inputs

No new archive is required. Copy the existing qualified paired source at
`E/projection-ar4-host-observation-cpu-v228-v1/sources/{ferric,fe2o3}` into a
fresh task directory, never modify that original tree or any previous target.
The baseline completion is 488,734 bytes, SHA
`7d8c08eeffab9cbebbf6abc973c0ba93d80616ad81063b35587e74d04df1d59c`.
Its 6,999-member source map is 1,269,472 bytes, SHA
`1d173129c684afe5bcdc009ec939f8e4f0371bad8a4edecdf62b4548cb04f920`.
All 312 original raw records and three selected production ELF bodies must
still exist at their original paths and hash-match before and after the run.

Apply only `E/p228-projection-ar4-shared-host-v1/source-manifest.json`, SHA
`faaece5ab1afea85b7b6f03b02d772847889565a0ae70922180d6f62a1770461`:
14 exact bodies, ten preimage-checked replacements and four additions.
The complete copied roster becomes 7,003 files. Rustfmt and its check may
change only the thirteen selected Rust bodies; Cargo.toml permits only the
new `tp-batch-engineering` binary declaration. The complete relocated Cargo
metadata/dependency graphs must match the baseline except for that one
parent target. No fe2o3, provider, compiler, KFD, driver or image change is
part of this proposal.

The exact selector is
`--engineering-native-projection-residual-decode-shared-host-v1` and the
new parent is
`ferric-qwen3-finite-projection-residual-decode-shared-host-engineering`.
The source explicitly selects `[false, false, true]`: fresh shared-full
currentness, not unchanged group-fence policy. Operational currentness and
admission caching remain disabled. Configuration duration is inclusive host
wall nanoseconds recorded before observer enable, outside snapshot intervals.
Old plain and default-full observer modes and their tests remain selected.

Existing pinned helpers remain unchanged:

- `E/p228-host-policy-cpu-v1/run.py`, `35abf1ed...`.
- `E/run_clean_worker_p228_v1.py`, `2f3ef5c8...`, source/pin helpers only.
- `R/evidence/wave-output-lowering-v216/bounded.py`, `e634e1b3...`.
- `E/p228-projection-residual-runtime-cpu-v1/run.py`, `6fc90b8c...`,
  configuration and old-target inventory helpers only.
- Actual observer controller, raw recipes, toolchain identities and external
  Cargo dependencies. Worker nightly and parent 1.97.1 stay separate.

Here `R=/home/harmenon/ferric-asrock-42` and
`E=R/evidence/finite-resident-integration-v220`.

## Test Scope

Keep all 61 actual baseline phases; add only the new binary list/test pair.
The existing parent report selector includes the eight new shared data
methods without duplicating the old report phase. Parent-client selection
includes the five new parent methods. No test is renamed or removed.

| Newly Executed Scope | Baseline Passes | Additions | Required Total |
| --- | ---: | ---: | ---: |
| Worker library and shared_wire | 518 | 14 | 532 |
| Parent selected library and binaries | 337 | 14 | 351 |
| Total | 855 | 28 | 883 |

These are required censuses, not observed outcomes. The worker's summaries
remain separate: library 519 passed / 4 ignored, then unchanged shared_wire
13 passed / 0 ignored. All four historical ignored identities must match.
The full worker and parent library inventories must equal actual prior names
union the proposal's exact additions. All seventeen prior parent binary test
harnesses remain selected, plus the new binary. Eight source tests compile
in both crates; twenty unique authored tests yield twenty-eight executions.
The historical runtime's 208 passes are provenance only, not rerun.

Cargo must report four production executables: worker, plain projection
decode parent, default-full observer parent, and new shared-full observer
parent. On complete success there are 63 phases and 322 raw records:
315 phase bodies plus seven source/configuration/old-target snapshots.

## Bounds And Invocation

Unchanged bounded lifecycle: ASROCK UID9661, CPU8/9, nice10, hidden GPUs,
40 GiB initial / 38 GiB ongoing free-space floors, aggregate fresh target
cap6 GiB, per-leaf address space12 GiB, 64 MiB streams and the exact original
per-command deadlines. Every leaf must exit naturally with an absent owned
process group. Source, configuration, dependencies, original executables and
old target inventories are checked afterward. Old targets include both
CPU1037 and CPU855, RPO and indexed compiler targets. No cleanup or other
build may run concurrently; storage reclamation is separate root work.

Root first runs fifteen synthetic policy tests under the established bounded
CPU-only invocation: `python3 -B -m unittest -v test_run`. They do not build,
copy a source tree, or execute GPU code. Then root invokes the reviewed body:

```text
taskset -c 8,9 nice -n 10 /usr/bin/python3 -B \
  E/p228-projection-ar4-shared-host-cpu-v1/run.py ACTUAL_CONTROLLER_SHA \
  projection-ar4-shared-host-cpu-v228-v1
```

The output label must be fresh. Result schema:
`ferric-p228-projection-ar4-shared-host-cpu-result-v1`. Actual receipts retain
named outcomes, source maps and Cargo artifact identities. This does not
qualify native execution, host latency, numerical acceptance, performance,
the full historical CPU1037 cohort or the compiler. New executable hashes
and any GPU admission remain future actual evidence, never inferred here.
