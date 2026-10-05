# Shared-Full GPU Comparison Preparation

The default/shared comparison harness passed its CPU-only policy tests on MI350
on 2026-10-05 UTC. This prepares a comparison of the separately
[qualified Rust routes](../projection-ar4-shared-host-v1/README.md); it is not a
GPU result, a speedup measurement or numerical acceptance.

| Test scope | Observed result |
| --- | ---: |
| Comparison supervisor and validators | 122 passed |
| Executable deployment selector | 14 passed |
| Input assembler | 11 passed |
| Errors, failures or skips | 0 |

The 122-test suite retains all 91 previous named tests and adds 31. It checks
the two route schemas, actual CPU qualification and executable identities,
shared configuration outside the zero-based snapshots, matching input histories,
bounded lifecycle handling and honest counter attribution. Source hashes and
named test inventories are unchanged before and after execution.

Review caught a payload-comparison bug before any GPU execution: the data reader
had been told to discard the bytes being compared. Both reads now retain their
bytes. A regression test models the actual reader contract and requires an
unequal payload to produce a false comparison, not equality between empty buffers.

## Comparison Design

Both arms select executables from the same CPU883 build and preserve the model,
prompt, seed, BF16 arithmetic and GPU images. The default arm runs first, then the
explicit shared-full arm, once each without retry. Sessions and output directories
are distinct. Each arm retains the existing 4,000-second native deadline,
4,300-second case bound, process ownership and six surrounding idle audits.

The shared route enables only fresh shared-full group checking. Admission caching
and operational-currentness shortcuts remain off. Configuration time is separate
from setup and forward counters. For each interval the reported currentness
numerator is `rank_full_ns + group_full_ns + publication_full_ns`; inclusive
command, read, write and dispatch counters are not added to it. These are host
diagnostics, not GPU timings or disjoint latency components.

Attribution requires matching actual own-output input histories. Four payload
comparisons report same-generation repeatability, not independent correctness.
A single fixed-order pair is subject to warmed caches and host scheduling; it
cannot establish sustained throughput or a general performance advantage.

## Evidence

- [Actual 122-test completion](pure/complete.json): `299c0d1e80b2329e6f55be14015d57c23e29654b55461a740e7da5aac63683db`.
- [Full named test transcript](pure/tests.log) and [tested source roster](pure/sources-before.json).
- [Separate 25-test primary observation](primary/adapter-tests.json): `9a48e238ecf27ed92d1910c2aeecea9e3a9741a35dacd60be7a91a79bc176c7a`.
- [Frozen package manifest](source/manifest.json): `32222c38d7af2b267707911f75a0281dcdd3f7a723215b6cd73baf60120c919d`.
- [Publication inventory](result.json), including original paths and byte hashes.

The adapter observation records root-observed commands and results, not a retained
full named transcript. Source-only wording inside copied proposal documents is
preserved verbatim; the actual results above supersede it. The data-only publisher
does not rerun tests, inspect ELF bodies or grant runtime approval.

Fresh executable audits, route input assembly and the GPU pair are separate
execution steps. All issue #42 milestones, independent numerical acceptance,
sustained 2,048/256 decoding and the 700 tokens/s target remain open.
