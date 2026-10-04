# RPO Finalizer Qualification

The finalizer built against the [qualified reverse-postorder compiler](../fe2o3-partial-move-rpo-v1/README.md)
passed 190 Rust tests on ASROCK through `ssh mi350-2`. All 15 historical
actual-capture tests remain ignored in this default CPU suite. The six
build/test phases and owner completed naturally, with clean source, input,
configuration, dependency and old-target postchecks.

| Check | Result |
| --- | --- |
| Finalizer tests | 190 passed, 15 historical ignores |
| Build/test phases | 6 completed naturally |
| Preserved compiler products | 5 identities |
| Newly built finalizer products | 3 identities |
| Controller policy tests | 16 passed, separate primary SSH observation |

## What This Establishes

The finalizer, its test executable and descriptor-metadata tool now belong to
the same tested compiler generation. All five prior compiler products and the
source snapshot remained unchanged. Compiler resource limits, proof checks
and Ferric's production route are unchanged.

[result.json](result.json) records the eight selected product identities,
named test outcomes and publication hashes. [complete.json](complete.json)
and [owner-complete.json](owner-complete.json) retain the remote results;
[raw](raw) contains the six phases' commands, output and process records.

The data-only publisher rehashes the retained evidence and joins the prior
compiler publication without changing it. It does not transport or rehash
executable/library bodies, rerun tests, or recursively verify every source,
dependency and tool referenced by a snapshot. The controller tests are a
retained primary-agent observation, not a remote supervisor receipt.

## Remaining Gates

The ignored actual-capture tests must be selected explicitly on the new
checked-lowering handoff; this default suite is not a substitute. Checked
RoPE lowering, both actual replay checks, gfx950 HSACO emission and GPU
numerical comparison remain separate gates. No new GPU image, full-model
accuracy acceptance, sustained 2,048/256 result or 700-token/s claim follows.
