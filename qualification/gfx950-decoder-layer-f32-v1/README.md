# Fused Decoder GPU Baseline

This is the first executable numerical milestone for [megakernel issue
#42](https://github.com/harsh-nod/ferric/issues/42). Ordinary fe2o3 Rust source
compiles through the semantic, ranked-memory, bounds, race and formal checks,
then LLVM/HSACO, and runs on the MI350-2 gfx950 GPU through the existing
direct-KFD engineering worker. There is no HIP kernel or CPU execution fallback.

One GPU dispatch computes input RMSNorm, Q/K/V projections, per-head Q/K
RMSNorm, RoPE, causal grouped-query attention over a two-token prefix plus the
current token, output projection and residual, post-attention RMSNorm, SwiGLU,
and the final down projection and residual. Eleven intermediate/final stages
are retained, totaling 40 checkpoint values per request.

The fixture is intentionally small: hidden dimension 4, two query heads, one
KV head, head dimension 2, intermediate dimension 4, and F32 throughout. Each
workitem independently owns one request. The grid contains 256 workitems in two
128-thread workgroups,
with two wave64 waves per workgroup. This is a complete fused *layer*, not a
full model, cooperative tiled implementation, persistent task scheduler,
autoregressive generation loop, MFMA implementation, or production-qualified
Ferric engine. No speedup or SoTA claim is made.

## Observed GPU Results

The four-case suite completed on MI350-2. Every dispatch passed input
immutability, output guard, finite-checkpoint, completion, explicit resource
release, worker-close and process-exit checks. All 40,960 checkpoint values
matched the independent reference. A preceding smoke run also passed, and its
mixed-case output was bit-identical to the suite's mixed output.

| Case | Compared values | Maximum absolute error | Result |
| --- | ---: | ---: | --- |
| Mixed, including basis/near-zero/large residual inputs | 10,240 | 3.503484e-6 | Pass |
| Zero | 10,240 | 0 | Pass |
| Saturated SwiGLU gates | 10,240 | 9.224651e-6 | Pass |
| Sharply peaked attention | 10,240 | 3.442022e-6 | Pass |

Tolerances were fixed before GPU execution:
`abs(gpu - reference) <= 2e-5 + 2e-4 * abs(reference)`.
The reference first rounds inputs and weights to the actual F32 wire values,
then evaluates independent FP64 matrices, head vectors, normalization, stable
softmax and SiLU. It does not share the device's unrolled scalar implementation.
Every stage is checked independently, so cancellation in the final residual
cannot conceal a wrong intermediate.

[results/summary.json](results/summary.json) contains per-stage errors and
source/object/worker/reference identities. Each case retains its raw GPU
output, dispatch report, comparison and case manifest. These are engineering
observations, not cryptographic execution attestations. No device UID or raw
worker stderr is published. The shared GPU had one unrelated resident process;
its process count and observed VRAM/GTT usage were unchanged after testing.

The observed HSACO uses 22 SGPRs, 96 VGPRs, no LDS, no private segment and no
spills. Optional ELF qualifiers are recorded as absent, separately from the
fixed source-declared ABI. See [BUILD.md](BUILD.md) for exact compiler, backend,
native tools, provider hashes and reproduction details.

## Reproduce

Build the [probe](../../tools/gfx950-finite-probe/README.md) and the existing
engineering worker, then produce the exact source's HSACO using the recorded
[build recipe](BUILD.md). The device selector is a private mode 0600 file
containing the operator-selected KFD device UID. Coordinate access to the GPU;
this is an explicit trusted-native-code engineering boundary, not a sandbox.

```sh
./verify.sh /path/to/ferric-gfx950-finite-probe \
  /path/to/fe2o3-gfx950-engineering-worker \
  /path/to/decoder-layer-f32-v1.hsaco \
  ../../device/gfx950-decoder-layer-f32-v1/src/lib.rs \
  /private/device-selector /path/to/new-evidence-directory
```

The script freezes all reference cases before launching, stops on any failure,
and writes a summary only when all runtime and numerical reports agree by hash.
It does not reset devices, modify settings, kill foreign jobs, or reuse a failed
worker. Each dispatch has a 10-second completion deadline.

Host-only reference and evidence-linkage tests:

```sh
python3 -B -I test_reference.py
python3 -B -I test_summary.py
python3 -B -I test_recorded_results.py
```

These 18 tests are separate from the actual GPU observations above. The host
probe has 13 additional protocol/cleanup/ABI tests; those are not GPU tests.
