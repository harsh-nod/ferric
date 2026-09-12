# V13 Integer Model Verification

The standalone integer model passed pinned Verus verification on mi300x at
source `ed2ceb502c539d00356213850cc32c023bf824c5`: 18 verified queries, zero
errors, whole-crate success and all 16 required proof-function records. It is
not wired into Cargo, runtime certificates, proof inventories, source gates or
existing M1 theorem admission. This is not verification of the actual GPU kernel.

The first attempt stopped before verification on an ambiguous integer literal.
The accepted correction only adds `int` suffixes to two literals; the original
failure is retained separately. The exact proof source SHA256 is
`1111950bab5b6613865be024b62cdda09a01b38833d443329322a84d88acc009`.
Raw successful JSON SHA256 is
`f1ec34e3d1e07973bc1f8ecfd3ff9c9b8462f013c2beafee504e415898ffda22`;
complete two-stream archive SHA256 is
`ce0feb569029a23933b744ab5b9c9e4316f57cbc691aa24ebd8c3c1355687702`.
All source/control/tool before-after checks and normal owned-group closure pass.

The arithmetic models the producer/finalizer indexing in the separately owned
v13 prototype `5cd315fc1e78a12c30a79c4e9d255373c315d181`, specifically its token
scan, six long shards, flattened scratch stores and integer key formula. This
is a mathematical model, not proof that the actual kernel refines it.

## Obligations

- Exact token bounds, inverse shard/lane/step mapping, coverage and uniqueness.
- The last-step guard, six long shards and 151936 reads per active row.
- Scratch and choice writer injectivity, active bounds and disjoint capacity tails.
- Decreasing `38 - step`, bounded induction and indices fitting a 32-bit unsigned
  range, hence also the declared 64-bit indexing domain.
- Integer key bounds, inverse and reversed ordering, without asserting FP32
  representation or reduction semantics.

The module follows `proofs/m1/kernel_contracts.rs` integer-spec/proof patterns
and uses only div/mod lemma names already used by Ferric. It has no `assume`,
`admit`, external proof body, or device/compiler/runtime premise that grants
execution authority. It proves neither FP32/NaN/subnormal behavior, subgroup
convergence, ABI, physical nonaliasing, initialized device reads, request
freshness, producer completion, memory ordering, GPU numerics nor performance.
The two dispatches still require separate admitted runtime ownership and a
successful producer completion before finalizer submission.

## Verification Boundary

The accepted bounded private mi300x gate used the existing Verus distribution from
`proofs/verus/VERUS_VERSION` (`0.2026.08.02.b677dd5`) and its unchanged complete
closure/SHA manifests. Do not substitute the newer local Verus installation.
No Cargo compilation or dependency installation ran for this vstd-only source.
The recorded command also fixed the Rust 1.97.1 sysroot, one verifier thread,
solver rlimit 30, private environment and resource/process-group bounds:

```sh
VERUS_Z3_PATH="$VERUS_ROOT/z3" timeout --signal=TERM --kill-after=5s 300s \
  "$VERUS_ROOT/verus" --crate-type lib --edition=2024 --no-cheating --output-json \
  --num-threads 1 --rlimit 30 \
  proofs/m1/sharded_argmax_v13.rs
```

Retain exact source/tool-closure hashes, raw status and JSON for all 16 proof
functions (15 public obligations and one private fixed-division helper).
Any solver or syntax failure must be retained before a reviewed correction.
The successful run establishes only these integer-model obligations; it must
not be reported as kernel, compiler, runtime, numerical or M1 admission.
