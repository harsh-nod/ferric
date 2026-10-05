# Ordered Projection Residual / MLP Ferric Proposal V2

Source-only candidate against Ferric `30a0613af2bca8901daa603e952b43379e073dc1` and the actual CPU883 source generation. No candidate imports, formatter, tests, compiler, GPU execution, or live-tree edits were performed by the author. The root owns qualification and integration.

## V2 Feature-Gate Repair

The frozen V1 proposal remains unchanged. Its actual CPU attempt recorded 1848 passing tests, seven historical ignores and five built executables, but failed the final `parent-default-check` with exit 101. The retained failed receipt is 624367 bytes, SHA256 `dd363e5d49249d5096418a97c574a2cf4852b342658e922eea922049f94040c4`. Those passing tests do not make that attempt a completed qualification.

V2 differs from V1 in two candidate files only: the parent `src/lib.rs` adds `#[cfg(feature = "tp-batch-engineering")]` to the ordered wire and observation exports; the existing ordered-bin test additionally checks both exact export gates and the Cargo binary's required feature. Its existing name and the 30/12/1 compiled-name lists are unchanged. The test uses the existing TOML dev dependency; no dependency or executable behavior changes.

The source-gate audit also checked that `tp_finite_client` already has the same feature guard, enclosing the new `projection_ordered` and `ordered_host` modules, and that the new Cargo binary already requires that feature. All new parent references are in those guarded scopes. Worker exports remain unconditional in their separate crate. The other 34 candidate bodies and all 23 original preimages are byte-identical to V1.

The source assertion complements, rather than replaces, actual compilation. The root must rerun the preserved `parent-default-check` with no engineering feature, alongside the complete selected feature-enabled suites and executable builds. V2 is authored only and has not been compiled or tested. The authenticated original runtime inventory has 900 names, plus 25 new names; its expected 922 passes/three ignores combine with Ferric 926/four to 1848/seven. This corrects the earlier 1846 prediction; the actual named inventories remain authoritative.

## Scope and Selection

`source-manifest.json` pins 36 candidate bodies: 23 exact preimages/replacements and 13 additions. Original files are retained under `baseline/`; the executable overlay is `candidate/`; `changes.patch` is their source diff. Only the worker and engineering parent adapters change. Compiler/provider sources and machine-code images are not changed.

The additive parent binary is `ferric-qwen3-finite-projection-residual-decode-ordered-host-engineering`, behind `tp-batch-engineering`. Its closed CLI is:

```text
--request ABSOLUTE_PATH --allow-unauthenticated-machine-code --observe-projection-residual-mlp-ordered
```

It selects only `--engineering-native-projection-residual-mlp-ordered-v1` in the worker and only autoregressive four-forward requests. Distinct bootstrap, profile domain, configuration, observation envelope, sidecar and Control decoder reject mixed legacy/ordered routes. Original Begin image, explicit projection image, MLP image, prefix image and their authenticated SHA identities remain separate. Existing plain, default-host, shared-host and device-clock selection remains unchanged; there is no fallback.

## Fixed Ownership and Ordered Boundary

Both Prefix284 producers must retire and validate before the combined dispatch. The new roster transition is `FirstResidualReady -> ResidualMlpInFlight -> FinalResidualReady`. The runtime receives one projection-residual packet followed by one MLP packet on each rank; both ranks publish before polling. Both embedded deadlines must be the same bounded 1..10000 ms value. One aggregate deadline includes all preflight, currentness fences, publication, signal waits and terminal validation.

The route calls the runtime candidate's `Group::dispatch_projection_residual_mlp_tiles_round_unchecked_v1` with two `Gfx950EngineeringPeerProjectionResidualMlpTilesDispatchV1` commands. Each binds its projection kernel/SHA, the same ordered pair of original O partials, its rank-owned residual input and the existing MLP command/state. Both returned 548-word states are compared with observed terminal states before either final residual is admitted. Any boundary error poisons the roster; there is no retry or successful partial result.

Ordinary `LayerBindings` retains its original `mlp[9] == prefix[13]` alias contract. That alias is unsafe within the compound segment: one rank could overwrite an O partial before its peer reads it. The new Owner therefore allocates exactly two private peer-readable Down buffers, one 16384-byte buffer per rank, once after original setup. A dispatch-only view changes only MLP root9. First residual reads the original O pair; final residual reads the distinct Down pair. Both final residual consumers retire before the next layer/forward can reuse either pair. There is no host copy-back or per-layer/per-forward allocation.

The original allocation census remains 714/710. Ordered setup, metadata upload, idle fences, reuse ledger and Close use the separately selected 715/711 census. The auxiliary buffers are private setup-owned tokens, checked for owner, exact extent and aliasing against every sealed layer root; callers cannot supply forged auxiliary scratch. Healthy Group Close releases them. Setup/publication/dispatch failures do not permit scratch reuse or another forward.

The current pinned SiLU Down implementation in `qualification/silu-materialized-lowering-v1/fixture/src/mlp_tile_numerics_v2.rs`, macro `qwen_claimed_mlp_down_tile2_v1`, initializes both accumulators to zero and writes each output directly after reduction. The 32 row pairs per tile and all64 terminal tiles overwrite all4096 FP32 outputs; no previous Down value is read. This is the source basis for uninitialized fresh scratch, not a new numerical measurement.

## Policy and Timing

Configuration is explicitly `configure_performance_v2(false, false, true)`: admission-cache off, operational-currentness shortcut off, shared-full-currentness on. Raw device timestamps remain independently off. Configuration occurs before observer enable and outside all seven zero-origin snapshot intervals.

This is a changed coherence policy: the intermediate host fence between projection residual and MLP is replaced by ordered GPU sequencing. Fresh checks remain before and after the complete segment, with bounded waits/fault checks. Scratch reuse caches storage identity, not currentness or admission proof. No claim is made that the entire per-kernel policy is unchanged.

Ordered Control records are exactly 241096 bytes. Each layer records a Prefix pair, one combined `segment_host_ns`, and a final-residual pair. No separate first-residual or MLP duration is invented; the legacy encoder rejects the new Timing variant. These are host diagnostics, not GPU durations or a throughput acceptance result. Own-output recurrence is validated before backend entry; four payloads, controls, chain, healthy Close, EOF and reap remain mandatory.

## Authored Tests

The manifest contains exact sorted compiled names: worker30, parent-library12, new-parent-bin1. There are35 unique methods and43 compiled executions because eight wire/report tests compile in both adapters. Existing test names are not removed or renamed. Most old fixture edits only preserve their paired timing values through the explicit legacy Timing variant.

Coverage includes ordered trace/global boundaries; poisoning and terminal phase refusals; fixed scratch allocation/repeated selection; wrong census before metadata writes; O/Down alias fallback; image/profile/mode isolation; single-segment timing and legacy serialization refusal; wrong prior token before backend; four own outputs/payloads; Control/Close/EOF/failure chains; exact policy configuration and sidecar intervals; and closed parent/worker selectors.

The expected Ferric increment is883+43=926 passing tests with the same four historical ignores, but that is not an actual result. The paired runtime/Ferric CPU controller must also build/run the complete reviewed KFD library cohort and the new runtime tests. No qualified source, image, numerical or performance claim follows until root execution.

## Root Qualification

Use the reviewed `p228-projection-ordered-segment-cpu-v2` controller and a root-authored pinned input plan containing this manifest plus the runtime manifest. Copy the three exact `added_tests` groups from this manifest, the runtime group's exact names from its own manifest, and the parent binary above. The controller handles isolated paired source copies, restricted formatting, actual inventories, all existing affected tests, selected products and source/dependency postchecks. Do not compile the candidate against the old runtime without its additive API.

No numerical thresholds, long-prompt/decode claims, model acceptance, sustained token rate or predicted speedup are introduced. A future GPU supervisor must decode this ordered Control/profile explicitly and compare same-generation payloads only as repeatability, not independent accuracy.
