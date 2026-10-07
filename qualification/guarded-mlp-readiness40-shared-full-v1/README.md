# Readiness40 Shared Full Currentness

This opt-in experiment shares a fresh topology discovery across the full
per-rank currentness checks. The worker and independent data checker are
CPU-qualified on `mi350`. Parent qualification and native model parity remain
pending; there is no measured speedup or numerical-acceptance claim yet.

## Optimization Boundary

The explicit worker selector is
`--engineering-native-guarded-mlp-readiness40-position5-shared-full-v1`.
It configures `configure_performance_v2(false, false, true)` exactly once on
a fresh group before setup allocations. Full pre/post participant checks and
generation checks remain. This is not cached topology across operations or
the reduced operational-currentness policy. Default routes are unchanged.

The ordinary Position5 request, wire, 144-page allocation, 40 prompt forwards,
zero generated outputs and four captures at positions 0/5/16/39 remain intact.
Kernel images, hidden readbacks, arena policy, dispatch bounds and Full2303
abort bounds do not change. Paired reads, paired-terminal execution, cached
kernel admission and the host observer are not enabled by this selector.

After successful Close and flushing the Closed frame, the worker publishes one
bounded canonical policy record to its existing stderr stream. This record
binds the session, devices, child identity, executable, profile, registration,
transcript and completed extent. The parent and independent validator must
authenticate the original bytes. The ordinary validator still requires empty
stderr; the SharedFull route does not discard or sanitize the policy record.

## Worker CPU Result

The actual run completed nine phases in 83.047 seconds, with 704 passes, no
failures and four existing ignored tests. All 698 predecessor outcomes were
preserved, including those ignored tests. Nine library tests and one real
executable CLI regression were added; inventory contains 708 unique names.

| Target | Passed | Ignored |
| --- | ---: | ---: |
| Worker library | 683 | 4 |
| Worker binary tests | 0 | 0 |
| Executable CLI regressions | 5 | 0 |
| Shared wire | 16 | 0 |
| Total | 704 | 4 |

All phases exited naturally and were reaped. Source, tool, private-cache,
dependency and product postchecks passed. The real executable used by the CLI
tests equals the final worker ELF. Seven worker files were integrated from
their actual tested postimages, including six observed formatter changes.
All 205 canonical worker files match the qualified map. The 815 runtime bodies
are unchanged. The 75 original capsule members are retained in `worker-cpu-v1`.

## Data Checker Result

All 29 checks passed in one naturally retired GPU-hidden phase: 17 unchanged
ordinary checks and 12 SharedFull policy/parity checks. The run took 1.925
seconds, with zero failures, errors or skips and clean source/tool postchecks.
Coverage includes canonical framing, exact policy bits, identity and transcript
joins, Close, bounds, original stderr retention, all 40 records and all four
payloads. Fixtures are synthetic, not model output. The 17 original capsule
members are retained in `checker-cpu-v1`.

| Artifact | SHA-256 |
| --- | --- |
| Worker CPU terminal | `4a016b7e09b0cc6f9b4bd32c98a5713f24709bd6509564a589c29b0478f7e337` |
| Tested source map | `2566997ffb73fb40812688178f4494e48c1db4c8e7522717d54b9176994fd3e1` |
| Worker ELF | `9dbebbb014562f1bb704e8d504e46b8b719745abff5e0c7674af18412541bda8` |
| Worker archive | `6dae7b42503d80d10b7692de2064b97243090c56590985d4e5741bc935002d6b` |
| Checker terminal | `7a9029ae0e864809fa98062f4f15a13c954ce67d8b1d20e8c2cb36baa888850c` |
| Checker archive | `cd89e1e2a519c154b9bef299c985047cc1aed86b08a29dad964815df5f941841` |

## Remaining Gates

The first parent CPU attempt stopped during its sixth phase when the live
storage guard fired. Its first five phases, including compilation and the
946-name library inventory, completed naturally. The sixth compilation was
terminated and fully reaped; no selected tests or qualified parent products
were published. This is a failed qualification, not a Rust test failure or a
passing parent result. Original source/cache/tool postchecks completed cleanly.

`parent-cpu-attempt-v1` retains all 165 original capsule members, including
the failed terminal, all 34 raw bodies and 113 lineage bodies. Terminal SHA-256
is `5d794322b863899d2feb17df4d8f344bde03f411097a95a8fb25c313deba73e9`;
archive SHA-256 is
`d3f907f8bafcc2cccdc0c27095876f71f7306ed565317851be795b2dd3aa4c6c`.
The original failure and forced-termination flags remain unchanged.
The retry uses a fresh source/target/private-cache directory, the same Rust
proposal and tests, and unchanged 40/38-GiB initial/live space floors.
Only disposable intermediates from completed or retained failed builds were
removed; sources, evidence and executable products were preserved.

The parent must preserve all 205 qualified worker sources and pass its own
selected tests before the native parity probe. The probe requires unchanged
40-record semantics and byte-exact equality of all four full captures.
Same-side parity is not independent framework accuracy. A fair timing comparison
also requires the same newly qualified parent/worker ELF pair and equal
instrumentation in fresh default and SharedFull runs. The historical timing
probe is not a matched control for this new executable generation.

Full-request Ferric 2,048/256 acceptance, sustained BF16 target-only throughput,
700 tokens/s and M0-M7 remain open.
