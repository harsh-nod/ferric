# Query-Hoist Wave Attention V14

Host-tested and emission-checked candidate, separate from every existing v5 root, crate, adapter route,
inventory and default. Source `1843d8f174b20ebd6dbc0e72d6fe9044e8d1f244`
passes all ten remote host steps: five contract and eight host-model tests,
strict Clippy, formatting and zero doctests. Complete archive
`9d025c4646b4751681230fb46036f738a6f38ebe5edd9bc4362a1e2f7dc7f579`
retains all 276 evidence files and fresh own artifacts. The separate nine-phase
emission gate also passes, including eleven synthetic control tests. Native
numerical qualification and performance comparison remain pending.

## Scope

The new single root is
`ferric_qwen3_tp_batch32_wave_paged_gqa_query_hoist_bf16_v14`. Its body is the
frozen v5 Wave attention body with exactly two changes: the root name and moving
the two invariant query load/conversion bindings outside the token loop. They
remain after all existing shape, row, position and query-base admission, directly
before `Math::current()`. Query operands and conversion expressions are unchanged.

Token induction/bounds, visible-token condition, page validation, QK products,
shuffle reduction order, deferred finite checks, online softmax, narrowing and
two-component output ownership remain unchanged. No GEMV optimization is included.
The source signature retains six slice descriptors plus five u32 arguments
(116 explicit ABI bytes), Wave64 and maximum grid 1024. Actual V14 emission
confirms hidden arguments start at 120 bytes, a 376-byte total kernarg segment
and eight-byte alignment. The descriptor reports 95 SGPR / 37 VGPR and zero
spills, fixed private/LDS, AGPR and dynamic stack. These are static resource
observations, not achieved occupancy or elapsed GPU time.

The hypothesis comes from a CPU-only inspection of actual resident v5 HSACO
`98b5fdb14ac7242e324e1801e1885c98e77473d622b2e9b3f9f12f14c7d75502`:
query loads at ISA `0x140f4` and `0x14110`, each followed by `vmcnt(0)`, remain
inside the token backedge. Static inspection archive
`224b645f49dcd3545ed394fdaed2251c3945543fa503d190516606fac9b41b0e`
retains the exact outputs. This does not establish elapsed GPU cost or predict
speedup. The measured host collective spans include ordered submission/IPC/wait.

## Invalid Inputs

Shape, row and position rejection still occurs before the moved loads. A first
invalid logical/physical page now follows a valid, bounds-checked query read,
where v5 checked that page before reading the query. Rejection and absence of an
output store are preserved; identical invalid-input memory-access ordering is
not claimed. Later page rejection similarly sees fewer preceding query reads.
The change adds no arithmetic or output side effects before those page checks.
Nonfinite products, values, reductions and final output retain their old checks.

## Focused Coverage

Verified host coverage: five structural/contract test methods and eight
host-model methods, 13 total, plus zero unit tests and zero doctests.

The structural comparator parses both complete source files, checks the exact
post-admission placement and load expressions, relocates only those two AST
statements back into the loop, restores the root name, and compares the complete
tokenized ASTs. Eighteen internal source mutations exercise wrong loads/halves,
wrong admission/loop/reduction/math/output, early placement and reversed bindings.
These are subcases of one method, not additional test methods. Host-generated
roster and source-signature checks bind the one-root/116-byte explicit ABI shape.
Ownership enumeration covers rows 1-32 and TP1/2/8.

The host model compares the two read schedules using identical CPU floating-point
operations, including CPU `exp`. It is not device-capability emulation, an oracle
for GPU transcendental lowering, or a proof of native numerical equivalence.
It records query/page read order and counts, keeps output uncommitted on model
rejection, and checks finite output bits, original inputs and inactive output.
Cases include TP1/2/8, rows 1/17/32, contexts 1/16/17/256/8192, first/last/page-edge
positions, remapped pages, future/decoy values, wrong mapping and second-half
mutations, minimum/full carriers, nonfinite values/overflow, and repeated calls
with changed queries and intervening invalid-page rejection.

## Source and Future Gates

Parent integration: `f9995afdc5bba5b7931bafecfeab221621c7556a`. Frozen v5 attention
SHA256: `5796b1b20cccd46e390a67318f0e1d4159b0b69142d35598f0246a7f2b52cc98`.
The standalone lock is copied from the accepted v13 closure with only the new
package identity and direct already-locked quote dependency. Both fe2o3 crates
remain pinned to `8efd4fd416d1ffae7a718144e4d299fe3c8f7590`.
The build script reuses the existing target helpers unchanged. Its fixed crate
binding is a host fixture, not a measured device artifact identity.

Remote formatting and the offline source-fresh host gate are complete, including
the shared target helpers and frozen v5 comparison source in custody. Emission
at compiler `8efd4fd` produces image
`8f21681fe9103b670ee5666f429a45682e77fc90eb16802a02c4b6fb93a192c8`.
Complete archive
`cb58d401389f3b85f06c81b126b59e2b12e8cad8da8d2b1425b77989fdacc7a0`
retains all 126 files. The actual LLVM worker remains build `216822` with a
source subtree identical to the pinned compiler; it is not relabeled as rebuilt.

Static ISA def/use review confirms query loads at `0x2360` / `0x239c` and their
conversions precede the recurring token backedges to `0x26a4`. Registers `v19`
and `v18` are reused without loop redefinition. The observed resident v5 uses
a different compiler, so this is not a same-compiler ablation or a numerical
proof. The retained observation authenticates only the 71,865-byte typed
handoff's digest, not its payload; independent typed/progress replay remains
unavailable. Frozen automatic pending flags are not rewritten by the later
manual ISA review.

No compiler/proof relaxation is permitted. A same-compiler control, native
finite attention fixtures and exact model qualification remain required before
performance attribution or routing changes.
Any comparison remains opt-in and conditional; no default or serving claim follows.
