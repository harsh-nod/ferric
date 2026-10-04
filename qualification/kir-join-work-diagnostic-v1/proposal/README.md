# Canonical KIR Join Work Diagnostic

Source-only diagnostic overlay. No compiler invocation, test, parser import or remote execution was performed by the author. This is not an algorithm fix, lowered image, certificate or GPU admission.

## Exact Base

The three `files[].before` bodies in `source-manifest.json` were read from the retained ordinary compiler source and SHA-checked against the actual RPO V2 `sources-before.json` map (77fbdd63...). They match the qualified RPO generation, not an assumed live checkout. Overlay only these three paths onto a fresh copy of that generation. Preserve the original source, target, tools, failed handoff and receipts.

Related exact qualified files:
- `fe2o3-kernel-ir/src/canonical_work_budget_v1.rs`: 7d4cbcd92e3d2eeda77030e9fe3ad00d3caa40e13eb6197289cf5fc875131e74.
- `fe2o3-kernel-ir/src/verification_resource_v1.rs`: 8212109e12e7116f01a1d7b05c80e915236a7a14f187632adc8ffad25a28dc38.
- `fe2o3-lower-mir-kernel/src/production_semantic_kir_v1/wave_formal_evidence_codec_v1.rs`: 4e18705bd530344d5fcd63c37b6b4c5018e7d1ba485186972986bcaf0c635512.
- `fe2o3-kernel-ir/src/wire_inert_published_archive_v1.rs`: b1e8544c3a325c999016c7fa646c75f7fc548d55116e5ce126e75d6797f5ad94.

## Observed Refusal

Actual inner failure: `L/rope-rpo-lowering-failed-v228-v1.json`, SHA 1fbbbd9a8889e1b33b724b3f8911c4ecc1074fffe8fc29377f6840686375ba1f. Owner failure: SHA 8fb2f02cde719c08fd41535d59d39980d8c6328f27cf952770c481f5b48edbbf. Both remain separate from this unexecuted proposal.

The checked handoff and actual semantic replay passed. The next actual ignored finalizer test naturally exited 101 at `wave_qkv_attention_output_tiles_v6_tests.rs:973`, inside `conditional_descriptor`, with attempted work 1,084,825,160 against 1,073,741,824. Its stdout SHA is 9e49bc58a17e52b611688438ed8f1eb56618cdc3839f689f733c28f216f7b840. That single source line does not locate the internal charge. No HSACO was emitted.

Retained inputs and raw commands live under:
`/mnt/c/Users/harmenon/ferric-session-evidence/20261004/rope-rpo-lowering-evidence-v228-v1/row-rope-materialized-rpo-checked-probe-v228-v1`.
The exact 4,078,537-byte handoff and its original remote path/SHA are in the manifest. No binary parsing of it was performed here.

## Instrumentation Contract

Only stderr diagnostics are added, enabled when `FE2O3_KIR_JOIN_WORK_DIAGNOSTIC_V1=1`. Every existing decoder, checked arithmetic operation, charge, reservation, release and error propagation remains. The shared work cap stays `1 << 30`; storage stays 128 MiB. No refund, second ledger, multiplier change, reduced representation or authority upgrade is introduced.

Ledger lines use:
`kir-join-work-v1 stage=NAME work=W remaining=R storage=S peak=P charge=C`.
Here W is accepted shared work, R is remaining allowance, S is live storage, P is observed peak, and C is the next bulk charge (zero for a checkpoint around a nested call). A failed charge does not advance accepted work. Therefore a before-only bulk line can be joined to the unchanged refusal using `W + C == actual` and `W + R == limit`.

Successful traversal would emit these stages in order:
1. outer-middle-before, outer-middle-after, outer-formal-before
2. formal-archive-before, formal-archive-after
3. formal-receipt-before, formal-receipt-after
4. formal-kir-before, formal-kir-after
5. formal-middle-before, formal-middle-after
6. formal-layout-before, layout-bytes-before, layout-bytes-after
7. join-shape, layout-join-before, layout-join-after
8. formal-layout-after, outer-formal-after
9. ocml-before, ocml-after

The run is expected to stop at a prefix of that sequence; do not require nonexistent after-markers or predetermine the failing phase. `join-shape` instead has exactly `rows=R blocks=B nodes=N charge=C`; verify `C = 128 * (R + B + 1) * N` if reached. Ordinary ledger lines must have nondecreasing work/peak, `storage <= peak <= 134217728`, `work + remaining == 1073741824`; adjacent before/after bulk pairs differ by C. No guessed row/node values are admission constants. Missing all markers, duplicate/out-of-order stages, an unrelated error, changed attempted work, or a successful test are diagnostic-run failures, not accepted improvements.

## Minimal Root Qualification

Use a fresh target and unchanged bounded CPU/helper policy. Format/check only the three copied source bodies and retain exact post-format pins. Build the finalizer test artifact with the existing qualified recipe; select its actual Cargo artifact, never a guessed executable suffix. No new test names are authored. Replay the existing full finalizer inventory (190 passing, 15 ignored in the predecessor) with diagnostic environment unset.

For the retained handoff, reuse the original `actual-inert-join-command.json` argv/environment and resource bounds, substituting only the newly built diagnostic test ELF plus fresh target/tmp and diagnostic environment. The selector is:
`--exact linux::wave_qkv_attention_output_tiles_v6::tests::wave_emission_actual_retained_v6_passes_full_inert_join --ignored --show-output --test-threads=1`.
Keep the actual handoff PATH/BYTES/SHA environment unchanged. Preserve native exit 101, its genuine failing-test transcript, all marker lines and source/tool before/after pins. Record it as an expected refusal diagnosis, not a passing inert join. No full compiler/replay rerun is needed merely to locate this charge.

## Scoped Follow-Up, Only After Measurement

The unmeasured leading candidate is `wave_formal_evidence_join_v1.rs:167-177,239-256`: it charges `128 * (rows + blocks + 1) * nodes` before the layout checks. Rows sum reason/allocation/memory/synchronization counts; nodes sum body parameters, blocks, one, block parameters/operations and operation results. Actual algorithms then include quadratic block-duplicate detection, linear block lookup per reason and whole-function pointer-type lookup per memory row. The later finalizer memory/source-span joins also scan repeatedly; they are a separate possible cost site.

Do not reduce 128 or raise the cap. If this join is confirmed, a candidate can build reserved, deterministically ordered borrowed block/value lookup indices once, then use bounded lookup instead of repeated scans and replace only the corresponding work estimate with explicitly charged construction/sort/lookup operations. Preserve all location/type/effect/order/raw-receipt checks and storage-floor restoration. Inert KIR is not semantically verified: duplicate value IDs currently use the first definition in traversal order, whereas duplicate block IDs are rejected. An index must preserve that behavior, including first-match precedence, rather than silently granting verified-module assumptions.

Differential tests must compare old/new decisions on valid and malformed archives, unsorted/sparse IDs, duplicate blocks/values, missing/out-of-range locations, mismatched pointer types/spaces/effects, root/receipt mutations and every reason variant. Add exact/one-under work and storage tests, overflow/refusal and floor restoration. Existing canonical bytes, identities and unresolved requirements must remain unchanged. No improvement or successful full lowering is claimed before actual qualification.
