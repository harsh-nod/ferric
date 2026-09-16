# gfx950 Finite Decode Megakernel

Implementation epic: [Ferric #42](https://github.com/harsh-nod/ferric/issues/42).
This document records the initial architecture and implementation boundaries;
it is not a GPU qualification receipt.

## Decision

Keep model planning and execution integration inside Ferric. Reusable GPU task
schemas, scheduling primitives, compiler lowering, and runtime ownership belong
in [fe2o3 #135](https://github.com/harsh-nod/fe2o3/issues/135). Do not create a
second inference project or a parallel compiler path.

## Initial Implementation Checkpoint

| Gate | Observed status |
| --- | --- |
| Public-main baseline | 39 CPU tests passed on `mi350-2` |
| Declared operation planner | Implemented and CPU-tested; no admission authority |
| Measurement/ablation validator | Implemented and CPU-tested; no actual performance results |
| Exact MI350-2 platform tuple | Additive upstream engineering profile; negative tests pass |
| Actual device binding | Read-only binding/currentness/cleanup pass with explicit XCP0 parent-render contract |
| Ordinary Rust atomic lowering | Rejected: callable memory-effect summaries incomplete |
| Tile worker scheduler / numerical handlers | Not implemented by this checkpoint |
| Full-model decode / performance qualification | Not executed; no claim |

The CPU test logs and failing device/compiler probes (device IDs redacted) are in
[the initial evidence directory](evidence/gfx950-megakernel-42/README.md).
The host's partition identifier and board identifier are different domains,
not a decimal/hex parsing error. A narrowly scoped upstream SPX/XCP0-to-parent
render contract now retains and independently rechecks both domains. It is
selected by the exact engineering profile, not by a failed equality comparison.
At that initial checkpoint, no GPU work had been run. Protected production
authority remains separately open.

## GPU Numerical Checkpoint

The next executable slice now compiles and runs end to end on `mi350-2`:
[fused F32 decoder source](../device/gfx950-decoder-layer-f32-v1/README.md) ->
ordinary fe2o3 semantic/ranked/formal checks -> gfx950 LLVM/HSACO -> the existing
direct-KFD engineering worker. One dispatch executes a complete tiny decoder
layer, retaining eleven stages and forty intermediate/final values per request.
Two 128-thread workgroups contain two wave64 waves each.

All four numerical families passed: 40,960 values against an independent FP64
matrix reference, with maximum absolute error `9.224651e-6`. Input immutability,
output guards, completion and explicit cleanup passed for every dispatch.
The source, build/tool identities, raw GPU outputs, per-stage comparisons and
reproduction scripts are in the [numerical evidence package](
../qualification/gfx950-decoder-layer-f32-v1/README.md).

This is a small F32 layer baseline, not the declared Qwen BF16 model envelope,
persistent scheduling, cooperative tiling, full-model generation or performance
qualification. No production deployment or SoTA claim is made. The engineering
runtime and its operator-trusted machine-code boundary are not promoted to
Ferric's protected runtime authority.

Upstream compiler work also admits from-start constant slice indices with their
exact runtime bounds guards and offers opt-in, source-audited MIR inlining.
Ten ordinary Rust atomic RMW operations reach gfx950 LLVM with ordering and
returned-value dataflow retained. That compiler test does not establish a GPU
scheduler or cross-workgroup publication protocol by itself. Those RMW source
checks and the GPU scheduler observation below are distinct evidence.

## Atomic Task Graph GPU Checkpoint

The [seven-task atomic integer graph](../device/gfx950-task-graph-v1/README.md)
now passes the checked ordinary Rust compiler pipeline, emits gfx950 HSACO, and
runs through the engineering direct-KFD worker. The fixed `mi350-2` suite passed
20 dispatches: 16 valid epochs and four stale-epoch negatives. It checked 260
state words: 244 deterministic values and 16 ownership encodings. This includes
112 exact payload values from the valid epochs. Every valid epoch used both
128-thread workgroups and recorded four cross-workgroup dependency edges.

The tested Ferric checkpoint is `e3d198ee`; the clean fe2o3 compiler is
`af4f1460`; HSACO SHA-256 is
`8f4904773d92c58b0eb2ebb06e0195eac536b5a28f35a92e3b9fa2bda5ba328f`.
See the [scheduler evidence package](../qualification/gfx950-task-graph-v1/README.md)
for exact source/tool identities, state checks, and reproduction.

This closes the bounded atomic-integer micrograph execution slice, not general
tensor visibility, authenticated tiled handlers, full Qwen generation, protected
runtime authority, or performance qualification. One-resident-worker progress
is covered by the separate host scheduler model; the observed GPU runs are not
a proof over all possible interleavings or residency conditions.

## Initial Envelope

Start with a finite decode forward pass. Workers persist through the pass and
return before the next token. This is distinct from keeping a GPU service alive
across tokens. Ordinary Ferric prefill initially owns prefill; an admitted shared
KV contract must connect it to decode.

The first envelope is Qwen3-0.6B and Qwen3-8B, one gfx950 device, BF16 with an
explicit FP32 accumulation policy, greedy target-only decode, batch 1/2/4/8,
and context buckets 1024/4096/8192. Actual prompt lengths and page positions are
per-step inputs. Shape support in a planner does not mean executable support.

## Current Implementation Boundary

`tools/megakernel-planner` is an isolated addressless **declared** model-plan tool.
It validates graph structure and bounded workspace declarations. Caller-supplied
identity bytes do not authenticate weights, model revisions, numerical policy,
or code. The crate has no GPU dispatch or production admission constructor.
It is outside the production workspace and Verus-qualified release closure,
like the repository's existing separate host tooling. Promoting its logic into
the runtime requires actual strict Verus coverage, not disabling that gate.

`qualification/gfx950-megakernel` checks engineering measurement records,
artifact-file digests, workload comparability, samples, and ablations. It does
not replace Ferric's production property/evidence closure and cannot authorize
a public faster claim by itself. Test fixtures are synthetic validator inputs,
not benchmark results.

`tools/gfx950-finite-probe` and `qualification/gfx950-decoder-layer-f32-v1` are
separate engineering execution and numerical tools. They exercise the existing
direct-KFD worker with fixed ABI/launch/buffer contracts and compare independently
generated reference data. Their successful GPU observations do not authenticate
model bundles or close the protected execution/property milestones below.

At the initial inventory, public Ferric main
`5d3d93d3e7f08645273d274bc35efbc79133e686` contains M0 engine/specification
components. The authorized M1 implementation branch has authenticated bundle,
exact Qwen graph, generated-runner, and physical KV work not present on public
main. Reuse those interfaces when integration is ready; do not recreate their
authority with caller-provided booleans or hashes. In particular, the private
catalog's retained bundle-admission record and exact generated-plan validation
must survive the transition to a finite task plan.

## Graph And Storage

The complete dense decode graph includes embedding; every decoder layer's input
RMSNorm, Q/K/V projections, Q/K normalization, RoPE, KV append, GQA attention,
output projection with residual, post-attention normalization, gate/up
projections, SwiGLU, and down projection with residual; then final norm, logits,
and deterministic argmax. Query projection width must be separate from hidden
width: Qwen3-0.6B does not satisfy `query_heads * head_dim == hidden_size`.

The declared numerical policy stores logits in FP32. For the fused
projection/residual operations it requires rounding the projection to BF16
before adding the residual, then rounding the result to BF16. Fusing an FP32
accumulator directly into the residual and rounding once is a different policy.
These are obligations for future handlers, not established numerical evidence.
Model declarations explicitly include RMSNorm epsilon `1e-6` and RoPE theta
`1000000`; dimensions and constants were cross-checked against the official
[0.6B configuration](https://huggingface.co/Qwen/Qwen3-0.6B/raw/main/config.json)
and [8B configuration](https://huggingface.co/Qwen/Qwen3-8B/raw/main/config.json).
Production admission must pin model revisions and authenticate their contents;
reading a configuration from `main` is not such admission.

Operation-level declarations are not tile schedules. Device work needs a closed
task family, exact tensor contracts, authenticated handler IDs, bounded tile
coordinates, and a proved or checked-to-the-stated-bound memory plan. Reusing a
scratch range requires an actual dependency ordering all former consumers before
the next writer, not merely a favorable topological listing.

The future device ABI must use explicit fixed-width records and bounded handles,
not Rust enum serialization or arbitrary function pointers. Bind task schema,
scheduler, fusion, persistent plan, executable and run identities from fe2o3 to
Ferric model, KV, request, and epoch identities. A structural plan is not that ABI
and must not be serialized directly as a GPU launch descriptor.

## Scheduler Contract

Use bounded ready-task distribution and unique task ownership. Release/acquire
publication must carry every producer's writes through dependency fan-in to the
consumer. A completion counter without the matching visibility protocol is
insufficient. An empty queue with tasks in flight is not termination.

Do not wait at a grid-wide barrier in an ordinary launch. Progress must not need
all workgroups to be resident simultaneously. State fairness/residency
assumptions separately from safety. Specify capacity/backpressure, epoch wrap,
exactly-once completion, counter reset, and cancellation before optimizing.
Timeout never authorizes freeing allocations the GPU may still reference.

Use wave64 and measure candidate multi-wave workgroups. Preserve gfx950 register,
LDS, private scratch, and object metadata in the resource contract. MFMA versus
GEMV, transpose instructions, software pipelining, multi-buffering and tile sizes
are experiments requiring numerical and resource tests, not mandatory wins.

## Integration Gates

1. Authenticate the model and exact graph using the existing M1 ownership APIs.
2. Complete ordinary Rust source lowering for the required scoped atomic,
   barrier, and handler operations; do not insert hand-written KIR/LLVM/HSACO.
3. Admit the exact gfx950 driver/runtime profile. Engineering direct-KFD support
   remains distinct from protected production authority.
4. Run cross-workgroup micrographs, numerical handlers, and one complete layer.
5. Bind the full model and physical KV; commit state only after exact completion.
6. Validate full-model numerics and generation, then tune against equal-work
   baselines with raw paired measurements.
7. Close release/property gates and qualify the exact executable without rebuild.

None of these gates is completed by a passing host-side planner test.

### Atomic Source Vertical Slices

The initially rejected ordinary Rust atomic fixture identified the following
upstream boundaries; it did not show that gfx950 lacks the instructions. The RMW
slice now compiles and executes in the bounded micrograph above. The original
breakdown below remains useful for broader atomic/handler work; an allowlist or
purity annotation cannot replace the required effect custody.
Paths below are relative to the fe2o3 repository.

| Boundary | Existing owner | Required implementation |
| --- | --- | --- |
| Pointer-to-atomic adapters | `crates/fe2o3-device/src/atomic.rs::global_atomic_view!` and `crates/rustc-codegen-fe2o3/src/production_semantic_terminal_v1.rs::is_traversed_reviewed_helper_v1` | Preserve the authenticated helper body, types, and returned pointer provenance through a bounded normalization or equivalent effect summary |
| Defined-call admission | `crates/rustc-codegen-fe2o3/src/production_ranked_projection_v1.rs::DefinedCallableEmptyEffectSummariesV1` and `require_bounds_neutral_callable` | Add exact pointer-result and memory-effect custody; scalar empty-effect summaries are insufficient |
| Allocation correspondence | The same file's `local_provenance_v1`, `local_allocation_contracts`, and `authenticated_source_allocation_contract_v1` | Retain allocation identity, offset, alignment, borrow/alias constraints, and source correspondence across normalized calls |
| Source intrinsic recognition | `crates/rustc-codegen-fe2o3/src/production_rustc_intrinsic_v1.rs::atomic_rmw_intrinsic_rule_v1` | Extend beyond RMW to atomic load/store and strong/weak CAS, including both CAS orderings and its result representation |
| Production KIR | `crates/fe2o3-lower-mir-kernel/src/production_semantic_kir_v1.rs` | Preserve the complete effects and SSA results; implement atomic load/store/CAS rather than using the scalar-helper path |

A bounded body-derived transformation of the closed authenticated adapter and
wrapper closure can keep general effectful helper calls unsupported. Inlining
only the pointer adapter does not close the atomic method/wrapper calls. Marking
them pure would erase the effects the scheduler relies on.

Implement and test the slices in this order:

1. Normalize the complete supported RMW call closure with exact callee/type/ABI
   identities, orderings, pointer provenance, and effectful result custody.
2. Require ordinary Rust RMW fixtures to reach production LLVM with observable
   results and intact side effects even when return values are unused.
3. Add load/store and CAS source-to-KIR support with explicit width, scope,
   ordering, and failure-order restrictions.
4. Bind coherent allocation and target/runtime visibility contracts, then run
   bounded multi-workgroup publication tests through direct KFD.

Negative tests must reject spoofed providers, changed helper bodies, stale
identities, missing provenance, conflicting aliases, unsupported widths and
orderings, recursion, and arbitrary pointer transformations. Do not identify
the first rejected callable solely from the diagnostic's numeric callable ID;
the archived log does not provide its name. The initial roadmap is not evidence
for every listed atomic operation; only the explicitly recorded source tests
and bounded GPU fixture establish their stated slices.

## Validation And Performance

Run the workspace tests, including the planner, with the pinned Rust toolchain:

```sh
cargo test --workspace --locked
cargo test --manifest-path tools/megakernel-planner/Cargo.toml --locked
cargo clippy --manifest-path tools/megakernel-planner/Cargo.toml --all-targets --locked -- -D warnings
python3 -m unittest discover -s qualification/gfx950-megakernel -p 'test_*.py'
```

The planner tests are separate engineering checks, not runtime proof coverage.

For device execution, follow the admitted fe2o3 runner's documented build and
lifecycle. Do not relabel a gfx942 receipt or bypass a failed platform check.
The new `mi350-2` environment must be recorded separately from previous MI350
runs. It has an observed MI350X, but observation is not an executable receipt.
Use only this work's allocations/processes, isolate builds, coordinate device
testing, and preserve source and raw evidence before cleaning owned scratch.

The initial unmodified public-main baseline on `mi350-2` passed 29 engine and
10 specification tests with Rust 1.97.1. These are CPU tests, not GPU execution
or Qwen numerical results. A source snapshot of fe2o3
`e3c359fb1bf39ec21c4239ac37ce59b7a3a51db9` is the initial upstream comparison.

Follow [PERFORMANCE.md](PERFORMANCE.md) for equal work, samples, confidence
intervals, stability checks, and the public-faster threshold. Compare the best
qualified Ferric batch path and applicable tuned ROCm vLLM/SGLang configurations.
Include all token-step dispatches, reset/sampling work, and CPU overhead.

Ablations must identify actual executables and raw observations. Measure dispatch
consolidation, scheduling, balancing, tile selection, prefetch, pipelining,
multi-buffering and layout changes separately; include rejected candidates and
leave-one-out studies for interacting optimizations. Do not fill pending tables
with predictions or interpret synthetic validator fixtures as measurements.

Derive an optimistic latency floor from required transfers, precision-specific
work, and dependencies:

```text
T_floor = max(B_min / BW_sustained,
              F_precision / C_sustained_precision,
              dependency_critical_path_floor)
efficiency = T_floor / T_measured
```

Document cache/reuse assumptions, active weights, KV traffic, descriptor traffic,
all counted operations, sustained calibration measurements, and omitted costs.
This is an explicitly assumed model, not an unconditional theorem. Never infer
a full-model speedup from isolated kernel timings.
