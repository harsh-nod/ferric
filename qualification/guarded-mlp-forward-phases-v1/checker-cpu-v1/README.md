# Three-Record Checker Qualification

All 146 tests passed on `mi350` in 73.691258 seconds: the unchanged 126-test
validator suite and 20 new forward-phase tests. Both supervised processes
exited naturally with status zero, were reaped, and left no process group.
There were no failures, errors, skips, timeout or cleanup signals, source
changes, or postcheck errors.

The new [checker](validate_forward.py) admits the canonical three-record
Readiness40 host diagnostic while preserving whole-original stderr custody.
It validates identities and LF-inclusive hashes, 40 ordered rows, nine phase
intervals, exact integer sums, duration bounds and nested callback containment.
The old decoder remains unchanged and refuses the third record. Same-side
comparison rereads the complete original stderr and still checks all 40 records
and four complete payloads.

The [20 new tests](test_forward.py) include malformed framing, caps, types,
overflow, field order, hashes, route confusion and repinned semantic tampering.
Synthetic timing data is not a GPU measurement or a model result.

## Original Evidence

The [original archive](../checker-cpu-v1.tar.gz) contains 43 members:
28 source files, the bound input manifest, 13 original evidence files and the
archive manifest. The [manifest](manifest.json) binds every other member.
The terminal is [evidence/complete.json](evidence/complete.json).

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Original terminal | 57,594 | `e3e65540576213ecd394be09e7c87ad9ac0a763e4e0c7954d84cd3e04b1b6d19` |
| Original archive | 83,731 | `f880721598ebd3b6b386df0da53df0bc4913e274f92e16ce7e52d557760cd7f0` |

This README is subsequent commentary, not an original archive member.
Independent data-only review rehashed all 28 source bodies, twelve raw records
and two tool binaries, and joined both raw test rosters and retirement records.
The CPU run used CPUs 8-9, nice 10, hidden GPUs, 512 MiB address space and two
120-second leaves within a 300-second whole-run bound, retaining the existing
50-second cleanup reserve and 40/38 GiB storage floors.

This historical host-bound controller is not a public GPU launch command.
The native CPU-artifact admission helper, preparer and retention adapter need
their own workflow qualification and actual deployment bindings before a new
GPU run. No Full2303 execution, exact generated-output acceptance, GPU overlap,
inference speedup or 700 tokens/s result is established here.
