# Ferric Qwen3 Aggregate Worker V3 Verifier V1

This standalone Ferric package marks the protected-verifier boundary for the
single 12-marker `M1AllKernelsWorkerV3RosterV1`. The current production backend
is intentionally fail-closed because Ferric does not yet have an independently
authenticated protected-verification receipt covering every aggregate roster
entry and its exact executable.

Every verifier call returns `MissingProtectedVerificationReceipt`. The adapter
does not construct fe2o3 verification evidence, enable fe2o3's synthetic test
support, and does not accept hashes as a substitute for protected proof, finalizer,
compiler-execution, source/target custody, layout, effect, or executable
verification.

This scaffold grants no verification, load, launch, or inference authority. It
has no direct KFD, HSA, HIP, engine, or model import and invokes none of those
surfaces. Its `fe2o3-host` dependency has a broader resolved runtime closure;
that transitive closure does not grant this adapter runtime authority. A future
implementation must replace the unconditional error only when a reviewed
protected backend can satisfy every obligation of fe2o3's unsafe aggregate
Worker V3 verifier trait.
