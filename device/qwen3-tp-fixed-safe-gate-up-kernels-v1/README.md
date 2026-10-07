# Fixed Safe Gate/Up Kernel

Default-off Ferric experiment derived from the source-reviewed
`perf-v8-fixed-gate-up-safe-loads-r1` proposal. This crate implements the
distinct `ferric_qwen3_c1_gate_up_fixed_safe_u32_bf16_r1` root, not a new
production selector. The original direct-slice source passed CPU/SDK tests but
its `candidate-emission-a002` attempt failed AMDGPU lowering with
`UnprovenBarrierConvergence` at `bb18`, operation 0. It produced no qualified
image or performance result. The r3 fixed-view source passed 30 tests in each
feature mode and clippy, then emitted under the pinned compiler in
`fixed-safe-emission-capture-a003`. Its archived source, image and native input
bundle are retained separately. The current r4 source additionally moves
finite checks out of the lane accumulation loop. It is a new, untested,
unemitted and unmeasured candidate requiring fresh CPU, emission and native
evidence; r3 evidence does not qualify r4 and it remains default-off.
The manifest pins observed upstream fe2o3
`1a5999f6e1c5f2363bc2d525af65e84c46502ce6`, migrated from
`c508e7a2cc94ea6d60ba3d56f0246a8599bd5898`. The pin migration does not reuse
older SDK, image or native qualification, and native qualification remains open. The candidate
remains default-off and its performance gain is unmeasured. Resolve and retain
the Cargo-produced lockfile on the approved remote CPU host; no Cargo.lock has
been fabricated.

The smaller experiment comes before the existing fused gate/up crate. The r3
revision exposed fixed bounds through checked SDK read views; its emitted loop
still had both weight guards. The r4 revision tests whether removing repeated
finite classification from that loop improves the generated code without
changing its packed ABI, read extent, arithmetic or runtime. Measure r3 versus
r4 separately before considering a combined comparison with the original
projection. Source appearance is not an ISA or speedup claim. Compare both arms
under the same pinned producer; the compiler upgrade is a separate transition.

## Contract

Only TP1/C1 roles 4 and 5, N12288/K4096, Wave64 and 12288 blocks of 64 threads
are admitted. Activation capacity is 2048..65536 words, weight length is exactly
25165824 words and output capacity is 12288..393216 BF16 elements. Capacity
tails are not accessed. Invalid geometry rejects before either read.

The maximum activation index is 2047 and maximum weight index is 25165823.
Reads use safely constructed `StridedReadView2D` values with literal dimensions
1x2048 and 12288x2048. Their total `load_or` operations reconverge before the
subgroup reduction, unlike a lane-dependent slice bounds panic. There are no
unchecked pointers. The unchanged admission and fixed coordinate bounds make
the zero fallback unreachable on valid inputs; malformed geometry is still
rejected before any load. Compiler convergence checks are not relaxed.
Two separate FP32 products/adds per packed word, low then high, the existing
Wave64 reduction and BF16 RNE narrowing are unchanged from the retained R2
source fixture. No reassociation, FMA or lower precision is introduced. The
fixture is historical comparison source, not a claim that r4 has emitted an
equivalent image.

The only r4 arithmetic-path change replaces sticky per-product and per-partial
`is_finite` checks with one check of the final lane partial. Under strict IEEE
FP32 multiply/add, a nonfinite product makes the next partial nonfinite, and a
nonfinite partial cannot become finite by adding any later product. Opposite
infinities produce NaN, not a recovered finite value. Thus induction from the
initial finite zero preserves the final rejection predicate, including
overflow caused by finite products. This argument requires strict operations:
no fast-math assumptions, reassociation or accumulator reset is allowed.
Rejection still occurs after the collective, never as a lane-early trap. The
post-reduction FP32 and post-narrowing BF16 finite checks remain required and
unchanged. Rejected NaN payloads are not an output contract.

`fixed-safe-gate-up-r1` explicitly enables the device modules and generated
roster. Default features expose only host helpers. Device emission requires
the exact gfx950, Wave64, xnack-disabled flags and either the existing managed
binding-wrapper route or a selected engineering compiler handoff. The latter
requires `FE2O3_EXTRACT_CRATE_V1` to name this exact library and
`FE2O3_EXTRACT_AMDGPU_COMPILER_HANDOFF_PATH_V1` to name a nonempty absolute
output path. These are routing checks, not source authentication. The selected
extractor derives and installs its own crate binding before macro expansion;
the build script supplies no fixture binding for either device route. Raw
AMDGPU builds remain rejected, and the fixture binding remains host-only.
Engineering observations grant no production publication, load or launch
authority; retained source custody and native admission remain separate gates.
The included activation-pack source is unchanged; an experiment may reuse its
previously qualified image only with that image's original provenance.

## Small Remote Gate

Run only on the approved remote CPU host inside the bounded owner-controlled stage. Use
the approved Rust toolchain, one build owner, four-core/eight-GiB limits,
nice 19, timeouts, stage cap/reserve and cleanup checks. No local build is
required or permitted. From this crate, after recording exact source hashes:

```sh
rustc --edition=2024 --test tools/standalone.rs -C opt-level=1 -C debuginfo=0 -o "$OWNED_OUTPUT/fixed-safe-host-tests"
"$OWNED_OUTPUT/fixed-safe-host-tests" --test-threads=1
```

The entry point reuses the same `host.rs`, `fixed_host.rs`, `fixed_shape.rs`
and source/target/build-route contracts as Cargo. Its authored tests cover all 25165824
admitted weight indices, invalid geometry, every BF16 packing pattern,
strict cancellation-sensitive arithmetic, finite rejection and source guards.
The r4 tests also compare every accumulation checkpoint and final row outcome
against the sticky-check model, cover each BF16 pattern against representative
product/partial classes, and exercise early/middle/final nonfinite products,
finite-product overflow followed by cancellation, signed zero, subnormals,
underflow, reduction overflow and BF16 narrowing overflow. Source contracts
permit only the finite-check relocation and require no early collective exit.
The dependency-free BF16 model is host-only. This gate does not build the SDK,
expand kernel macros, run a GPU or prove generated bounds elimination.

## SDK And Native Gates

When the root-controlled build budget permits, resolve and retain the lockfile
for the exact SDK pin on the approved remote CPU host, then run:

```sh
cargo test --offline --locked --no-default-features
cargo test --offline --locked --no-default-features --features fixed-safe-gate-up-r1
cargo clippy --offline --locked --all-targets --no-default-features --features fixed-safe-gate-up-r1 -- -D warnings
cargo fmt --check
```

The Cargo suite also cross-checks the host BF16 model against the actual SDK for
every 16-bit pattern and six FP32 low-half/rounding cases, checks explicit
feature selection, and exercises actual SDK fixed-view reads at both activation
capacities and representative weight rows including the final row. The
dependency-free suite still checks every admitted integer coordinate; SDK view
tests are Cargo-only. Direct host tests are not a substitute for those checks.

Emit the distinct projection root using the pinned managed compiler, retain
IR, metadata, descriptor, image and disassembly, and verify the three-slice,
five-u32 ABI. Rejection must dominate reads; no spill, extra read, changed
FP32 operation/reduction order or loss of finite rejection is acceptable.
Inspect the actual admitted loop. If the intended guard/load/wait scheduling
does not improve, record that negative result and stop before a model run.

On the approved MI350 host only, a reviewed kernel harness must next check
full-shape finite parity against the retained r3 image, both roles,
minimum/maximum capacities and untouched tails. Rejection/trap qualification
remains a separate gate; do not launch unreviewed native negative cases. Require matched,
alternating-order repeated kernel timings before adding any engine selector.
Only an explicit opt-in integration with passing full-model parity and native
ABBA/HTTP comparisons can support a TTFT/TPOT improvement claim.
