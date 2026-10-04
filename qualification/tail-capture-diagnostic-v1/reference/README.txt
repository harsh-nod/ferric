P222 WHOLE ORIGINAL-WEIGHT TWO-FORWARD DIAGNOSTIC

This isolated proposal does not execute a GPU, read GPU intermediates, authorize
a model, or establish a numerical acceptance threshold. It extends the retained
P219 independent staged equations through all36 layers and the original NxK
language-model head. Both hidden recurrence and every layer's causal KV history
are computed here. Original weights are pinned independently of native packing.

Input roster (all FilePins have exactly path, bytes, sha256):
{
  "schema": "ferric-p222-original-model-reference-inputs-v1",
  "model": "Qwen/Qwen3-8B",
  "revision": "b968826d9c46dd6066d109eabc6255188de91218",
  "mode": "teacher_forced",
  "tokens": [FIRST, SECOND],
  "token_provenance": FILE_PIN,
  "source_authentication": FILE_PIN,
  "config": FILE_PIN,
  "index": FILE_PIN,
  "shards": [FILE_PIN1, FILE_PIN2, FILE_PIN3, FILE_PIN4, FILE_PIN5]
}
Config/index/shards must match the exact original hashes freshly couriered by
root in p222-original-target-pins.json. Canonical absolute paths may be relocated
without changing content identities. This code independently validates every
original safetensors tensor's role, full dimensions, dtype, offsets, complete
payload partition, index membership and complete399-tensor roster.

The two provenance receipts are retained and hash-checked, not interpreted as
new compiler/native authority. Root must supply the authenticated source intake
receipt and exact token selection provenance. No default or fabricated token
roster is supplied. For autoregressive mode set mode="autoregressive" and pass
exactly [FIRST]; the second token is this CPU reference's own first greedy choice.
If CPU/GPU first choices differ, their second trajectories have different inputs.
Use separately named teacher-forced reports for deliberate same-token diagnosis.

Root-run commands (not executed by proposal author):
  /usr/bin/python3 -B -m unittest discover -v -s PROPOSAL -p test_model_reference.py
  /usr/bin/python3 -B PROPOSAL/model_reference.py ROSTER EXPECTED_SHA NEW_OUTPUT_DIR

Fourteen unit tests use small synthetic2-layer/2-token tensors plus production
128-head RoPE and retained independent attention-oracle cross-checks. The CPU
environment is NumPy2.2.6 on a little-endian host; no new package is required.
The original model is16,381,470,720 payload bytes, mapped read-only. Reserve a
separate bounded address-space budget above the complete mappings (24GiB is an
initial accounting candidate, subject to root's RAM check), not a12GiB generic
helper envelope. Full original-file hashes are verified before and after work.
Numerical scratch is row-chunked; at most64MiB of stage arrays and4MiB manifest
are written create-exclusive. Partial output on failure is preserved, with no
complete manifest or acceptance claim. No output arrays feed back into numerics.

The policy is deliberately diagnostic: FP64 dot/norm equations are not exact
wave-tree/OCML/MFMA/sequential-FP32 arithmetic. QKV/gate/up/head have the declared
FP32-to-BF16 output boundary; O/down have rank-local FP32 partials followed by
the unchanged integer-bit ordered TP2 residual. This preserves independent
whole-model error visibility rather than applying a permissive aggregate bound.
Acceptance must be separately preregistered and must not hide failed stage checks.

The six helper files are byte-for-byte copies of frozen evidence, not modified
implementations. model_reference.py checks their existing pinned identities.
Production 128/4096 norms and 32Q/8KV/head128 attention dispatch directly to the
retained helpers. The dimension-generic equations are only for smaller synthetic
topology fixtures; they are not a new GPU arithmetic implementation.
Source receipt/ISA/GPU/benchmark qualification stays entirely with the primary.
