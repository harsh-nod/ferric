# Independent All-Layer Observation Controller

The new controller passed all **84 CPU policy tests on MI350** on 2026-10-04,
with no errors, failures or skips. [Actual receipt](pure/complete.json),
[transcript](pure/tests.log), [tested source manifest](source/manifest.json).
All 21 source members have identical before/after hashes. This checkpoint does
not yet publish a new all-layer GPU result or numerical acceptance.

The controller combines the unchanged CPU633 parent and CPU475 worker with the
separately compiled V7 prefix image. It authenticates both deployments, the six
actual standalone GPU cases and six conditional numerical results. It retains
the original model, prompt, setup/MLP images, resource caps and owner lifecycle.

| Test group | Passing tests |
| --- | ---: |
| Historical worker deployment | 10 |
| Host capture comparison | 6 |
| Host observation validation | 10 |
| New independent intake | 18 |
| New structural observation | 10 |
| Portable deployment custody | 15 |
| Controller and cleanup | 15 |
| Total | 84 |

Unlike the preserved older paired-bitwise path, the new path records structurally
complete four-forward captures without requiring equality to a different native
image. It still requires all 152 tensor rows, typed Close, owned-process reap,
three pre-/three post-device audits and exact request/image identities. Tests
cover rejecting old success schemas, malformed captures, failed audits, changed
inputs, forced cleanup and unauthorized numerical claims. Most are synthetic
policy fixtures; they do not prove GPU correctness.

Successful structure alone cannot establish numerical acceptance. The completion
explicitly leaves full-model, independent-tensor and performance acceptance
false. Mode-specific framework references remain separately authenticated for
subsequent CPU diagnostics. Four forwards at positions 0 through 3 are not the
2,048/256 benchmark, even though the original prompt contains 2,048 tokens.

## Reproduction

The exact [test runner](run_tests.py) verifies the manifest and file census before
imports, runs each declared suite and rechecks every source afterward. Root ran
it on `mi350` with CPU affinity 8,9, nice 10, Python `-B`, unset PYTHONPATH and
PYTHONHOME, and empty HIP/ROCR/CUDA visibility. Limits were 2 GiB address space,
120 CPU seconds, 16 MiB files and no core dumps, inside a 150-second timeout.
Frozen historical evidence packages at the recorded paths are prerequisites.

See [intake contract](source/INTAKE.md), [controller contract](source/RUN.md),
and [source overview](source/README.md). Their author-stage notes are preserved
byte-for-byte; the actual test receipt above supersedes the prior unrun status.
