# Default-Off Inert Semantic Extraction Selector

This CPU-only component advances Ferric [issue #42](https://github.com/harsh-nod/ferric/issues/42)
after the [selected-invocation transport component](../guarded-mlp-inert-v3-invocation-v1/README.md).
It adds a real early selector preflight to the protected binding wrapper and
a staged eligibility check requiring retained sealed parent custody.
**The extraction-only child consumer remains unqualified and disabled.**
No worker compilation, HSACO, GPU, numerical/performance result or M0-M7
completion is claimed.

## Implementation

The [selector module](source/crates/cargo-fe2o3/src/protected_compiler_handoff_v3/inert_semantic_extraction_v1.rs)
recognizes exactly this versioned request:

```text
FE2O3_INERT_SEMANTIC_EXTRACTION_V1=semantic-v3-gfx950-inline16384-v1
```

This is **not a runnable feature switch**. A requested route currently returns
an explicit consumer-unqualified refusal, before tool pinning, managed attempt,
capability admission or rustc spawn. There is no publication fallback. With
the selector absent, ordinary preflight succeeds unchanged, except that
caller-supplied raw invocation descriptors are now explicitly refused.

The [binding-wrapper entry](source/crates/cargo-fe2o3/src/binding_wrapper.rs)
calls this preflight first. It does not mutate the environment, inject a raw
descriptor, create output or execute rustc. Empty, unversioned, duplicate and
non-UTF-8 selectors are rejected. Existing `FE2O3_EXTRACT_*` modes conflict
with the opt-in; their presence without the opt-in does not activate this route.

The separately staged eligibility method accepts a **live retained
`ParentRustcInvocationCustody`**, not raw schema bytes or a boolean claiming
authentication. It revalidates the sealed parent image and captured descriptor,
holds a borrow of that original parent, and produces the existing inert
`SelectedInvocationTransportV3`. It has no conversion to protected publication
or execution authority. No real protected compiler child or successful live
eligibility was executed in this campaign; successful descriptor tests use
explicit schema fixtures only.

Eligibility requires the exact gfx950 AMD target, AMDHSA triple, CPU/features,
opt-level3, always-encode-MIR, inline-MIR, hint16384 and disabled JumpThreading.
The parser walks the existing frozen rustc separate-value option table so
`--cfg` values cannot impersonate profile flags. Missing settings, duplicate
controlled options including underscore aliases, changed hints/target/features,
and extra MIR-normalization overrides are refused. The exact transport still
binds every process input and all six actual closure roles supplied by custody.

## Actual Qualification

All work ran on mi350 CPU8/9, nice10, jobs2/offline, with owned process-group
supervision, 12 GiB service memory maximum and swap disabled.

| Run/Cohort | Actual Result |
| --- | --- |
| First: invocation library | 103 passed |
| First: preserved engineering frontend | 25 passed |
| First: capture/closure upgrade | 8 passed |
| First: new selector tests | 10 passed |
| First controller | Failed after 11 clean leaves: wrong requested wrapper cohort name |
| Recovery: actual wrapper lifecycle cohort | 6 passed |
| Recovery: prior qualified compiler gate tests | 2 passed |
| Recovery controller | Passed, 3 clean leaves, 2.508988 seconds |
| Combined named outcomes | 154 passed, no completed cohort repeated |

The first harness asked for nonexistent `binding_wrapper::tests::` instead
of actual `binding_wrapper::lifecycle_tests::`. The
[original failed receipt](cpu/first/failed.json), source, logs and test ELFs
remain unchanged. Its 146 completed tests passed, but the original controller
is still a failure, not relabeled as success. Its postchecks passed over
5,813 source inputs; elapsed75.744807 seconds. The driver's five ignored
declarations remain unchanged and unrun.

The fresh [recovery runner](recovery/run_cpu.py) binds the exact failed receipt,
all original raw evidence/source/tool/artifact pins, and reparses every prior
named outcome. It submits **only** the six never-submitted lifecycle tests
and two gate tests. No Rust build or completed test cohort is repeated.
All 5,889 recovery input pins remain unchanged. The two compiler gate tests
reuse the prior qualified compiler ELF; the full compiler/device/lowerer
suite is not rebuilt or rerun. All 14 submitted leaves across the two runs
exit naturally with status0, their leaders reaped and process groups absent.

See [recovery receipt](cpu/recovery/complete.json),
[selector outcomes](cpu/first/selector-tests.stdout),
[lifecycle outcomes](cpu/recovery/binding-wrapper-lifecycle-tests.stdout),
and [publication audit](publication-audit.json).

## Remaining Integration

The real protected child-rustc path already carries exact V3 inputs in a sealed
capability, but its [parent completion intake](source/crates/cargo-fe2o3/src/protected_compiler_handoff_v3.rs)
expects a protected publication transaction. The [backend entry](source/crates/rustc-codegen-fe2o3/src/lib.rs)
requires protected admission and completes that transaction before host LLVM
codegen. Merely enabling the existing semantic extraction branch would not
provide a separate extraction-only lifecycle or lossless consumer.

Next is a bounded separate extraction-only parent completion/consumer component:
retain the exact invocation and outer conditional capsule/premises, reject
ordinary V2 intake and protected publication adoption, and qualify CPU controls.
Do not remove this request refusal until that complete route is qualified.
If the real protected-child route cannot support that separation, use a distinct
versioned in-process extraction schema, not fabricated argv or wrapper roles.

The engineering callback route is still incompatible with frozen child-process
V3. The eight runtime conditions, early-stop CFG issue and GPU completion hold
remain unresolved. This component grants no signed-Verus, compiler-refinement,
numerical, artifact, load or launch authority, and does not satisfy 700 tokens/s.

## Provenance

[First runner](first/run_cpu.py), [input preparer](first/prepare_inputs.py),
[recovery runner](recovery/run_cpu.py), [exporter](export_combined.py),
[originals ledger](originals-ledger.json) and [archive](originals.tar.gz)
preserve the exact originals. The data audit rehashes all recovery inputs and
both runs' raw evidence before export. The archive contains relevant source,
controllers, receipts and raw logs, not complete compiled/toolchain/source trees.
Artifact hashes and complete source rosters remain in the receipts.
