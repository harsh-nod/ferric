# Current Own-Input Attention Gate

Source-only CPU proposal. The author has not imported these modules, run their
tests, evaluated attention or executed a model. The primary owns MI350 runs.

This gate evaluates all twelve captured native attention states (positions
0 through 5, ranks 0 and 1), each against its **own captured Q/K/V**, using the
unchanged historical independent dense FP64 reference and preregistered
attention policy. It separately evaluates the twelve framework states against
their own captured operands. Each side has 24,576 BF16 output words. There is
no cherry-picked acceptance sample and no substitution of one side's Q/K/V
into the other side's observation.

## Existing Policy and Its Scope

`attention_reference.py` is byte-identical to the original retained body at
`qualification/prefix-profile-numerical-v1/p227-prefix-numerical-sidecar-v1/attention_reference.py`.
The gate calls its original `dense_reference()` and `compare()` functions,
not its fixture launcher or diagnostic online-emulation function. No NumPy,
PyTorch or GPU import is needed by those two functions. `head.py` is the
unchanged qualified finite BF16 integer/JSON helper from the exact-head gate.

The policy remains:

- Position zero: exact BF16 output, including the one-token value control.
- Other positions: at most one BF16 step **OR** absolute error at most
  `5e-5 * per_mapped_KV_head_max_abs_V` over the causal value history.
- Finite Q, every causal K/V word, scores and outputs; exact logical extents.
- Reference narrowing: FP64 two-pass oracle, then FP32 RNE, then BF16 RNE.
- Scale: the original FP32 constant `0x3db504f3`, not a fitted scale.

This is an unchanged, previously declared **conditional component gate**.
It is not a new full-model tolerance, a framework internal-arithmetic proof,
or complete qualification of the current kernel's exponential/ISA error
contract. The retained profile comparator already made this distinction:
`p228-prefix-profile-numerical-v1/README.md` requires separate artifact math
review and never turns conditional checks into runtime admission. The current
result likewise keeps numerical acceptance and all authority fields false.

## Actual Inputs and Six Diagnostics

The exact original comparison is 285,894 bytes / SHA256
`3f94fe36125e3cbbf92e33caca6049b94fd33963956ef1477bf304dcec6bf38f`.
It reauthenticated the full native and reference capsules, original prompt,
model/bundle, same-side parity, all forty records and both reference passes.
This new gate relies on that pinned admission rather than rebuilding another
capsule verifier. It rehashes the same eight original body aliases already
staged for the QKV oracle, at the exact `INPUT_ROOT` in `run.py`, before/after
use. Those aliases and their full SHA256 values are unchanged.

The original native sidecar is decoded with its per-position offsets; the
framework sidecar uses global offsets. Every consumed part is hashed. The
framework head/token/channel KV layout is converted by the byte-identical
`cache_rows()` function from the qualified causal observer. All four full
tensors per state are joined to the original comparison pins. Reference
sidecar payloads and complete part metadata must agree across both passes.
All five QKV outcomes and every model shard remain unchanged and are not
recomputed here. No model shard is opened by this gate.

Only these six attention words differ between the two recorded sides:

| Position | Rank | Local Output | Query Head | Channel | Native | Framework |
| --- | --- | --- | --- | --- | --- | --- |
| 2 | 1 | 1455 | 11 | 47 | bb36 | bb37 |
| 2 | 1 | 1820 | 14 | 28 | b50b | b50c |
| 3 | 1 | 216 | 1 | 88 | b96a | b969 |
| 3 | 1 | 1804 | 14 | 12 | b97b | b97a |
| 4 | 1 | 1334 | 10 | 54 | b3cf | b3d5 |
| 5 | 1 | 1901 | 14 | 109 | b79e | b79d |

All six have byte-identical captured Q/K/V. The position-4 difference spans
six BF16 encodings near cancellation but has absolute difference only
`2.7939677238464355e-9`; encoding distance alone is not the existing gate.
Input Q differs on positions/ranks 0/0, 1/1 and 2/0, but those attention
outputs agree. They still receive separate own-input checks, not an assumed
shared-input interpretation.

The six scalars also receive an independent two-pass Decimal calculation at
80 and 160 decimal digits. The 128-term QK dot is accumulated exactly as an
integer in BF16 product units before conversion to Decimal. It uses the
original exact binary FP32 scale, global score maximum, exponential weights,
and the mathematical weighted-value sum. It does not replay Wave64,
FP32 recurrence, OCML or framework SDPA ordering. Decimal results are rounded
directly to BF16 by integer ratio comparisons, independently of the historical
FP64 -> FP32 -> BF16 narrowing. Both precisions and their word stability are
reported. Precision agreement is **not** an outward interval or a proof of
correct rounding; it is not used in the pass/fail policy. An unresolved
precision comparison must not be described as an exact mathematical oracle.

## Source Interpretation

The actual selected prefix image has SHA256 `29fd58e7...`, rather than the
setup image. Its retained fixture and LLVM at
`qualification/rope-indexed-checked-emission-v1` show:

- `fixture/src/prefix_tiles_numerics_v6.rs:100`: each lane forms two FP32 QK
  products, adds them, uses the fixed Wave64 reduction and scales afterward.
- `fixture/src/attention_online.rs:26`: only causal tokens are loaded, in
  increasing order. Token zero initializes exactly to V; later tokens update
  a max-rescaled FP32 denominator and two numerators, then FP32 divide/BF16.
- `artifacts/extracted/module.ll:2428`: XOR tree and separate FP32 adds;
  `:2850` calls OCML exp; `:2856` onward retains separate recurrence products
  and adds. These differ from a dense two-pass arithmetic schedule.

The captured framework files show grouped KV repetition and math-only SDPA
with FP32 intermediates. `implementation-modeling_qwen3.py:179` specifies
`head_dim**-0.5`; `implementation-sdpa.py:55` calls SDPA. Its exact internal
reduction tree is not captured. No bitwise identity or current native error
proof is inferred from those source facts, and no arithmetic patch follows
merely from the six differences.

## Execution and Next Gate

Stage the six source bodies plus manifest under the fresh root:

`/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-causal-attention-conditional-v228-v1`

The primary invokes `/usr/bin/python3 -I -B <root>/run.py <manifest-sha256>`.
The exact source manifest is supplied after peer review. Eight focused tests
precede actual input admission: immutable policy/scale, both-rank GQA/value
geometry, uniform-score cancellation, independent own-input checks, exact KV
permutation, nonfinite/extent refusals, Decimal rounding and a known mean.

The compact QKV-controller pattern is reused: MI350 host/owner, GPU hidden,
affinity 8/9, nice at least 10, 512 MiB address space, 120 CPU seconds, 180-second
continuously rearmed wall bound, no core and 2 MiB output-file cap. No child,
model, compiler or GPU is launched. The old input directory is read-only;
fresh `output/` refuses retries. Original test stderr and complete/failed
receipt remain. An exhausted hard deadline may leave only a prefix, never an
accepted success. All source and selected input hashes are rechecked.

Passing would close this one conditional attention question under the
existing policy. It would not prove every layer/operator or the full request.
The 39/40 figure is a teacher-forced prompt-position argmax diagnostic, not
generated-token correctness. Existing generated-output gates compare actual
generated token IDs and decoded bytes with an independent reference. Their
scope must stay distinct from intermediate BF16 bitwise matching. The genuine
2,048-prompt/256-generation workload and its independent acceptance remain
separate goals; these findings do not justify fitting a new tolerance or
stalling indefinitely on forcing PyTorch's reduction order.
