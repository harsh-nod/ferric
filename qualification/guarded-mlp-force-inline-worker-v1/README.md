# Worker Accessor Inlining And Callback Boundary

This is a CPU-qualified source candidate for [Ferric issue #42](https://github.com/harsh-nod/ferric/issues/42).
Both checked GPU-compilation attempts still fail. No HSACO, native execution,
model correctness, speedup or milestone completion is claimed.

## Exact Change

After the [tile-view correction](../guarded-mlp-force-inline-accessors-v1/README.md),
two borrowed-self accessors remained without the established GPU-only inline
attribute. The [patch](correction.patch) adds
`#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]` to
`WaveMlpTileStorageV2::state` and `WaveMlpTileWorkerV2::observations`.
Their bodies, signatures, safety comment and completion semantics are unchanged.

The new host regression uses genuine Fixture storage to check that `state()`
returns the exact atomic root without mutation. It does not fabricate a Worker
or call a device-only workgroup API on the CPU. All 16 previous MLP tests remain
byte-exact. The compiler regression checks the two exact method bodies and
immediate attributes, current source closure acceptance and previous closure
refusal. It does not treat local `observations()` as a terminal certificate.

The strict 53-file source closure changes from
`f58fbbbbcd003546719cddc201768131353c211f3ca57fd313d6b943a040a7e5`
to `91b3c53fa4235b478427f4569d1d0b77ccbf996ac509dba8e58fc36aa828f950`.
[The source review](source-review.json) binds both complete provider rosters.
No helper-ABI check or production admission predicate changes.

## Qualification

| Check | Actual result |
| --- | --- |
| Full compiler library | 1,331 passed, 24 unchanged ignored, 0 failed |
| Full device library | 359 passed, 0 ignored, 0 failed |
| CPU qualification | 11 clean phases, 231.635129878 seconds |
| V11 fixed-round compile | Natural exit 1, no HSACO |
| V11 early-STOP compile | Natural exit 1, no HSACO |
| Both canonical source joins | Passed; callback enum boundary identified |

The [terminal CPU receipt](cpu/correction/complete.json),
[compiler output](cpu/correction/compiler-tests.stdout) and
[device output](cpu/correction/device-tests.stdout) retain actual results.
The backend was rebuilt after the tests and is SHA-256
`fdecbb057bfac2cd7780ce5e13f31e77c649e2ba133f98cf700349229260c749`.
An independent data audit reconciled all named outcomes, 5,814 inputs,
15,543 external dependency bodies, 2,247 Rust-source bodies, eight tools,
four compiled products and all 59 raw CPU files.

## Remaining Compiler Boundary

| Arm | Whole probe seconds | MIR bytes | Original result |
| --- | ---: | ---: | --- |
| Fixed | 118.443720022 | 873,821 | [result](pair/fixed/result.json), [stderr](pair/fixed/compile.stderr) |
| Early | 117.993674112 | 874,900 | [result](pair/early/result.json), [stderr](pair/early/compile.stderr) |

Both attempts use the qualified backend, unchanged frontend, original lock,
normalization profile, deadlines and bounds. The callback remains
`inline(never)`; fixed and early differ only in the scheduler entry.
Both failed naturally, were reaped, left no owned process group and passed
source/tool postchecks. The new diagnostic is a by-value enum with indirect
ABI, not the previous borrowed `observations()` argument.

Both [fixed](next-helper/fixed.json) and [early](next-helper/early.json)
canonical joins identify helper
`400563b7f5decd99749f30086cb928825d45a6772d80b310e319d71d6e7fba74`:
`execute_task(task: WaveMlpTileTaskV2<'_, '_>)`, fixture bytes 1,902..2,377,
lines 55:1..64:2. The helper dispatches the five task variants.
The reader decodes actual MIR; a separate content check joins it to the
predeclared arm-specific fixture path and hash. It never follows an arbitrary
source-map display path. The two full fixture hashes differ because their
scheduler calls differ; the callback body is identical.

The next step is a reviewed callback-lowering correction, coordinated with the
compiler owner. Earlier forced-inlining experiments exposed a FnMut-origin
rejection. Supporting this boundary requires preserving ownership and source
provenance, not accepting arbitrary enum/pointer helpers. Neither enum ABI
support nor callback inlining is declared solved by this packet.

## Evidence And Next Gate

The [manifest](manifest.json) binds [originals.tar.gz](originals.tar.gz):
7,664,371 compressed bytes; SHA-256
`37351287d1f4ec108f8691a17555d2ea881ff4eebb63032e393bf8841488a2da`.
There are 132 members, including 131 original files and one archive index,
53,916,760 expanded bytes. Original commands, successes, failures, captures,
source preimages/postimages and joins are preserved by the [exporter](export.py).
Hash-bound ELF products are not bundled as runnable authority.

All authoring, builds, tests and source joins ran on `mi350`, with CPU8/9,
nice10, hidden GPUs and unchanged limits. CPU qualification uses explicit
Cargo jobs2. Compile probes retain the frontend's unchanged job policy;
their inner Cargo job limit is not established by the outer jobs2 setting.
Local work is publication and mechanical copy/hash/link verification only.

A real checked HSACO and its complete artifact/ABI joins must precede native
validation. The next numerical gate should retain the existing two-generation
paired-segment reference, owner/rearm checks and resource contracts; passing
that component gate would still not establish full-model correctness.
The Qwen3-8B BF16 target-only 2,048/256 workload, independent 256 output IDs/raw
bytes, lifecycle acceptance and 700 tokens/s target remain unmet.
