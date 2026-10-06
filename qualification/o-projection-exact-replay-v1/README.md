# Exact O-Projection Replay On MI350

The CPU-only replay completed on `mi350` on 2026-10-06. It explains the
isolated historical layer-zero O-projection differences; it does not change
kernel arithmetic, execute a new GPU workload, or establish full-model
numerical acceptance. All issue #42 milestones remain open.

## Results

| Check | Matching values | Total |
| --- | ---: | ---: |
| Captured native FP32 partial vs modeled kernel order | 8,192 | 8,192 |
| Derived native BF16 projection vs once-rounded exact dot | 4,095 | 4,096 |
| Framework BF16 projection vs once-rounded exact dot | 4,093 | 4,096 |
| Derived native projection vs framework projection | 4,093 | 4,096 |
| Native residual vs conditional boundary replay | 8,192 | 8,192 |
| Native residual vs framework residual | 8,188 | 8,192 |

The two native attention halves exactly equal the genuine framework attention
operand. Source weights are joined to both registered/uploaded rank shards.
Every captured FP32 partial matches separate FP32 multiplication/addition in
the pinned kernel's 64-lane, 32-step, six-stage XOR reduction order. No rounded
products or subnormal products/sums were observed for these operands.

Only three derived projection words differ from the framework:

| Row | Once-rounded exact dot | Native | Framework | Native residual | Framework residual |
| ---: | --- | --- | --- | --- | --- |
| 1024 | `bc9d` | `bc9d` | `bc9e` | `bd2d` | `bd2e` |
| 1356 | `b490` | `b492` | `b491` | `3c89` | `3c89` |
| 3444 | `3b88` | `3b88` | `3b87` | `bb86` | `bb87` |

Entries are BF16 encodings, not decimal values. Both native ranks have the
listed residual. At row 1356 both implementations differ from the ideal
once-rounded dot; the framework value is closer. At rows 1024 and 3444 the
native projection equals the ideal. These observations support reduction-order
rounding as the explanation for the isolated differences, not a missing BF16
materialization boundary. They are not a reason to force native outputs to
match a particular framework accumulation order.

## Scope And Method

The retained capture is layer 0, token 9112, position 0, from the historical
SiLU-materialized run. It is not a fresh internal-stage capture of the current
guarded model. Each exact BF16 dot is accumulated as an integer in units of
`2^-266`; the FP32 simulator implements round-to-nearest-even after each
operation. The source, LLVM and actual ISA are pinned to the historical image.
All 18 arithmetic/input tests passed on MI350 before the full replay.

The combined native BF16 projection is **derived from directly captured FP32
partials**, not directly captured. The residual replay uses the genuine
framework embedding because the old native capture did not retain its initial
embedding. Its agreement is conditional on that operand. The framework's own
projection-plus-embedding boundary reproduces its residual independently.
No framework accumulation tree is inferred.

An independent data review recomputed all rows' rounded encodings, distances,
midpoints, totals and residual boundaries using integer decoding and a
binary-search rounding oracle without importing project code. It joined all
8,192 captured partials and residuals to raw payloads and checked both framework
passes. That review did not locally reread the full remote model shard or
recompute its exact dots; the executed MI350 replay performed those checks.
The [independent review](independent-review/README.md) includes conditional
rounding bounds and a standalone reproduction script. That packaged script
was syntax-checked, not executed; the review was performed with separate
read-only data checks.

## Retained Evidence

- [Complete all-row report](complete.json.gz): gzip container SHA256
  `ffb00e00d8783d694a4f0379e23a9d96252944371696304c4807638c716dddbf`.
  Decompression reproduces exactly 10,946,336 bytes, SHA256
  `869541d4852c6cc1af45164f2a2521b075db30c62ed0487309cf0dd16abaa515`.
- [Actual 18-test result](tests-complete.json): 4,605 bytes, SHA256
  `701e09f47946c2974c0f79eca0d14e5b1cb21e94ee197955dc12ee796ea075d9`.
- [Executed source manifest](source/source-manifest.json) and
  [source methodology](source/README.md), preserved byte-for-byte, including
  their pre-execution proposal status. This page records subsequent execution.
- [Primary test harness](tests-primary.py) and
  [primary replay invocation](replay-command.txt). These are primary-session
  observations, not receipts from an earlier qualified process supervisor.

The replay ran with GPU visibility disabled, CPU affinity 8/9, nice 10,
512 MiB address-space and 600 CPU-second limits, a 900-second outer timeout,
and a 16 MiB result limit. Its SSH invocation exited zero. The original
4 GB safetensors shard was hashed before and after use; the selected source,
reference, capture and codegen inputs were also rechecked. Large model weights
and original capture payloads are not duplicated here.

The next gates remain current guarded internal-stage capture, independent
full-model numerical acceptance, host-overhead attribution, and sustained
single-request BF16 2,048/256 performance. This result supplies no GPU timing,
speedup, production admission or 700 tokens/s claim.
