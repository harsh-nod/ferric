# M0 Property Contract

This document defines the claims that the M0 qualification must emit. It is a
qualification input, not evidence that M0 has passed. A property acquires the
listed status only when the exact source, statement, proof result, negative
mutations, tool closure, and release artifacts are bound in a closed property
contract.

Ferric uses the status and property vocabulary from
`fe2o3-proof-contracts`. Statuses are independent: a `Proved` source theorem
does not promote a `Contracted` completion premise or an `Unsupported` machine
refinement claim.

## Required Records

| Property | fe2o3 kind | Required status | Exact M0 statement |
| --- | --- | --- | --- |
| `m0.request_generation` | `GenerationSafety` | `Proved` | A live `RequestId` names exactly one current slot generation; stale or exhausted generations are never admitted as live. |
| `m0.greedy_speculation` | `FunctionalCorrectness` | `Proved` | For valid target choices, the executable greedy round emits the maximal accepted draft prefix followed by the target correction or bonus; invalid lengths return the specified error. |
| `m0.scheduler_transition` | `FunctionalCorrectness` | `Proved` | Every successful scheduler body implements its stated deterministic transition and frame; every rejection is the exact enabledness failure and preserves state. |
| `m0.scheduler_lifetime` | `LeaseSafety` | `Proved` | Dispatch members remain unavailable for reuse through exact completion, KV finalization or detachment, and generation advance. Cancellation never makes executing storage reusable. |
| `m0.scheduler_bounds` | `ResourceBounds` | `Proved` | For `0 < C <= 32`, all scheduler metadata and rings remain within `C`; dispatch scans at most `C` slots and other transition work is bounded by the affected batch or ring operation. |
| `m0.kv_transition` | `FunctionalCorrectness` | `Proved` | Successful KV bodies implement their exact enabled transition and frames; failed transitions preserve the pool. Reads are admitted only inside the logically initialized range. |
| `m0.kv_sharing_rollback` | `FunctionalCorrectness` | `Proved` | Only full committed pages are shared, shared pages are sealed and payload-immutable, extension allocates a fresh writable page, and rejected tentative suffix pages become unreachable without changing other requests. |
| `m0.kv_generation` | `GenerationSafety` | `Proved` | A sole-reference reclaim advances the page generation before reuse, request detach advances the request generation, and stale page or request identities are rejected. |
| `m0.kv_bounds` | `ResourceBounds` | `Proved` | Page, request, and page-table lengths remain at their generated construction bounds; live chains, free metadata, token ranges, and reference masks remain within those bounds. |
| `m0.engine_composition` | `FunctionalCorrectness` | `Proved` | The public engine preserves exact scheduler/KV request-generation agreement, publishes completed KV before redispatch, and detaches quiescent KV before returning a slot to the free ring. |
| `m0.hsa_exact_completion` | `SynchronizationSafety` | `Contracted` | An `ExactCompletion` is constructed only after the external ordered HSA authority establishes quiescence for that exact epoch and every retained resource. |
| `m0.proof_erasure` | `ProofErasureCorrespondence` | `Checked` | The qualified release is the artifact emitted by the strict same-source `cargo-verus build --release`; the default erasure checks completed for the authenticated source and tool closure. |
| `m0.no_transition_allocation` | `ResourceBounds` | `Checked` | The admitted structural check rejects its enumerated allocation and growth constructs outside exact constructors, and capacity tests observe unchanged metadata storage; this is not a general allocation-effect proof. |
| `m0.device_kv_initialization` | `MemorySafety` | `Unsupported` | M0 models initialized KV ranges but contains no device allocation, device write, or kernel dispatch, so it makes no physical device-memory initialization claim. |
| `m0.machine_refinement` | extension `machine_refined` | `Unsupported` | M0 has no Rust-to-LLVM, object, AMDGPU ISA, driver, firmware, or hardware semantic refinement evidence. |

## Evidence Binding

Each `Proved` record must bind:

- this exact statement and its compiler-reported executable function paths;
- the complete read-only source closure and same-source specifications;
- the structured whole-crate Verus result produced with `--no-cheating`;
- the authenticated Verus, `rust_verify`, Z3, vstd, Rust, and configuration
  identities used by the qualification;
- the exact negative mutations assigned to the property; and
- the qualified release artifact and the source-to-model correspondence
  witnessed by the executable body's requires and ensures clauses.

`Checked` records bind the exact checker, input, result, and limitations.
`Contracted` and `Unsupported` records bind their contract or rationale
artifact rather than citing the Verus transcript as authority.

The generated set must pass `ContractSetV1::validate_closed`. Structural
validation does not authenticate a digest or establish a theorem, so the
qualification entrypoint must calculate and authenticate every identity before
constructing the set.

The qualification builds `proofs/property-binder` against
`fe2o3-proof-contracts` at exact fe2o3 commit
`a6fa86b5ccf8f0438925cfec8f48a5d713874da3`. The binder checks this table
against `proofs/M0_PROPERTIES.json`, reconciles every `Proved` path and required
actual-body mutation, invokes `ContractSetV1::validate_closed`, and emits a
canonical property artifact. Its binary, source closure, lockfile, complete
dependency and build-script TCB, manifest, evidence index, and artifact are
measured in the durable qualification receipt. The fe2o3 validator supplies
structural validation only; Ferric's qualification supplies the measured
identity and evidence reconciliation, under the contracted ambient Rust and
qualification-host TCB.

## M0 Non-Claims

M0 does not contain an LLM model runner, GPU kernels, physical KV buffers, an
HSA queue implementation, or a serving API. Tests on `mi300x` validate the
portable Rust artifact on that host; they do not create a `gfx942` execution,
performance, secure-inference, or machine-refinement claim.
