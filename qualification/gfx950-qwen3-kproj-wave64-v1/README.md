# Wave64 Qwen3 Key Projection

This separate engineering candidate computes the same actual-weight BF16
`[1024,1024]` key projection as the
[scalar baseline](../gfx950-qwen3-kproj-v1/README.md). It is not a full model,
a cooperative task-graph numerical handler, or a production-authorized kernel.
No speedup is inferred from the source layout alone.

## Coalesced Reads And Exclusive Output

One wave64 owns a row. Lane `l` accumulates columns `l + 64*j` for `j=0..15`,
so each wave's adjacent lanes read adjacent BF16 checkpoint values. Six
`ds_bpermute` butterfly exchanges and FP32 additions reduce the partial sums.
Each 128-thread workgroup handles two rows; 512 workgroups handle 1024 rows.
The checkpoint tensor is neither transposed nor repacked.

`StridedReadView2D::from_shared_slice` validates the borrowed read views.
Its total `load_or` access returns zero for invalid logical coordinates instead
of introducing a lane-dependent panic before the convergent reduction. This
matters: initial ordinary slice-index source variants failed the compiler's
convergence proof. The final source uses an existing checked API; no compiler
gate or bounds check was disabled.

All lanes finish reduction before leader selection. Lane zero writes through
`WriteOnlyDisjointSlice<f32, RowStriped2D<Index1D,64,1>>`, using the compiler-issued
row-stripe witness with `rows=1024, columns=1, row_stride=1`. The integer row is
not write authority. This mapping permits exactly one compact output per wave
and rejects other lanes rather than permitting overlapping mutable references.

## Numerical Contract

The inputs and independent FP64 expected values remain the frozen baseline
fixtures. This candidate has a separately frozen error policy: each product
traverses at most 16 FMA roundings and six reduction additions. For
`gamma(n)=n*u/(1-n*u)` and `u=2^-24`,

```text
(1 + gamma(16)) * (1 + gamma(6)) - 1 <= gamma(22).
```

The row error bound is therefore the upward-rounded
`(gamma(22,2^-24) + gamma(1024,2^-53)) * sum(abs(weight*input))`, with the same
explicit finite, underflow-lattice, overflow, and FP64-reference rounding
checks as the scalar baseline. The old serial gamma1024 bound would be more
conservative but is not silently relabeled as the wave-tree bound. Zero and
basis outputs remain exact; the constructed cancellation row must be exactly
zero. These requirements are fixed before any wave-kernel GPU comparison.

`reference.py generate` writes per-case `policy.json` and `bounds.f64le`.
`reference.py check` validates the frozen policy and outputs a numerical file
comparison. A caller must additionally bind that output to the exact wave64
dispatch report; a file comparison alone does not establish GPU execution.

## Compiled Artifact

The checked Rust source compiles through semantic MIR, ranked PLIRON, KIR V9,
composed memory/ownership/convergence checks, and gfx950 LLVM. `build.sh` decodes
the compiler-bound inert handoff into unchanged LLVM and records every
native-tool/provider input used to assemble an inert HSACO.

The initial successful artifact is
`evidence/kproj-wave-build-v5/qwen3-kproj-wave64-v1.hsaco`, SHA-256
`d594e2c8e202a69c41398e91e447857611af877f15d5907c093a61ff5b65c3f3`.
Observed metadata: 48 explicit kernarg bytes, alignment 8, wave64, required
workgroup `[128,1,1]`, zero LDS/private bytes, 24 SGPRs, 16 VGPRs, and no spills.
The source requires grid `[65536,1,1]` work-items and at most `[512,1,1]`
workgroups. Native ELF omits optional access/alignment/max-grid qualifiers;
the fixed host wrapper validates the separate declared ABI without inventing
ELF observations.

The engineering worker source requests CPU-mappable device-local VRAM for
user buffers, while queue/code/control allocations use their own profiles.
That allocation intent is not a measured residency or HBM-bandwidth result.
There is no HBM roofline, device timing, library comparison, or performance
claim in this compilation evidence.

## GPU Numerical Evidence

The complete `verify.sh` run passed four cases on `mi350-2`, checking 4096
outputs against the predeclared gamma22 policy. All input immutability checks,
allocation guards, completion, reverse frees, queue close, and worker exit
passed. The [sanitized evidence](evidence.json) binds the observations to the
exact source, HSACO, executable tools, checkpoint tensor, reference, and outputs.

| Case | Max absolute error | Max error / bound |
| --- | ---: | ---: |
| Zero | 0 | 0 |
| Basis | 0 | 0 |
| Mixed | 1.1920928955078125e-7 | 0.005085048481565353 |
| Cancellation | 0 | 0 |

Run the fixed contract, without arbitrary launch parameters:

```sh
bash verify.sh PROBE WORKER HSACO SOURCE PRIVATE_SELECTOR \
  WEIGHTS_DIR BASELINE_CASES_DIR WAVE_POLICIES_DIR NEW_EVIDENCE_DIR
python3 -B -m unittest -v test_reference test_binding
```

The script revalidates all four references before any dispatch and rechecks
source/tool hashes afterward. `bind_evidence.py` checks actual dispatch identity
and the independent numerical result together. The first run used probe
`af4cfe14183c1ec976872b08d77b2d470136213a6190f22cdea0be566a4762d4`
and wave reference
`40ddfa2e466f948ea38f7590234815f1eda5c2e15e3805927a49e64e1b9d15ca`.
These observations establish this fixed projection's correctness, not a
full-model inference result, measured speedup, or production qualification.
