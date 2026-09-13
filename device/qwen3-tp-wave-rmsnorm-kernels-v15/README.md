# Wave RMSNorm V15 Candidate

Separate kernel candidate. The guarded-read source at `39be94d` passes remote
formatting, 23 focused host tests, strict Clippy and emission with compiler
`ae267179`. The emitted image passes ABI/resource inspection and independent
static review. Native qualification and performance remain pending. The later
revision-only update to `c9c7036` needs its own validation. No adapter,
controller, existing kernel, default, inventory or external manifest is changed.

## Closed Scope

One root: `ferric_qwen3_tp_batch32_wave_rmsnorm_bf16_v15`.
Only pure behavior 0, width 4096 and rows 1..32 are admitted. Both auxiliary slices
must be empty. Input/output lengths are exactly rows*4096, weight length is
exactly 4096, grid is exactly [rows,1,1], and epsilon is exactly `1e-6_f32`.
The Wave64 workgroup is [64,1,1]. Width 128 Q/K normalization, draft 1024 and
residual-fused normalization are not part of this candidate.

The signature retains the original five slice descriptors and four scalar
arguments: 96 explicit bytes, with pointer alignment 2 and scalar alignment 4.
The ae267 image measures hidden offset 96, total 352 and kernarg alignment 8,
matching the predeclared ABI. Empty auxiliary slices still need
the caller's valid zero-length slice/sentinel contract. This source does not
authenticate raw pointers or aliasing at a device launch boundary.

Lane L accumulates columns L+64*k for k=0..63. All physical lanes then perform
sum64 and invalid-max64 collectives before any numeric rejection. After uniform
shape/length/grid admission, a shared read view uses rows by4096 with stride4096.
The first pass uses `load_or(row,column,0x7fc0)`: valid coordinates preserve the
input bits; an out-of-view coordinate supplies quiet NaN without a bounds trap,
sets the sticky invalid flag and reaches both reductions before rejection.
These are nonvolatile first-pass reads, not a volatile-equivalence claim.
Input storage must remain immutable through execution. The retained
second pass rereads each owned input and its weight and uses the old pure
output path: FP32 division, epsilon addition, sqrt, reciprocal, two separate
multiplications and BF16 round-to-nearest-even. RowStriped2D<Index1D,64,64>
retains one writer per active element and no writer outside the supplied slice.
No LDS, global scratch, register-array cache, FMA or approximate rsqrt is added.

## Arithmetic Boundary

This is not the old scalar left fold. A host case with a leading 256 and 4095
copies of 0.03125 explicitly demonstrates different sum bits. BF16 output bits
and downstream model tokens may change at rounding boundaries. Universal
bitwise equivalence is not claimed and no frozen oracle is changed.

Nonfinite inputs, products and partial sums are recorded without an early
lane exit. Both collectives execute before the uniform result/invalid check.
The second pass retains input/weight/intermediate/narrowed-output checks and
has no later collective. Failed numerical execution may have partial stores;
there is no transactional output promise. A future runtime must retain its
existing failed-dispatch poison/quarantine behavior.

The baseline has a width-4096 source-level serial chain in every lane; the
candidate has 64 local additions plus a six-level sum and invalid reduction.
Compiler scalarization, caches and memory broadcast mean this is not evidence
of 64x physical traffic or speedup. Retained host spans include IPC/submission/
wait; they are not per-kernel GPU durations. No GPU, HTTP, stability, default,
competitive or M1 claim follows from this source.

## Focused Tests and Device Gate

Six contract methods and fourteen host-model methods pass on mi300x, 20 total,
at source `c534e35258f6379b481380f0088e777356547efb`, with fe2o3 `61e014a`
and Pliron `9de42fc`. Strict Clippy also passes. Immutable host result
`f230c4eb` and complete archive `3c63d3e7` retain this checkpoint; later
dependency migration does not relabel its producer. Contract tests bind the
one-root generated roster, five-slice/four-scalar ABI, exact entry/collective
AST, seventeen source mutations and all 32 row ownership maps. The post-sum
arithmetic and complete second pass are compared against the existing RMSNorm
AST with only its unreachable fused branch replaced by the pure expression.

The CPU model simulates the six XOR levels [1,2,4,8,16,32], records both
collectives before invalid rejection and checks both input passes. It uses CPU
sqrt, not device-capability emulation or a GPU mathematical oracle. Nonuniform,
signed, exponent-mixed and association-boundary inputs are checked against a
sampled f64 reference with a fixture-only BF16 rounding sanity bound, not a
native acceptance tolerance. Separate tests cover even/odd signed midpoint
rounding, narrowing overflow, tiny inputs/zero signs, invalid shapes/lengths,
exact epsilon/grid, auxiliary modes, nonfinite inputs in every lane, square/
sum/output overflow, guarded inactive capacity and changed inputs after error.

Source base: Ferric `0588f997`, with the original candidate retained at
`9921546`. Revision-only migration `94eb4e1` pinned the successful emission's
dependencies to `ae26717922b1fb7ad62fdd5ad70814d83eb01177`. Active dependencies
now use `c9c7036cf98034b638886f54a745701e70ac3571` after the separate exact
revision substitutions at `3ababa7` and `319315e`; validation remains pending.
Kernel and host arithmetic are unchanged by these dependency migrations.
The subsequent source correction changes only the first-pass read API and its
uniform constructor; post-sum arithmetic, ABI and second-pass volatile reads
remain unchanged. Seven contract and sixteen host methods, 23 total, pass
at `39be94d` with ae267. Added checks bind the read-view
constructor/NaN fallback, reject a restored volatile/trapping first pass, cover
every valid coordinate for rows1..32 and model one-lane/all-lane out-of-view
fallbacks reaching both collectives before rejection.
No local Cargo resolution or hand-derived lock graph was used. Build.rs
reuses the unchanged aggregate target helpers; its binding is a host fixture.

The first ae267 emission attempt stopped before compilation: its private
offline cache lacked `smallvec 1.16.0`. Result `910a724a` and archive `b52ecb05`
retain that failure. The separately corrected R2 dependency preparation passed
exact standalone163 and vendor198 package checks, including all40 pinned
build-std registry packages. Its unchanged94eb kernel was then rejected by
ae267 typed lowering: `UnprovenBarrierConvergence`, bb33 op0. No image was emitted;
archive64c99339 retains the failed attempt. The safe volatile-read intrinsic
introduces a lane-dependent bounds branch/trap before the reductions, unlike
the guarded view used by admitted wave kernels. This is the evidence-backed
convergence hypothesis; no retained MIR proves an exact guard-to-bb33 mapping.
The guarded-read correction passes the unchanged ae267 compiler in separate
R3. All twelve phases pass, including the 23 tests, strict Clippy, capture and
independent ELF inspection. Complete archive `c4f3be27` retains all 33,585 files;
independent streams, file hashes and 40,774 metadata entries agree. This resolves
the concrete compilation rejection without weakening compiler checks.

Image `a9da1d79` is 11,240 bytes, with a 2,940-byte kernel body, 70 SGPRs and
22 VGPRs. It has zero register spills, private memory, LDS and AGPRs, with no
dynamic stack. Static review observes RNE rounding, preserved denormals and
IEEE mode; restored EXEC before both six-stage collectives; square-root and
division refinement; separate output multiplications; and integer BF16 RNE.
This supports the intended arithmetic policy, not a complete ISA numerical
proof, native parity, occupancy measurement or speedup. The producer remains
ae267 even though active source dependencies have advanced.

The remote-only device gate must retain the exact crate,
baseline `../qwen3-all-kernels-v1/src/rmsnorm.rs`, shared target.rs and
build/target_contract.rs, final lock and compiler/tool closure. Detailed
typed/effect/progress payload remains an open obligation: the standard emission
path retains only its digest, not an admitted detailed payload. Verify all bounds
and full-wave convergence, inspect both load passes/shuffle association/stores,
and check actual ABI/resources without suppressing compiler checks. A new
c9c7036 compiler build/emission and the eight-case analytical native fixtures
remain next steps. Any later
finite native profile, whole-buffer guards, model oracle or opt-in adapter route
requires separate review; none is implemented here.
