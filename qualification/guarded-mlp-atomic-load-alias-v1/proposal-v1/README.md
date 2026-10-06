# Atomic Load Alias Derivation

This is a source-only compiler proposal. No new compiler suite, candidate
lowering, HSACO load, GPU execution, numerical comparison, or performance result
is claimed here. The parent agent owns integration and remote qualification.

## Observed Failure

The combined-state candidate's third lowering attempt terminated naturally with
exit 1 after 363.190 seconds in the compile child (366.925 seconds overall).
Source observations were unchanged, postchecks were clean, and the artifact was
null. This was not a timeout. The retained 8,480-byte stderr contains exactly:

```text
production descriptor evidence has an internal formal alias obligation not discharged by Rust ownership mismatch
```

The retained receipt is 19,077 bytes, SHA-256
`d072928318581d8f492d296dfa4cc4a1f42865fbd3a7d95fdac01dddac7e5ab0`;
stderr is
`5d3adcd162d465f76cca68e5590494692be075905070ae9a9795d11e60fe9b75`.
Both are in
`qualification/guarded-mlp-combined-state-lowering-v1/attempt-v3/evidence`.
The message does not print the root or rejected parameter pair. Identifying the
R2 read-only guards as the source of shared/shared obligations is a source
deduction, not an observed pair-level diagnostic or a new timing measurement.

The base compiler is the qualified memory-bounds-DAG generation: its complete
receipt is `bd1fa482...`, source map `37355332...`, input `3c60b907...`,
and controller `151f0a91...`. The exact pins and two overlay rows are in the
source manifest. The base has 5,808 compiler source files; this proposal adds
one integration-test file, giving 5,809 source files and 5,811 map entries when
the controller and parser helper are included.

## Diagnosis

In the qualified `formal_memory_obligations.rs`, every
`OperationKind::Atomic` derives a `FormalMemoryAccessKind::Atomic` record.
The old alias-envelope construction treats every non-`Read` access as a
writer. Consequently an atomic load is conservatively counted as a writer,
even though its verified opcode is explicitly `AtomicKind::Load`.

The R2 kernel has three ordinary shared read inputs, one exclusive output, and
two shared atomic guard inputs that are only loaded. The old rule generates
shared/shared alias-separation requirements involving those load-only guards.
The descriptor correctly refuses to discharge such requirements using Rust
ownership: shared references do not prove disjoint allocations. In contrast,
the combined state validator has only one allocation-bearing parameter, so it
cannot generate a distinct-parameter alias pair by this rule.

This proposal does not change the descriptor gate or claim shared atomic
references are exclusive. `SharedAtomicSliceU32` remains a ReadWrite global
U32 slice in the descriptor. No suffix capability, host-owner transition, peer
visibility, or runtime alias authority is added.

## Narrow Change

The existing private per-allocation envelope map is populated immediately when
an access is successfully derived, using the same immutable verified operation
and the same formal access record:

- Ordinary loads and both guarded-load success paths contribute no write.
- Ordinary and guarded stores contribute a write.
- Atomic loads contribute no write; every other AtomicKind contributes a write.
- Multiple accesses union the same byte ranges and OR the write bit. A store,
  RMW, or compare-exchange can never be erased by a later load.
- The final address-space and distinct-allocation pair rule is unchanged.

The compiler-owned closure receives this bit only at these five closed
derivation sites. There is no caller-supplied effect map, missing-location
default, cached cross-function classification, or model/symbol special case.
Unknown/unsupported effects still produce the existing incomplete reasons;
failed derivations do not become successful accesses. Unreachable blocks and
unbounded guarded-read envelopes retain their existing behavior.

This is a semantic refinement of alias-obligation generation and can change
admission. It is not merely a performance optimization. The source manifest
therefore marks semantic and alias predicate changes explicitly. No structural
limit or proof ceiling is increased.

## Preserved Evidence

Public access kinds, Atomic wire tag 3, receipt policy/version, bounds
requirements, inter-invocation conflict derivation, atomic legality, and Rust
ownership discharge remain unchanged. An atomic load is not relabelled as an
ordinary read. The existing conservative mixed ordinary/atomic conflict rule
is retained even when both operations happen to be reads.

An inert formal-memory receipt does not contain an atomic opcode, ordering,
or scope, and does not authenticate producer completeness. Its bytes must
remain joined to the owning KIR/compiler lineage. The existing production
formal-memory owner rederives obligations for equivalence verification; parsing
an old receipt cannot authorize omission of obligations for a changed store or
RMW. The tests explicitly show that two different legal load metadata choices
can have the same inert receipt, so that receipt is not a replacement for KIR
identity.

## Resource Accounting

The same BTreeMap, keyed by accessed formal allocation, still contains at most
one envelope per accessed allocation. The range computation and map update
occur once per successfully derived access, as before; pair enumeration stays
quadratic in the number of accessed allocations. There is no new sidecar,
graph walk, or serialized state. The map now lives across extraction and
bounds collection, so unchanged peak live allocation is not claimed. Allocation
failure handling and all caps are unchanged. This proposal makes no compile
speedup claim.

## Required Qualification

The new `formal_atomic_load_alias_v1` integration executable contains nine
tests, with literal expected outcomes independent of the alias derivation:

1. Two load-only shared atomic allocations require no separation.
2. R2-like six-slice inputs retain exactly the five pairs involving output.
3. All ten atomic writer kinds retain separation against ordinary and atomic
   readers, in both operation orders.
4. Writes dominate loads within one allocation in both orders, with exact
   byte-range unions.
5. Bounds, conservative mixed-access conflicts, and serialized Atomic tags
   remain intact.
6. Ordinary read/write ranges and unused/empty allocations remain unchanged.
7. Unreachable stores do not pollute reachable reader envelopes.
8. Invalid atomic ordering, scope, and alignment are rejected; unknown launch
   extents remain incomplete.
9. Changing loads to stores/RMW/CAS changes rederived obligations, while load
   metadata and inert receipt authority remain distinct.

Run the complete kernel-ir library and integration suites, not only the new
test executable. Existing guarded-read fallback, arithmetic overflow, private
slot, receipt-decoder, and formal-memory tests must remain in the census.
Retain the existing full compiler and Pliron qualification and repeat these
unchanged compiler tests:

- `compiler_descriptor::tests::atomic_slice_descriptor_requires_exact_readwrite_u32_global_slice`
- `compiler_descriptor::tests::atomic_slice_descriptor_does_not_discharge_aliases_from_shared_rust_ownership`

Their source body, `compiler_descriptor_atomic_slice_tests.rs`, is unchanged:
3,706 bytes, SHA-256
`5f9217c9727455595ee3bf4a763f8f98a2db0ed673ede41bec0ccac00cdcbe3b`.
A clean full CPU result must precede a fresh
tool audit and checked combined-state lowering attempt. The candidate CPU and
vendor receipts may remain valid while their exact pinned input tree is unchanged.
Only that actual lowering attempt can establish
whether this refusal is gone or another independent gate remains. GPU and
end-to-end inference milestones remain separate.
