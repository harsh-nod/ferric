# Peer Signal Completion: CPU Qualification

This checkpoint supports [issue #42](https://github.com/harsh-nod/ferric/issues/42).
It corrects the experimental completion contract after the
[V2 terminal-join diagnostic](../peer-dependency-terminal-join-gpu-attempt-v1/README.md).
This is CPU qualification, not native GPU or model acceptance. All M0-M7,
independent numerical acceptance, sustained 2,048/256 decoding and 700 tokens/s
remain open.

## Implementation

Runtime commit
[`097b4f796`](https://github.com/harsh-nod/fe2o3/commit/097b4f796a283f554339cc0c0ef5c2c8c3858d2a)
adds a separate unsafe V3 entry and example. The V1/V2 examples and their strict
retirement contracts remain unchanged. V3 uses the same eight packets per rank
and sixteen distinct completion signals as V2.

V3 requires all sixteen signals to complete, both queues to be published, the
exact reserved write frontier, monotonic bounded actual reads, and current
identity/fault checks. It returns the actual counters from the successful
completion sample. It does not require read-equals-write for completion, write
the hardware read cursor, or award ring capacity for a completed signal.
Subsequent reservation still uses the actual hardware read pointer.

When private peer-dependency arenas exist, teardown destroys every participating
queue before releasing any peer arena. It then rechecks currentness while all
CONTROL mappings remain live, and finishes each context's ordinary cleanup.
Partial failure stops cleanup and preserves the process-terminal poison/retention
contract. Ordinary groups retain their existing close route.

## Actual MI350 Results

Root built and tested over `ssh mi350`, host `smci350-rck-g03-b19-03`, with GPU
visibility disabled. All thirteen bounded phases exited naturally with code zero.

| Check | Result |
| --- | ---: |
| AQL tests | 43 passed |
| KFD engineering library and integration tests | 994 passed, 3 ignored |
| V1 example tests | 7 passed |
| V2 example tests | 9 passed |
| V3 example tests | 11 passed |
| Total | 1,064 passed, 0 failed, 3 ignored |
| Three example builds and no-default-feature KFD check | Passed |

New coverage includes lagging-but-monotonic reads, full-ring refusal despite
completed signals, strict V1/V2 compatibility, queue-first close ordering,
failure at every coordinator callback, and isolated disabled-token poisoning.
These are CPU tests, not real KFD ioctl fault injection. The three ignored tests
need separate artifact fixtures, as recorded in the raw logs.

## Evidence

[`result.json`](result.json) binds all 76 retained files, including 69 raw
logs/ledgers, the controller, original/pruned manifests and lockfiles, and
[`complete.json`](retained/evidence/complete.json). The complete receipt SHA256 is
`b4d6edccd6232fc925305d8053b844c3bf072a3f2aade9a774f44d3e67db059f`.
All 779 tested crate files match the runtime commit. All three ELFs were selected
from Cargo output and postchecked; binaries and Cargo targets are not committed.

The retained controller records exact offline commands, the nine-crate workspace,
`nightly-2026-04-03`, two build jobs, CPU affinity 8/9, nice 10 and resource bounds.
Native execution is a separate qualification step.
