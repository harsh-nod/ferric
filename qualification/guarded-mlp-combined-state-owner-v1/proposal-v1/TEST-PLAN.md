# Authored Tests And Root-Owned Qualification

This proposal authors eight owner and six memory tests, with names recorded in
the source manifest. None is executed. The existing queue-first-close kind
fixture is also updated without renaming it. Root owns integration, additional
coordinator tests and all execution. Authored coverage is not a passing outcome.

The owner tests call the actual pointer-region check and shared production
activation/snapshot/generation predicates. The exhaustive pointer matrix has
2,880 combinations and exactly six admitted combinations. Native `submit` is
called only on rejected phases, before any context operation; the actual
`outcome` method is tested for poison/custody behavior using a hardware-free
group shell. No test invokes unsafe native complete/rearm with fake paired
completion proof. Both native success paths remain unqualified.

The memory tests use fresh naturally aligned storage, create552 real atomics,
and perform isolated atomic reads/stores/reset. Ordinary byte reads cover only
the tail after construction, or the whole fresh storage after refusal before
construction. They do not create a mutable byte slice over live atomics.

## Exact Storage And Atomic Lifetime

- Fresh2208-byte request creates exactly552 genuine AtomicU32 values; prefix
  equals the existing profile INITIAL_STATE, suffix equals [1,0,0,0], and page
  tail beyond2208 is unchanged. Reject2192,2207,2209,4096,0 and usizeMAX requests,
  short/inactive/inaccessible/unaligned mappings before stores.
- Exercise real atomics through terminal writes and quiescent rearm. Read
  verdict Acquire before tags; no mutable ordinary byte slice spans any atomic.
  Reset clears verdict before prefix/tag stores and never reconstructs objects.
  Test low/high generation boundaries and exact reserved0.
- Authored cross-generation checks prove the old memory helpers refuse2208 and
  the old state pointer/profile extents still equal2192. Retain all old2192
  tests, including exact token, no-peer-map requirement,
  wrong requested extent, page-tail preservation and activation/refusal tests.

## Provenance And Peer Policy

- New owner kind/request/token/group/local allocation must match. Reject wrong
  owner, group, id, missing allocation, changed request, mapping phase/count,
  unexpected/missing peer GPU, and aperture overflow. Two ranks only.
- Authored pure pointer helper tests cover owner-prefix(80,0,2192,RW),
  owner-validator(0,0,2208,RW), local/peer suffix(64or80,2192,16,Read).
  Reject peer prefix/full RW, peer prefix Read, suffix RW/Write, wrong argument
  offset, empty/shifted/oversized extent, and out-of-world rank. Exercise the
  actual prepare hook additionally in integration, not only the convenience
  view and invoked region check.
- A present ReadWrite metadata annotation still rejects peer Read. Preserve
  alignment, extent, zero pointer-slot and overlapping mutable-argument guards.
- Additional compile-fail/visibility tests must establish no old StateV2 conversion, generic
  buffer exposure, Clone owner or public callable allocation/dispatch API.

## Lifecycle, Failure And Custody

- Initial readback is required before Ready. Submit is single-use and refuses
  any prefix/tag/verdict change. Repeated submit, completion before submit,
  rearm before completion, zero/replay/skip/overflow generations are terminal.
- Actual terminal prefix and current Valid guard both required. Pending,
  Invalid, stale high/low, nonzero reserved and any invalid548 word refuse.
- Native rearm must reread and match the expected terminal snapshot. Enter Rearming
  before first mutation; injected failures at every store/readback/fence point
  poison owner AND group. Never publish Ready or a new generation after error.
  Current tests exercise shared predicates and actual poison handling, not
  injected failures inside a real mapped native transaction.
- A forged data snapshot alone cannot demonstrate coordinator completion.
  The future coordinator needs fake/native backend tests proving it calls the
  unsafe owner gates only after both published batches, ten actual signals,
  both current guards and all deadline/fault/quiescence checks. Owner CPU
  fixtures must not claim to prove those absent coordinator transitions.
- Allocation failures after backing retention, aperture check, partial peer
  map, initialization and final fence preserve group custody. No individual
  owner Drop frees memory; failed/unclosed group retains quarantine behavior.
  Verify both queues destroyed before the first peer unmap on clean Close,
  plus failure on each destroy/unmap/release stage without unsafe retry.

## Separate Integration Gates

The future new MLP adapter must bind this prefix without constructing the old
StateV2. It must preserve old profile metadata/data-region/paired overlap
checks. Candidate-v2 lowering and emitted metadata are independent gates.
The paired guarded coordinator, new artifact loader, typed Ferric roster,
generation ledger, real native counter census and final numerical comparisons
are not implemented here and cannot be certified by this owner test cohort.
