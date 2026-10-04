# Independent Four-Forward Observation

This engineering controller selects the independently compiled V7 prefix image
with Ferric's existing CPU633 parent and CPU475 worker. It retains every layer's
hidden state, final normalization and logits across four forwards, then checks
capture structure, ownership, Close and host-policy diagnostics. It does not
accept whole-model numerics or performance. Old paired-bitwise tests and frozen
controllers remain unchanged.

Root has reviewed the new routing and frozen the 21-file package for an actual
84-test MI350 CPU qualification. The manifest and separate test receipt identify
the exact executed sources. The authored INTAKE and RUN notes describe their
pre-qualification state; a manifest alone is not a test result or GPU result.

The intake authenticates the unchanged parent/worker deployment and separately
verifies V7 provenance, six actual GPU prefix cases, and their six independent
conditional numerical results. It replays custody checks, not the numerical
computation. Root supplies a fresh request and scoped reviews. Three pre-audits,
one native attempt, three post-audits, finite resource limits, full capture
retention and owned-process cleanup are mandatory. No retry is automatic.

The four-forward test starts at position zero. It does not execute the target
2,048-token prefill and 256-token decode workload. A structurally complete run
must be followed by separate independent tensor diagnostics. Finite outputs or
matching argmax tokens are not whole-model correctness, and nested host timings
are not GPU timings or steady-state token throughput.

See INTAKE.md for exact plan/review schemas and RUN.md for the execution contract.
