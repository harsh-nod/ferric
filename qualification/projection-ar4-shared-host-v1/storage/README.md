# Performance Adapter Cache Reclaim

Source-only draft. No tests, remote census, plan, deletion or build was executed
by the author. Root owns qualification, plan review and apply.

The only removable paths are direct ordinary children of
`/home/harmenon/ferric-asrock-42/target-performance-adapter/debug/deps` with
suffix `.rlib`, `.rmeta`, or `.o` that pass the unchanged reviewed classifier.
The classifier permits only host x86-64 ET_REL `.o`, never executable/shared
ELF or AMDGPU code objects. All hardlinks, executable modes, explicit pins,
backend aliases containing `rustc_codegen_fe2o3`, and all other files stay.
No directory is removed. The entire target's other ordinary bodies and
directory roster are rehashed/compared; source/model/archive trees are not
eligible. Symlinks or other nonordinary entries in the target refuse the run.

The historical task helper retained locally at
`p218-inputs/evidence/full-forward-rmsnorm-v15-prep/clippy_reuse.py:28`
names this target in `CARGO_TARGET_DIR`. That establishes task use, not a
completed cohort. No missing legacy owner or terminal success is invented.
Root has separately measured cache availability; that measurement does not
authorize a particular deletion. Only the exact plan does.

## Reused Policy

The adapter authenticates these three existing E-relative files, unchanged:

- `reclaim_legacy_build_caches_p228_v4.py`, SHA
  `d5f5df32e267d4a614c845353a97892e413d65d5fa6fb24d25d23f3d27c2fc3f`:
  original bounded terminal receipt census, narrowed to this one target.
- `reclaim_rope_retry_caches_p228_v1.py`, SHA
  `8378bbcfff5578fc0f4227788ece33714b5c86c23780ecd777cfb6a6ff95d743`:
  unchanged ordinary-entry and disposable-file classifier.
- `reclaim_projection_decode_caches_p228_v1.py`, SHA
  `e80436177cd09e66338950d1aa09773be00447a0c575c539347b6159e84a328a`:
  unchanged file pin, plan writer and same-UID process/cwd/map quiescence.

The terminal census preserves every referenced target path even if historical
hashes conflict. An additional bounded census covers nonterminal JSON names
containing `manifest`, `candidate`, `retained` or `inputs`, using the original
source/cache/target walk exclusions. Absolute target-path keys and string
aliases are also protected, not just current FilePin shapes. Documents are
rehashed before selection and after deletion. This is not a claim that every
arbitrary external JSON document or build dependency has been discovered.

The old process policy and 2-GiB address-space/180-CPU-second/16-MiB output/core0
limits are unchanged. Full-target inventory is bounded to 200,000 entries,
64 GiB of bodies and 2 GiB per body. Receipt/manifest scanning is bounded to
300,000 visited entries, 32 MiB per JSON and 1 GiB aggregate. No cap is bypassed
on refusal. Root should run a bounded outer timeout with no concurrent builds.

## Root Commands

Transfer `retire.py` and `test_retire.py` together to a fresh E package directory
`p228-performance-adapter-cache-reclaim-v1`; retain the three helpers above at
their existing paths. The twelve synthetic tests exercise only adapter scope
and protection, never plan/apply, `/proc` quiescence or deletion. They do not
requalify the historical classifier.

```sh
cd /home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/p228-performance-adapter-cache-reclaim-v1
timeout 120s taskset -c 8,9 nice -n 10 python3 -B test_retire.py
timeout 900s taskset -c 8,9 nice -n 10 python3 -B retire.py plan
timeout 900s taskset -c 8,9 nice -n 10 python3 -B retire.py apply ACTUAL_PLAN_SHA256
```

`plan` creates only fresh E/`performance-adapter-cache-reclaim-v228-v1/plan.json`.
Review actual allocated bytes, candidates and protected paths before `apply`.
Apply reconstructs the same selection and all protected hashes, checks fresh
quiescence, then rechecks each inode/device/UID/mode/link/size/time/allocated
stamp and content immediately before unlink. `removed.jsonl` records progress;
an interrupted apply must not be blindly retried. Success requires all other
target bodies/directories, documents and helpers unchanged plus final quiescence.
The plan/complete records are cleanup observations, not compiler/GPU authority.
