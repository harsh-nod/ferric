# gfx950 Finite Decode Megakernel

Implementation epic: [Ferric #42](https://github.com/harsh-nod/ferric/issues/42).
This document records the initial architecture and implementation boundaries;
it is not a GPU qualification receipt.

## Decision

Keep model planning and execution integration inside Ferric. Reusable GPU task
schemas, scheduling primitives, compiler lowering, and runtime ownership belong
in [fe2o3 #135](https://github.com/harsh-nod/fe2o3/issues/135). Do not create a
second inference project or a parallel compiler path.

## September 15 Implementation Checkpoint

| Gate | Observed status |
| --- | --- |
| Public-main baseline | 39 CPU tests passed on `mi350-2` |
| Declared operation planner | Implemented and CPU-tested; no admission authority |
| Measurement/ablation validator | Implemented and CPU-tested; no actual performance results |
| Exact MI350-2 platform tuple | Additive upstream engineering profile; negative tests pass |
| Actual device binding | Rejected: KFD XCD ID differs from DRM board ID |
| Ordinary Rust atomic lowering | Rejected: callable memory-effect summaries incomplete |
| Tile worker scheduler / numerical handlers | Not implemented by this checkpoint |
| Full-model decode / performance qualification | Not executed; no claim |

The CPU test logs and failing device/compiler probes (device IDs redacted) are in
[the initial evidence directory](evidence/gfx950-megakernel-42/README.md).
The host's partition identifier and board identifier are different domains,
not a decimal/hex parsing error. Fixing that requires a reviewed XCP-to-render
identity contract upstream; disabling equality or hardcoding one device pair
is not an acceptable fix. Protected production authority is separately open.

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
