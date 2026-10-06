# Lowering Admission Tests

Seven synthetic controller tests passed on MI350. They exercise the updated
loader receipt/FilePin and final producer joins, reject stale or mismatched
metadata, and verify that missing audit/vendor bindings refuse before effects.
The tests mock document access and do not perform compiler or loader execution.

The [complete receipt](attempt-v1/evidence/complete.json) records one natural
zero-exit, reaped child with an absent process group and clean postchecks.
All seven exact names passed with zero failures, errors or skips. Whole-run
time was 0.120 seconds; the child took 0.101 seconds.

The [retention manifest](attempt-v1/retention-manifest.json) pins fourteen
original files, including seven raw records and five source bodies. Receipt
SHA-256: `13de546d08c09aca3f85402f2e301a1a2c72345a3e24fffd99186c5bc2f588b5`.

The tested lowering source is `554f2f16b9f2da280be237dddc8c51e95cc2f93416affe4c4fe27beeca0c9981`,
with its two real-input bindings deliberately absent. The later executable
controller binds only the actual passing loader receipt and vendor-controller
hash, yielding `a30338334d5eb439b51441092bac314ced1627ce2241d41360c866261abcf768`.
That two-assignment change was reviewed separately; these tests are not
claimed as an execution of the later bytes. The original compiler command,
process/resource bounds and output validation are unchanged.

These synthetic tests do not establish checked gfx950 emission, ABI/ISA
acceptance, GPU correctness, model numerics or decode performance.
