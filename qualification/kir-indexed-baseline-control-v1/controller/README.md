# Unchanged RPO Lower-Library Control

Source-only bounded observer, with 16 authored synthetic tests and no author execution. This control determines whether the one observed indexed-candidate library assertion also occurs in the unchanged qualified RPO source. It neither suppresses that test nor qualifies a failing library.

## Observed Candidate

Actual candidate failure `88d0fa37...` and owner `b845ba6e...` are immutable prerequisites. The seven attempted phases naturally completed; `lower-tests` exited 101, with 784 passing names and one failing old name out of 785, zero ignored. All twenty declared new indexed tests passed in that full suite. The indexed subset-repeat, finalizer suite and actual retained inert join were not reached.

The old test is `production_semantic_kir_v1::wave_task_entry_parameter_tests::access_roots::retained_fields::retained_nested_enum_referent_scalar_move_invalidates_saved_references`. Its assertion reports actual `(8, Some(7), 5)` versus expected `(7, Some(4), 5)`. Source inspection suggests an RPO/FIFO first-error location difference, but this is not declared preexisting until measured by this independent unchanged-source control.

## Six Phases

Copy all 5,783 original RPO source bodies from `E/rpo-compiler-cpu-v228-v2/source/fe2o3`, exactly authenticated by CPU `56fc51fc...` / source map `77fbdd63...`. Apply no overlay and no formatter. Build in fresh `E/kir-indexed-formal-join-baseline-cpu-v228-v1/target`.

1. Offline locked Cargo metadata, exactly relocated from the original generation.
2. Build only `cargo test --offline --locked --jobs 2 --manifest-path COPY/Cargo.toml -p fe2o3-lower-mir-kernel --lib --no-run --message-format=json`.
3. Actual full library test listing.
4. Actual ignored-test listing.
5. Run the exact old failing name with `--exact NAME --show-output --test-threads=1`.
6. Run the complete ordinary library suite with `--test-threads=2`.

The actual baseline listing must be exactly the failed candidate's 785 names minus its twenty declared additions: 765 names, no ignored tests. A missing, added, substituted or newly ignored name refuses the control. The focused and full-suite outcomes are separately retained.

## Negative Observation, Not Qualification

For the two test leaves only, the original bounded helper is still called with `expected_exit=0`, and that fact stays in its command record. A natural exit 101 raises its final `AssertionError(phase_name)`. The observer may record that exact assertion only after verifying the saved natural101 result, no resource reason and absent process group. It then requires every expected test outcome, the exact summary and exactly the one known failed name with exactly the observed `(8, Some(7), 5)` / `(7, Some(4), 5)` tuple. Other failures, ignored tests, truncated inventories, timeouts, signals, resource reasons or surviving groups refuse the observation. Nothing in the helper or test source is changed.

Natural zero is also recorded if it really occurs. `preexisting_rpo_failure_observed` is true only when both the focused and full-suite invocations reproduce the sole exact failure. `baseline_library_passed` reports the actual full-suite outcome separately. A top-level `passed=true` means the bounded observation completed with intact custody; `candidate_qualified` and `library_qualified` always remain false. The prior candidate failure is never rewritten, and no finalizer/join/emission/GPU or numerical acceptance is inferred.

## Custody And Invocation

Reuse the frozen indexed controller `3ad6ed9c...` prerequisite functions only, not its main or build routine. Authenticate its package `38206925...`, source proposal `6501750a...`, original RPO/finalizer/diagnostic records and products, and actual failed candidate ELF/raw records. The original RPO source and local/external dependencies, configurations, all prior targets (including failed candidate and diagnostic targets), selected ELF bodies and source records are preserved and rechecked. Any root-owned cache retirement must finish before this control snapshots protected targets.

Unchanged ASROCK UID9661, CPUs8/9, nice10, hidden GPUs, 12 GiB AS, 1 GiB file, 6 GiB cache, 64 MiB streams and 40/38 GiB free floors apply. Fresh owner is `E/kir-indexed-formal-join-baseline-cpu-owner-v228-v1`; no cleanup or existing target reuse occurs here.

Root first qualifies the 16 pure observer tests, then invokes its existing bounded outer CPU wrapper:

```text
/usr/bin/python3 -B E/p228-kir-indexed-formal-join-baseline-cpu-v1/run.py PACKAGE_MANIFEST_SHA
```

No future baseline receipt, outcome, executable identity or repair is assumed.
