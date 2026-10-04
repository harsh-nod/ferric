# Tentative Indexed Inert Formal Join

Source-only candidate, not a qualified fix. The failing charge has not yet been measured by the separate diagnostic overlay. No tests, builds, imports, remote execution or live-tree changes were performed here. Do not integrate based on the cost hypothesis alone.

## Base and Scope

`source-manifest.json` binds the actual RPO CPU V2 generation (56fc51fc...) and its source snapshot (77fbdd63...). Both replaced files were checked byte-for-byte against that snapshot. The candidate is based on those original qualified sources, not on the logging overlay.

Four Rust files: two existing bodies plus two new private includes. Only `validate_layout_module` uses the new index. `validate_live_indices`, its `join_work`, original linear lookup helpers, codecs, archive identities, certificates, policy flags, receipt parsers and finalizer limits remain unchanged.

The in-place heap algorithm follows the qualified `fe2o3-kernel-ir/src/verification_index_v1.rs` pattern (f455814314aec52373990c9816ef6be42c30c005b5332be67d333e5a35f3cc74). That helper is crate-private, so this change does not expand public kernel-ir APIs. No map allocation is proportional to the largest untrusted ID.

## Algorithm and Equivalence

- Build borrowed block and value rows from the actual function, retaining original traversal ordinals. Value traversal is zipped function parameters/signature, then each block's parameters and each operation's results, exactly matching the old lookup.
- Sort rows deterministically by (ID, original ordinal). Adjacent equal block IDs still reject. Equal value IDs are not rejected or collapsed: lower-bound returns the first original definition.
- Binary-search block IDs for covered-reason locations and value IDs for memory pointer types. Operation index bounds, pointer kind/address space and first-definition precedence remain checked.
- Traverse allocation/memory/synchronization records in original body order. Sorting the lookup indices does not sort or normalize the authoritative row streams. Complete-effect checks, branch priority, zero access ordinal, roots, raw binding, row counts, trailing bytes and reason/location equality are unchanged.
- Borrowed tables cannot escape validation. They are dropped before the caller's original storage floor is restored, including allocation, work, encoding and correspondence failures.

Inert KIR is not a semantically verified module. In particular, a duplicate value definition must not silently become a rejection or a later-definition lookup. Tests explicitly retain that distinction.

## Accounting

The finalizer work/storage caps stay `1 << 30` / 128 MiB. The old 128 multiplier is not reduced: the algorithm's repeated scans are removed and replaced by separately charged index operations.

- Original KIR-byte charge: `16 * kir_length`, unchanged.
- Retained linear validation envelope: `128 * (kir_length + rows + roots*roots + kernels)`, checked before traversal. Whole encoded KIR bytes conservatively cover fixed-catalog intrinsic/effect/name scans; record and exact root-cross-join costs remain.
- Counting pass charges before each container/count read; construction prepays `128 * (nodes + definitions)`.
- Before allocation, reserve both exact-capacity borrowed vectors and their owner metadata. The original `16 * kir_length + 1024` effect scratch remains reserved concurrently.
- Before sorting, prepay `4 * n * ceil_log2(n) * (3*sizeof(row) + 4)`. This bounds the controlled heap's comparisons and row moves; it is not an assumed standard-library sort complexity. No sort allocation occurs.
- Duplicate-block scan prepays `2 * blocks`. Every binary-search iteration charges 8 before inspecting a row, plus 4 before the final result lookup.
- All size products/sums are checked; storage reservations and allocation failures remain fail-closed. Accepted work is never refunded.

These are abstract verifier work units, not CPU timing claims. Whole-wire accounting relies on the existing decoder-to-layout join: production supplies the exact decoded module from `layout.kir`, not an independently supplied module. Differential unit fixtures deliberately exercise the private layout validator with mutants; they do not grant full archive authority.

## Tests and Root Qualification

Twenty authored tests are appended to the existing `fe2o3-lower-mir-kernel` test module. The test-only legacy validator is copied exactly from the qualified pre-candidate function (name only changed). It is not used in production.

Coverage includes six- and fifteen-root valid fixtures; sparse/nonmonotonic IDs; all three duplicate-value precedence classes; absent definitions; duplicate blocks; roots/types/census mutations; row ordering/truncation; incomplete effects; covered-reason mismatch/missing/out-of-bounds; exact and one-under work/storage; refusal prefixes and storage floors; deterministic sort with duplicates; pre-mutation sort refusal; checked arithmetic overflow; immutable KIR identity/unit-only result; and repeatable resource accounting.

Build and run the actual library test artifact for `cargo test -p fe2o3-lower-mir-kernel --lib` using the existing bounded, locked, offline qualified tool/dependency recipe. The full names are in the manifest; their prefix is `production_semantic_kir_v1::wave_formal_evidence_v1::tests::indexed_formal_join_`. Discover the original library inventory from the qualified source; no existing count is guessed here. The compiler/pliron/finalizer test binaries do not automatically execute these dependency-library tests.

After independent source review and a confirming diagnostic, root must qualify the original dependent cohorts and a newly built finalizer against the same retained handoff. Require unchanged archive/module identities, raw joins, unresolved requirements, no-authority flags and actual bounded outcome. A passing private layout test is neither an inert full-join pass nor checked emission. No new HSACO, GPU correctness or performance outcome is claimed.
