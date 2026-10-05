# Ordered Segment: MI350 Runtime Preparation

Both newly qualified comparison parents and their common worker passed fresh
runtime-library audits on MI350. The shared-full and ordered input assemblies
also completed. This is deployment and input-preparation evidence, not a
completed GPU pair or numerical/performance acceptance.

| Executable | Resolved libraries | Audit result |
| --- | ---: | --- |
| Shared-full parent | 4 | readelf and ldd passed |
| Ordered parent | 4 | readelf and ldd passed |
| Common worker | 3 | readelf and ldd passed |

All six bounded commands exited naturally with empty stderr, reaped processes
and no forced cleanup. The selected host, boot and static topology remained
unchanged across the audits. Both parents resolve libgcc, libm, libc and the
loader; the worker resolves libgcc, libc and the loader. There is no missing
library or RPATH/RUNPATH override. Each executable is an exact product of the
[successful CPU V2 qualification](../projection-ordered-segment-cpu-v1/README.md),
not the earlier CPU883 build or the failed first ordered-segment attempt.

## Matched Inputs

Both assemblies bind the actual CPU completion, the passing V3 174-test policy
receipt, fresh executable identities and their separately authored runtime
reviews. Each records 1,847 consumed input identities and successful integrity
postchecks. The original model, prompt, BF16 arithmetic, images, dispatch
bounds and full unsigned 64-bit device identities are unchanged.

Relative to the prior observer request, both change only `decode.worker`,
`decode.session` and `decode.evidence_directory`; the ordered request also
uses its explicit ordered schema. Between the new requests, the worker is
identical and only the schema, session and output location differ. Each
selects its own parent and review, while sharing the qualified CPU generation,
worker, model, kernel images and supervisor evidence.

The root reviews explicitly address the changed intermediate host-fence
boundary, paired queue publication/completion, fixed retained arenas and
distinct per-rank Down-output scratch. Both routes configure shared-full
currentness with admission caching and operational shortcuts off. The
assembler copies reviewed decisions; it does not generate approval.

## Evidence

- [Shared-parent audit](audits/shared-parent/complete.json): `f0ee7b71127c47835173408a1831565a26e5719e06cdfc7ad0bdb45a2a76728c`.
- [Ordered-parent audit](audits/ordered-parent/complete.json): `f9b9bf1559d5d140b6428cc39a1a5d504029c953f82e50124960579e92e0f5be`.
- [Worker audit](audits/worker/complete.json): `3446564eae8b8fdda0edc40b703496b29c4cf843d31ee4a3d1c0ec6848166d6d`.
- [Shared assembly](inputs/shared/assembly.json): `1bea8a2a4046dadc390b2af3eb23e947a4d40439b88e8dbf2bc3fdcea1f475e3`.
- [Ordered assembly](inputs/ordered/assembly.json): `5b19f8e7d2b981e427210b16e9993277a9950e52e782ef3e26a6e14ca979996b`.
- [Publication ledger](result.json): all 51 audit files, eight assembly files,
  seven root input/review files and the publication helper, copied byte-exactly.

The local publisher verifies copied metadata and body hashes; it does not
reopen remote executable/library bodies, replay the full assembly input graph,
rerun commands or infer GPU success. The paired native run must independently
pass fresh process/device audits, terminal-state and lifecycle checks, and
actual output comparison. All issue #42 milestones, independent numerical
acceptance, sustained single-request Qwen3-8B BF16 target-only 2,048/256 decoding
and 700 tokens/s remain open.
