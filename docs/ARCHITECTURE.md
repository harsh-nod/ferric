# Architecture

Ferric keeps the serving runtime small by moving model parsing, proof,
optimization, autotuning, and runner generation offline.

## Final Workspace Shape

```text
crates/ferric-spec       executable sequential semantics and Verus targets
crates/ferric-kernels    fe2o3 kernels, contracts, and finite schedules
crates/ferric-generated-runner checked-in inert runner declarations
crates/ferric-engine     scheduler, KV, speculation, and generated runner
crates/ferric-build      model admission, packing, planning, tuning, bundles
src/bin/ferricd.rs       thin unverified serving boundary
proofs/                  system-level refinement proofs
benches/                 identity-bound qualification harnesses
```

`ferric-spec`, the first `ferric-engine` state model, the finite structural
K1-K7 profile catalog in `ferric-kernels`, and the pinned Qwen3 configuration,
tokenizer, and streaming safetensors admission slices of `ferric-build` exist
today. Tokenizer admission authenticates the exact shared
payload and exhaustively binds its vocabulary IDs, merge order, processing
pipeline, added tokens, special tokens, and chat-template metadata. The sealed
authority now retains the admitted vocabulary and merge program for bounded,
deterministic UTF-8 encode and exact byte decode. The one authenticated Qwen3
program uses pinned Unicode 9 NFC tables and a private fixed Split regex over
bundled Oniguruma Unicode 16 tables; it does not expose a general tokenizer or
caller-selected regex path.
Safetensors admission authenticates exact full files and validates the closed
Qwen3 BF16 tensor schema without buffering tensor payloads. A fixed-width
canonical record revalidates all pinned and derived identities, but does not
sign the record or confer authentication authority for external files. These
slices do not establish full Hugging Face tokenizer equivalence, transform or
pack tensors, inspect tensor values, or load device memory. The structural
kernel catalog covers every
operation in the 22 exact B3 plans, but its reviewed upstream sources are
unmerged fixture/model foundations and profiles that exceed those exact
fixtures remain explicitly marked as required extensions. It grants no proof,
artifact, compilation, load, launch, dispatch, hardware, performance, or
qualification authority. Crates are added when they contain real final-path
behavior; placeholder GPU execution is not permitted.

The data-only `ferric-generated-runner` crate is the checked-in output of the
current deterministic Qwen3/gfx942 declaration generator. It names all 22
target-then-draft B3 selections, their exact operation offsets and counts, and
four logical scalar-input schemas. `ferric-build` expands those declarations
against authenticated admission, the sequential plan catalog, the structural
kernel catalog, and the preliminary identity closure. The resulting retained
record covers all 10,648 exact typed operations, including buffer kinds and
shapes, and regeneration is tested for byte equality. A non-clone publication
step revalidates and consumes that complete build-owned declaration before the
engine accepts logical custody. The current bounded bridge uses the acyclic
`ferric-engine -> ferric-build` dependency so no public caller-authored record
can mint equivalent custody. Engine lookup binds a request-local `StepPlan` to
an exact retained plan identity and independently bounds its operation span.
This is not a physical runtime: it contains no address, allocation, artifact,
loader, queue, packet, launch, completion, hardware observation,
graph-refinement proof, performance result, or qualification authority.

## Dependency Boundary

The runtime may depend on the reviewed fe2o3 artifact, descriptor, host,
completion, and HSA APIs. Correctness-critical runtime crates use pinned vstd
macros at build time so Verus verifies their actual executable bodies. The
shipping artifact must not link Verus, vstd, Z3, or proof/ghost state. It also
must not depend on compiler crates, LLVM, COMGR, HIP launch APIs, vendor GEMM
libraries, PyTorch, Python, runtime JIT facilities, or the fe2o3 legacy
compiler.

The authenticated tokenizer path has exactly two admitted crates.io roots: `onig`
6.5.3 with default features disabled and `unicode-normalization-alignments`
0.1.12. `proofs/RUNTIME_DEPENDENCY_TCB` binds their complete resolved package,
source, checksum, feature, target-kind, build-script, proc-macro, and edge
closure. The source gate regenerates that closure from full Cargo metadata and
the checksum-bearing format-4 lockfile, rejects every other registry root, and
binds the canonical records into its source inventory and dependency TCB.
Oniguruma C compilation and unsafe FFI and both libraries' Unicode tables are
contracted dependencies rather than verified Ferric code.

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

The final generated runtime is required to embed exact:

- target and draft model identities;
- tokenizer, vocabulary, and numerical policy identities;
- weight and workspace layouts;
- supported batch, context, and speculation buckets;
- kernel descriptors, proof requirements, and launch geometries;
- prefill, target decode, speculative, and sampling command graphs; and
- target, compiler, schedule, and complete plan identities.

The current generated declaration slice fixes 22 operation templates and four
logical scalar patch schemas, but it does not instantiate device command
packets, addresses, or runtime patch values. Those operations remain future
runtime and independent-validation work.

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
of full committed prefix pages, and copy-on-write extension. The M1 logical
foundation now adds an exact one-request, 16-token-page physical metadata
projection over the 8192-token envelope, including generational ownership,
initialized-prefix reads, accepted-prefix commit, rejected-tail rollback,
cancellation, retirement, and quiescence-gated reuse. A separate verified
32-slot interleaving model proves that one generational request transition
frames every other request and that cancellation cannot publish or resurrect
retiring work.

These are source-level sequential refinements. They do not allocate, address,
initialize, copy, or share GPU memory, produce quiescence from a device
completion, or connect the generated declarations to a queue. The production
multi-request device allocator and runtime composition remain open M1 work.

`ferric-engine::device_cache` now adds a single-request engine custody bridge
over that refinement: separate non-clone target/draft page-lease tables, an
exact pending-write/completion typestate, initialized-only mapping and commit,
rollback and cancellation retirement, and exact-epoch terminal quiescence.
Page-allocation and initialized-write authorities have no production
constructors because fe2o3 allocation authority and exact packet, buffer, and
KV-write-effect authority are still missing; scoped unit-test stand-ins do not
fill that gap. Rollback-retired pages from an earlier epoch require their own
exact completion settlement before later cancellation can become terminal.
The quiescent state exposes no release or reuse operation until fe2o3 provides
the corresponding physical leases. This bridge does not implement the
still-open multi-request device allocator, physical runner, hardware
initialization, or M1 path obligation. It also consumes exact-completion
authority directly; the future physical runner must compose and fan out one
ordered queue completion into scheduler, KV, and resource permits without
duplicating that linear authority.

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
