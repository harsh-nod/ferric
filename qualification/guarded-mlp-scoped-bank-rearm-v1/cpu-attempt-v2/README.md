# Bank Rearm CPU Attempt 2

The fresh MI350 coupled qualification passed all 26 phases in 166.963 seconds.
The complete runtime suite passed 1,152 tests with eight existing ignored;
the complete worker suite passed 760 with four existing ignored. All nine
focused scopes passed, including 22 mixed-bank tests, along with ten selected
facade doctests and eight parser regression checks.

The sole repair from the original failed proposal is a function-local unsafe
allowance and safety comment on its empty-input CLI test. Production behavior,
test names and assertions were not changed by that repair. Both original and
repaired proposal manifests are preserved. The original failed attempt remains
in `../cpu-attempt-v1` and is not relabeled as successful.

All phases exited naturally with exit zero, were reaped, and left no owned
process groups. Source, dependency, tool, cache and executable postchecks passed.
The genuine worker executable built before its integration tests matches the
final product. GPU execution was disabled for this CPU gate.

The capsule retains 190 original files, including 137 raw bodies, 11 lineage
inputs and all 23 actual formatted Rust postimages. Eleven executable products
are authenticated by metadata; binary bodies are not included. All 1,042
source rows are retained in the original source map. Initial/live free-space
floors remained 40/38 GiB, with CPU 8/9 and two Cargo build jobs.

- Terminal: 2,395,639 bytes, `e522aafde4204c24be8fac9b1ecf8d49f8e3389819ad8cfda765f2145647c432`.
- Source map: 423,235 bytes, `73e3ad534a3d48bddd717780215142dd588f4a776dd25f003305854f70d7c285`.
- Worker: 6,416,128 bytes, `7f1755cb6fdef9b9b1f50d2cb15bcc1594cd10e7c0d4029d99cd9280d0ebb3e1`.
- Archive: 1,795,419 bytes, `dee00f62d7ca7b7fedbb3dbbfef11e778ca1b925dee88daa977622bdd8257c86`.

Root and independent peer audits joined the original data before integration.
Only the 12 runtime and 11 worker Rust postimages were copied. The integration
postcheck rehashed all 2,091 composed canonical bodies, including 1,055 unchanged
nonworker Ferric bodies; runtime fixture Cargo files were not copied.
This establishes CPU qualification, not native GPU or model acceptance.
