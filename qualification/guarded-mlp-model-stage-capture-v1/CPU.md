# Capture CPU Qualification

These isolated CPU recipes qualify the additive stage-capture source proposal
`source-manifest.json` (7,739 bytes, SHA-256
`2dc554b35f227180b1367f926d38e3332305f99666d9f9bd52b941240d1d899e`).
They do not run a model or GPU and do not establish numerical or performance
acceptance. The source proposal, historical evidence, and canonical repositories
are not changed by these scripts.

## Worker

The worker quartet is `worker_cpu.py`, `pack_worker_cpu.py`,
`stage_worker_cpu.py`, and `stage_worker_cache.py`. It derives from the actual
qualified worker V6 controller and preserves its nine leaf recipes, bounded
owned-process supervisor, offline/GPU-hidden environment, toolchain, private
29-package cache, and 807 qualified runtime source bodies. The fresh remote root
is `E/guarded-mlp-stage-capture-worker-cpu-v228-v1`, where `E` is
`/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`.

The actual V6 baseline is pinned by its receipt `927e6519`, source map
`0f00c39f`, and full raw list/test streams. Its 590 named outcomes (586 passes,
four ignored) must survive unchanged. Exactly five capture library tests are
added. Conditional success is therefore 595 names, 591 passes, zero failures,
four ignored, and four selected Cargo artifacts. No ignored test is executed.

The source pack has 190 ordinary bodies, including all 181 worker files, two
controller files, and seven local lineage/input bodies. Its input source map
contains 990 rows: 181 worker, 807 runtime, and two harness files. The runtime
bodies are copied from the authenticated qualified interface tree, not from a
build cache. The eight selected lineage records include that interface's receipt
and source map plus the worker baseline receipt, raw tests, raw list, source map,
committed-source snapshot, and current capture proposal.

Local assembly: `python3 -B pack_worker_cpu.py`. The source stager accepts the
actual archive SHA and input SHA; the cache stager accepts the unchanged actual
cache archive SHA and manifest SHA. The remote controller is staged as
`run_cpu.py` and accepts the actual input SHA. Every output namespace is fresh.

## Parent

The parent quartet is `parent_cpu.py`, `pack_parent_cpu.py`,
`stage_parent_cpu.py`, and `stage_parent_cache.py`. It derives from the actual
parent V3 controller, preserves all 54 phase recipes and 46 selected test scopes,
and retains the same Rust 1.97.1 tools, lockfile, 118-package/three-Git dependency
cache, owned-process supervision, and resource ceilings.

The current committed source revision is
`90b17ca054631aea59b03d50169d114c4663468f`. Its 1,217-file Ferric closure is
checked against the actual parent V3 source map `a88284ad`, updated only by the
181 actual worker V6 bodies from `0f00c39f`. Exactly eight inherited formatting
differences are required before applying the new capture overlay. The capture
overlay itself has eight worker replacements and four parent rows (two
replacements, two additions). No historical source mismatch is silently accepted.

The parent source map contains 1,222 files: 1,219 Ferric bodies and three harness
files. Its archive has exactly 1,325 ordinary members, including 102 selected
lineage bodies and the input manifest. The complete old 848-name library
inventory, ten earlier guarded additions, and four new capture names yield
862 library names. The existing parent-client scope selects the four new library
tests; the guarded binary gains one test. Conditional success is 380 selected
passes, zero failures/ignores, over 46 scopes. The entire parent library suite is
listed but is not executed. Four old parent binaries plus the guarded binary
remain required products.

Local assembly:
`python3 -B pack_parent_cpu.py NEW_OUTPUT_DIR ACTUAL_PARENT_CACHE_MANIFEST`.
The reviewed parent source-stage template (`a30b08cc`) left `ARCHIVE_PIN` and
`INPUT_PIN` as `None`. Primary integration subsequently bound only those two
actual assembled pins, yielding `1c559446`; its fixed member count is 1,325.
The fresh remote root is `E/guarded-mlp-stage-capture-parent-cpu-v228-v1`.
The cache stage changes only that root relative to the qualified V3 recipe.
The remote `run_cpu.py` accepts the actual input SHA.

The parent keeps the old original proposal/snapshot as historical test-roster
lineage. Current source authority comes from the actual V3/V6 maps, the exact
committed revision, and current capture manifest, not those older postimages.
Both controllers report `capture_source_added=true` and
`capture_native_execution=false`; neither grants native/model authority.

## Worker Retention

`export_worker_cpu.py` derives from the executed V6 exporter. It is bound to the
observed current input `91564acc`, controller `4b290e97`, source stage
`686e3d82`, and cache stage `69fe8d71`; its argument is the actual terminal SHA.
The observed terminal body at
`W/guarded-stage-capture-worker-complete-v1.json` is 1,607,204 bytes,
SHA-256 `64855967eb15f5b5b5fadb98cbc30e8e382968099ec439090562cacc174eec47`.
That body reports all nine phases and 591/0/4 outcomes. Full raw-byte closure is
checked by the exporter and independent capsule audit, not inferred from this
summary.

Conditional clean-success retention is 256 members, 255 pins, and 50 raw bodies.
It preserves every worker source body, five runtime manifests, all eight selected
lineage bodies, input/controller/supervisor/stage/cache provenance, complete raw
streams and full source/dependency pin maps. It rehashes all current source and
external dependency bodies before export. Host ELF bodies and dependency package
bodies are not copied. Clean natural failed prefixes remain failed and are not
promoted. Existing 320-member/64-MiB export caps and terminal lifecycle checks
remain unchanged.

`retain_worker_cpu.py ARCHIVE SHA256 BYTES FRESH_DESTINATION` verifies the closed
manifest/body set before writing the fresh local capsule. No future archive pin
or parent success is asserted by this proposal. All assembly, staging, CPU
execution, binding, and canonical retention are primary-agent actions; the
author performed source/AST/data review only.
