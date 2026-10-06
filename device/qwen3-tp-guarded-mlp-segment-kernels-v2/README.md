# Combined-State Guarded MLP Kernels

Experimental additive gfx950 candidate. Its [MI350 CPU qualification](../../qualification/guarded-mlp-combined-state-cpu-v1/README.md)
passes all ten phases and 39 tests. This crate is not connected to Ferric's
inference path and has no successful HSACO compile, GPU execution, model
numerical acceptance or performance result. The v1 crate and its 548-word
state contract are unchanged.

## One Real Atomic Allocation

`ferric_qwen3_mlp_state_guard_v2` accepts one shared `&[AtomicU32]` of exactly
552 words, followed by the two 32-bit generation halves. The allocation layout
is the existing 548-word state prefix (2192 bytes) and a four-word guard suffix
at byte offset 2192, total 2208 bytes. It is not two references relabelled as
one origin: every validator load and store uses the same original argument.

One Wave64 workgroup is required. Lane zero loads all 548 state words with
Acquire, including after an early predicate failure. The exact original state
predicate is shared by an extent-548 legacy test wrapper and an extent-552
combined wrapper; no state check is removed. Guard suffix contents do not
participate in this predicate.

The four-word guard wire format remains generation low, generation high,
verdict, reserved zero. Publication uses these literal combined offsets:

| Word | Field | Ordering |
| ---: | --- | --- |
| 548 | Generation low | Relaxed |
| 549 | Generation high | Relaxed |
| 551 | Reserved zero | Relaxed |
| 550 | Verdict: 1 valid, 2 invalid | Release, last |

Wrong length, zero requested generation or wrong launch extent traps before
state validation. The publication helper also refuses wrong length or generation
before any store. Verdict 0 remains pending. No validator subslice, pointer cast,
mutable-reference fabrication or compiler no-alias claim is introduced.

## Unchanged Projection Consumer

`ferric_qwen3_tp2_guarded_projection_residual_bf16_v2` differs from the v1
projection kernel only in its entry symbol. Its two guard parameters remain
read-only four-word atomic slices. The host must provide the actual suffix
range from each rank's combined allocation. The consumer checks both current
nonzero generation tags, valid verdicts and zero reserved words before any
payload access; invalid guards suppress all payload reads and output writes.

The kernel retains 64 Wave64 workgroups, 4096 elements, the FP32 rank order,
intermediate BF16 projection materialization and final BF16 residual addition.
Changing the storage owner does not prove a cross-dispatch ordering relation.

## Why This Candidate Exists

The retained DAG-qualified v1 compile advanced beyond memory-bounds analysis
but refused with `FE2O3-RACE-002`: two shared, potentially aliasing allocation
origins had no relative-base-offset proof. Its receipt is SHA-256
`1943926bd4d138d79505f6d0a4d97722d7efa54af6323cccab319be9d53c9140`.
It emitted no artifact. This candidate addresses the kernel allocation
contract; the compiler's provenance, bounds, race, atomic and resource gates
remain unchanged. The
[actual combined-state v3 compile](../../qualification/guarded-mlp-combined-state-lowering-v1/README.md)
fails at descriptor ownership admission with a formal alias obligation not
discharged by Rust ownership. It emits no artifact. A successful new compile
is still required.

## Tests And Remaining Gates

The existing 27 CPU test sources are retained, including the independent
range-based state oracle and staged BF16 arithmetic oracle. Twelve additional
tests cover combined layout, every state/suffix
mutation, exact load coverage, wrong extents, release-last publication, generation
halves, stale guards, real atomic corruption checks, payload suppression,
generated marker function signatures, and literal source/ABI contracts.
All 39 tests pass in the retained MI350 CPU qualification.

Source-signature tests establish one Rust slice plus two scalar arguments,
not a measured GPU ABI. The predicted guard layout is four physical operands
(pointer, length, low, high), 24 explicit bytes. If the existing 256-byte
implicit segment applies, the total would be 280 bytes. Root must confirm the
actual compiler descriptor, LLVM and HSACO metadata rather than treating this
prediction as acceptance.

The host needs an additive 2208-byte atomic owner with a typed 2192-byte prefix
borrow and a 16-byte suffix range, completion-scoped leases, unique generations,
and ordered producer/validator/consumer dispatches. It must prohibit concurrent
reset or prefix mutation during validation and guard consumption. Existing
548-word owner types are not resized. The
[private owner passes isolated CPU qualification](../../qualification/guarded-mlp-combined-state-owner-v1/README.md),
but canonical runtime integration, the paired coordinator, protected admission,
GPU negative controls and full-model benchmarking remain separate.

Dependencies remain pinned to fe2o3 revision
`5a500d63c29b78f8788356b20dfcbb5c41ec20c9`. This source dependency pin is
distinct from the separately qualified compiler/tool closure used for future
lowering.
