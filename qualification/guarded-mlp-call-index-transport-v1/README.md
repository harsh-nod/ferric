# Checked Index-To-U64 Helper Arguments

This engineering checkpoint follows the
[observed lowered helper-argument mismatch](../guarded-mlp-call-type-diagnostic-v1/README.md).
It qualifies a narrow compiler correction, not GPU execution or full-model
acceptance. All issue #42 M0-M7 milestones remain open.

## Source Change

The [four-file patch](correction.patch) changes defined-call argument lowering
only when the actual Kernel IR value is Index and the helper parameter is U64.
It routes that pair through the existing checked transport emitter. Exact-type
arguments keep their original value ID without an extra operation. Other
unequal types retain the bounded typed rejection; no helper ABI shape,
provider source, precision, scheduler or runtime path changes.

The [production lowerer](source/crates/fe2o3-lower-mir-kernel/src/production_semantic_kir_v1.rs)
uses ordinary checked emission, including the shared operation budget.
[Copy/Move regressions](source/crates/fe2o3-lower-mir-kernel/tests/production_semantic_kir_v1/shared_slice_helper_v1.rs)
freshly admit a three-function semantic owner and check the exact slice-length,
cast and call sequence, ABI, statement/terminator correspondence and effects.
Exact U64 controls emit no transport cast. Unit controls cover exact and
one-short emission budgets and exclude signed, narrower, float, pointer and
slice equivalence.

## Target Representation

This is not a generic claim that every Index or every pointer is 64-bit.
The current production AMDHSA profile validates the live target layout and
64-bit Rust usize before collection. The existing Kernel IR transport plan
allows the exact Index/U64 bridge, and the verifier checks that plan.
Both current gfx942 and gfx950 LLVM emitters represent these two scalar
kinds as i64. Local/private address spaces are a separate issue.

The [dual-target LLVM regression](source/crates/fe2o3-lower-mir-kernel/tests/production_semantic_kir_v1/shared_slice_helper_llvm_v1.rs)
checks both Copy and Move through the checked lowering route for both target
CPUs. It requires exactly one same-width add-i64-zero transport before the
i64 helper call, the original i64 signature, deterministic output and no
truncation or sign/zero extension. These are CPU emitter tests, not native
gfx942 or gfx950 runs.

## Actual CPU Qualification

On MI350, CPU8/9, nice10, offline Cargo jobs2:

| Suite | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Compiler library | 1,336 | 0 | 24 unchanged |
| Device library | 359 | 0 | 0 |
| Semantic-lowering integration | 125 | 0 | 0 |
| Semantic lowerer library | 766 | 0 | 0 |

All 19 phases exited naturally with status0 and clean owned process
retirement. Elapsed: 270.512373 seconds. Independent review rehashed all
5,812 inputs, 99 raw files, 15,543 dependency bodies, 2,247 Rust-source bodies,
eight tools and six selected artifacts. No source/tool/dependency drift or
postcheck errors occurred.

See the [CPU terminal](cpu/complete.json), [compiler](cpu/compiler-tests.stdout),
[device](cpu/device-tests.stdout), [integration](cpu/lowerer-tests.stdout)
and [unit](cpu/lowerer-unit-tests.stdout) originals.

```text
CPU receipt: fd40879fc85e939f527407ff77f6b7d3fe71ea6c784253f5505e0c4d4a6fd484
Backend ELF: bace8a715f762d91698275c70ee8eecfbebb0ab7876679c320c5154d878a64ae
```

## Failures And Parent Control

The [failure manifest](failures/manifest.json) retains separate bounded
archives, not just the final passing run.

| Attempt | Unit outcomes | What happened |
| --- | --- | --- |
| Transport V2 | Not executed | Harness expected the wrong unit-test namespace; 17 prior phases passed |
| Transport V3 | 765 passed, 1 failed | Full library suite exposed a stale expected first-error location |
| Unchanged parent | 764 passed, 1 failed | Reproduced the identical error on all 5,808 unchanged parent project files |
| Corrected V4 | 766 passed, 0 failed | Full library suite and final compiler-product phase pass |

The inherited [nested-reference fixture](source/crates/fe2o3-lower-mir-kernel/src/production_semantic_kir_v1/retained_nested_enum_transport_v1_tests.rs)
has invalid direct reloads in blocks7 and8. Deterministic SSA reverse-postorder
visits block8 first. The old test expected block7/statement4; both actual
failed runs reported block8/statement7, with the same function0/local5 and
MaybeMovedValueUsed error. The repair changes only that exact expectation
and its comment. It does not weaken the rejection or remove the subsequent
saved-reference refusal check.

Original failed terminals and unit output are retained for
[V2](failures/transport-v2/failed.json),
[V3](failures/transport-v3/failed.json) /
[unit output](failures/transport-v3/lowerer-unit-tests.stdout), and
[parent](failures/unchanged-parent/failed.json) /
[unit output](failures/unchanged-parent/lowerer-unit-tests.stdout).
Their archives preserve original controllers, inputs, source bodies,
commands, logs and lifecycle evidence:
[V2 archive](failures/transport-v2.tar.gz),
[V3 archive](failures/transport-v3.tar.gz),
[parent archive](failures/unchanged-parent.tar.gz).

## Actual Worker Compilation

Both original V14 workers use the exact qualified backend through the
checked `engineering hsaco` route, preserving the provider and paired
fixed/early scheduler source. Neither compilation succeeds.

| Arm | Natural exit | Wall seconds | HSACO |
| --- | ---: | ---: | --- |
| Fixed rounds | 1 | 121.103547 | None |
| Early exit | 1 | 121.424471 | None |

Both clear the former defined-call Index/U64 mismatch and instead report:

```text
production compilation general kernel verification failed:
ranked projection structural validation failed:
semantic-to-Kernel-IR correspondence no longer matches its exact owner
```

This generic diagnostic does not identify the specific failed ownership
predicate. Source investigation is pending; no validation bypass or claimed
correspondence repair is included in this checkpoint.

Fixed [result](pair/fixed/result.json) / [stderr](pair/fixed/compile.stderr)
and early [result](pair/early/result.json) / [stderr](pair/early/compile.stderr)
retain the exact observations. Both have unchanged before/after source maps,
clean child/group retirement, no timeout, no cleanup signals and no postcheck
errors. Canonical MIR and source-map captures are retained, but their
source/call identity has not been independently decoded for this attempt.

The [publication manifest](manifest.json) and [originals archive](originals.tar.gz)
contain the CPU qualification, source preimages/postimages, exact compiler
recipes, diagnostic captures and both failed worker attempts. Failed CPU
controls are linked separately above rather than silently discarded.

## Remaining Gates

The device-completion hold is unresolved. No new native execution,
independent model output, speedup, or 700 tokens/s result is claimed.
Separate gfx942 and gfx950 native/model/lifecycle evidence, the final
Qwen3-8B BF16 target-only 2,048/256 run, and all milestone gates remain open.
The outer compile environment sets jobs2, but the frontend clears the inner
Cargo environment; this does not establish an inner two-job limit.
