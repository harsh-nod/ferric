# Ordered Peer Dependency Packets: CPU Qualification

This checkpoint supports [issue #42](https://github.com/harsh-nod/ferric/issues/42).
It qualifies the additive AQL packet representation and publication protocol,
not GPU peer visibility or a complete decode runtime. All M0-M7, independent
numerical acceptance, sustained 2,048/256 decoding and 700 tokens/s remain open.

## Implementation

The implementation is committed as
[`774034d4`](https://github.com/harsh-nod/fe2o3/commit/774034d4e03817088ff1d245f47e98f3155a9902)
and pushed to both fe2o3 forks on `codex/p228-finite-runtime-integration-v1`.

- A 64-byte `AqlPeerBarrierAndPacketV1` holds two distinct producer signal
  addresses and a distinct completion signal, with checked 64-byte alignment.
- The system-scoped, wait-for-prior barrier header is `0x1503`. The existing
  publication predicate remains unchanged; a separate opt-in predicate admits
  the new packet.
- `AqlPreparedPeerPacketBatchV1` accepts ordered kernels and dependency barriers.
  Every INVALID packet body is written before any release header is published.
  A failed callback stops publication immediately.
- Signal addresses retain all 64 bits. The packet API does not establish
  allocation ownership, peer mapping, queue epochs, graph acyclicity or native
  retirement. Those remain responsibilities of the native runtime owner.

These are prerequisites for replacing host-mediated peer rendezvous with
device-side dependencies. No native header writer or GPU scheduler is enabled
by this checkpoint, and no performance gain is claimed.

## Actual MI350 Results

The root agent ran the tests through `ssh mi350` on
`smci350-rck-g03-b19-03`, boot
`2dbbbc5a-d8b7-46ac-bd1f-236e5847e24a`. This was a CPU-only run with GPU visibility
disabled, using `nightly-2026-04-03`, two build jobs and offline dependencies.

| Check | Result |
| --- | --- |
| Existing library unit tests | 2 passed |
| Existing AQL ABI tests | 31 passed |
| New peer dependency tests | 10 passed |
| Total Rust tests | 43 passed, 0 failed, 0 ignored |
| C oracle against installed ROCm `hsa.h` | Exact 64-byte encoding and `0x1503` header matched |
| Bounded execution phases | 7 natural successful exits; no timeout or forced cleanup |

The ten new tests cover exact layout, high address bits, each misaligned signal,
duplicate/self dependencies, all-bodies-before-headers, every body/header failure
position in the three-packet test batch, rejection of unordered kernels, batch
count boundaries and isolation from the existing header admission policy.

The seven phases record the compiler version, format the two new Rust files,
check their formatting, generate the isolated offline lockfile, run the Rust
tests, compile the C oracle and run it. All child process groups were absent at
completion. Resource limits and exact commands are retained in
[`run.py`](retained/run.py) and the phase records; this is not the full Ferric or
KFD test suite.

## Reproduction And Provenance

The retained thin workspace preserves the production crate manifest and all
tested AQL sources. From `retained/`, with the recorded toolchain and cached
dependencies, the core checks are:

```sh
cargo test --offline --locked -p fe2o3-aql -- --test-threads=1
cc -std=c11 -Wall -Wextra -Werror -I/opt/rocm/include/hsa \
  crates/fe2o3-aql/tests/oracles/aql_peer_dependency.c \
  -o /tmp/fe2o3-peer-dependency-oracle
/tmp/fe2o3-peer-dependency-oracle
```

For isolated reruns, choose a unique task-owned output path instead of reusing
the illustrative `/tmp` name. The original controller is retained as evidence
of this run, not a portable launcher: it pins this host's source root, toolchain
and ROCm header. No executable or Cargo target directory is committed.

[`result.json`](result.json) hashes all 45 copied files, including raw logs,
per-phase start/completion records, the thin manifest, lockfile, controller and
tested sources. [`primary-observation.json`](primary-observation.json) records
the root execution and transport observations. The principal receipt is
[`complete.json`](retained/evidence/complete.json), SHA-256
`06f981beb65b5b76c60e0a82adeba118b6e6b5c9ee6795aaca148d7bd7249e96`.

The committed Rust and C implementation/test files match the tested final
sources byte for byte. Only the two new Rust files were formatted during the
run. The current runtime README was expanded afterward; the retained README is
the original test-workspace version, not the later documentation revision.

## Remaining GPU Gate

The next native test will retain peer-mapped completion arenas and run this
fixed sequence on each of two ranks:

```text
producer0 -> wait(both producer0) -> consumer0 -> wait(both consumer0)
          -> producer1 -> wait(both producer1) -> consumer1
```

Different immutable input patterns, retained outputs for both iterations and a
reused intermediate buffer will make ordering observable. An explicit bounded
diagnostic will leave rank1 unpublished until rank0's producer is complete but
its dependent consumer remains pending. All fourteen completion signals,
healthy queue frontiers, output values and guards must pass before retirement.
Partial publication, exceptions or timeout must poison the operation without
reset/retry. None of those native properties has been qualified by this CPU run.
