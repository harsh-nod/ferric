# Bounded Fast Draft Profile Plan

Read-only design against root 24dddba. This profile is not implemented, emitted,
proved or GPU-qualified. The standalone TP-v1 draft canary is a prerequisite,
not a fast or speculative path. Target TP2 is not a Draft06B model alias.

## Device Boundary

Add a separate closed TP1/32-row Draft06B image with draft-specific symbols.
Preserve every target v5/v8/v9 root and its immutable provenance. Initial bounds
are logical context 8192, page size 16, at most 512 physical pages and 32 rows;
exclude wave, peer, large-KV, sequences and ordered batches until independently
qualified. Use the current public fe2o3 compiler and separate artifact pins.

The standalone image has 14 roots: a renamed body-equivalent generic RMSNorm;
embedding; scalar and MFMA BF16 projections; scalar and MFMA FP32 partials;
SwiGLU; RoPE; paged append; GQA attention; FP32-partial/BF16-residual addition;
scalar and MFMA FP32 LM-head; FP32 argmax. If generic RMSNorm instead comes from
an already admitted base image, the supplemental roster has 13 roots; its host
contract must require that exact base root. Prefer the standalone closed14
image so role admission does not depend on loading unused target math.

| Projection | N | K |
| --- | ---: | ---: |
| Query | 2048 | 1024 |
| Key / value | 1024 | 1024 |
| Attention output | 1024 | 2048 |
| Gate / up | 3072 | 1024 |
| Down | 1024 | 3072 |
| LM-head | 151936 | 1024 |

RoPE has 16 query and eight KV heads of width128; GQA ratio is two, not four.
Embedding/residual width is1024, SwiGLU is3072. All projection dimensions are
16-divisible; existing MFMA tiling and masked inactive-row stores can be reused
with new exact shape guards. Retain authenticated NxK weights and setup-only
KxN transposes. FP32 head/argmax avoid introducing a new BF16 head tie into the
fast candidate, but constitute an explicit new numerical profile.

Paged append TP1 math has the same 1024-wide KV row as target TP1 and can be
body-equivalent apart from symbol/profile identity. Attention must change head
count and Q-to-KV mapping. Prove/check all helper and inlined-copy differences,
not just exported bodies. Draft payload remains1,503,264,768 bytes; legacy
512-page KV payload is939,524,096 bytes (28 layers, K+V, BF16,1024 channels).
Optional32-row FP32 logits add19,447,808 bytes, excluding other workspaces and
transposed weights. These are arithmetic payload bounds, not measured RSS.

## Minimal Host Boundary

- `tp_execution/batched.rs:1366` rejects every non-Target8B role. Add a separate
  `new_draft32` constructor with exact Draft06B geometry and a sealed loaded
  draft-image binding before allocations; leave target constructors unchanged.
- Normal forward callsites already derive projection dimensions and layer
  counts from authenticated model/rank geometry. `projection.rs:65` transposes
  role-local authenticated sections. Reuse these; give symbol selection an
  explicit role/profile mapping instead of pretending the draft is targetTP2.
- `batched.rs:221` head selection and worker image-symbol rewriting currently
  select target v7/v8. Add separately named draft head/profile routing with
  exact workspace/loaded-image checks; do not broaden old profile labels.
- Numerical capture at `batched.rs:1234` hardcodes target projection dimensions;
  reject it for draft initially. Preserve target capture behavior and tests.
- Pool metadata already uses an opaque nonzero model identity. Use a separate
  draft-model scope/session and verify that the draft constructor receives that
  scope, not the target bundle/model pool. Never share role-local page ownership.
- Keep initial entrypoint a standalone draft batch canary using authenticated
  `open_with_draft`. Target-only serving/runtime defaults remain unchanged.
  A persistent target-plus-draft coordinator is a later integration slice.

## Gates And Subsequent Settlement

CPU gates must cover all role/shape/extent rejections, exact tied weights,
MFMA layout/input tails, full FP32 argmax including exact ties, finite traps,
GQA ratio2, causal page bounds and duplicate writes. Recording tests bind every
actual command's kernel, geometry, tensors and dispatch count at rows1/5/16/17/32
to the draft profile. Misbinding target image, target pool or draft-as-TP2 fails
before allocation. Default target command recordings remain byte-identical.

Root-owned native fixtures should cover each projection shape with dyadic exact
references at rows1/5/17/32; full-vocabulary head/argmax ordinary-ID separation;
full active and guard buffers; Q16/KV8 normalization/RoPE; same-sequence causal
multirow append/attention crossing positions15/16 and the last admitted page;
different KV heads with distinct values to expose a ratio-four mapping. Invalid
or nonfinite device-fault cases remain host-only on the shared GPU machine.

Only after native gates and the standalone independent draft reference pass,
run matched scalar/FP32 versus MFMA/FP32 draft model cases. Preserve all strict
tokens/bytes and report setup separately. New dtype/reduction order is never
automatically accepted because outputs are plausible.

K+1 target verification can reuse ordinary target multirow math, but current
`tp_paged.rs:739` commits the whole prepared batch and rejects submitted aborts.
The independently owned accepted-prefix settlement work must retain submitted
suffix storage until completion, commit only the accepted role-local prefix,
and avoid publishing ordinary scheduler choices prematurely. First integrate
one bounded K=4 round with clean closure; persistent speculation requires the
additional full-accept draft catch-up and shared cursor/publication contract.
