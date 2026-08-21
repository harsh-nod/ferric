# Architecture

Ferric keeps the serving runtime small by moving model parsing, proof,
optimization, autotuning, and runner generation offline.

## Final Workspace Shape

```text
crates/ferric-spec       executable sequential semantics and Verus targets
crates/ferric-kernels    fe2o3 kernels, contracts, and finite schedules
crates/ferric-engine     scheduler, KV, speculation, and generated runner
crates/ferric-build      model admission, packing, planning, tuning, bundles
src/bin/ferricd.rs       thin unverified serving boundary
generated/               model/target-specific generated runner
proofs/                  system-level refinement proofs
benches/                 identity-bound qualification harnesses
```

`ferric-spec`, the first `ferric-engine` state model, and the pinned Qwen3
configuration, tokenizer-metadata, and streaming safetensors admission slices
of `ferric-build` exist today. Safetensors admission authenticates exact full
files and validates the closed Qwen3 BF16 tensor schema without buffering
tensor payloads. It does not yet semantically admit `tokenizer.json`, transform
or pack tensors, inspect tensor values, or load device memory. Crates are added
when they contain real final-path behavior; placeholder GPU execution is not
permitted.

## Dependency Boundary

The runtime may depend on the reviewed fe2o3 artifact, descriptor, host,
completion, and HSA APIs. Correctness-critical runtime crates use pinned vstd
macros at build time so Verus verifies their actual executable bodies. The
shipping artifact must not link Verus, vstd, Z3, or proof/ghost state. It also
must not depend on compiler crates, LLVM, COMGR, HIP launch APIs, vendor GEMM
libraries, PyTorch, Python, runtime JIT facilities, or the fe2o3 legacy
compiler.

The qualified release is produced by the strict `cargo-verus build --release`
invocation. Rebuilding the binary afterward with a different Cargo invocation
would create a new, unqualified artifact even when source files are unchanged.

The offline build owns:

```text
strict model admission
  -> canonical Model IR
  -> algorithm and execution planning
  -> fe2o3 kernel compilation and proof
  -> finite schedule qualification
  -> generated runner
  -> signed deployment bundle
```

## Generated Runtime

The generated runner embeds exact:

- target and draft model identities;
- tokenizer, vocabulary, and numerical policy identities;
- weight and workspace layouts;
- supported batch, context, and speculation buckets;
- kernel descriptors, proof requirements, and launch geometries;
- prefill, target decode, speculative, and sampling command graphs; and
- target, compiler, schedule, and complete plan identities.

At admission, Ferric authenticates the bundle and instantiates bounded command
templates. The generation loop patches only admitted addresses, lengths,
positions, page-table locations, and RNG counters.

## State Transition

The engine is the sole owner of mutable inference state:

```text
current state
  -> construct immutable StepPlan and reserved StateDelta
  -> reserve KV and workspace regions
  -> submit one command batch
  -> receive exact completion
  -> validate the compact result record
  -> atomically apply StateDelta
```

Cancellation changes logical request state but cannot release storage. Storage
is retired only after the last referencing device epoch is quiescent.

## Paged KV

The target and draft models use distinct page pools. A future sharing-capable
page has one of these states:

```text
Free
Exclusive { owner }
SealedPrefix { references }
Speculative { owner, branch }
Retired { after_epoch }
```

Required invariants include generational identity, initialized read ranges,
exclusive writes, immutable sharing, copy-on-write before extension,
commit-only publication, rollback unreachability, and quiescence before reuse.

The M0 metadata state machine supports exclusive writable pages, sealed sharing
of full committed prefix pages, and copy-on-write extension. It does not yet
allocate, initialize, or share physical GPU KV buffers; that refinement begins
with the device runtime milestone.

## Speculative Round

The intended generated device batch is:

```text
draft K tokens on device
  -> target verifies K tokens in one pass
  -> acceptance and residual-or-bonus sample
  -> compact commit record
```

Provisional KV remains private. Only a validated completion publishes the
accepted prefix and retires the rejected suffix. The host does not round-trip
once per draft token.

## Required fe2o3 Work

Reusable capabilities belong upstream in fe2o3:

- a connected production D1-D7 path for Ferric's closed Rust subset;
- parameterized LLM kernel families and proof-bound finite schedules;
- long-lived HSA queues, typed allocations, asynchronous copies, and bounded
  multi-packet command batches;
- generated multi-kernel dispatch and graph identity binding;
- runner-visible quiescence capabilities;
- numerical contracts for BF16, FP32, FP8, FP4, MFMA, exponentials, and
  reductions; and
- translation validators and machine-resource gates.

Ferric consumes these APIs. It does not duplicate them.
