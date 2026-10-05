# Terminal Peer Join: CPU Qualification

This checkpoint supports [issue #42](https://github.com/harsh-nod/ferric/issues/42).
It adds an opt-in terminal join to diagnose the previous
[native retirement failure](../peer-dependency-native-gpu-attempt-v1/README.md).
It does not establish GPU ordering, model correctness or performance.
All M0-M7, numerical acceptance, sustained 2,048/256 decoding and 700 tokens/s
remain open.

## Implementation

Runtime commit
[`203cfcbef`](https://github.com/harsh-nod/fe2o3/commit/203cfcbef166932b04683cd5c88f3414fe3b8a25)
preserves the original seven-packet V1 entry and example. The separate unsafe
V2 entry and example add a terminal barrier to each queue:

```text
P0 -> wait(both P0) -> C0 -> wait(both C0)
   -> P1 -> wait(both P1) -> C1 -> wait(both C1)
```

All sixteen distinct completions and both actual read/write frontiers must
match the reserved end. The last barrier is an experiment, not a guaranteed
cursor flush. No reset, retry, host-written read cursor or relaxed retirement
check was added. Private arenas retain their existing size and lifetime.

V2 failures retain sixteen changed signal/frontier/header snapshots, the
latest sample, a dropped-change count and first observed all-zero time.
Sequential host samples are not simultaneous GPU timestamps or reuse authority.
V2 uses distinct JSON schemas and rejects the V1 fourteen-completion result.

## Actual MI350 Results

Root built and tested over `ssh mi350`, host `smci350-rck-g03-b19-03`, with GPU
visibility disabled. Eleven bounded phases exited naturally with code zero;
both example ELFs were selected from Cargo output and postchecked.

| Check | Result |
| --- | ---: |
| AQL tests | 43 passed |
| KFD engineering library and integration tests | 970 passed, 3 ignored |
| Unchanged V1 example tests | 7 passed |
| New V2 example tests | 9 passed |
| Total | 1,029 passed, 0 failed, 3 ignored |
| Both example builds and no-default-feature KFD check | Passed |

Twelve new runtime tests cover exact first-seven-packet preservation, terminal
dependencies, sixteen unique signal slots, all-signal and actual-frontier
conjunctions, nonzero initial queue positions, and bounded trace behavior.
The ignored tests require separate artifact fixtures, as recorded in raw logs.

## Evidence

[`result.json`](result.json) binds all 66 retained files, including the controller,
59 raw logs/ledgers, original/pruned lockfiles and
[`complete.json`](retained/evidence/complete.json). Its SHA256 is
`4df566a9df1298f396bb5d55308591abbb252d3a57618403f493ad455a7f6956`.
All 778 tested crate files exactly match the runtime commit, including docs.
The original V1 example is byte-identical to the preceding runtime commit.

Reproduction inputs are the retained nine-crate manifest, runtime sources,
`nightly-2026-04-03` toolchain and exact offline commands in the raw ledgers.
The controller is host/workspace-pinned and uses two jobs, CPU affinity 8/9,
nice 10 and explicit process/resource bounds. ELFs and Cargo targets are not
committed. Native V2 execution is not part of this CPU checkpoint.
