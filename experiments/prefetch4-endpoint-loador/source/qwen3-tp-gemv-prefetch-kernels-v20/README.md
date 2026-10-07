# V20 Four-Pair GEMV Prefetch Proposal

Candidate outside the active Ferric repository. Eight CPU tests passed at the
current c4c compiler dependency pin. Its first strict Clippy run reached the
unchanged staging-space reserve and was terminated; the relocated strict rerun
passed on mi300x-2 with clean owned-process teardown and unchanged resource limits.
No emission, native run, artifact admission, routing change or speedup is claimed. Existing
V5/V17/V19 sources, defaults and producer identities remain unchanged.

## Source and Evidence

This proposal changes only the inner load schedule of the two Wave64 roots in
`device/qwen3-tp-batch32-kernels-v5/src/projection.rs`, lines 15-234, read at Ferric
`38b3213a90c4a29a791a1cea909967d70d8eaa31`. The complete original projection file
SHA256 is `fa5b723f989dbdb4a3cbfc9313ec2f5777bcc94d1171b7a6a93cf4fa9529bea3`.
`tests/fixtures/v5-gemv.rs` retains exactly its first 234 lines, SHA256
`f6159318607f5ffa084211989bb1cfef8bbc128e573a756ec2df3734b662343e`.
The copied existing target-build helper remains byte-identical, SHA256
`590e235c06df8041b8717caffaea9d1f56e40738d03d6947f4a3324cb171865a`.

The retained ISA evidence is under
`ferric-c1-resident-isa-v1/retained-r1/commands/`:

- `gemv-bf16/stdout`: ushort loads at 612/623/632/642 each immediately wait for
  `vmcnt(0)`; arithmetic at 588-599 retains sequential multiply/add/check pairs.
- `gemv-partial/stdout`: loads at 328/336, 361/370 and 398/407 likewise wait;
  arithmetic at 340-346, 374-380 and 411-419 is serial.
- Retained metadata reports 18 VGPR/38 SGPR for BF16 and 20 VGPR/42 SGPR for the
  partial root, with no spills. These are the old images, not V20 metadata.

This suggests a possible load-latency opportunity, not a measured kernel cost.
The approximately 57 ms attention-group and 27 ms feedforward-group controller
scopes overlap and are host wall latency, not per-kernel GPU timings. They cannot
establish V20's expected gain. Current TP1/C1 target routing uses rows=1,
k=4096 for Q/K/V/gate/up and partial k=4096/12288 for attention/down output.
FP32 output-head V8 remains separate; the retained BF16 vocabulary case is not
the current FP32-head route.

## Exact Source Change

Each wave still owns one row/output-column. Load four consecutive lane-stride
raw BF16 input/weight pairs into eight named scalar locals, then consume them in
the original ascending order. BF16 uses 16 groups of four; partial uses 48
groups of four, preserving the original 192-step upper bound. Every partial
pair keeps its own `inner < k` predicate for both loads and arithmetic, including
inactive attention-output tails. No array or local-memory staging is added to
device code.

The roots accumulate a finite flag and do not trap inside the original numeric
loop. Thus these in-bounds immutable reads can move ahead of arithmetic without
moving any early numerical trap. Each product is still separately rounded FP32
multiplication, followed by the same single-accumulator addition and both finite
checks. No FMA, multiple partial sums, reassociation or new reduction is used.
All lanes remain active through the exact same subgroup reduction. The original
post-reduction finite checks, BF16 narrowing and typed disjoint output store
remain unchanged. Invalid nonfinite NaN payloads are not an output contract;
the same rejection state must persist before any store.

All original rows 1..32, TP1/2/8, projection, n/k, slice extent, view construction,
grid extent and output ownership checks remain statement-identical to the V5
fixture. Host input immutability and nonaliasing admission remain prerequisites;
this source does not authenticate arbitrary device pointers.

New independent roots:

```text
ferric_qwen3_tp_wave_gemv_prefetch4_bf16_v20
ferric_qwen3_tp_wave_gemv_prefetch4_partial_f32_v20
```

Both have three slice pointer/count pairs followed by five u32 scalars. Expected
explicit bytes are 68, the padded implicit offset is 72, and total kernarg bytes
are 328 with alignment 8 and the existing 256-byte COV6 implicit tail. This is a
source expectation, not a substitute for checking actual emitted metadata.
Wave64 geometry and maximum workgroup counts remain 4,861,952 and 131,072.

## Gates and Compiler Risks

The isolated crate pins fe2o3 `c4c5cdd0f69f3844386440a5addb4d4c3dce0e4b` and uses
the existing typed-kernel, marker-roster and build-support conventions. Root
must create/review its lockfile and exact vendor closure remotely before using
`--locked`; required Pliron revision is `7ebf6e6638c2a3bcec179423993b01211a9689b4`.
Use the existing resource-limited remote build wrapper for each phase:

```text
cargo test --offline --locked --manifest-path <proposal>/Cargo.toml
cargo clippy --offline --locked --manifest-path <proposal>/Cargo.toml --all-targets -- -D warnings
```

Three source-contract tests compare both signatures, all preloop guards and all
postloop reduction/store statements against the frozen V5 fixture; assert the
complete four-pair load/arithmetic statement sequence; and pin target/producer
requirements. Five independent host-model tests compare every FP32 update and
finite flag across all admitted reduction widths and lanes, cancellation,
nonfinite/product/partial overflow, poisoned inactive storage, and TP endpoint
address bounds. These tests do not run the compiled kernel or emulate the GPU
reduction; equality is checked before the unchanged reduction terminal.

For emission, adapt the already bounded `emit-v19-r1.sh`/successful V19 successor
without weakening tool, vendor, target, resource or exact-output-replay checks:
use crate `ferric_qwen3_tp_gemv_prefetch_kernels_device_v20`, this standalone
manifest, and a new output root. Retain source/tool hashes, logs and real
canonical observation/handoff identities. No existing artifact validator may
silently admit these roots as V5 or V17.

Specific risks must be resolved by real emission and native checks:

- `load_or` bounds/control flow may still serialize loads. Source ordering does
  not guarantee multiple outstanding VMEM operations. Inspect actual ISA before
  claiming that the intended scheduling change survived.
- Eight live raw values may increase VGPR pressure or spills and reduce
  occupancy. Record VGPR/SGPR/scratch/LDS metadata, instruction counts and waits.
- Four conditional tuple results in the partial root may expose a compiler
  SSA/range limitation. Retain any rejection; do not relax bounds, convergence,
  verifier, trapping or ownership gates to force emission.
- Confirm emitted arithmetic still uses the original multiply/add/check order,
  without contraction/reassociation, and the same convergent reduction. Source
  tests alone do not prove compiler or GPU arithmetic parity.
- Independently admit the actual ABI and both roots, then compare full output
  bits against V5 for finite/cancellation/boundary fixtures with guarded buffers.
  Trap cases need the existing supervised failure procedure, not a successful
  output comparison. Do not mix failed/trapping dispatches into latency results.
- Only after native parity may a distinct opt-in adapter/controller route be
  proposed. First preserve TP1/C1 and fallback behavior, unchanged 128/128 Qwen
  reference/output profile, independent producer labels and same-image control
  loading; measure isolated kernels and repeated matched end-to-end arms.

No formal verification claim or active source-coverage inventory is changed.
