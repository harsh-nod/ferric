# Observed Lowered Helper-Argument Mismatch

This CPU/compiler diagnostic checkpoint follows the
[canonical source-call inspection](../guarded-mlp-semantic-call-v1/README.md).
It preserves the rejection, not a conversion. It is not HSACO, GPU or
full-model acceptance; all issue #42 milestones remain open.

## Bounded Diagnostic

The existing unequal argument/signature comparison now returns a dedicated
`DefinedCallArgumentTypeMismatch` carrying caller/block/callee indices,
source argument, tuple/component projection, and exact scalar kinds.
`None` explicitly means non-scalar; recursive types and MIR are not dumped.
The old failure prefix remains, successful lowering paths are unchanged,
and no new capture hook, environment selector or admission bypass is added.

See the [two-file source patch](correction.patch) and qualified
[lowerer source](source/crates/fe2o3-lower-mir-kernel/src/production_semantic_kir_v1.rs).
The new [regressions](source/crates/fe2o3-lower-mir-kernel/tests/production_semantic_kir_v1/shared_slice_helper_v1.rs)
freshly admit a real three-function semantic owner. Copy and Move of a slice
length observe Index-to-U64 rejection; genuine U64 controls still lower and
pass checked equivalence. Formatting tests retain signed/width distinctions
and bound the display. All previous fixture/test bodies are unchanged.

## Actual CPU Qualification

MI350 host, CPU8/9, nice10, offline Cargo jobs2:

| Suite | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Compiler library | 1,336 | 0 | 24 unchanged |
| Device library | 359 | 0 | 0 |
| Production semantic-lowering integration | 124 | 0 | 0 |

All 15 phases ended naturally with status0, reaped children and absent owned
process groups. Elapsed: 258.680797 seconds. Source, dependency, tool,
Rust-source and selected-artifact checks passed. Independent review rehashed
all 5,812 inputs, 79 raw files, 15,543 dependency bodies, 2,247 Rust-source
bodies, eight tools and five ELF artifacts.

Originals: [CPU terminal](cpu/complete.json),
[compiler tests](cpu/compiler-tests.stdout),
[device tests](cpu/device-tests.stdout),
[lowerer tests](cpu/lowerer-tests.stdout),
[manifest](manifest.json) and [complete archive](originals.tar.gz).
The archive has 130 members / 129 pinned originals, 56,576,746 expanded bytes.

```text
CPU terminal: 28886f4d804ff72204539809c4e3599f755352f4c9e28effc83f93b328eab2eb
Backend ELF:  fed71259d219e3b966c3b08e78f9b5b8a993d1a719475f092a683d02f8e1a4f6
Archive:      e5c5426363ad86c8e6dbce4eaf4e0cd3fc0be4395454a765274b847c58d50d95
```

## Actual V13 Compilations

Both original scheduler variants were rebuilt through the checked
`engineering hsaco` route using that exact backend, not an edited provider
or a replacement HIP implementation.

| Arm | Caller | Block | Callee | Source arg | Component | Actual | Expected | Wall seconds |
| --- | ---: | ---: | ---: | ---: | ---: | --- | --- | ---: |
| Fixed rounds | 0 | 348 | 2 | 1 | 0 | Index | U64 | 119.604041 |
| Early exit | 0 | 629 | 2 | 1 | 0 | Index | U64 | 119.392614 |

Tuple projection is absent in both. Each compiler exits naturally with
status1, clean process retirement, unchanged source/tool inputs, no postcheck
errors, and **no HSACO**. The shared ledger covers 11,141 source entries.
Fixed [result](pair/fixed/result.json) / [stderr](pair/fixed/compile.stderr)
and early [result](pair/early/result.json) / [stderr](pair/early/compile.stderr)
retain the observations, not just summaries.

The function/block indices changed from V12. The V12 source-joined callee
identity must not be assumed from V13 numeric IDs. V13 directly establishes
the lowered Index/U64 mismatch; it does not newly establish a source callee
name. The outer compile environment sets jobs2, but the frontend clears the
inner Cargo environment; this is not proof of an inner two-job limit.

## Next Correction And Remaining Gates

A separate private candidate now routes only Index-to-U64 through the
existing checked bitcast emitter. Exact-type pass-through and every other
mismatch refusal remain. Qualification is still pending, including explicit
review of target Index representation, exact cast/call correspondence,
Copy/Move controls, and operation-budget denial. This document does not
claim that unqualified candidate compiles the original kernels.

The device completion hold remains unresolved. No new native execution,
independent model output, performance improvement or 700 tokens/s result
is claimed. Separate gfx942 and gfx950 native/model/lifecycle evidence,
the final 2,048/256 Qwen3-8B BF16 run and all milestone gates remain open.
