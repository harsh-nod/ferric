# Actual-Weight Qwen3 Key Projection

This is a fixed batch-one scalar GEMV correctness baseline, not an optimized
GEMV, a cooperative task handler, full-model inference, or production execution
authority. The safe Rust kernel is
[`device/gfx950-qwen3-kproj-v1/src/lib.rs`](../../device/gfx950-qwen3-kproj-v1/src/lib.rs).
It compiles through the production source extractor and verified lowering into
gfx950 LLVM; unchanged LLVM is assembled with the recorded ROCm native tools.
The resulting HSACO runs only through the existing explicitly unauthenticated
direct-KFD engineering worker. There is no HIP/HSA fallback or hand-written IR.

## Fixed Workload And Ownership

The tensor is `model.layers.0.self_attn.k_proj.weight` from `Qwen/Qwen3-0.6B`,
revision `c1899de289a04d12100db370d81485cdf75e47ca`, stored as BF16 `[1024,1024]`.
The safetensors file SHA-256 is
`f47f71177f32bcd101b7573ec9171e6a57f4f4d31148d38e382306f42996874b`.
The unmodified tensor SHA-256 is
`68dd761a149b0ff707ad0c1ac6d61dfed75d6fedfbbdbfedde4889f5ca0706f4`.

`extract_checkpoint.py` parses the structured safetensors header, rejects
duplicate keys, malformed extents, overlaps, holes, and an incorrect full-file
hash, then records the revision, dtype, shape, byte offset, tensor hash, and
extractor hash. It does not download a mutable model branch or transpose the
weights. Model weights are not included in this repository.

There are three pointer/length arguments: 1024 BF16 input words, 1,048,576 BF16
weight words, and 1024 FP32 outputs. Explicit kernargs occupy 48 bytes. Eight
workgroups of 128 work-items use wave64; each invocation exclusively owns one
output row and performs 1024 explicitly fused FP32 multiply-adds after exact
BF16 widening. There is no LDS allocation or cross-workgroup communication.
The output's typed disjoint ownership is not reconstructed from an integer
pointer. The host validates the fixed symbol, layout, launch dimensions, and
observed code-object metadata. Missing optional ELF qualifiers remain absent
observations; contradictory qualifiers are rejected rather than fabricated.

## Predeclared Numerical Policy

The reference is independent FP64 `math.fsum` over exactly widened BF16
products, not an emulation of the device's accumulation sequence. Let
`u32 = 2^-24`, `u64 = 2^-53`, and `gamma(n,u) = n*u/(1-n*u)` with `n = 1024`.
Each row uses the upward-rounded absolute bound

```text
(gamma(1024,u32) + gamma(1024,u64)) * sum(abs(weight[k] * input[k]))
```

The implementation also rounds the intermediate upper bounds upward and
accounts for reference rounding in the absolute-product sum. The FP32 term
bounds the 1024 FMA accumulation errors; the FP64 term conservatively covers
the independent reference. There is no fitted tolerance or arbitrary absolute
error floor. Finite-operand, non-underflow-lattice, and overflow-envelope checks
make the assumptions explicit. `reference.py` and all four fixture files were
frozen before the first GPU dispatch and are unchanged afterward.

| Case | Additional exact requirement | GPU max absolute error | Max error / bound |
| --- | --- | ---: | ---: |
| Zero | All 1024 outputs exactly zero | 0 | 0 |
| Basis | All 1024 outputs equal checkpoint column 37 | 0 | 0 |
| Mixed | Fixed signed BF16 input pattern | 1.7881393432617188e-7 | 0.0001264578406250646 |
| Cancellation | Constructed row-zero dot exactly zero | 0 | 0 |

All 4096 values passed on MI350 gfx950. Each dispatch also passed immutable
input checks, prefix/suffix canaries on all three allocations, completion,
reverse-order freeing, queue close, and worker exit. These are correctness
results, not device performance measurements or a model quality evaluation.
No comparison with an optimized GEMV library is claimed.
The complete four-case `verify.sh` rerun reproduced these results with frozen
CPU preflight before dispatch. [evidence.json](evidence.json) contains the
sanitized file-bound engineering summary and per-case receipt hashes.

## Reproduce And Bind Evidence

`build.sh ROOT NEW_BUILD_DIR` records extractor/native-tool inputs and emits
the source handoff, unchanged LLVM, HSACO, observed metadata, and hashes.
Run `extract_checkpoint.py --help` once to extract the pinned checkpoint, then
generate all four cases with `reference.py generate`. Host tests are:

```sh
python3 -m unittest -v test_reference test_binding
```

The sole device orchestration entry is:

```sh
bash verify.sh PROBE WORKER HSACO SOURCE PRIVATE_DEVICE_ID \
  WEIGHTS_DIR CASES_DIR NEW_EVIDENCE_DIR
```

The device selector is private local input, never tutorial evidence. The script
revalidates all frozen references before dispatch, inspects the exact artifact,
executes the four cases serially, and rechecks hashes of all numerical source,
tools, weights, and object inputs afterward. `bind_evidence.py` creates
`comparison.json` and `dispatch-numerical.json`, binding each independent
comparison to exact source/object/probe/worker/input/weights/output hashes,
dispatch metadata, guards, completion, and cleanup. Substituted identities,
geometry, policy, bounds, authority, or extractor source fail closed.

The first four root runs live under `evidence-gpu/kproj-CASE-root-v1` on the
qualification host. Their original `numerical.json` files remain intact.
Their later CPU revalidation manifest is explicitly labeled post-run: it
confirms frozen fixture identity but is not proof that this later binding step
preceded those historical dispatches. Fresh `verify.sh` runs establish that
ordering directly. All evidence is engineering-only (`authority: none`).

First-run HSACO SHA-256:
`53a4d535ec1d9f7eb75b8b501af840cc92e4bc9b98d70c1c915aa3b7ec776792`.
First-run probe SHA-256:
`c0cd2b465abe0dff6aa37c4f72731c843c53615d8111947b651d7c62e53bb3d3`.
The complete scripted rerun used probe SHA-256
`1301933c47bc1361513247516a9f8d7ab666612374541e5951ad06fa3d68240d`.
Subsequent runs must retain their own executable hashes rather than inheriting
these values after a rebuild.
