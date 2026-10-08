# Scoped Active-Pause Candidate

This is an isolated source and CPU-qualification candidate, not a measured
optimization or a native GPU qualification. The candidate is retained in qualification
archives only; it is not installed on the runtime production branch.
No candidate GPU result, matched gain, full-model decode acceptance or throughput
result has been observed.

## Change

Relative to the qualified closed-layer source, exactly three runtime Rust
paths change: the private paired coordinator, its existing test registration,
and one new private test file. No worker or parent Rust body changes.

For the coordinator's existing `Currentness::Scoped` branch, the pause targets
the existing 50-microsecond budget using at most 4,096 spin hints, then sleeps
only a positive remaining duration. This is a target interval, not a promise
that OS scheduling cannot overshoot it. The `Currentness::Full` branch keeps
its original single 50-microsecond sleep without an extra clock read.
These internal branch names are not the Full2303 workload profile.

The candidate adds no public feature or selector. It applies to the existing
Scoped branch in both diagnostic and default builds. Currentness predicates,
checkpoint placement and per-poll ordering, ownership, poison behavior, completion/durability checks,
kernel arithmetic, wire records and all deadlines remain unchanged. The
diagnostic feature only selects the existing host-duration observations.
Elapsed pause scheduling and observed poll iteration counts can differ; no
currentness predicates or checkpoint positions are skipped or widened.

Twelve new deterministic tests cover the pause and coordinator contracts in
both runtime modes. Existing named outcomes and ignores must remain unchanged.
The source capsule contains the cumulative 31-file Rust overlay and original
preimages/split-repository patches; only the three paths above differ from
the immediate qualified predecessor.

## Observed CPU Results

These are MI350 CPU results from the inherited reduced qualification
workspace, not repository-wide qualification. Counts in the primary column
exclude separately repeated runtime scopes and rustdocs.

| Role | Mode | Primary / Selected Passed | Unchanged Ignored | Owned Phases | Elapsed Seconds | Original Terminal SHA Prefix |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Runtime | Diagnostic | 1,221 | 8 | 20 | 75.875883734 | `c58189db` |
| Runtime | Default | 1,192 | 8 | 20 | 72.781956167 | `224cbfc0` |
| Worker | Diagnostic | 911 | 4 | 8 | 122.739846744 | `9fdc9702` |
| Worker | Default | 838 | 4 | 8 | 121.707246545 | `522bd86b` |
| Parent | Diagnostic | 641 | 0 | 70 | 462.325766680 | `8764fc5b` |
| Parent | Default | 584 | 0 | 67 | 458.703164435 | `ecef2f08` |

Runtime diagnostic/default also pass 131/113 focused repeat invocations and
ten rustdocs each. These are not additional globally distinct tests. The
twelve active-pause tests pass in each runtime mode; they are not 24 distinct
test definitions. Parent diagnostic/default select 641/584 invocations across
61/58 scopes from 1,104/1,047-name library inventories; these are not globally
distinct test totals. All six qualifications completed with natural zero exits,
reaped children and absent owned process groups.

Original capsules:
- [Source](source-v1.tar.gz), including the exact immediate predecessor
  proposal `937393856017d088991e20df697b1b12df449b08a9151e65a968dbdf8603491f`.
- [Runtime diagnostic](runtime-diagnostic-cpu-v1.tar.gz).
- [Runtime default](runtime-default-cpu-v1.tar.gz).
- [Worker diagnostic](worker-diagnostic-cpu-v1.tar.gz).
- [Worker default](worker-default-cpu-v1.tar.gz).
- [Parent diagnostic](parent-diagnostic-cpu-v1.tar.gz).
- [Parent default](parent-default-cpu-v1.tar.gz).

The sealed candidate proposal is 444,678 bytes,
`55f6af2baa21ed8be123ff815265966ba2b098cb856413eabd052fdee3e81161`.
It binds 2,139 project entries, including 834 runtime entries and 242 worker
entries. The runtime count includes the inherited reduced Cargo workspace
and original manifest/lock aliases; no repository root manifest or lock is
changed. The source archive is 847,910 bytes,
`37461ac3a88233531ce307fefde6ecc843c403ccf88de71298e7350b4c9a993d`,
with 67 members. CPU archives retain original commands, streams, source/tool
and product pins, without copying ELF products or private caches.

## Child CPU-Usage Tests

The [14 pure usage tests](usage-tests/test_layer_cpu_usage.py) pass in one
bounded MI350 CPU process in 0.092727 seconds, with sources unchanged.
The [original result](usage-tests/complete.json) is 4,886 bytes,
`2e7b99402411b8ea3872a6cdda48fb2a2a35274085d4739f77847c26c3327b85`.
Its [controller](usage-tests/run_usage_tests.py) retains the three exact source
bodies and original stdout/stderr. No child process, GPU/native execution or
live `getrusage` read occurred.

This covers arithmetic, schema rejection and AST sampling-placement checks
only. It does not qualify a whole native workflow, establish live child CPU
accounting, or measure the candidate's resource cost.

## Native Workflow CPU Qualification

The candidate and unchanged control now pass their separate bounded MI350
CPU workflow qualifications. These use real admitted CPU artifacts alongside
synthetic preparation/retention fixtures; they do not execute the native
model or measure live child CPU usage.

| Workflow | Passed | Owned Phases | Elapsed Seconds | Original Terminal SHA Prefix | Original Archive |
| --- | ---: | ---: | ---: | --- | --- |
| Candidate v2 | 336 | 8 | 168.889884753 | `e18ccf07` | [Candidate](native-admission-cpu-v2.tar.gz) |
| Control v1 | 330 | 8 | 166.223272139 | `6aceca40` | [Control](control-native-admission-cpu-v1.tar.gz) |

Both runs have zero failures, errors or skips, natural zero exits, reaped
children, absent owned process groups and 377 unchanged staged sources.
Each archive has 423 members, including 422 pinned originals and 42 raw
evidence files. The candidate archive is 9,048,216 bytes,
`a0a51e79b5b088556ca79232da47c7247a33af352df87e93c9ae3637f941641c`;
the control archive is 9,046,022 bytes,
`43ac0ac1c84d225688bf258864ecf2608176d24a82100374b3c47c1cec8a5ff4`.
Independent data-only review rehashed every archive member and joined every
retained original to its run source, raw evidence or authenticated stager.

The totals overlap: each includes the same 227 predecessor workflow tests
and 14 usage tests, alongside the arm-specific admission/preparation/retention
suites. They are not 666 distinct tests or native measurements. The earlier
candidate 335-test v1 was staged but unrun; it is neither a failed attempt nor
credited qualification. CPU qualification elapsed time is not an A/B native
performance result. Live resource accounting and original native case
retirement remain to be observed.

## Measurement Gate

The [preceding closed-layer measurement](../guarded-mlp-layer-phases-v1/native-gpu-v1/README.md)
recorded 30.941303993 warm seconds in paired poll inside 33.483701772 paired
body seconds. Warm forward bodies totaled 68.141092151 seconds. These are
nested host intervals containing required checks and device waiting, not
GPU compute, sleep-only cost or a removable fraction.

All six artifact qualifications and both CPU workflow qualifications are
complete. The next gate is fresh control/candidate/candidate/control (ABBA)
GPU cases run serially, each with its own observed preparation and bound
native admission.
A is the unchanged qualified closed-layer runtime; B is this isolated
candidate. Each arm must have its own matching runtime, worker and parent
artifact bindings, the same model/image/prompt pins and same 40-forward
Readiness profile, captures at 0/5/16/39, unchanged three-record validation,
40 semantic rows and numeric-payload parity checks. Generated tokens remain
zero. Fresh sessions and original per-case evidence must be retained; the
historical measurement is not the fresh control arm.

Keep the 4,000-second native leaf and 4,300-second whole-case bounds, initial
40 GiB/live 38 GiB storage floors, CPU affinity 8/9 and nice 10. CPU
qualification hides GPUs; native cases use only their admitted device set.
Report paired poll and complete warm-forward body separately. Promotion
requires an improvement in both, with matched CPU/resource consumption
reported alongside the timing results. Whole-case user/system CPU, resource
counters and peak memory must be labeled at their actual measurement scope,
not presented as warm-only costs. Do not subtract cumulative peak RSS values
or mix nested timing levels into an additive savings estimate.

No measured gain is claimed until that gate closes. A lower polling interval
alone does not establish faster warm forwards, and increased CPU consumption
must remain visible. No tolerance, exact-output requirement or abort bound
may be relaxed to obtain a favorable result.

## Still Open

Native Full2303 feasibility within the unchanged one-hour bound, 2,048 prompt
tokens followed by 256 own-generated outputs, exact independent agreement of
all 256 generated IDs and decoded bytes, and the 700 tokens/s target remain
unmet. This TP2 short profile does not satisfy the original single-GPU matrix.
Issue #42 M0-M7 remain open. See the [decode gates](../guarded-mlp-readiness40-causal-layer0-v1/DECODE-GATES.md)
for the distinction between prompt-position diagnostics and generated-output
acceptance.
