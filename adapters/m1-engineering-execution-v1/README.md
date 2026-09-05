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
