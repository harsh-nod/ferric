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
and makes no correctness, benchmark, qualification, or M1-closure claim.

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
result. The R33 action frontend must connect to an independently supervised
service because the collector terminates every action process group. A stable
20-window service still requires a typed transition that retires the current
roster and re-leases queue-retained KV pages for the next roster without
reloading model allocations; the current controller does not claim that
transition exists.
