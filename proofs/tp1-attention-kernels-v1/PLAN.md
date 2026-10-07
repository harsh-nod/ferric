# TP1 Attention Exact Finite Fixtures V1

Status: implementation for independent review; no native run or numerical qualification claimed.

## Frozen Inputs

Eight ordered cases are uniform rows 1, 17, 31 and selector rows 32, each baseline then wave.
The roots are `ferric_qwen3_tp_batch32_paged_gqa_bf16_f32_v5` and
`ferric_qwen3_tp_batch32_wave_paged_gqa_bf16_v5`. No old fixture, kernel,
reference, or compiler source changes. The source-pinned core helper is
`proofs/tensor-parallel-kernels-v1/probe.py`, SHA256
`a9e702e77f00e414984ac44dc1d72da15145ae3013c6077932c2daef5004203b`.

Only the existing v5 image SHA256
`98b5fdb14ac7242e324e1801e1885c98e77473d622b2e9b3f9f12f14c7d75502`
is admitted. Its leaf is `13a145352770d4cf1b0b6445fc1349f7002822b576c7965089a5c4b0ec82938b`,
with original Ferric `eff229` and fe2o3 `3e74a9324a5acd7107e96a4a9b5319d3dd5ecde8`
provenance, not this helper's newer source ancestry. The existing worker's
separate identity remains an explicit launch pin; no new worker or image is built.

## Geometry And Memory

TP1, 32 query heads, eight KV heads, 128 dimensions, capacity 32 rows,
16 tokens per page, two logical pages per sequence, 68 physical pages,
32-token context. Row r maps logical pages to `[63-2*r, 62-2*r]`.
The last four pages are finite decoys. Positions cycle
`[0,1,14,15,16,17,30,31]`; the one-row fixture uses position 1.
Both page sides, partial pages, odd active rows and inactive output tails are checked.

The six buffers are query 262144 bytes; keys and values 2228224 bytes each;
positions 128 bytes; table 256 bytes; output 262144 bytes. Each fits the
unchanged helper's 4 MiB transfer bound, including two 64-byte guards.
Scalars are `[rows,1,2,68,32]`, grid `rows*32*64`, workgroup 64.
Prior frozen-image native metadata confirms 17 explicit metadata arguments,
116 explicit bytes, hidden offset 120/size 256, kernarg 376/alignment 8.

Every active/inactive floating input is finite. Unused query rows contain -2;
unassigned pages have K=3 and V=-64. Each sequence's future V slots are 224.
Output is initialized to finite BF16 bits `0x3555` and all inactive bytes
must remain unchanged. The helper checks every input byte and both guards,
not only the active output. No intentional device trap is introduced.

## Exact Arithmetic Oracle

Let `B=2*r+8*h+(d%16)+16*(d//64)`, with KV head h and dimension d.
Visible token t has `V=2*t+1+B`. Its maximum is 212, below 256, so all
source integers and expected integer outputs are exactly representable BF16.

Uniform inputs use dense nonzero Q=`1+(query_head%4)` and K=1. All visible
scores are equal positive values. With n visible tokens, their V sum is
`n*n+n*B`; the exact mean is `n+B`. No tolerance or host softmax is used.

Selector inputs use four dense orthogonal Hadamard sign patterns: constant,
bit6, bit0 and bit6 XOR bit0. Query subhead s uses pattern s; token t's K is
32 times pattern `t%4`. Complete dot products are exactly 4096 or zero.
Their scale is the frozen f32 bits `0x3db504f3`, yielding score separation
greater than 256. If matching tokens exist, only that class contributes in
the frozen machine recurrence; otherwise every visible score is zero.
For m matching tokens, the expected output is `B+2*s+1+4*(m-1)`; with no
matches it is `B+n`. Integer sums remain exact f32 and conversion is exact BF16.

The exact frozen fe2o3 lowering maps exp to `__ocml_exp_f32`. Read-only
inspection of the pinned HSACO's 112-byte function at 0x154e8 found an
explicit lower clamp using f32 `0xc2ce8ed0` (approximately -103). Thus
nonmatch differences below -256 return zero, independently of denormal mode.
Zero follows exact zero operations through `v_exp_f32(0)` and `v_ldexp_f32(...,0)`.
Inspection used local LLVM18 disassembly with explicit gfx942 decoding for
these shared instructions because automatic gfx950 decoding was unavailable;
it was not a host-stub test or a new compilation. Actual native byte checks
remain required. Mathematically nonmax softmax mass is also far below a
BF16 rounding boundary, but this is not a general exp-accuracy claim.

## Independent CPU And Native Checks

Ten CPU test methods cover the closed roster, all shapes, all-byte
baseline/wave fixture identity, integer representation and dense orthogonality,
an independent physical-address/integer-dot/full-output oracle, masks,
page permutations/decoys/tails, missing-match selector behavior, helper pins
and malformed launch/output contracts. Independent wrong-page, future-token,
wrong-head, wrong-slot, dropped-QK and half-dot interpretations must differ
from the correct expected values. These mutations never launch on a GPU.
The independent interpreter reads physical bytes and computes integer dot
products rather than calling the production expected-value formula.

Native execution uses the unchanged core probe lifecycle and `--operational`
mode under a separately reviewed root-owned wrapper. Each result must match
the predeclared complete output bytes and unchanged inputs/guards. The two
symbols receive identical bytes and compare with the same oracle, establishing
byte equality without tolerance. The wrapper authenticates the ordered eight
results, exact input/output hashes, metadata, worker identity and normal close.
The root wrapper owns shared-host idle checks, resource caps and cleanup.

This is bounded engineering evidence, not Verus proof, model parity, a broad
moderate-logit softmax guarantee, serving enablement, or a performance result.
Worker dispatch timestamps must not become a benchmark or a speedup claim.
