# Parallel KV Append V16

Isolated one-root `gfx950` candidate for the existing batched KV append. No
existing kernel, controller selector, CLI, worker, or default route is changed.
Source-v3 admits exactly one TP1 row, four physical pages, page-table stride
four, and logical positions below 64. It requires exactly 64 Wave64 workgroups,
one per physical token slot. Its six slices and four u32 scalars retain 112
explicit argument bytes. Native compiler proof, actual metadata/ABI, and
ordinary artifact admission remain mandatory gates. The changed grid must be
explicitly admitted; this is not a drop-in launch for the old one-group root.

## Exact Copy And Ownership

The original scalar kernel validates every selected physical slot, rejects
duplicate final slots, then copies u16 K/V words. V16 retains those checks and
performs the same complete validation in every lane before stores. The stricter
one-row scope is an additional guard, not removal of the original all-slot
prepass. It uses the existing `RowStriped2D<Index1D, 64, 16>` writer, not an
unchecked scatter or a dynamically fabricated row-ownership token.

For physical-slot workgroup `g`, lane `l`, and chunk `j < 16`, the invocation
index is `g*64+l`. Its row-striped destination is `g*1024+l+64*j`, with checked
geometry `rows=64, columns=1024, stride=1024`. Only the workgroup whose `g`
equals the validated selected physical slot copies the source row. The other
63 groups make no stores. Every destination has one group/lane/chunk owner,
even with permuted page tables. Each active lane copies 16 K and 16 V words
instead of one grid leader copying all 1024 K and 1024 V words. No floating-point
conversion or arithmetic is introduced; every u16 bit pattern, including
signed zero and nonfinite BF16 encodings, is copied unchanged.

The additional 63 idle workgroups repeat metadata checks, which may offset the
parallel-copy benefit. No performance improvement is assumed. The earlier
source-v2 dynamic-Blocked candidate remains frozen with its failed native-v1
proof: frozen FAA supports only constant Blocked components. Source-v3 instead
uses the existing runtime row-striped projection used by the v15 RMSNorm root;
it does not change or relax compiler policy.

Inputs and mapping tables must remain immutable for the dispatch. Exclusive
physical ownership, immutable-prefix copy-on-write, and disjoint buffer access
remain host/runtime contracts; this candidate does not weaken them or add
cross-dispatch synchronization. CPU tests are models and source contracts,
not GPU validation or performance evidence. The first intended matched GPU
scope is one TP1 row, context 64, four physical pages, the unchanged full
32-token independent reference, and the same full-forward stack in both arms.
No throughput, overlap, model-quality generalization, or 700-token/s claim is
made before a separately admitted strict capture.

## CPU Checks

`tests/contract.rs` parses the root and compares the whole body against the
original v2 function with only the explicit lane-ownership and exact-copy
substitution allowed. It pins the unchanged all-slot prepass, scalar guards,
ABI and launch, and rejects mutations of those contracts.
`tests/exact_copy.rs` compares complete cache contents with the original scalar
schedule across every logical position and physical page, both group/lane
orders, page permutations, all 65536 u16 patterns, invalid selected mappings,
extent errors, and inactive tails. Broader row/world/page and incorrect grid
profiles reject without stores. Source contracts retain the original duplicate
slot prepass even though duplicate active rows cannot occur in this narrow scope.
