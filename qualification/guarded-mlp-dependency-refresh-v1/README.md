# Guarded MLP Dependency Refresh

The experimental guarded MLP crate now selects published fe2o3 revision
`5a500d63c29b78f8788356b20dfcbb5c41ec20c9` for its device and host dependencies.
Only those two revision pins changed. All five Rust files, kernel contracts
and 27 named tests are unchanged. This crate is not connected to the inference
path; the update grants no production, HSACO, GPU or performance authority.

## Actual MI350 Check

The [complete receipt](attempt-v1/evidence/complete.json) records ten naturally
successful, reaped phases in 74.533 seconds: compiler version, formatting,
format check, metadata, host check, test build, full/ignored inventories,
tests and host build. All 27 tests passed with no failures or ignores:
twelve staged-arithmetic tests and fifteen state/guard tests. Source and
dependency postchecks were clean; no timeout or forced cleanup occurred.

The test input contains the exact published fe2o3 source tree. Its two Git
dependencies are replaced only by corresponding local paths for this isolated
offline CPU check. The canonical manifest is retained separately. Metadata
may prune inactive lockfile packages/edges under the recorded rules; no other
source transition is permitted. The input-hash crate-binding fixture is
host-only and must not enter device lowering.

The [retention manifest](attempt-v1/retention-manifest.json) pins 68 original
files, including 56 raw records, the controller, full input/source maps and
candidate source/lockfiles. Receipt SHA-256:
`34df5002c9ca586b5c6dd7e6fc212341b857eedc7ce64bdd4ce0871ccfd3f34f`.

## Offline Dependencies

Fresh [vendor preparation](vendor-v3/evidence/complete.json) also passed on
MI350 in 2.547 seconds. One offline, locked vendor command exited naturally
and was reaped; 145 packages and 6,352 vendor files were recorded. Candidate
sources/lock, nightly Rust sources/lock and input/tool/product postchecks
were unchanged. No Cargo configuration was installed, and no synthetic
host-only crate binding was passed. The retained metadata does not include
the vendored package bodies. Receipt SHA-256:
`187c8e82c98b6e8b43b0df4f692d8cc3eab5aa83e06db0a0d6c96d60047b3c0b`.

This closes the candidate's dependency-update CPU gate, not the prior
[lowering refusal](../guarded-mlp-lowering-attempt-v1/README.md). The
[combined runtime compiler](../guarded-mlp-s-rpo-qualification-v1/README.md)
is a separately derived source generation. Its
[loader audit](../guarded-mlp-s-rpo-tool-audit-v1/README.md) and fresh offline
vendoring passed; a new gfx950 lowering attempt must precede GPU validation.
Independent model numerics and the 2,048/256 BF16 decode benchmark remain open.
