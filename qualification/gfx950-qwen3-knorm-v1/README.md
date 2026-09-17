# Qwen3 Key Projection And K-RMSNorm Reference

This fixed engineering graph uses real `Qwen/Qwen3-0.6B` checkpoint weights
with synthetic single-token BF16 activations. An ordinary-buffer wave64 key
projection is followed by a separate K-RMSNorm dispatch, ending before RoPE.
The reference and numerical policy were fixed before GPU execution. The native
suite has now passed on `mi350-2`; the exact records and limits follow below.

This is not full-model inference, a single-launch megakernel, tensor publication
through atomic channels, a performance result, or protected-runtime authority.
No PyTorch/Transformers installation, framework execution, or additional model
download is needed. Framework source is a semantic reference, not backend-parity
evidence.

## Native GPU Evidence

[The machine-readable summary](evidence-native-v2.json) binds clean Ferric
`e5ed1185c55e8b46dddf3749e2c7ebbfc1b9f89b`, the frozen compiler from fe2o3
`3dfa5b3fdac1832bd7d8902e32f591d81300d1e3`, both HSACO objects, the independent
reference, and every case/report/raw-output hash. The probe was frozen from
Ferric `28a6d3eee9af35a1f5d70c640396358e3b99b1c5`; its exact fe2o3 dependency
closure remained unchanged at `3dfa5b3`, despite parallel edits elsewhere in
the compiler checkout. This is not a claim that that entire live checkout
was clean. Source-to-HSACO lowering used the normal checked Rust pipeline.

| Case | Projection max absolute error | Error / bound | Exact BF16 norm matches | Composed enclosure |
| --- | ---: | ---: | ---: | --- |
| zero | 0 | 0 | 1024 / 1024 | pass |
| basis | 0 | 0 | 1024 / 1024 | pass |
| mixed | 1.1920928955078125e-7 | 0.005085048481565353 | 1024 / 1024 | pass |
| cancellation | 0 | 0 | 1024 / 1024 | pass |
| epsilon | 0 | 0 | 1024 / 1024 | pass |

Ten dispatches checked 5120 projection values, 5120 intermediate BF16 keys,
and 5120 final BF16 norm values. Every conditional consumer interval was a
singleton; no sqrt/div tolerance was widened. All final values also fell
inside the independently composed intervals. The diagnostic FP64 model-center
values happened to agree in these cases, which does not establish framework
bitwise parity. Input bytes, all six allocation guards, unchanged projection
storage, producer-before-consumer order, reverse frees, queue close, and worker
exit passed. The intermediate GPU allocation was reused without host writes;
the harness did read it back between dispatches for observation.

The producer uses 512 workgroups and the consumer four, each with two wave64
waves. Their respective SGPR/VGPR counts are 24/16 and 28/19, with zero spills,
LDS, or private segments. Native consumer ISA contains `v_sqrt_f32` and the
division refinement/fixup sequence; strict IR sqrt/div semantics are retained.
These are correctness/resource observations, not a speedup measurement.

Raw records, frozen pre-run references, and manifests remain under
`/home/harmenon/ferric-gfx950-42/evidence-gpu/knorm-chain-native-v2/`; native
handoffs/IR/ISA are under `evidence/knorm-chain-native-v2/`. Hardware identifiers
remain private. Shared-GPU pre/post observations found the same idle foreign
process and memory usage; no foreign process or device configuration was changed.

## Checkpoint Identity

Model revision: `c1899de289a04d12100db370d81485cdf75e47ca`.
The existing `model.safetensors` is 1,503,300,328 bytes with SHA-256
`f47f71177f32bcd101b7573ec9171e6a57f4f4d31148d38e382306f42996874b`.

| Tensor | Shape / dtype | SHA-256 |
| --- | --- | --- |
| `model.layers.0.self_attn.k_proj.weight` | `[1024,1024]` BF16 | `68dd761a149b0ff707ad0c1ac6d61dfed75d6fedfbbdbfedde4889f5ca0706f4` |
| `model.layers.0.self_attn.k_norm.weight` | `[128]` BF16 | `65ba32dce94cb1fde9037d627efd8b734a0e2db04df21ea33c026393371b2c88` |

Gamma is shared across eight 128-element key heads. Its values range from
0.03466796875 to 96.5; it is not a vector of ones. The safetensors data offsets
are `[641208320,641208576]`, absolute file offset 641243880. The header occupies
35552 bytes and has SHA-256
`399d16f500e925c7e923fe05966c6df6862ab64da60916843119e802f1801bca`.

`extract-gamma` parses structured safetensors metadata with duplicate-key,
shape, dtype, byte-range, overlap, and complete-layout checks. It hashes the
entire pinned checkpoint, checks file identity across the read, and writes
unchanged gamma bytes plus their provenance to a new directory. No checkpoint
code is evaluated. The projection tensor uses the existing independently
checked extraction, with its exact tensor hash checked again here.

The model contract and retained primary sources are on `mi350-2` under
`/home/harmenon/ferric-gfx950-42/evidence/qwen3-k-rmsnorm-contract-v1/`:
`REPORT-v2.md`, `sources.tsv`, `SHA256SUMS`, and the source files they name.
The source audit pins Transformers commit
`0720e206c6ba28887e4d60ef60a6a089f6c1cc76` and PyTorch commit
`134179474539648ba7dee1317959529fbd0e7f89`.

## Three Separate Numerical Checks

1. **Independent projection.** Each F32 projection value is checked against
   the FP64 `math.fsum` of exact widened BF16 products. The upward-rounded
   bound is `(gamma22_f32 + gamma1024_f64) * upward(sum_abs/(1-gamma1024_f64))`,
   reusing the wave kernel's 16 FMA plus six addition depth argument. Zero,
   basis, and epsilon cases require exact projection values; cancellation
   requires exact row-zero cancellation. The saved BF16 K must equal the exact
   ties-even BF16 cast of the validated F32 intermediate. Independently, it
   must lie in the outward BF16 image of the projection interval. An interval
   that crosses a BF16 midpoint is not replaced by the rounding of its center.
2. **Exact conditional consumer.** Given those validated BF16 keys, evaluate
   the exact emitted F32 operation sequence and require all 1024 final BF16
   words, including signed zeros, to agree exactly. No output-ULP tolerance is
   used. Each wave owns one head; lane `l` squares keys `l` and `l+64`
   separately, adds them, and reduces using XOR distances 1,2,4,8,16,32.
   Multiply by exact `1/128`, add F32 `1e-6`, take correctly rounded F32 sqrt,
   divide F32 one by that result, then multiply each key. Cast this normalized
   value to BF16 **before** multiplying by real BF16 gamma widened to F32.
   Cast the product to BF16 again. These two casts are not interchangeable.
3. **Composed reference.** Propagate all independent projection BF16 intervals
   through the same staged sqrt/div consumer using monotone interval endpoints
   and exact rounding at every operation. This conservative composed enclosure
   is checked in addition to the exact conditional check. A separate diagnostic
   model-center file uses FP64 projection, BF16 K, FP64 mean/rsqrt, BF16 normalized
   values, and BF16 gamma products. Center differences are reported, not silently
   promoted to failures or used to excuse conditional failures. Neither this
   FP64 center nor the composed sqrt/div enclosure is a bitwise eager-framework
   rsqrt reference. The audited model specifies rsqrt and does not fix the
   projection/reduction association.

The consumer contract has **zero adjacent-F32 allowance** for sqrt and division.
The actual consumer LLVM has plain `llvm.sqrt.f32` and `fdiv`, no `afn`, `arcp`,
fast flags, or `fpmath` relaxation, separate squares/addition, `fp-contract=off`,
and IEEE denormal modes. This checks the source/compiler trusted-computing-base
contract, not a proof of hardware behavior for every possible input. The
[LLVM 22 semantic reference](https://raw.githubusercontent.com/llvm/llvm-project/llvmorg-22.1.0/llvm/docs/LangRef.rst)
defines correctly rounded core operations in the default floating-point
environment and conforming sqrt behavior without `afn`; this release reference
is not asserted to be the precise vendor compiler source revision.

Rounding in the Python checker uses exact rational midpoint comparisons.
Square root is seeded by host sqrt but its final F32 choice is decided by
exact squared-midpoint comparisons, not host-libm accuracy. BF16 midpoint
checks avoid double rounding through F32. The projection domain retains its
finite, product-lattice, and overflow checks. Conditional keys must be zero
or have magnitude in `[2^-50,2^20]`; real gamma must lie in `[2^-10,2^8]`.
These cases avoid overflow and subnormal consumer arithmetic. The code never
widens a bound in response to a GPU result.

Cases are `zero`, `basis`, `mixed`, `cancellation`, and `epsilon`. The first
four reuse the existing projection inputs; epsilon is the basis activation
scaled by exact `2^-12`. Unit tests additionally exercise BF16 ties/off-ties,
epsilon-dominant normalization, signed zero, the incorrect missing pre-gamma
cast, and corrupted artifact, reference, numerical, and lifecycle records.

## Files And Reproduction

Six guarded allocations contain hidden BF16[1024], Wk BF16[1024,1024],
projection F32[1024], gamma BF16[128], quantized K BF16[1024], and norm
BF16[1024]. The producer binds allocations `[0,1,2]`; the consumer binds
`[2,3,4,5]`. Both use workgroup `[128,1,1]`, with grids `[65536,1,1]` and
`[512,1,1]` respectively. The same projection allocation is retained across
exact producer completion and consumer dispatch, with no host rewrite/reupload.
A readback is an observation, not a new producer. Device-local public allocation
intent is not independently measured HBM residency or a COHERENT allocation flag.

`build.sh` requires committed clean Ferric sources and a matched, immutable
compiler snapshot from fe2o3 `3dfa5b3fdac1832bd7d8902e32f591d81300d1e3`.
It rechecks the snapshot's clean-source provenance and binary hashes instead
of trusting a concurrently edited compiler checkout. Both stages pass the
normal source pipeline; LLVM/provider linking never edits the emitted IR.
The suite separates CPU-only reference preparation from GPU dispatch so the
shared machine can be checked immediately before device work.

```sh
FE2O3_COMPILER_BIN=FROZEN_COMPILER bash build.sh WORK_ROOT NEW_BUILD_DIR
bash verify-suite.sh prepare PROBE WORKER BUILD FERRIC_ROOT WEIGHTS GAMMA PRIVATE_SELECTOR NEW_EVIDENCE
# Recheck shared-host GPU availability before this explicit dispatch phase.
bash verify-suite.sh run PROBE WORKER BUILD FERRIC_ROOT WEIGHTS GAMMA PRIVATE_SELECTOR EVIDENCE
```

```sh
python3 -B reference.py extract-gamma --checkpoint MODEL_SAFETENSORS --out-dir NEW_GAMMA_DIR
python3 -B reference.py generate --case mixed --weights-dir WEIGHTS --gamma-dir GAMMA \
  --case-dir NEW_CASE --artifact ARTIFACT --producer-source PRODUCER_RS \
  --producer-object PRODUCER_HSACO --consumer-source CONSUMER_RS \
  --consumer-object CONSUMER_HSACO --probe PROBE --worker WORKER
python3 -B reference.py check --weights-dir WEIGHTS --gamma-dir GAMMA \
  --case-dir CASE --artifact ARTIFACT --producer-source PRODUCER_RS \
  --producer-object PRODUCER_HSACO --consumer-source CONSUMER_RS \
  --consumer-object CONSUMER_HSACO --probe PROBE --worker WORKER \
  --run-dir RUN --result NEW_NUMERICAL_JSON
python3 -B -m unittest -v test_reference
```

Generation binds source/object/tool/artifact hashes and recomputable reference
bytes in `case.json`. Checking revalidates everything, then reads
`projection.f32le`, `quantized.bf16le`, `output.bf16le`, and `report.json`.
It requires two completions, unchanged inputs/projection, all allocation guards,
producer-before-consumer order, intermediate reuse without host write, frees,
queue close, and successful worker exit. The result is created exclusively;
an existing numerical result is never overwritten. Report booleans are bounded
engineering observations, not production memory/launch authority.
