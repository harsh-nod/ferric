# Ordered Residual / MLP CPU Qualification V2

Source-only draft. Root must review the unchanged runtime-v1 and corrected
Ferric-v2 proposals, write and pin `inputs.json`, run the policy tests, then
execute this controller.
No test, compiler, GPU or numerical result is implied by this document.

The baseline is the actual CPU883 paired source, not a fresh Git checkout or
the current publication tree. The controller verifies its completion, all raw
records, four executables and 7,003-file source map before copying to a fresh
directory. Both overlays require exact preimages. Only fe2o3-kfd Rust files and
Ferric adapter Rust files plus one additive parent Cargo target may change.
Formatting is restricted to those selected Rust files. Lockfiles, dependencies,
features and build profiles remain unchanged.

## Generation Boundary

The first controller reached the final `parent-default-check` only after 1,848
selected test passes, seven historical ignores and five production builds, but
that final check failed. Those passes do not qualify the failed generation.
Ferric-v2 adds the missing parent feature gates; the runtime proposal and
ordinary CPU883 source baseline remain unchanged.

This controller selects only `p228-projection-residual-mlp-ordered-runtime-v1`
and `p228-projection-residual-mlp-ordered-ferric-v2`. It leaves all v1 source,
receipts and executables untouched, includes the failed v1 target in the old
target inventory, and requires a fresh v2-or-later output label. There is no
resume from cached tests, no skip of the default-feature check, and no recipe
reordering. All original qualification gates remain.

Input and result schema strings stay v1 because the record shape is unchanged.
The exact v2 controller, root plan and Ferric proposal pins distinguish this
generation. Old consumers bound to the v1 controller/source proposal must not
be relabeled as accepting this controller. The primary owns the actual plan
and any later measured successful completion; no future result is assumed.

## Coverage

The complete fe2o3-kfd library inventory is compared with the actual 900-name
gfx950-clock baseline plus explicitly declared additions. Its full default suite
is selected through the worker manifest with engineering-gfx950 enabled and
live-validation absent. A read-only source review found CPU-only tests, including
mock device checks, temporary fake sysfs, anonymous mappings and self-exec
abort/poison tests. Three existing retained-image tests remain ignored; they are
not counted as passing. No `--include-ignored` or live examples are selected.

All 883 prior selected worker/parent tests remain, with the same four historical
worker ignores. The new runtime, worker, parent-library and parent-binary tests
must appear in their respective compiled inventories. New parent tests outside
the old selectors are selected individually with `--exact`. Nothing is accepted
from a source-only test count. The required passing total is computed from the
actual named inventories; the total ignore count is seven.

Five production executables must be built: the existing worker and three
selected parents, plus the distinct ordered-route parent. This is not a new
compiler qualification, GPU execution or numerical acceptance.

## Bounds

Use ASROCK, UID9661, CPUs8/9, nice10, hidden GPUs and the existing pinned compiler
tools. Each leaf retains its prior deadline, a 12 GiB address-space limit and
disabled core dumps. The fresh aggregate target is capped at 6 GiB, with initial
40 GiB and ongoing 38 GiB free-space floors. Environments are copied from the
qualified recipes, not inherited; no FE2O3_TEST child-selection flags are added.
Every owned leaf must exit naturally and have no remaining process group.

Source, baseline source, tools, proposals, old targets, dependency manifests,
configuration and all executable identities are checked afterward. No old target
or source tree is modified. No cleanup or second compiler run may overlap this run.

After all inputs are frozen and checked, the invocation is:

```text
taskset -c 8,9 nice -n 10 /usr/bin/python3 -B \
  E/p228-projection-ordered-segment-cpu-v2/run.py ACTUAL_CONTROLLER_SHA \
  E/p228-projection-ordered-segment-cpu-v2/inputs.json ACTUAL_INPUTS_SHA \
  projection-ordered-segment-cpu-v228-v2
```

Here `E=/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220`.
First run `test_run.py -v` with a bounded CPU-only invocation. Seventeen authored
synthetic methods preserve all fifteen v1 tests and add proposal/output-generation
refusal cases. None was executed by the author. These admission tests never call controller `main`, copy source trees or launch compilers.
