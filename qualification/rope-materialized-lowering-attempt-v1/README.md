# RoPE Checked Lowering: Retained Refusal

The CPU-qualified BF16 RoPE candidate did **not** produce a GPU image in this
attempt on ASROCK through `mi350-2`. The checked Rust frontend rejected it:

```text
production semantic SSA partial-move validation for function 0 requires 2097153 storage words, limit is 2097152
```

The reported number is the first over-budget accounting point, not a measured
total memory requirement or evidence that increasing the limit by one would
complete compilation. Compiler resource limits and proof checks were unchanged.

## Actual Outcome

| Stage | Outcome |
| --- | --- |
| Existing CPU arithmetic qualification | 33 tests passed, including explicit exhaustive run |
| Separate lowering-controller policy tests | 14 passed |
| Fixture metadata | Exit 0, naturally completed and reaped |
| Checked Rust extraction | Cargo exit 101, storage-limit diagnostic |
| Semantic replay, finalizer, HSACO inspection | Not reached |
| GPU execution | Not attempted |

The owning process tree exited with code 1; all owned processes were reaped and
groups were absent without forced cleanup. Both scopes' input postchecks passed.
Recorded source, configuration, fixture, dependency and preserved-target
snapshots were unchanged. The candidate's ten lowering input files match the
[CPU-qualified source and original Cargo pair](../rope-materialized-cpu-v1/README.md).

An earlier invocation stopped even before compiler launch because the package
manifest used an incorrect JSON closing delimiter. The corrected manifest kept
all four controller source bodies unchanged. Both that rejected manifest and a
primary-agent tool-output observation are retained under [observations](observations).
The fourteen policy tests predate the delimiter correction and exercised the
unchanged controller bodies; they did not validate that manifest's JSON syntax.
Neither primary-agent observation is a remote process-supervisor receipt.

## Evidence

[failed.json](failed.json), the [owner result](owner/failed.json),
[raw compiler stderr](raw/checked-lowering-stderr), the [recipe](recipe.json)
and [controller](controller) retain the actual attempt. The failed compiler
stage is recorded in its own raw files even though the controller's successful
command list contains only fixture metadata.

[result.json](result.json) records copied-file hashes and the checked courier
archive. Publication rehashed both scopes' retained raw snapshots and the ten
fixture bodies; the larger snapshots remain outside Git. It did not rerun the
compiler or rehash every transitive tool/library body. No emitted image exists
for this attempt, and publication success is not compilation success.

Inner failure SHA-256:
`32aeb4127b38d0f50aabba67afd907f369185b46b64ebc012e2425acf31294f0`.
Owner failure SHA-256:
`46586eedb8fc35243c5e4bc920ef35e17323a1c0e371d0a33a8f94df6e9cf99e`.

## Next Candidate

The [next source experiment](../rope-materialized-cpu-v2/README.md) replaces the pair-validity expression's
short-circuit conjunctions with eager boolean conjunctions. All operands are
pure finite checks of already-computed values. The aim is to reduce control-flow
branches without removing any checks or changing arithmetic. That explanation
is a hypothesis about this refusal, not a successful compiler result. Its fresh
CPU qualification passed 33 tests, but its subsequent
[checked lowering](../rope-materialized-lowering-attempt-v2/README.md) also hit
the same storage limit. Neither attempt produced a candidate GPU image.
