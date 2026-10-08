# Consistent MLP Tile-View Inlining

This is a qualification candidate for [Ferric issue #42](https://github.com/harsh-nod/ferric/issues/42),
not production admission or a measured speedup. All source edits, builds,
tests and compiler attempts ran on SSH host `mi350`. Local activity is
limited to publication and mechanical verification.

## Result

| Check | Actual result |
| --- | --- |
| Full compiler library | 1,330 passed, 24 unchanged ignored, 0 failed |
| Full device library | 358 passed, 0 ignored, 0 failed |
| CPU qualification | 11 clean phases; 237.963921841 seconds |
| Fixed-round checked compile, V10 | Natural exit 1; no HSACO |
| Early-STOP checked compile, V10 | Natural exit 1; no HSACO |
| Canonical source diagnosis | Both failures identify `WaveMlpTileWorkerV2::observations` |
| GPU execution / numerical / performance acceptance | None |

The [previous checkpoint](../guarded-mlp-force-inline-reject-v1/README.md)
resolved the Norm reject helper and identified `MlpDownTileV2::input`.
This correction applies the existing AMDGPU-only force-inline convention
consistently to the remaining small tile-view methods. It does not relax
the helper ABI or source-authentication checks.

## Source Change

[The patch](correction.patch) contains exactly three changed source files:
the [device provider](source/correction/crates/fe2o3-device/src/finite_join/wave_mlp_tiles_v2.rs),
its [tests](source/correction/crates/fe2o3-device/src/finite_join/wave_mlp_tiles_v2_tests.rs),
and the [compiler source pin and regressions](source/correction/crates/rustc-codegen-fe2o3/src/trusted_device_items.rs).

Nineteen methods receive `#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]`:

| Tile view | Newly annotated methods |
| --- | --- |
| Norm | `lane`, `input`, `weight` |
| Gate | `lane`, `reject`, `input`, `weight` |
| Up | `lane`, `reject`, `input`, `weight` |
| SwiGLU | `lane`, `reject`, `gate`, `up` |
| Down | `lane`, `reject`, `input`, `weight` |

All signatures and bodies are unchanged. The five writers and Norm reject
were already annotated; all 25 methods across these five views now follow
the same convention. This is a lowering correction, not a demonstrated
performance optimization.

The strict 53-file provider closure advances from
`d6ac31898b9e4d594d3aa36d6c5ced378059a4ffff54881463f57c10eba0563c`
to `f58fbbbbcd003546719cddc201768131353c211f3ca57fd313d6b943a040a7e5`.
The new compiler regression checks the closed five-by-five method roster,
immediate attributes, current closure acceptance and previous closure refusal.
Production admission predicates are unchanged.

The new device regression visits all 64 lanes and both ends of each
Gate/Up/Down tile range. It checks last-valid, first-invalid and maximum
indices, both weight axes, repeated rejection, unchanged payload/state and
zero write credit. It preserves the existing distinction: Norm reads and
weight reads remain available after rejection; Gate/Up/Down inputs and
SwiGLU inputs do not. All 15 previous MLP tests are preserved byte-for-byte.
The [source review](source-review.json) records the exact delta and closure.

## Actual Compiler Retry

Both attempts use the rebuilt qualified backend, the previously qualified
diagnostic frontend and the unchanged `optimized-inline-hint-16384-v1`
profile. The callback remains `inline(never)`; fixed and early differ only
in the scheduler entry. No helper-argument check is disabled.

| Arm | Whole probe seconds | MIR bytes | Diagnostic |
| --- | ---: | ---: | --- |
| [Fixed](pair/fixed/result.json) | 118.383682675 | 879,829 | Shared aggregate helper argument |
| [Early](pair/early/result.json) | 118.405568409 | 880,908 | Shared aggregate helper argument |

Both processes exited naturally, were reaped, left no owned process group,
and passed source/tool postchecks. Their [fixed stderr](pair/fixed/compile.stderr)
and [early stderr](pair/early/compile.stderr) are retained unchanged.

The canonical reader and independent source-content join identify helper
`7eddc4a0d2b9ce6d1a9ca7dd2780781a00d45048fbc70b19792409f824892b21`
as `WaveMlpTileWorkerV2::observations`, bytes 25,325..25,405
at lines 710:5..712:6 of provider SHA-256
`db694a50c39cd29d93e1d70c2f58dbbf638410c360f1a3e19dc4387676259f8d`.
Both [fixed](next-helper/fixed.json) and [early](next-helper/early.json)
joins verify the captured typed argument and exact source body. The method
copies the local result; it does not certify terminal graph completion.

## Evidence And Limits

The [manifest](manifest.json) binds the [original archive](originals.tar.gz):
7,668,309 compressed bytes, SHA-256
`75022c897ce08fa9217902d7705da86ef6db197782470b3550ef0c8dd77e690b`.
It contains 132 members: 131 original files and one archive index,
54,107,024 expanded bytes. It preserves commands, full raw CPU results,
input/source/dependency/tool ledgers, source preimages/postimages, both
compiler failures, original MIR captures and typed reader/source-join outputs.
The [exporter](export.py) records the checked extraction procedure; compiled
ELF products are hash-bound in receipts, not bundled as runnable authority.

The CPU [terminal receipt](cpu/correction/complete.json),
[compiler output](cpu/correction/compiler-tests.stdout), and
[device output](cpu/correction/device-tests.stdout) are also directly readable.
CPU work used cores 8/9, nice 10 and explicit Cargo jobs 2. GPU visibility
was empty. Build and probe deadlines and resource bounds were unchanged.

The next correction is the same GPU-only annotation for the two remaining
borrowed-self provider accessors, `storage.state()` and
`worker.observations()`, with exact-body and genuine-storage tests.
That successor receives no qualification credit from this packet.
A checked HSACO and fresh native numerical validation remain necessary.
No full 2,048/256 decode, independent model equivalence, throughput or
700 tokens/s claim follows from these CPU results. Issue #42 M0-M7 stay open.

