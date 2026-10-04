# RPO Finalizer Tools V2 Draft

Source-only successor of the actual ordinary-induction finalizer controller
`564b70189259bbdb6fbf6eec42d424ecb1b2ffc912a4d5a76e1c4c4b63a14de3`.
No author imports, tests, builds, remote commands or GPU execution. Sixteen
policy tests are authored, not qualified. Root owns review, freeze and runs.

## Inputs

The frozen RPO CPU controller is
`p228-partial-move-rpo-cpu-v2/run.py`, SHA
`6cb1dc984761bfb44bf03246c08623faba225433cad70d745a606893ab5125ba`.
It supplies the unchanged authenticated probe/owned/bounded helpers and actual
Cargo/inventory parsers. No old controller main is called.

The CPU package must be exactly 1,917 bytes with SHA
`9fe67c55e38e4713c692abb4e145cd58bcea81ecf162879de57568b81d729603`.
The V2 source manifest is 5,887 bytes with SHA
`939b76eb28df8d7e2b53ae0b4f5034257185f12f077d20e581d9880b006a252c`.
V1 remains immutable and cannot qualify these tools: its test fixture failed
compilation before tests ran. V2 changes only the fixture's typed edge-role
construction; the scheduler and all finalizer phase bodies remain unchanged.

CLI takes three explicit hashes: package manifest, actual RPO CPU completion,
and actual RPO CPU owner completion. Future CPU/owner or product hashes are
not guessed. Their paths are closed to
`E/rpo-compiler-cpu-v228-v2/complete.json` and
`E/rpo-compiler-cpu-owner-v228-v2/complete.json`, where
`E=/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`.
The full qualified source and target must still exist at that CPU namespace.

Validate all 12 CPU phases, the actual source/tool/dependency/configuration
snapshots, five compiler products, the exact RPO source manifest and the
ordinary compiler predecessor. Replay both full CPU suites from named logs.
The new pliron names must equal all old names plus the 17 reviewed additions;
the compiler names and old ignored names stay unchanged. Expectations are
1,504/1 and 1,196/24, not results until those receipts exist.

The prior finalizer receipt is 45,313 bytes with SHA
`874a6d2026029619a02283ee9650d682690941c14904d9fa59c101b58b89d6bc`.
Its raw six-phase logs, inventories and named outcomes are rehashed and
replayed. The candidate must keep its exact 205 names and 15 ignored names,
not merely their counts.

## Execution Boundary

Use the same RPO source and target only after CPU/owner qualification. Fresh
evidence directories are `rpo-finalizer-tools-v228-v2` and
`rpo-finalizer-tools-owner-v228-v2`. The six phases remain metadata,
finalizer-build, finalizer-build-tests, finalizer-list,
finalizer-ignored-list and finalizer-tests.

Build both `finite_join_engineering_hsaco_v1` and
`finite_join_request_metadata_v1` examples plus the finalizer test ELF using
actual Cargo artifact records. Default tests retain 190 passing/15 ignored;
the actual-capture inert join remains ignored here and must run separately
against a genuine new checked handoff. No old finalizer ELF is substituted.

Preserve offline/locked jobs2, nightly2026-04-03, CPU8/9, nice10, hidden GPUs,
12 GiB AS, 6 GiB aggregate target, 40/38 GiB floors, 1,800-second leaf maximum
and 10,800-second owned deadline. TMPDIR remains inside the CPU target.
All other old targets and all selected compiler products remain pinned. This
controller does not clear caches, modify sources, raise compiler limits or
mint image/certificate authority.

After root freezes the flat three-body package:

```text
taskset -c 8,9 nice -n 10 python3 -B E/p228-partial-move-rpo-finalizer-tools-v2/run.py PACKAGE_SHA ACTUAL_CPU_SHA ACTUAL_OWNER_SHA
```

Pure test invocation is `python3 -B test_run.py -v` inside this package,
under the same bounded CPU-only test envelope. All 16 method names remain.

Root must measure fresh storage and wait for other owned builds before launch.
The schemas are `fe2o3-p228-rpo-finalizer-tools-result-v1` and
`fe2o3-p228-rpo-finalizer-tools-owned-result-v1`. Results retain the actual
compiler generation, source snapshot, three new finalizer product pins, six
phase outcomes, complete named results and before/after custody. Checked
lowering, actual capture join, HSACO emission, GPU, numerical, performance and
production claims remain false.
