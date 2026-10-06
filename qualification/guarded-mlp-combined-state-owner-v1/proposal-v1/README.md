# Combined MLP State Owner: Source-Only Proposal

This is an unintegrated host-owner proposal, not a runnable guarded worker, a
qualified runtime, or GPU evidence. No author build, test, project import or
native execution was performed. The parent owns all integration and execution.

The measured predecessor failed with `FE2O3-RACE-002`: alias class 1 in Global
memory contains two distinct allocation origins and lacks relative base
offsets. The retained stderr is 123,593 bytes,
`b7c4435210a46414e49ed798624eff224c0436935a51d6715eccd8ee61c72f0a`;
the failed receipt is 19,105 bytes,
`1943926bd4d138d79505f6d0a4d97722d7efa54af6323cccab319be9d53c9140`.
The one-origin candidate and this owner are alternatives to broadening compiler
alias admission. No compiler predicate or capacity is changed here.

## Scope And Source Authority

`integration.patch` modifies three existing runtime files and adds two private
modules plus their two focused test modules. Every existing preimage and listed dependency is authenticated against
the retained RT5a `guarded-mlp-dependency-refresh-v1/attempt-v1/input-manifest.json`.
The exact source closure is in `source-manifest.json`. It does not apply the
earlier, unintegrated guarded-arena proposal. A future aggregate integration
must reconcile the two independent queue-first-close kind additions explicitly.

The additions are a private `CombinedMlpStateV1`, a borrowed
`CombinedMlpRegionsV1`, and private Linux atomic-memory operations. Registration
does not export a public allocation or dispatch API. No worker, profile,
resident state roster, artifact loader or existing device crate is changed.
The existing cleanup kind fixture is updated, and eight owner/six atomic-memory
tests are authored. None has run. Their coverage and remaining integration
obligations are distinguished in `TEST-PLAN.md`.

## Exact Storage And Binding Contract

The actual allocation request is 2,208 bytes, not a 2,192-byte request with
assumed page padding. The owner constructs 552 genuine `AtomicU32` objects once.
The first 548 words use the existing MLP profile's `INITIAL_STATE` and complete
548-word terminal predicate. The suffix has generation-low, generation-high,
verdict and reserved-zero in words 548, 549, 550 and 551.

Private preparation views allow only:

| Consumer | Owner relation | Byte region | Access | Pointer argument offset |
| --- | --- | --- | --- | --- |
| Existing MLP body through a future new adapter | Owner rank | 0..2192 | ReadWrite | 80 |
| New one-slice validator | Owner rank | 0..2208 | ReadWrite | 0 |
| R2 guard0 or guard1 | Owner or sole peer | 2192..2208 | Read | 64 or 80 |

The new prepare-time check enforces the same regions even if a private caller
manually constructs a pointer description. The old peer-read mapping check,
pointer extent/alignment checks and metadata access equality remain unchanged.
The whole backing must fit the peer GPU aperture. A physical mapping covers the
whole allocation; suffix-only access is enforced by typed argument admission,
not claimed as hardware suballocation isolation.

The opaque owner is not Clone and does not expose its buffer token. Borrowed
views do not create a `Gfx950EngineeringPeerWaveMlpTilesStateV2`. The old type
continues to require an exact 2,192-byte owner-only, non-peer-mapped record with
its old buffer kind. Its allocation, observer, reset, profile and public APIs
are untouched. A future new MLP command adapter must reuse the old profile's
metadata/argument/data-region checks with the new private prefix binding; it
must not spoof the old typed token or weaken its checks.

## Atomic Lifetime And Generation

Allocation starts at generation 1. This is a per-owner sequence, not an assumed
mapping to the older all-layer arena's global segment ledger; the new
coordinator must explicitly resolve that relationship before worker reuse.
Initial readback must equal the existing
548-word initial state and `[1, 0, Pending, 0]`. `Ready` permits preparation;
`submit` consumes it after fresh group checks and exact initial readback, before
either queue is published. All post-submit failures require terminal poison.

`complete_quiescent` is private unsafe and validates this owner's full prefix
and current Valid guard between existing group fences before marking Completed.
It does not establish the required paired completion proof. Its caller still
owes both published batches, ten actual completion signals, both current guards,
no fault/deadline violation, and absence of old queued/users' accesses.

Rearm requires Completed, exact next nonzero generation, the original terminal
snapshot matching a fresh full readback, and a healthy group fence. It enters
Rearming before the first store. The existing objects are atomically stored,
never reconstructed: verdict is cleared first, all prefix words restored from
the existing profile, both new tag halves/reserved written, and Pending stored
with Release. Full readback and a final fence precede Ready/new-generation
commit. Overflow, replay, bad phase, stale tag, invalid prefix, readback mismatch
or backend failure leaves the owner and group poisoned. A snapshot alone is
never completion, quiescence, ring-capacity or reuse authority.

All native initialization/observation/rearm methods use raw pointers to genuine
atomic objects without forming a mutable byte slice or `&mut AtomicU32` shared
with device users. The new device validator, separately authored, must retain
548 Acquire reads plus three Relaxed stores and one Release verdict store.
Host rearm stores are a different, strictly quiescent operation.

## Custody And Failure

Exactly two ranks are admitted. The group retains each allocation and mapping
before later aperture/map/initialization checks can fail. Partial mapping or
initialization is terminal; no error path retries initialization or releases
uncertain resources. Dropping the opaque owner does not release group custody.
Presence of this kind selects the existing queue-first Close protocol: both
queues are destroyed before any peer allocation is unmapped. A failed or
unclosed group keeps its existing quarantine/leak behavior, not ad-hoc cleanup.
Public ordinary read/write remains denied because the kind is not PublicVram.

## Remaining Gates

1. Root-run focused CPU fixtures and inherited runtime tests, including
   pointer-region negatives, generation/poison paths and queue-first failure
   ordering. This proposal has no CPU qualification receipt.
2. Candidate-v2 CPU tests and checked compiler lowering for both exports. One
   root is a source change, not proof of successful alias/bounds/race admission.
3. Actual emitted ABI and instructions. One shared slice plus two u32 values
   predicts four physical arguments/24 explicit bytes; 280 total assumes the
   existing 256-byte implicit tail and remains unobserved. R2's two shared
   guards are actual read-only accesses, but its metadata must accept Read
   fixups. A present ReadWrite `.access` still refuses a peer Read binding;
   `.actual_access` is not consumed by the current host descriptor.
4. A real paired guarded coordinator and owner-specific MLP adapter. The unsafe
   contracts here are obligations for that implementation, not its substitute.
5. Native positive and negative guarded runs, measured allocation/mapping
   census, reusable arena/generation tests, and then a distinct Ferric route.
   No inference-route, performance or allocation-count claim follows here.

See `INTEGRATION-MAP.md` for the canonical boundaries and the original CPU/GPU
acceptance gates. The old candidate and every failed evidence capsule remain
unchanged.
