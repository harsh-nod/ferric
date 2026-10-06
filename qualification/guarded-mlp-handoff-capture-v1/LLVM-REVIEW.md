# Captured Guarded MLP LLVM

Status: **exact pre-worker LLVM captured and decoded on MI350**. This is
data inspection, not another compiler invocation, GPU test, runtime admission
or completed memory-coherence proof.

## Identity And Scope

The [checked replay](README.md#actual-checked-replay) joins the captured handoff
to both compiler observations, the prior HSACO and its canonical descriptor.
The [decoder](decode_handoff.py) ran on MI350 and produced this
[receipt](llvm-inspection-v1/complete.json): 2,993 bytes, SHA-256
`8f0fcea1b86f8868526159110dc4b6e3150a866b0c1525a344bf3c48b9b95772`.

The V2 outer-wire layout was checked against the qualified
`crates/fe2o3-compiler-ffi/src/module_handoff_v2.rs`, 46,043 bytes, SHA-256
`cc8c883f9bbdb5826cea4d08ae4307fd4dfb039531b2110ef2b415f589dafc25`.
The decoder enforces the exact whole-handoff identity, bounded field lengths,
target `gfx950:xnack-`, code-object version 6, LLVM-text kind, module and
symbol-manifest hashes, UTF-8 and exact end of input. It does not re-admit
the nested envelope or symbol-manifest semantics.

| Retained body | Handoff offset | Bytes | SHA-256 |
| --- | ---: | ---: | --- |
| [Envelope](llvm-inspection-v1/envelope.bin) | 136 | 53 | `493e67c3eeef2855691fa4bff461e25253e5de51bb6e4406c5c153c2f8744108` |
| [Symbol manifest](llvm-inspection-v1/symbol-manifest.bin) | 189 | 336 | `2af69809a9a9b93a08e369788e2afd784ca648a68eb97945e08caf5dc966b215` |
| [LLVM module](llvm-inspection-v1/module.ll) | 525 | 288,217 | `60ca7e17854b4b6ef1184e352f590110890617fbfbd1664a20b11709d3127b56` |

These are unchanged byte slices of the actual handoff. The retained decoder
copy is 5,709 bytes, SHA-256
`6c7b9381c6fa458ee0ef515eaee1653ca120395b7c26251e48b51c8c576801d9`.
All input bodies were rechecked after decoding. No compiler, object or GPU
execution occurs in the decoder.

## Actual R2 Control Flow

Line numbers below refer to the retained module, before worker optimization.

- The R2 definition starts at line 5574. Guard pointers are `%arg4.data` and
  `%arg5.data`; each length is checked to equal four. Generation arguments
  are `%arg6/%arg7`.
- Lines 5659, 5666, 5673 and 5680 load the first guard's verdict, low tag,
  high tag and reserved word. Lines 5696, 5703, 5710 and 5717 do the same for
  the second guard. All eight instructions are `load atomic ... acquire`,
  aligned to four bytes, with no explicit `syncscope` operand. Addresses
  depend only on kernel arguments and constant word offsets 2/0/1/3.
- Each guard must have verdict 1, matching low/high generation, and reserved
  zero. The generation must be nonzero. `%v238` combines both guard results;
  line 5728 branches to `%bb50` on true and `%bb39` on false.
- The false path reaches return without a payload load or output store.
  The true path reaches three volatile payload loads at lines 5737, 5761
  and 5790. Bounds and finite-value checks precede the sole output store
  at line 5823. There is no R2 guard write or atomic RMW.
- The module does not contain `amdgpu.uniform`, `invariant.load`, ballot or
  read-first-lane annotations/intrinsics. R2's pointer arguments have no
  `readonly` annotation. Its attributes select gfx950, Wave64 and a fixed
  64-thread workgroup; they do not explicitly declare the validity branch
  uniform.

The validator also retains its intended publication order in this input:
548 acquire loads, then monotonic stores to generation low/high and reserved
at lines 5537/5543/5549, followed by the release verdict store at line 5562.
This does not itself prove cross-GPU publication on the live allocations.

## Relation To The Final ISA

The [actual image review](../guarded-mlp-atomic-load-alias-lowering-v1/IMAGE-REVIEW.md)
observes a wave-any validity branch without an explicit per-lane EXEC
intersection before the first payload load. The captured input shows a normal
LLVM conditional branch, not an explicit front-end wave-any replacement.
This narrows the investigation to downstream transformation/code generation
and its uniformity assumptions; it does not identify a particular worker pass
or establish a general compiler bug.

The checked request uses O2, verification and debug stripping. The worker also
performs export retention and implicit-argument canonicalization. A standalone
`opt -O2` invocation would not, by itself, reproduce that complete worker
pipeline. No optimized LLVM body is retained by this observation.

## Required GPU Tests

If both validator publications are visible before R2 starts and no guard word
changes until both R2 dispatches finish, all lanes read the same guard values.
Under that condition a wave-any branch and per-lane validity are equivalent.
The proposed packet graph orders both validators before R2, but its paired
coordinator, peer visibility and quiescent rearm remain unqualified.

The next focused engineering tests must therefore distinguish:

1. Stable component behavior: real atomic storage, both-rank valid guards,
   stale low/high tags, pending/failed verdicts, nonzero reserved words and
   corrupted validator input. Rejected cases retain all output sentinels;
   valid cases match an independent BF16 reference for all 4,096 outputs.
2. Producer/consumer ordering: actual validators, both-validator completion
   barrier, both R2 consumers, all completion signals, then quiescent rearm
   with the same allocations. Serial host ordering does not prove this graph.
3. General atomic divergence, separately: record each lane's loaded value and
   conditional output from that same value, with bounded writer activity and
   varying-address/RMW controls. No mixed observations means an inconclusive
   contention test, not a proof that atomic observations are always uniform.

Keep generic protected SharedAtomic refusal gates unchanged. The private
engineering test must bind the exact image and actual owner regions; it cannot
convert this inert evidence into protected launch authority. GPU correctness,
all issue #42 milestones and the 700 tokens/s target remain open.
