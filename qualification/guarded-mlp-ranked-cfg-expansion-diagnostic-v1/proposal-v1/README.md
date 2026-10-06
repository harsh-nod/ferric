# Ranked CFG Expansion Diagnostic Proposal

This is a source-only diagnostic successor to the qualified borrow-lookup
generation. It has not been built, imported, tested, or used for extraction.
No canonical source, kernel, compiler limit, or admission predicate is changed.
All edits are contained in this proposal; root owns integration and execution.

## Measured Failure and Static Attribution

The retained borrow-generation attempt is
`/home/harsh/ferric-p227-integration/qualification/guarded-mlp-indexed-atomic-borrow-lookup-lowering-v1/attempt-v1`.
Its failed receipt is 14,686 bytes, SHA-256
`cb1def314b36717465478dbf88d521f094749b34d805b9f0e07ed5f3c5421778`;
`evidence/compile.stderr` is 8,018 bytes, SHA-256
`646b7e35b6dbf8cd8b00e43f52008824ccc06634d636238fbc23fc33d9ccdb83`.
The raw failure contains only the generic ranked CFG projection block-limit
message. It does not supply an actual function, root, semantic count, or
expanded count. This proposal makes no attribution of those missing values.

In the qualified compiler source, that exact message has one production site:
`crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1.rs:20834`, in
`build_ranked_cfg_with_wave_sync_v1`. The only production call to that builder
is at line 3896 in the ranked-root projection path. These are static source
facts, not additional observations from the failed stderr. The parent source
is 1,652,751 bytes, SHA-256
`a68eba52621e7dce2b11030b536e094584e0ea31a8241474bc5fda27a4390c61`.

## Exact Change

The parent hunk changes only the error construction at the existing strict
`block_count > MAX_RANKED_BOUNDS_BLOCKS` gate. Its count starts at one for the
synthetic entry, then adds `projected_block_expansion` for each reachable
semantic block. Guarded items, predicates, exact switches, and analysis splits
retain their original expansion formulas and checked arithmetic. All prior
analysis, reachability, counting, work charges, and refusal ordering remain.
The gate still precedes ranked block allocation and the operation-count gate.

The existing `CfgBlockLimit` error is reused with a new
`ProjectedCfgExpansion { blocks }` site. The record reports the already
computed projected count as `blocks` and the O(1) declaration length as
`semantic_blocks`. It reports the same 2,048 limit, fixed function identity,
role, and source provenance. There is no recount, graph walk, code dump, or
new successful path. No new error variant or `Error::source` arm is needed.

The two earlier CFG sites keep their exact previous display text. The new
site preserves the prior generic failure prefix, adds the structured fields,
and does not incorrectly say the failure happened before loop analysis.
Existing deterministic root attachment at parent line 3363 supplies context
when this error propagates through that boundary; the constructor itself
does not fabricate a root or function index.

Kernel export and root names reuse the existing bounded byte arrays and ASCII
escaping. Each is capped at the existing 128 raw bytes, at most 512 escaped
bytes plus a truncation suffix. Other fields have fixed or integer-bounded
representations. The focused fixture bounds the full rendered diagnostic
below 2,048 bytes, including maximum `usize` and two truncated names.
Successful compilation does not construct this failure record. Failure-only
record storage grows by fixed-size scalar fields; no compiler resource ceiling
or accounting counter changes.

## Files and Tests

`integration.patch` is the complete four-file patch against the exact qualified
source pins in `source-manifest.json`. Three small postimage source files are
provided beside it; the large parent postimage is represented by the patch and
its expected hash, avoiding a full source-tree copy. All four preimages were
independently joined to the actual borrow CPU source map. No source file is
added; the source count remains 5,803.

The existing multi-switch expansion fixture retains its exact-limit success
and oversized refusal. Its expected error is updated to the structured variant
and checks both counts. The existing CFG fixture helper additionally checks
the semantic count; all nine old test names remain.

Three new tests use the common filter
`production_ranked_projection_v1::tests::cfg_projected_block_limit_diagnostic_`:

- `exact_and_next_boundaries`: real reachable chains produce 2,048 ranked
  blocks successfully and refuse 2,049 without changing the gate.
- `expansion_counts_and_identity`: a real multi-switch produces a 2,050-block
  refusal from 1,026 semantic declarations, preserving exact function metadata
  and initially unattributed context.
- `context_is_bounded`: component formatting at `usize::MAX` preserves escaped,
  bounded export/root context; this is synthetic test data, not a measured
  production block count.

The unchanged zero, exact raw-block boundary, graph-work, edge-limit, callable
context, and old bounded-format tests remain part of the full suite. Source
review and in-memory patch/hash checks are the only verification performed by
the author. Compilation and every test remain root-owned execution gates.
There is no HSACO, GPU result, performance claim, or reason here to increase
capacity based on an unobserved projected count.
