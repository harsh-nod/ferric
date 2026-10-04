# Parent Device-Observation CPU Evidence V2

This publisher is a standard-library-only retained-evidence replay. Its author
has not imported or executed it. Root must supply actual successful CPU and
pure receipt SHA256 values; no expected successful result or parent binary hash
is invented by this draft. The corrected CPU-v2 package is bound to its actual
frozen manifest `753760a5ea122e0d2d0c47fe393d9171c41efef94917bae0011738ae934e69f1`;
this source-package identity is not a successful CPU qualification.

The first actual parent attempt is retained as a failure. Its compiled inventory
was correct, but one new test used `+= 1` on an intentionally maximal `u64`, so
the test panicked before its intended validation call. That parent-client leaf
reported 146 passes and one failure. Root repaired only the fixture mutation to
`^= 1`, preserving the intended corruption and every test name. No production
change or numerical-policy relaxation was used to make that fixture proceed.
The failed receipt SHA256 is
`d5fae24609182d0b8cb3818b3fd67c9647fb7efe8d32f0046440dbb540b52e57`.
V2 success requires a separate actual CPU run and sixteen-test pure run; the
first run is never counted as a successful qualification.

It uses the exact previously published
`qualification/native-device-routing-v1/publish.py` (SHA256
`af21f9226556d79e9a8b3ae9a96b759808ccaca573f348eaa575eb0c8049d9b9`)
only for its bounded read/pin, strict JSON, archive-map and test-result helpers.
The helper is authenticated before loading under a non-main module name; its
original `main` is not called. No qualification controller, test suite, native
executable, compiler or subprocess is imported or executed. The result records
this existing repository dependency explicitly rather than duplicating it.

Required arguments are `--cpu`, `--cpu-sha`, `--pure`, `--pure-sha`, `--prior`,
`--parent`, `--repo`, `--package`, `--source-inputs`, `--archives`,
`--pure-controller`, `--failed` and `--publish-to`. Paths identify local retained copies;
original remote FilePins stay unchanged. `--prior` selects the retained actual
CPU633 directory, not the CPU609 worker result. `--failed` selects the retained
first parent run containing `failed.json` and all eighteen raw records. They
are authenticated, including the natural failing exit/reap and actual test
diagnostic; only the failure receipt and five parent-client diagnostic files
are copied into the new publication. `--parent` is the separately
retained new parent ELF. The fresh destination must be directly under the
specified repository's `qualification` directory.

The replay checks all 41 new parent phases, 208 raw records, natural exits and
reaped groups, the exact stable command/environment changes from the old 38
parent leaves, complete old-plus-new library inventory, 245 disjoint selected
library passes and twelve bin passes. Nested device tests are counted only once
inside the old parent-client filter. It requires 257 actual parent passes and
zero ignores, plus sixteen exact named pure outcomes. Historical worker tests
are not rerun and are not added to this count.

Source archive maps are reconstructed with `tarfile` without extracting another
source tree. Exact archive/commit/tree pins, seven preimages and overlay bodies,
full compiled before/after maps and the seven integrated live source hashes are
joined. The actual parent metadata retains 209 packages (28 local and 181
external), with exact relocated manifests and dependency identities. All twelve
build artifacts are joined to raw Cargo messages, while only the selected new
parent executable's retained bytes are locally rehashed and checked as ELF.

Audit limits are explicit: the CPU controller itself recorded before/after
custody, but this local publisher does not independently rehash every one of its
233 input bodies, external dependency manifest bodies, or all twelve parent
executables. Corresponding result fields are false. It validates the actual
selected parent and all supplied retained records, not a new remote build.

Publication includes the replay source, frozen CPU package, source manifest,
actual CPU/pure records and bounded raw evidence, with a 64 MiB total limit. It
does not duplicate archives, binaries or the seven integrated Rust source files.
Parent-only CPU success is not native execution, GPU validation, timestamp
calibration, cross-device clock alignment, overlap, numerical acceptance,
full-model acceptance, a performance result, or production authority. Those
claims remain false. Root must still qualify and audit the new parent/worker
combination before a bounded device-timing GPU attempt.
