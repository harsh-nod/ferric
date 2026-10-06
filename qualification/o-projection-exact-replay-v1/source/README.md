# Exact Historical O-Projection Replay

This is an unexecuted CPU-only diagnostic proposal, not a kernel change or a
numerical acceptance rule. Root owns source qualification and the bounded MI350
run. No model framework, project compiler, GPU binary, or original controller is
executed by the replay. No original evidence namespace is modified.

## Closed Scope

The diagnostic reads all 4,096 O-projection rows and both 2,048-column rank
shards for layer 0, token 9112, position 0. It consumes the retained historical
SiLU capture (`67a23528...`) and genuine repeated framework capture
(`cf751202...`). Both native attention halves must concatenate byte-for-byte to
the genuine framework's 4,096-word attention operand before any O comparison.

The complete original safetensors shard is SHA256-checked twice, before and
after computation. The index and exact header identify the BF16 [4096,4096]
O tensor, with no bias and a bounded 32 MiB extent. Deinterleaving each source
row's two 2,048-column halves must reproduce both actual registered/uploaded
O-shard hashes. All smaller inputs and source files are authenticated and
rechecked after use with ordinary canonical read-only file custody.

The retained historical prefix image is exactly 53,560 bytes,
`4885204c8d510122588549107f42d2bc6f180f48fbc4eddd3bb1260e8d6629c5`.
`row-reciprocal-checked-probe-v228-v7` contains its source, LLVM, image, and
actual successful disassembly. The replay binds all eight selected bodies and
joins the image to the capture request. This is not an inference from a later
image that happens to use the same source.

## Arithmetic

Every finite BF16 operand is an integer multiple of 2^-133. The exact real dot
therefore uses signed integer units of 2^-266, with no floating-point summation
or tolerance. The previously qualified QKV `exact_bf16.py` body is reused
byte-for-byte; its once-rounded BF16 result and exact distances are retained
for every output row, including equal rows.

The independent FP32 simulator rounds each multiplication and addition to
24-bit RNE with gradual underflow, preserves signed-zero rules, and refuses
nonfinite source operands or intermediate results. It models 64 lanes, 32
terms per lane in `step*64+lane` order, followed by synchronous XOR masks
1,2,4,8,16,32. It compares all 8,192 predicted FP32 partial encodings to the
directly captured native encodings. It also records rounded-product and
subnormal-product/sum counts, rather than assuming those cases absent.

The exact historical LLVM `df75a6fd...` uses separate `fmul` and `fadd` for O
at lines 3234/3236, six XOR stages at 3260-3297, and `fp-contract=off`, IEEE
denormals, and no unsafe/approximate math at line 6320. In actual disassembly
`ff67d340...`, the 32 products occupy 0x3cfc-0x4390; ordered separate adds run
0x4398-0x46b4; reduction is 0x46c4-0x475c; the FP32 store is at 0x47a8.
These are static source/codegen observations, not an exhaustive machine
semantics proof or a substitute for comparing actual captured partials.

The native materialized BF16 projection is **derived**, not directly captured:
`BF16_RNE(FP32_RNE(FP32_RNE(+0+p0)+p1))`. The residual replay materializes that
projection before adding the genuine framework embedding. The old native
28-stage capture does not contain its initial embedding operand. Consequently
the residual control is explicitly conditional on the framework embedding;
it does not claim that native residual input was directly observed. The
observed native output residual remains a distinct retained field. The
framework's own projection-plus-embedding boundary must reproduce its genuine
residual exactly, independently of the native partials.

For differing projection values, the report retains both exact errors and the
exact midpoint distance/side. Midpoint numerators use scale 2^-267; other exact
dot/error numerators use scale 2^-266. A source-order match can distinguish a
valid association result from a replay mismatch, but neither framework
accumulator order nor whole-model numerical acceptance is assumed. A native
value nearer the exact real dot is not grounds to change it to match a
framework value on the other side of a midpoint.

## Tests And Execution

The 18 authored tests comprise 12 arithmetic tests and six closed-input tests.
The FP32 rounding oracle uses Fraction values, binary search over ordered
finite encodings, and exact nearest-neighbor distance/tie selection. It does
not reuse the simulator's bit-length/shift rounding. Coverage includes normal
and subnormal midpoints, overflow thresholds, negative underflow, signed zero,
all 64 lanes/32 steps, cancellation/order sensitivity, exact product underflow,
materialization-before-residual, no per-rank BF16 narrowing, malformed shapes,
wrong shards/bias/extent/rank, duplicate/missing uploads, truncated/swapped
weight columns, and an intentionally wrong residual boundary control.

Root should execute both named test modules before the all-row replay:

```text
python3 -B -m unittest -v test_fp32_replay test_replay
python3 -B replay.py o-projection-exact-replay-v228-v1 ACTUAL_REPLAY_SHA256 \
  --reference-directory FRESH_REFERENCE_ALIAS \
  --kernel-directory FRESH_CODEGEN_ALIAS
```

Use a qualified bounded parent to retain original stdout/stderr and terminal
status. The replay enforces MI350 UID/hostname, affinity 8/9, nice 10, hidden
HIP/ROCR/CUDA devices, 512 MiB address space, 600 CPU seconds, 900 wall seconds,
16 MiB result size, and zero core files. The result label must be fresh.
The two optional alias directories preserve the original pin paths in the
receipt while rehashing physically staged copies; they never rewrite old
directories. The kernel alias retains the eight relative paths in
`KERNEL_PINS`; the reference alias contains capture.json plus eight selected
pass-stage bodies. Complete source and codegen hashes are in the manifest.

The program emits `complete.json` only after all read and source postchecks.
`completed=true` means this diagnostic finished, not that native/framework
outputs match. A failure or timeout has only the enclosing controller's raw
failure evidence and must not be promoted to a complete report. The report
keeps GPU execution, full-model correctness, numerical acceptance, production,
performance, and sustained 2048/256 claims false. Current guarded layer0
internal stages are being captured separately; this replay does not claim to
have observed them.
