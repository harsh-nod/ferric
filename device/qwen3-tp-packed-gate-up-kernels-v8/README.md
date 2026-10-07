# Fixed-Shape Fused Gate/Up V8

Experimental Ferric-authored kernels using fe2o3 at the fixed revision
`b7d5f2bf7bf9e66037df1ff1d7ca2738ffdf98dd`. Default features are empty. The
explicit `fused-gate-up-r1` feature enables a distinct two-root gfx950 image:

- `ferric_qwen3_c1_gate_up_activation_pack_u32_v8`: 4096 BF16 input elements,
  2048 u32 output words, 32 Wave64 workgroups, integer-only lane-time packing.
- `ferric_qwen3_c1_gate_up_packed_u32_bf16_v8`: one 2048-word activation view,
  two separate 12288x2048-word weight views, two separate 12288-element BF16
  output views, 12288 Wave64 workgroups. No dynamic projection or shape tags.

Both roots require exact view extents; parent allocations may be larger only
when a future authenticated binder supplies those exact disjoint views. There
is no Q/K/V, head, down-projection, TP2, multirow or automatic fallback route.
All accesses use the existing checked typed views and disjoint output API.

The fused loop loads one activation word and two weight words per group/lane.
Each output independently retains the original low-then-high multiply/add order,
sticky finite state, six-stage Wave64 sum and BF16 narrowing. Both reductions
and both finite/narrowing checks precede either output store. No FMA, MFMA,
reassociation, final-only finite check, unchecked load or shared accumulator is
introduced. Gate and up weight bytes remain separate and unchanged in size.

Host models and source-contract tests are authored but unrun. They are not
device execution or a numerical/performance qualification. NaN payloads are
normalized only in the rejected-input host trace comparison; packing retains
every BF16 bit. The model's paired store does not claim GPU transactional memory.

No current adapter selects these roots. Existing packed R2/R3 integration,
actual 413 images, V19/V27/split8 composition and prefill selectors are untouched.
Future image admission needs actual managed-emission source/roster/ABI/HSACO
identities and a separately reviewed same-image control/candidate route.

Qualification must generate and freeze the actual lockfile in the approved
offline environment, run default and selected-feature tests/Clippy, emit with
the existing managed gfx950 Wave64 toolchain, inspect strict FP and descriptors,
and compare both outputs against two separate strict projection dispatches on
full N12288/K4096 data, including activation-pack cost and output guards. Reject
unexpected spills or arithmetic drift. Full-model token/byte parity and matched
TTFT/TPOT are required before any performance claim. None has been run here.
