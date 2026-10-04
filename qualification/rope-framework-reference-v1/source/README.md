# Installed-framework CPU RoPE reference

Source-only draft. Fourteen focused standard-library tests are authored, not
executed. The installed-framework probe has not run. Root owns testing and the
single bounded CPU invocation. No model weights or GPU are needed.

The independent side calls the actual installed `Qwen3RotaryEmbedding` and
`apply_rotary_pos_emb`, pinned to the retained Transformers 4.51.0 source body,
with Torch 2.12.1+rocm7.2. `modeling_qwen3.py` is `704c9145...`; the separately
root-read installed `modeling_rope_utils.py` is `c28b3e88...`. Both are checked
before import and again after the probe. The original interpreter is also
rehashed. This is not a hash closure of every installed Torch/Transformers
dependency or native library.

## Inputs and Scope

The actual passed framework owner `46fd9acb...` binds capture `cf751202...`.
The probe authenticates both original passes' Q-normalized, K-normalized,
rotated Q/K, and cosine/sine buffers. It uses the genuine position-0 token-9112
Q/K values: 32 query heads and 8 key heads, each 128 elements in BF16. Those
same inputs are conditionally rotated at positions 0, 1, 2, 3, 2047, 2048,
and 2303, with theta 1,000,000. Nonzero positions are **not genuine later
position activations**. A second small corpus covers signed zero, subnormals,
adjacent BF16 values and cancellation. The original position-0 table and
rotated outputs must reproduce exactly.

For each corpus/position/Q-or-K, five outputs are retained:

1. The genuine installed eager BF16 function on CPU.
2. Independent arithmetic with the same BF16 table, BF16 materialization of
   each product, and BF16 sum.
3. The same BF16 table but FP32 products before the BF16 result.
4. A Python/libm model of the native f64 frequency/trig to FP32 table, with
   FP32 products before the BF16 result.
5. That f64 table model narrowed to BF16, then materialized BF16 products/sum.

The independent arithmetic imports only the previously pinned standard-library
integer oracle `551f89f9...`. Its exact integer FP32 RNE addition and BF16
narrowing are reused. Multiplication uses the fact that a product of two finite
FP32 inputs has at most 48 significant bits and cannot underflow binary64;
the binary64 product is exact before the explicit binary32 RNE packing step.
No candidate kernel is imported or treated as the reference. Tests include
normal/subnormal ties, overflow refusal and signed-zero negation ordering.

The f64 table is explicitly a **Python/libm construction model**, not an
authenticated Rust table or proof of identical Rust `powf`/trig results.
The actual framework CPU inv-frequency bits and all CPU BF16 table bits are
retained. CPU trig is not GPU trig evidence. The output records discrepancies;
successful processing does not require a hypothesized arithmetic variant to
match, and confers no kernel or full-model numerical acceptance.

## Root Invocation

Use the retained original oracle at
`E/p228-independent-layer-reference-v1/helpers/residual_oracle.py`, whose exact
SHA is checked. The pure test module loads only `run.py` and that stdlib oracle,
not Torch. Root may run `test_run.py` first using an existing bounded CPU wrapper.
The actual invocation, after computing the real source SHA, is:

```sh
env -u PYTHONPATH -u PYTHONHOME -u LD_PRELOAD \
  HIP_VISIBLE_DEVICES= ROCR_VISIBLE_DEVICES= CUDA_VISIBLE_DEVICES= \
  timeout --signal=TERM --kill-after=5s 190s \
  taskset -c 8,9 nice -n 10 \
  /home/harmenon/ferric-asrock-42/evidence/qwen3-long-reference-env-v1/venv/bin/python \
  -I -B /actual/root-staged/run.py rope-framework-reference-v228-v1 ACTUAL_RUN_SHA256
```

The script enforces ASROCK UID 9661, original interpreter/prefix, CPU 8/9,
nice 10, hidden GPU environment, two Torch compute threads, one interop thread,
offline framework loading, 12 GiB address space, 120 CPU seconds, a 180-second
alarm, 16 MiB per file, 128 descriptors and no core dump. All tensors explicitly
use CPU; `cuda.is_initialized()` must be false before and after, without
querying GPU availability. The outer timeout remains root-owned; no new
process supervisor is introduced. No output directory may preexist.

Outputs are small exact little-endian BF16/FP32 buffers plus `complete.json`
with all source/input/output pins, framework versions/build, 28 comparison
rows, table differences and explicit limitations. Input files and output
hashes are rechecked before completion. Failure leaves an incomplete fresh
directory, never a fabricated success. The original owner receipt is pinned,
not requalified: its historical audit/ownership closure is not replayed here.
