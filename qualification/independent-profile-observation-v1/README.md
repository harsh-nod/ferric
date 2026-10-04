# Independent Profile Observation

This is a tested, **no-launch Python adapter**, not an end-to-end GPU result.
Its [23 synthetic policy tests](cpu-result.json) passed on `mi350-2`; the
[transcript](tests.log) and exact [source manifest](source-manifest.json) are
retained. The complete source is under [source/](source/).

## What It Checks

The adapter consumes the new independent native-profile reports without
requiring baseline/candidate bitwise equality. It preserves the frozen child
validator unchanged: ten child sidecars, owned process identities, executable
and request pins, inputs, full captures, and closed native runs.

Each profile gets its own plan for the unchanged
[independent numerical comparator](../prefix-profile-numerical-v1/README.md).
Capture, rank, stage and original weight identities must match. A numerical
rejection remains specific to that profile; it cannot accept the other one.

Input authentication failures are different. `CustodyReader` records a pin
before the first read and raises `NumericalCustodyError` if either reader or
byte verification fails. That exception aborts validation before the next
profile. Tests cover the case where the original Reader fails before recording
any input, plus conflicting pins, changed inputs and incorrect returned bytes.

The original paired-comparison mode and numerical bounds are not changed.

## Executed Tests

All 23 tests passed with no skips, using Python on `mi350-2`. They exercise
closed schemas, whole capture extents/hashes, finite computed stages, typed
terminal states, profile/rank ordering, original input joins, exact owned leaf
commands, distinct numerical rejection and fatal input-validation behavior.

The mathematical return values in these tests are **explicit mocks**. The
comparator's real-reference tests and historical capture replays are separate
results; this adapter does not inherit GPU correctness from them.

The synthetic suite can be run without GPU access or NumPy:

```sh
cd qualification/independent-profile-observation-v1/source
python3 -B -m unittest -v test_observe
```

## Remaining Integration

`compare_retained` still needs end-to-end execution with real custody helpers,
the new native binary and fresh captures. There is deliberately no launch CLI
or deployment adapter. New image/ISA/arithmetic reviews, all three pre-run and
three post-run device audits, bounded cleanup and final input checks remain
required. Independent numerical acceptance, production authority and all
performance claims stay false.

[The retained design notes](source/README.md) specify that integration contract.
Their introductory unexecuted status describes the authoring-time snapshot;
the result here records subsequent root-run policy tests, not GPU execution.
