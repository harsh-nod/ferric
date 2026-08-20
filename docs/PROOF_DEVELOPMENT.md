# Proof Development

This document defines the development and review protocol for Ferric's
correctness-critical code. The claim vocabulary and trusted boundaries are
defined in [ASSURANCE.md](ASSURANCE.md); this document defines how a change
earns one of those claims.

## Proof-Required Surface

A change is proof-required when it affects:

- executable request, scheduling, KV, rollback, sampling, or completion
  semantics;
- resource ownership, bounds, generations, epochs, or device-visible layouts;
- construction or validation of a kernel dispatch;
- correspondence between a Ferric state transition and a fe2o3 contract; or
- admission, identity, or evidence checks used by a proof-required bundle.

Rust's type system and `unsafe_code = "forbid"` remain mandatory, but do not
establish these functional properties. Tests and hardware comparisons are
additional validation, not substitutes for Verus proofs.

## Change Contract

Every proof-required issue and pull request records:

1. the exact theorem or invariant being established;
2. the executable implementation and abstract model it relates;
3. preconditions, postconditions, frame conditions, and preserved invariants;
4. trusted functions, axioms, external contracts, and other assumptions;
5. explicit non-claims and the last covered refinement boundary;
6. source, model, feature, target, schedule, Verus, solver, and dependency
   identities needed to reproduce the result; and
7. positive tests and negative mutations that would fail if the claimed
   property or its evidence binding were weakened.

An implementation without a closed proof must report the affected property as
`Contracted` or `Unsupported`. It cannot be admitted by a bundle that requires
that property to be `Proved`.

## Proof Layout

Proofs are organized by ownership boundary:

```text
crates/ferric-spec/src/    executable sequential models and direct proofs
crates/ferric-engine/src/  production transitions, specifications, and proofs
proofs/                    authenticated source, tool, and property admission
proofs/negative/           actual-body mutations the admission gate rejects
```

Correctness-critical transitions are executable functions inside the admitted
same-source Verus subset. Verus verifies those function bodies directly and
ordinary Cargo compiles the erased bodies. A separate model is permitted only
with an explicit, proved correspondence to the executable implementation;
source hashes, differential tests, or a second "production-shaped" model do
not establish that correspondence.

Strict proof jobs use a fresh, dedicated target directory because verifier
arguments are not part of Cargo's freshness fingerprint. They pass
`--no-cheating`, retain an identity-bound result, and build the exact release
artifact that will be qualified. A target directory previously used without
the strict arguments is not admissible evidence.

## Required M0 Theorems

The foundation milestone closes these obligations:

- a live `RequestId` designates exactly one current request generation;
- scheduler transitions are deterministic and preserve request isolation;
- each writable KV page has one live owner and initialized reads stay in
  bounds;
- sealed prefix pages are immutable and extension uses copy-on-write;
- tentative KV is unreachable from committed state until publication;
- rollback makes rejected KV unreachable without affecting other requests;
- submitted resources remain unavailable for reuse until their completion
  epoch is quiescent; and
- the public engine preserves scheduler/KV identity agreement and orders KV
  publication or detachment before scheduler reuse.

The system refinement theorem composes these component results. A component
proof may not silently assume another component's conclusion.

M0 has no device dispatch path. Refinement from an accepted engine dispatch to
an exact fe2o3 kernel contract is an M1 obligation and remains unsupported
until that path exists.

## fe2o3 Boundary

Ferric treats a fe2o3 kernel contract as an external lemma only when its exact
descriptor, schedule, numerical policy, source closure, proof receipt, and
toolchain identities match the generated plan. The integration proof must show
that Ferric constructs every contract precondition and correctly consumes the
postcondition.

This initially proves or validates behavior only through fe2o3's admitted
`gpu.*` representation. LLVM, object generation, HSACO, the driver, firmware,
and hardware retain the statuses and trusted-computing-base treatment declared
in [ASSURANCE.md](ASSURANCE.md). Verus success must not be presented as general
source-to-machine refinement.

## Agent Ownership

Each implementation agent owns one transition family and its local proof
obligations. The integration agent owns cross-module refinement and evidence
binding. Agents work in separate branches and worktrees; only the integrator
changes shared manifests, public module exports, policy documents, or the
pinned fe2o3 revision.

The author of a proof cannot be its only authority review. A reviewer checks
the theorem statement, assumptions, correspondence, negative mutations, and
claim boundary before integration.

## Merge Gate

A proof-required change is complete only when:

- production code and the executable oracle agree on positive and adversarial
  traces;
- state invariants pass after every public transition;
- required negative mutations fail for the intended reason;
- pinned Verus and solver executions close with `--no-cheating`, without `assume`, `admit`, an
  unrecorded `external_body`, timeout, or omitted dependency;
- the proof transcript and complete source closure are identity-bound;
- formatting, Clippy, Rust tests, and proof checks pass on the supported Rust
  version;
- exact-source host checks pass on `mi300x`; a `gfx942` behavior gate becomes
  mandatory when the qualified code contains a GPU execution path; and
- an independent review confirms the exact property status and non-claims.

Proof checks should fail closed when the pinned toolchain or authenticated
closure is absent. Any exception requires an assurance-policy change, not a
local bypass.
