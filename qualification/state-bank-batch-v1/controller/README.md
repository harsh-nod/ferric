# State-Bank Combined CPU Qualification Draft

This is a new candidate qualification, not an amendment to the actual prior
522-pass/4-ignored result. No author imports, syntax checks, tests, builds, or
GPU work have been performed. Fourteen pure test methods are authored here;
they test closed source/recipe helpers, not actual archive reconstruction or
Rust behavior. Root owns freezing and execution.

## Inputs to Freeze

`overlay.json` binds both descriptor FilePins and all ten `after` content pins
from the root-owned formatted source bytes retained locally. All seven existing
`before` pins and three null preimages are preserved. Source directories are the separate formatted successors
`p228-state-bank-batch-runtime-v2` (flat source bodies) and
`p228-state-bank-batch-ferric-v2` (nested draft bodies); original V1 proposals
are not changed. The runtime preimages document and Ferric source-pins document
are authenticated and cross-checked against these ten rows.

Freeze exactly `run.py`, `overlay.json`, `test_run.py`, and `README.md` in a
manifest with schema `ferric-p228-state-bank-batch-cpu-package-v1`; each files row
has `path`, `bytes`, and `sha256`. The caller supplies its actual digest. Root
can add advisory pure-test metadata but must not invent execution evidence.

## Source and Test Joins

The runner authenticates the actual CPU522 completion digest, all 37 retained
input pins, all 88 raw files, seventeen exact command/environment/result
records, successful named test results, and the original retained worker.
It does not require the retired old compiled source tree or dependency output
directory. Historical command reconstruction changes only string roots in
the unchanged environment helper; it does not execute old commands.

Fresh original archives are overlaid with host-policy13, group-fence3, and
resident-state8. Their complete intermediate 6,928-file source map must equal
the actual CPU522 source maps. Then runtime7 and Ferric3 are applied by byte
preimages. Seven replacements plus three additions yield exactly 6,931 files.
Every overlay is joined to actual preceding bytes, not a newer Git label.
Source maps are retained as sources-base, sources-prior, sources-before, and
sources-after. The latter two must match exactly.

The original seventeen Cargo phases remain. Four more runtime selectors are
added: ten new state-bank tests and three existing real mapped-atomic groups
of 4, 4, and 2 tests. The full compiled runtime inventory must equal the old
actual runtime inventory plus exactly ten named additions; all old runtime
selector names and all six prior private-accessor additions remain selected.
The full worker lib/shared_wire inventory must equal the old 402 names plus
exactly eleven named additions. Four ignored names must remain identical.

Only after those compiled inventories match are successful outcomes accepted:
runtime 144, worker 409, total 553 passed and four ignored, over twenty-one
owned leaves. Shared-wire's thirteen tests are counted once in this worker
cohort. This does not execute the entire fe2o3-kfd suite; it executes the same
old selected runtime coverage plus the explicitly listed additions.

The worker is built using the original default/test Cargo profile semantics
and inherited opt-level 2 overrides, not a new optimization/profile change.
Actual Cargo artifact metadata must identify the selected binary in the new
target. The old worker and all consumed immutable inputs are rehashed after
the run. No old result, binary, or target is overwritten.

## Bounds and Invocation

Reuse the unchanged pinned `bounded.py`: ASROCK uid/host, CPU affinity 8,9,
nice 10, two serial Cargo jobs, empty GPU visibility, 12 GiB leaf address-space
limit, 6 GiB target cap, 40 GiB initial and 38 GiB active free-space floors,
64 MiB streams, 1,200-second phase deadlines (metadata 120), and original
TERM/KILL/wait/group-absence behavior. No bound is raised, no old target reused,
and no cleanup is performed. Python `-O` and any `PYTHONOPTIMIZE` variable are
rejected by a non-assert guard before authenticated helpers are loaded.

After root pure tests and final source freeze, the root-owned CPU launch is:

```text
taskset -c 8,9 nice -n 10 /usr/bin/python3 \
  /home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/p228-state-bank-batch-cpu-v1/run.py \
  ACTUAL_MANIFEST_SHA state-bank-batch-cpu-v228-v1
```

Use a fresh label matching `state-bank-batch-cpu-v228-vN`. Result schema is
`ferric-p228-state-bank-batch-cpu-result-v1`; it adds `prior_cpu_complete` and
uses `overlay` for the combined source input. Successful raw census is 109
files (21 leaves times five records plus four source maps). Sources, raw logs,
dependency manifests, tool pins, and artifact pins are checked after the run.
Failures retain their completed leaf evidence and do not become successes.

This controller establishes only CPU source/build/test evidence. It makes no
GPU, numerical, performance, production, whole-model, or 700-token/s claim.
Fresh deployment/runtime review and bounded GPU comparison remain separate.
