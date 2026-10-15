# Actual Fixed-Capture Formal Replay

This CPU-only checkpoint advances Ferric [issue #42](https://github.com/harsh-nod/ferric/issues/42)
after the [BF16 predicate inline qualification](../guarded-mlp-finite-predicate-inline-v1/README.md).
It exercises the **actual retained fixed-round V18 capture**, not a
declaration-only fixture. It does not recompile a worker, emit an HSACO,
run a GPU, grant runtime authority, or close an M0-M7 milestone.

## What Ran

The new bounded [runner](run.py) and [input preparer](prepare_inputs.py)
reuse the already-qualified compiler test ELF without changing production
source or the existing test assertions. The exact selected test is:

```text
production_pipeline::conditional_formal_tests::actual_mlp_v39_reaches_formal_archive_and_outer_lineage_replay
```

Its name is derived uniquely from the qualified inventory and confirmed
in the unchanged ignored inventory. The runner verifies both actual binary
inventories before executing only:

```text
TEST_ELF --ignored --exact EXACT_TEST_NAME --nocapture --test-threads=1
```

The two capture environment values point to the original file and its
literal SHA-256. GPU visibility is empty. Package cwd and library paths
come from the original qualified test command; temporary/cache paths are
isolated to this replay. The default 24 ignored declarations are unchanged.

The runner pins all 5,846 source, tool, artifact, controller, capture and
provenance inputs. It joins the actual failed compiler receipt and its
unchanged source snapshots to the independently retained
[prior evidence archive](../guarded-mlp-finite-predicate-inline-v1/originals.tar.gz).
That archive's digest is checked before reading its source-snapshot ledger.
The original fixed caller fixture and qualified provider/backend source
pins must agree. Every replay input remains unchanged afterward.

## Actual Result

SSH host mi350, CPU8/9, nice10, service memory maximum12 GiB, swap disabled,
600-second leaf limit and 700-second service limit:

| Check | Result |
| --- | --- |
| Reader/test phases | 3 natural status0 exits, reaped leaders, groups absent |
| Selected test | 1 passed, 0 failed, 0 ignored |
| Other compiled tests | 1,364 filtered out |
| Default ignored inventory | Same 24 entries |
| Elapsed | 48.193726 seconds |
| Source/tool/artifact/capture postchecks | Passed |
| Full compiler/device suite rerun | No |

The earlier 2,596-test qualification remains separate. This result is
one explicitly opted-in actual-capture test, not 2,597 tests rerun here.

Observed stage evidence:

| Stage | Actual evidence |
| --- | --- |
| Recipe | 11 roots, 59 accesses, 221 memory entries, 260 effect operations |
| Ranked | 1 root, 86,551 bytes of ranked IR |
| Conditional formal | 11 roots, 8 unresolved requirements |
| Outer-lineage replay | Complete; formal preimage size595,372 bytes |

See the [raw stdout](replay/fixed-formal-replay.stdout),
[raw stderr](replay/fixed-formal-replay.stderr), and
[terminal receipt](replay/complete.json).

## What This Establishes

The unchanged [test source](source/crates/rustc-codegen-fe2o3/src/production_pipeline/wave_mlp_tile_capture_v2_tests.rs)
successfully checks exact V39 canonical decoding and encoding,
rejection by the V37/V38 decoders, the real MLP tile source ownership
profile, its 88-byte/alignment8 carrier with 13 fields, and all eleven
memory roles. Its authenticated exp terminal must occur in the actual
kernel body; a declaration-only replacement cannot satisfy this case.

The test verifies materialized ranked/Kernel-IR equivalence, conditional
formal ownership and a 4,096-invocation structural witness. The
[outer replay helper](source/crates/rustc-codegen-fe2o3/src/production_semantic_lineage_v3/replay_actual_mlp_tile_lineage_v2.rs)
constructs the source receipt, target binding and conditional lineage
evidence, then validates the middle-end, formal-memory and correspondence
receipts against the exact source and neutral IR identities.

**The 595,372-byte formal preimage was constructed and replayed in memory.
This test does not retain a serialized formal preimage file.** The
qualification archive preserves the original capture, raw logs, receipts,
controllers and inspected source, not a new production handoff.

## Assurance Boundary

Eight runtime requirements remain unresolved throughout the successful
test. No safe-Rust numerical reference is registered for this source:
empty reference bindings grant no numerical refinement. The test checks
that the conditional owner grants neither artifact/launch authority nor
compiler refinement authority, and that signed-Verus proof custody is
unavailable. These are retained refusals, not evidence of discharged
premises.

The [ordinary handoff guard](source/crates/rustc-codegen-fe2o3/src/production_pipeline.rs)
and [conditional formal policy](source/crates/rustc-codegen-fe2o3/src/production_pipeline/conditional_formal_v1.rs)
are unchanged. The original fixed compile still refuses V2-only extraction
that would drop conditional runtime premises. The original early-stop
compile still refuses its non-unique induction-region header exit.

This replay helper does not establish an actual production rustc
invocation descriptor or production transaction custody. It must not be
used as a substitute for either.

## Next Bounded Work

Implement and test a genuine lossless semantic V3 engineering extraction
path for the fixed-round source. First trace the selected frontend command,
actual rustc invocation descriptor producer, existing semantic V3 backend
method and worker consumer. Add one explicit, closed selector and canonical
invocation transport only where the existing APIs support it, retaining the
conditional semantic capsule and ordinary V2/protected-publication refusals.

Tests must bind the actual selected invocation and target, preserve exact
conditional premises, reject altered/missing descriptors and incompatible
modes, and keep artifact/launch authority separate. Do not manually inject
invented invocation bytes, manufacture custody, or replace the refusal
with unconditional success. The early-stop CFG diagnosis remains separate.

## Retained Evidence

[Manifest](manifest.json), [data-only audit](publication-audit.json),
[original capture](capture/semantic-mir-v1.bin),
[source map](capture/semantic-source-map-v1.json), and
[evidence archive](originals.tar.gz) are retained. The archive has
48 members, 47 pinned originals and 37,564,092 expanded bytes, with exact
member-by-member readback. Its scope is logs, captures, controllers, source
ledgers and relevant inspected source. Compiled ELFs, the full source tree
and toolchain are hash-pinned inputs, not embedded in this archive.

A separate data-only audit rehashes all 5,846 inputs and all 47 archive
originals without executing project code. Local publication checks are
mechanical only; development and tests remain on mi350.

```text
Replay:      12e961aaf9a5bf4d2a1a4750517568ec7284a974b82a852674c678559ca2f507
Test ELF:    b6b03b519e08c0ffb842508263fb24aadde2e450bd0d0d93e19171a80928ecf3
Capture:     7a149f67fd9fa233c6e72f038c6462016475e585bc14172a9b70cc94ed958bed
Controller:  7aa6339a3cda8144db8117538cc005e47de8f71df0e4c0c7516d714e448e51d8
Data audit:  e31a602b1a7dd1ab52fed627c74ca6342c4d9730fa89a3aa88609f48290dfbe6
Archive:     317fb541caeaa70d74f2869a77ea7fb74cd4e700206cbaf0132eb74449e52c34
```

No new HSACO, GPU numerical result, decode, speedup, or 700 tokens/s claim
follows. The unresolved device-completion hold remains closed.
