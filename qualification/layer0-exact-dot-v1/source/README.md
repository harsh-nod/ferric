# Exact Layer-Zero QKV Dot Audit

This source-only CPU diagnostic compares all 6,144 Q/K/V projection words at
token 9112, position zero, against an exact sum of the original BF16 products,
rounded once to BF16. It does not model either implementation's FP32 reduction
tree, identify a semantic bug, or establish numerical acceptance. No model or
GPU invocation occurs. O projection is deliberately outside this first audit:
its native capture contains two FP32 partial vectors, not a BF16 projection.

## Inputs And Mapping

- Current SiLU layer capture: original completion `67a23528...`, native summary,
  request, bootstrap, registration, upload manifest, and 9,670,656-byte capture.
  These are read at their existing original `E/prefix-silu-materialized-capture-gpu-v228-v1`
  paths. This is not an observation of the newer RoPE image's intermediates.
- Genuine framework `capture.json`, SHA `cf751202...`, and eight payloads:
  `pass{1,2}-{input-norm,q-projection,k-projection,v-projection}.bf16`.
  Root transports exactly these nine files from
  `L/layer0-framework-capture-v228-v1/` to
  `E/layer0-exact-dot-reference-v228-v1/`.
  The result retains original and alias identities separately. Both consumed
  passes must agree byte-for-byte. The six redundant Q/K/V input payloads are
  not opened; their authenticated receipt pins must equal the consumed norm.
- Existing MI350 model root:
  `/home/harmenon/ferric-mi350-model-b968-c189-r1.kUjwIKs0/model/target`.
  Index, config, and shard 1 retain the framework's original model identities.
  The complete 3,996,250,744-byte shard is streamed and hash-checked before and
  after computation, without loading it into memory. The exact 9,328-byte
  safetensors header, BF16 dimensions, fixed offsets, no-bias config/index,
  and all six rank-specific contiguous matrix upload digests are checked.
- Frozen oracle `exact_bf16.py`, SHA
  `b9e53a3afa4fb851a8e7231c020e5ed19fb55deb7ce4d9f0bff1454e472acddf`,
  and its separately qualified 18-test source are rehashed, not tested here.

Q has 4,096 rows and K/V each have 1,024, all with 4,096 columns. Each native
rank owns half the contiguous model rows and records `[Q2048, K512, V512]`.
Both native norms and both genuine framework norms must be the same 8,192
bytes (`49b3afa6...`). Every BF16 operand and observed result must be finite.
The already-observed Q row 168 disagreement is not a selection criterion: all
rows are evaluated, including rows where native and framework already agree.

Each row records its original shard offset, exact signed integer sum in
`2^-266` units, ideal BF16 encoding, both observed encodings, both exact
absolute distances, ideal-agreement booleans, and which observed value is
closer. Equal distances remain ties. Ideal overflow and signed underflow
follow the oracle; exact cancellation uses canonical positive zero. These
are mathematical diagnostics, not an assertion that either reduction must
equal the single-rounded exact sum.

## Root Execution

Root owns all tests and execution. The author has not imported or executed
the diagnostic or tests. `test_audit.py` has six synthetic tests covering
rank/row/capture offsets, exact geometry, and malformed-header/index refusal;
the independent oracle has 18 arithmetic tests.

One bounded MI350 CPU command is sufficient after the separate pure tests:

```text
env HIP_VISIBLE_DEVICES= ROCR_VISIBLE_DEVICES= CUDA_VISIBLE_DEVICES= \
  PYTHONDONTWRITEBYTECODE=1 timeout --signal=TERM --kill-after=5s 360s \
  taskset -c 8,9 nice -n 10 python3 -B audit.py \
  layer0-exact-dot-v228-v1 ACTUAL_AUDIT_SHA256
```

The script checks UID 9661, the exact MI350 nodename, CPU affinity 8/9,
nice 10, and hidden GPUs. It sets 512 MiB address space, 300 CPU seconds,
16 MiB output, and zero core limits. The caller supplies the wall deadline;
no subprocess is spawned. Only the pinned small oracle module is loaded.
Selected matrices total 48 MiB, but processing is row-wise. All small inputs
and source bodies are rehashed afterward. A fresh output namespace is
required and only `complete.json` is written, after all checks succeed.

No lifecycle replay, GPU timing, acceptance threshold, whole-model correctness,
new-image internal-stage claim, or 2048/256 claim is made. Existing position-zero
residual differences and later attention/reduction questions remain independent.
