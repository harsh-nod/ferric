# V13 Integer Proof Candidate

Source-only, unverified proof module. No Verus, build, test or formatter has run
for this candidate. It is not wired into Cargo, runtime certificates, proof
inventories, source gates or existing M1 theorem admission.

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

## Proposed Remote Verification

Only after root reviews the exact source and approves a bounded private
mi300x stage, verify using the existing pinned Verus distribution from
`proofs/verus/VERUS_VERSION` (`0.2026.08.02.b677dd5`) and its unchanged complete
closure/SHA manifests. Do not substitute the newer local Verus installation.
No Cargo compilation or dependency installation is needed for this vstd-only
standalone source. A proposed command inside that stage is:

```sh
VERUS_Z3_PATH="$VERUS_ROOT/z3" timeout --signal=TERM --kill-after=5s 300s \
  "$VERUS_ROOT/verus" --crate-type lib --edition=2024 --no-cheating --output-json \
  proofs/m1/sharded_argmax_v13.rs
```

Retain exact source/tool-closure hashes, raw status and JSON for all 16 proof
functions (15 public obligations and one private fixed-division helper).
Any solver or syntax failure must be retained before a reviewed correction.
A successful future run would establish only these integer-model obligations;
it must not be reported as kernel, compiler, runtime, numerical or M1 admission.
