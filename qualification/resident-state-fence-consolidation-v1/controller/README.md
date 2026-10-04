# Resident-State CPU Controller

Unexecuted V2 package derived from the CPU-qualified resident-state V1 runner.
The only runner changes select the V2 package and source directories. The
source bodies differ only by removal of one redundant final blank line from
six files; the other two sources and all preimages are byte-identical. V1
remains unchanged. Its results do not qualify the V2 byte generation; root
must repeat the same complete 522-test cohort and worker build.
No imports, syntax checks, tests, Cargo builds, SSH commands, or GPU work
were performed by the author. Root must review/freeze the package and own
all execution and publication.

## Inputs and Source Join

Stage this three-file package plus manifest.json at
E/p228-resident-state-fence-cpu-v2, where
E=/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220.
Stage the exact eight V2 source bodies and unchanged preimages.json at
E/p228-resident-state-fence-consolidation-v2.

The runner requires the existing exactly pinned host-policy CPU helper,
source-archive input record, extractor, bounded helper, two Git archives,
13-file host-policy overlay and three-file group-fence overlay. It creates
a fresh full source pair, applies those original overlays, and only then
validates all eight resident-state preimages against the resulting bytes.

overlay.json pins the eight new source bodies and the author's preimages.
Its baseline commit725ecc6a records the review origin; it is not falsely
equated to the older6964f612 archive. Each sequential overlay validates
physical file bytes/hash and the full current snapshot before proceeding.
Unlisted source/lockfile changes are refused.

## Root Invocation

Pass the actual package-manifest SHA and a new output label:

```sh
taskset -c 8,9 nice -n 10 /usr/bin/python3 -B \
  "$E/p228-resident-state-fence-cpu-v2/run.py" \
  "$ACTUAL_PACKAGE_MANIFEST_SHA256" resident-state-fence-cpu-v228-v2
```

The existing bounded helper remains byte-identical. It checks ASROCK UID/host,
affinity8,9/nice10, 40GiB initial free space and38GiB during each command,
12GiB address space, jobs2, 6GiB target,64MiB streams, deadlines and owned
process-group reap. No GPU visibility is enabled. The source/target directory
is new and exclusive; external Cargo dependency cache reuse is explicit.
Nothing is deleted. Prior targets are neither selected nor reused.

## Actual Result Gates

The runtime inventory must yield124 disjoint selected tests: the prior77
plus prefix-state10, prefix-resident10, MLP-state10, MLP-resident10 and
MLP-timestamp7. All six new tests must be present. Exact raw test outcomes
must match their compiled inventory with no ignored runtime tests.

The unchanged worker inventory must contain402 names and actual summaries
385passed/4ignored plus13passed, giving398 worker passes. The subsequent
worker build must yield the authentic opt2 non-test compiler artifact at
the fresh target path. A complete receipt requires522 actual passes and
four ignored tests, plus successful source/tool/input/dependency/binary
and target-cap postchecks. These numbers are expectations, not results
established by this draft.

The17 bounded leaves preserve full command/start/stdout/stderr/result records.
On a caught command/test/build failure, postchecks still run and failed.json
is written; no success receipt is synthesized. Prerequisite failures before
the bounded test phases fail immediately.

This qualifies only selected CPU runtime regressions and a new worker build.
It neither rebuilds the parent nor executes a long session, produces a GPU
result, proves coherence, grants numerical/production authority, or measures
a speedup. Root must renew runtime custody and perform the separately scoped
same-input V7 GPU validation before adopting the runtime change.
