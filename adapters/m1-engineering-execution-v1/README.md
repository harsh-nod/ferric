# Ferric M1 engineering observation adapter

This standalone crate admits an exact non-authoritative
`cargo fe2o3 engineering hsaco` observation. It validates the canonical JSON,
content directory, finalized gfx942/COV6 object, current aggregate descriptor
roster, generic loader plan, and Ferric's twelve program symbols and ABIs.

The adapter is deliberately outside Ferric's verified production workspace.
It depends one-way on `ferric-engine` and the internal move-only source-custody
crate. It cannot authenticate compiler origin, select a Worker V3 publication,
or grant production load or launch authority.

`ferric-m1-engineering-target-smoke` is the explicit diagnostic execution
boundary. It accepts a canonical prepacked snapshot, engineering observation
directory, identity closure, GPU unique ID, generation bound, and raw prompt.
Its JSON output retains authority `none`, identifies every admitted artifact,
and makes no correctness, benchmark, qualification, or M1-closure claim. The
V2 observation also reports the target-smoke controller's monotonic-raw
single-request timing with the production target-smoke field names. That timing
starts after artifact, model-memory, and tokenizer setup and is explicitly not
comparable to R33 serving, vLLM, or SGLang measurements.

## Bounded R33 lifecycle controller

The library also exposes an additive, in-process controller for one R33
measurement window. It preadmits at most 32 canonical pretokenized requests
into Ferric's real `Engine` and serving registry, retains their exact prompt
tokens for physical input construction, binds the same live Engine to
`M1ServingPhysicalRunnerOperationsV1`, and derives output callbacks from
Ferric's checked completion records. Arrival, first-token, and terminal offsets
use `CLOCK_MONOTONIC_RAW`; no delay or fabricated timing source is accepted.

This is a bounded preadmitted window, not unrestricted continuous admission.
It has no HTTP surface and is not by itself an R33 process adapter or benchmark
result.

## R33 supervised-service foundation

`ferric-m1-r33-adapter` is the short-lived collector frontend. It never starts
or owns the service process. It loads a canonical held service plan and exact
pretokenized 60-row workload, captures the collector's reserved environment,
and performs one bounded canonical request/response exchange with an externally
supervised Unix-domain service. The transport checks `SO_PEERCRED`, service-plan
identity frozen in the collector environment, service, hardware slot, instance,
action, and row bindings. Frames
have fixed magic/version/kind/length fields, a SHA-256 payload identity, an
8 MiB limit, exact canonical ASCII JSON, and exact EOF requirements. The daemon
commits a response-visible transition only after the frontend validates the
complete response and returns its digest-bound acknowledgement.

The daemon-side coordinator enforces one `start`, one `ready`, exactly 20
ordered preadmitted windows, and one `stop` per backend instance. The backend
instance remains live across all action processes and windows. A backend error,
timeout, or abandoned post-mutation response faults the instance; only its exact
`stop` binding is then admitted. External supervision is intentionally outside
collector authority because the collector kills every action process group.

This foundation grants no compiler authentication, publication, load, queue,
allocation, or launch authority. Its sealed backend is not yet joined to the
authenticated target executor or the queue's new-window recycle transition, so
it cannot truthfully serve hardware measurements yet. It also does not provide
HTTP, unrestricted continuous batching, or late arrival.

## Authenticated production ownership join

`M1R33AuthenticatedProductionBackendV1` is the fail-closed daemon-side owner
for capabilities prepared outside the authority-free collector transport. Its
constructor consumes an already-authenticated `M1AuthenticatedPhysicalRunnerV1`
and an already-initialized `M1PartitionedModelMemoryKvPoolV1`; it accepts no
artifact path, engineering aggregate, verifier callback, raw KFD handle, or
measurement callback. An exact `start` constructs a fresh 32-slot Ferric
`Engine` and moves the owners from `Dormant` to `Active`. The exact owner set is
retained through `Faulted`, and only a matching, in-budget `stop` drops it and
enters `Stopped`.

This ownership join is intentionally not serving-complete. The opt-in
`new_with_s1_t128_prefill_bootstrap` constructor binds one exact canonical
S1/T128 row, consumes the active authenticated runner, initialized model/KV
pool, fresh Engine, pretokenized prompt, and paired workspace plans, and reaches
authenticated prepublication. It retains the queue-ready batch plus successor
KV pages and rollover intent, then returns
`authenticated-window-execution-unavailable` and transitions the exact instance
to `Faulted`. The default constructor still returns
`authenticated-window-bootstrap-unavailable`.

The next required slice must consume that prepublication owner through
authenticated queue creation, submission, a deadline-bounded wait, checked
direct completion, Engine/KV settlement, and prefill-page release. The current
backend performs none of those effects, never routes through the structural
physical runner, observes no clock or token, and never constructs a measurement
report.
