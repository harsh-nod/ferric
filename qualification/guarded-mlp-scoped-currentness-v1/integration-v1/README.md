# Qualified V3 Integration Emitter

Source-only data tool. Root invokes it only after the observed V3 success
archive is retained and independently reviewed. It imports only the Python
standard library, never loads a retained module, never runs tests/processes,
and never writes to either canonical repository.

The CLI requires observed archive and terminal hashes. No successful outcome
or postimage is embedded. It rechecks all 207 original archive members,
the closed 206-pin manifest, 137 raw pins, 26 clean owned phases, baseline
and new runtime/worker outcome names, four repair-chain manifests, 35 exact
overlay preimages and the complete 1,033-row source composition.

This is an integration admission layer, not a replacement for the reviewed
exporter/retainer: it does not repeat live remote dependency/cache/tool
hashing, every command-recipe check or every doctest parser assertion.
Those remain authenticated original receipt claims from the observed CPU
qualification and retained capsule. All eleven products are joined to their
original Cargo records, and the pretest/final worker pin must agree.

## Emit

```text
python3 -B integrate.py emit ABSOLUTE_ARCHIVE OBSERVED_ARCHIVE_SHA OBSERVED_TERMINAL_SHA ABSOLUTE_FRESH_OUTPUT
```

The output directory must be fresh and outside both canonical repositories.
It contains only runtime.patch, worker.patch and integration.json. Patches
are in apply_patch format; root applies them separately after review.

Before emission, every one of 811 existing nonfixture runtime bodies and 205
existing worker bodies must match the original qualified baseline. All nine
new paths must be absent. The exact 22 runtime and 13 worker bodies come only
from the actual formatted retained archive, not authored source substitutes.

The full verification map contains 816 runtime nonfixture bodies, 209 worker
bodies and two original Cargo identities. Reduced Cargo.toml/Cargo.lock
fixtures are never emitted. Their original aliases map only to canonical
Cargo.toml and Cargo.lock, which must already match and stay unchanged.
Canonical Cargo.toml.original and Cargo.lock.input must remain absent.

## Read-Only Postcheck

After root applies both patches:

```text
python3 -B integrate.py check ABSOLUTE_OUTPUT/integration.json OBSERVED_MAP_SHA
```

This rehashes all 1,027 recorded canonical identities, both original emitted
patches, the same emitter source, and the map. It checks alias absence and
prints a separate postcheck receipt to stdout; it does not modify the original
map or repositories. Root retains that output when it succeeds.

Inputs are stable-read and posthashed before publishing a plan or successful
postcheck. Limits are 180 seconds, 768 MiB address space, 64 MiB archive and
expanded bytes, 207 archive members, 32 MiB per body, and 64 MiB output-file
limit. Fresh-output refusal occurs before any output-directory creation.

No native execution, temporal equivalence, numerical acceptance, performance
or Full2303 feasibility is claimed. V1/V2 failures remain untouched.
