# Native Peer Dependencies: CPU Qualification

This checkpoint supports [issue #42](https://github.com/harsh-nod/ferric/issues/42).
It implements and CPU-tests the native two-rank dependency sentinel. It does
not establish GPU peer visibility, model correctness or decode performance.
All M0-M7, independent numerical acceptance, sustained 2,048/256 decoding and
the 700 tokens/s target remain open.

## Implementation

Runtime implementation:
[`998021225`](https://github.com/harsh-nod/fe2o3/commit/998021225db853720b71241ca303d2d43045a99e),
on `codex/p228-finite-runtime-integration-v1` in both fe2o3 forks.

The engineering-only unsafe entry accepts four commands per rank. It publishes
this fixed seven-packet sequence on each of two GPUs:

```text
P0 -> wait(both P0) -> C0 -> wait(both C0)
   -> P1 -> wait(both P1) -> C1
```

The system-scoped barriers protect peer reads and intermediate-buffer reuse.
Fourteen distinct completion slots are initialized once in two private,
peer-mapped signal/kernarg arenas. All commands are prepared before reservation;
INVALID packet bodies precede release headers. Retirement requires all signals,
both actual queue frontiers, clear exceptions and fresh currentness checks.
Any failure poisons the group. Arenas remain retained until healthy Close.
This is not yet a fixed-storage repeated-decode API.

The new example uses the existing BF16 peer-copy object with two different seed
patterns, two outputs per rank and reused intermediates. It checks all ten
buffers and their 64-byte guards. Witness mode deliberately withholds rank1
until rank0's producer completes while all dependent completions remain pending.
CPU tests do not establish that this witness works on hardware.

## Actual MI350 Results

Root execution used `ssh mi350`, host `smci350-rck-g03-b19-03`, boot
`2dbbbc5a-d8b7-46ac-bd1f-236e5847e24a`, with GPU visibility disabled.

| Check | Result |
| --- | ---: |
| AQL tests | 43 passed |
| KFD engineering library and integration tests | 958 passed, 3 ignored |
| New sentinel example tests | 7 passed |
| Rust total | 1,008 passed, 0 failed, 3 ignored |
| Log-parser regression tests | 8 passed |
| Example build and no-default-feature KFD check | Passed |
| Bounded CPU phases | 9 natural successful exits |

The three ignored tests require separate retained GPU artifacts; their names
remain in the raw test report. New coverage includes driver callback failures,
publication ordering, every completion and witness condition, queue-frontier
retirement, exact packet bytes, missing-barrier counterexamples and the private
native header writer. Seven example tests cover request, metadata and output
contracts without executing its GPU main program.

## Evidence And Reproduction

[`result.json`](result.json) hashes the retained logs, commands, controller,
thin manifest, original/pruned lockfiles and parser tests. The CPU completion
receipt is [`complete.json`](retained/evidence/complete.json), SHA256
`baad997a5de3d2b85ce926f9f684d923c3c049457df1242630faa2b35d734a73`.
The selected 1,878,976-byte example ELF is identified in that receipt; binaries
and Cargo build output are not committed.

All 777 tested crate files were compared against the runtime repository.
The only difference is the KFD README expanded after testing. Production code,
tests and manifests match the tested bytes. The full source roster and hashes
are in `retained/evidence/sources-after.json`; source bodies are provided by
the runtime commit, not duplicated in this evidence directory.

Use the recorded nine-crate manifest with those runtime sources and the pinned
`nightly-2026-04-03` toolchain to reproduce the offline Cargo commands in
`retained/evidence/*.command.json`. The original controller pins a specific
host and fresh workspace; it is an evidence record, not a portable launcher.
It uses two build jobs, CPU affinity 8/9, nice 10 and explicit resource bounds.

Three prior CPU attempts are preserved separately: V1 rejected removal of an
unused optional lockfile edge; V2 exposed a missing compiler-library search
path; V3 passed the Rust suite but its parser missed two parent outcomes split
by intentional child-abort tests. V4 admits only the exact observed lockfile
change, supplies and pins existing compiler libraries, and recognizes those
two exact output sequences. No Rust test was removed or relaxed.

The first GPU controller attempt stopped before native execution because its
library audit did not recognize the ELF interpreter as also satisfying its
`DT_NEEDED` entry. Its post-audit found all eight GPUs idle. Native GPU
qualification and any performance claim are not included in this checkpoint.
