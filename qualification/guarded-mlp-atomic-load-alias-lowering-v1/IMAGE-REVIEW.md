# Guarded Image Review

This review concerns only the exact 28,440-byte image with SHA-256
`de880dcebf79425ccb555c1a2bdca3ff78b926f7b2062d06b918763da4de0f66`.
Checked lowering succeeds. GPU execution, independent numerical correctness,
protected runtime admission and performance are not established.

## Actual Inspection

The [inspection controller](inspect_image_v1.py) ran on `mi350`, using the
installed ROCm 7.2.1 `llvm-readobj` and `llvm-objdump`. The
[receipt](image-inspection-v1/complete.json) records both bounded children
exiting naturally with status zero, reaped with absent process groups. The
image, original lowering receipt, both inspection tools and `prlimit` have
identical before/after hashes. Both stderr streams are empty; GPU visibility
is explicitly empty. This inspection never loads or launches the HSACO.
The controller received source review, not a fault-injection test suite.

The receipt is 3,072 bytes, SHA-256
`8c0e170b146d4a8bd85f70378793362a2dd40855ba7ed4b565886afebd386c2f`.
The 35,991-byte archive has SHA-256
`ae7a9d0a051d796142087d3ba1d419a4ee11bba0b63e045ae255368c16a9ea6f`.
Its [manifest](image-inspection-v1/retention-manifest.json) pins fourteen
bodies in fifteen members, totaling 325,046 expanded bytes. Raw metadata and
disassembly remain unchanged; the interpretation below is a separate review.

## Observed ABI

The [metadata](image-inspection-v1/metadata.stdout) reports two Wave64 kernels,
both requiring `[64, 1, 1]`, with eight-byte kernarg alignment, zero static
group/private bytes, zero register spills and no dynamic stack.

| Kernel | Explicit Bytes | Total Kernarg Bytes | Physical Arguments | VGPRs | SGPRs | Code Bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| State guard v2 | 24 | 280 | 4 | 60 | 102 | 16,332 |
| Guarded projection/residual v2 | 104 | 360 | 14 | 10 | 42 | 1,088 |

The validator's atomic pointer is at byte 0, length at 8, generation halves
at 16 and 20. R2 has `p0`, `p1`, residual and output pointers at 0, 16, 32
and 48; guard pointers are at 64 and 80 with lengths at 72 and 88, and
generation halves at 96 and 100. Each slice length is a separate eight-byte
physical argument. These are observed offsets, not the older R2 profile.

The embedded fe2o3 descriptor retains `SharedAtomicSliceU32`, shared borrow,
`ReadWrite` access and `SharedAtomic` aliasing for the atomic arguments. The
AMDGPU argument metadata omits **both** `.access` and `.actual_access` for
every pointer. Absence is not a read-only proof. There is no embedded LLVM or
bitcode section supplying a complete intermediate effect body.

## Observed Instructions

Addresses below are virtual instruction addresses in the retained
[disassembly](image-inspection-v1/disassembly.stdout), not file offsets.

For R2, eight separate `global_load_dword` instructions access the two guard
bases at `0x61dc`, `0x61f0`, `0x6204`, `0x6218`, `0x622c`, `0x6240`,
`0x6254` and `0x6268`. Each guard is read in verdict/low/high/reserved order,
at byte offsets 8/0/4/12. Each load has `sc0 sc1`, followed by
`s_waitcnt vmcnt(0)` and `buffer_inv sc0 sc1`. The payload loads occur later
at `0x62f0`, `0x6344` and `0x63d0`; the sole R2 store is the output
`global_store_short` at `0x6464`. There is no guard store or atomic RMW
instruction in that R2 symbol's code range.

The validator has 548 individual dword loads covering every four-byte offset
from 0 through 2188, and 548 `buffer_inv` instructions. It stores generation
low/high and reserved at `0x6070`,
`0x6080` and `0x6090`, targeting combined-allocation offsets 2192, 2196
and 2204. `buffer_wbl2 sc0 sc1` at `0x609c` and a wait precede the
verdict store at `0x60a8`, targeting offset 2200. This is an observation of
the emitted cache/order sequence, not independent proof of system-wide
coherence on the intended live peer allocations.

## Guard-Masking Caveat

At `0x62d0`, R2 combines the per-lane guard comparisons into `s[2:3]`.
At `0x62d8`, it computes `vcc = exec & validity`; `s_cbranch_vccz` at
`0x62dc` skips payload execution if **no** active lane is valid. The visible
path to the first payload load does not intersect `exec` with the validity
mask. This is a wave-wide any-valid test, not an explicit per-lane mask.

If a closed runtime establishes that all lanes observe identical, immutable
guard values after both validators complete, this branch is equivalent for
that restricted invocation. The image alone does not establish that premise.
It does not demonstrate safe behavior for a mixed-validity wave during
concurrent guard changes. Compiler uniformity handling and the intended
paired lifecycle therefore need examination before GPU admission. This
review does not claim a proven general compiler miscompile or a completed
guard-before-payload proof.

## Runtime Boundary

The proposed private combined owner exposes only bytes 2192..2208 as Read
for the R2 guards. The generic engineering pointer checker already accepts
absent access annotations; there is no reason to introduce a generic
ReadWrite-to-Read exception. A private image/owner capability must instead
bind the exact object, descriptor and effect review, both owner identities,
rank/group membership, suffix extents, generation and exclusive round custody.
No caller-provided read-only/coherence boolean is sufficient.

Keep all generic protected SharedAtomic refusal gates unchanged. This
engineering route cannot manufacture protected load/launch authority from
an inert observation or `HOST_VISIBLE_COHERENT` allocation flags. Next tests
must include both-rank valid/stale/mismatched guards, unchanged output on
rejection, paired publication and quiescence, and an independent BF16 R2
reference over all 4,096 outputs. None has run for this new image yet.
