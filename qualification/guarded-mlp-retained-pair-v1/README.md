# Borrow-free guarded MLP pair custody

This is a CPU-qualified private runtime building block for multi-layer scheduling,
not a production worker route or a new GPU qualification. The preceding
[two-generation native run](../guarded-mlp-paired-reuse-native-v1/README.md) proved
bit-exact reuse through the borrowing `Session`; its receipt does not qualify
this checkpoint's new executable.

## Implementation

`RetainedPair` owns two genuine 2,208-byte combined MLP states without retaining
borrows of the Group or kernel handles. Its immutable binding records every
ordered payload token and kernel identity/image. Each run checks that binding
and repeats the existing metadata, allocation, alias and dispatch validation.
There are no mutable owner views or caller-supplied completion proofs.

Sources: [retained pairs](cpu-attempt-v2/fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_combined_mlp_paired_retained_v1.rs),
[retired proof](cpu-attempt-v2/fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_combined_mlp_paired_retired_v1.rs),
and [coordinator](cpu-attempt-v2/fe2o3/crates/fe2o3-kfd/src/engineering_gfx950_peer_combined_mlp_paired_v1.rs).

After both batches and typed owners complete, `Retired` privately retains the
actual arena custody. It can recheck those completed batches after other layers
advance the queues. It authenticates the original allocations, queue identities
and epochs, acquires all ten old USER signals as zero, and checks fresh current
idle queues, faults and monotone observed counters. The existing in-flight
reservation check still requires the original exact frontier.

`rearm_pairs` validates every selected pair before the first atomic reset. It
requires the same nonzero local generation and its exact checked successor,
rejects duplicate owners/arenas, resets both ranks, and rechecks every initial
state before making any pair available again. Short-lived RAII guards quarantine
the group and selected owners on errors or unwind, including partial allocation
and reset. Absolute deadlines cover execution, proof sealing and rearm commit.

Completed signals do not manufacture hardware read credit. Old arena storage
remains group-owned until queue-first Close; this code does not reset, recycle
or free arenas early. A pending poll may reach the existing 50-microsecond sleep
after expiry, but cannot reach another GPU operation before the deadline gate.

## Tests on MI350

Both attempts built and tested on `ssh mi350`, using the retained nightly
2026-04-03 toolchain and offline, locked nine-crate dependency closure.

| Evidence | Result |
| --- | --- |
| [First attempt](cpu-attempt-v1/evidence/failed.json) | Library: 1,032 passed, 1 failed, 6 ignored; later four targets did not run |
| [Corrected attempt](cpu-attempt-v2/evidence/complete.json) | Five targets: 1,053 passed, 0 failed, 6 ignored |
| Prior tests | All 1,047 previous named outcomes preserved |
| New coverage | 8 retained-pair tests, 3 retired-frontier tests, 1 shared-deadline test |
| Build inventory | 1,059 tests total; 1,039 in the library; five test executables |
| Corrected lifecycle | 15 phases, natural zero exits, reaped children, absent process groups |

The first attempt caught a missing post-publication deadline check: after rank 0
published at the cutoff, the coordinator could enter the next rank's fence before
refusing. The corrected source adds that check; the failed attempt is preserved,
not overwritten. Both attempts have no forced cleanup, timeout or postcheck
error. The CPU suite did not execute any ignored GPU diagnostic.

The new tests exercise 1/2/36/72-pair orchestration, before/after-effect failures,
every counted deadline boundary, zero/mixed/overflow generations, ordered role
binding changes, pre-GPU refusals, and unwind quarantine. Retired-frontier tests
accept completed intervening work with lagging reads and reject busy queues,
regressions and nonzero signals. These are CPU tests and injected failures, not
GPU fault-injection or a successful native interleaving test.

Resource limits remain two build jobs on CPUs 8/9 at nice 10, 12 GiB address
space, 6 GiB target-cache bound, 64 MiB captured streams, 40/38 GiB setup/live
disk floors, and bounded child/whole deadlines. No local build was performed.

## Retained Evidence

The [local retention script](retain_cpu.py) verifies archive pins, closed member
sets and every manifest body before writing fresh attempt directories. Retained
data includes actual formatted source overlays, controller/supervisor, exact
inputs, raw test/build streams, lifecycle records and prior source/test lineage.
It contains no executable binary bodies and does not execute retained source.

The failed capsule contains 69 members / 68 pinned bodies (5,533,235 bytes
expanded); the successful capsule contains 109 / 108 (5,845,573 bytes expanded).
The 932,306-byte successful archive SHA-256 is
`02d0b98f5814ccde37e6527183d03334978b9af1478788b133d5f01e67fc1811`.

Successful receipt: `1df321362ed50ffa24f83163be463d8b0ea04ac8bbc3024d5b212bd1f1a8d98f`.
Input manifest: `15e663b9aaf9b34ee7557927589ca7ef861d56d9b4a856ee435ded35e72cb687`.
The qualified library executable is 11,161,152 bytes with SHA-256
`f4519757078f6aa3b92ff8e2db8b016433d44113c92c44f1581ec7ac5f384dd6`.

The source overlay changes three existing files and adds four; the full frozen
map has 801 compiler source entries plus two harness entries. This remains a
private fe2o3 qualification overlay in Ferric, not a promotion of an unqualified
runtime tree or a change to generic SharedAtomic admission.

## Next Gate

Run a native two-layer, two-bank, four-forward fixture. Bank selection should be
0/1/0/1 while each pair's local guard generations are 1/1/2/2. Recheck each bank's
old proofs after the other bank has advanced the queues, use distinct numerical
witnesses, and reset both layer pairs in one call.

Ferric's Prefix284 whole-bank validation, global forward ledger, 36-layer guarded
route and finite arena reclamation remain separate integration work. Full-model
independent correctness, the 2,048/256 BF16 workload, performance/overlap evidence,
700 tokens/s and issue #42 milestones M0-M7 remain open. No model throughput or
performance improvement is claimed by this checkpoint.
