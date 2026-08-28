# Ferric Qwen3 `SwiGLU` Worker V3 Envelope V2 Adapter

This standalone Ferric package is the authority-free adapter between fe2o3's
receipt-bearing Worker V3 load envelope V2 and Ferric's pending M1 `SwiGLU`
verifier request. It is outside the legacy root workspace and pins one exact,
current fe2o3 integration revision in `Cargo.toml`.

The raw-wire entry point strictly decodes V2 and returns only a replayable,
inert pending request. The recovered entry point consumes fe2o3's move-only
`RecoveredWorkerV3LoadEnvelopeV2` and retains it in a distinct non-cloneable
result. Validation failures return the owner with the error. Both paths project
every compiler-receipt identity from the single envelope-owned carriage. There
is no API that accepts caller-authored parallel identity fields.

Before producing a pending request, the adapter compares the envelope's exact
replay, claim, compiler closure, source evidence, compiler handoff, finalizer,
inspection, and output identities with the checked-in Ferric `SwiGLU` build. The
compiler handoff and inspection identities bind the ABI that produced the
exact artifact. A recovered owner receives an additional exact artifact-byte
check and envelope-readiness binding check.

The V2 owner does not authenticate repository commit labels or expose the
selected, host-admitted descriptor lineage. Those identities become available
only after the later fe2o3 host-admission transition consumes the recovered
owner. Accordingly, this slice checks the carried compiler closure, source
evidence, handoff, inspection, and artifact identities but makes no repository
revision or descriptor-source authentication claim. Static commit labels in
Ferric's pending request remain policy metadata, not owner-derived evidence.

The adapter does not verify the carried issuer policy against protected
configuration, enforce the rollback transition, authenticate compiler process
supervision, create a fe2o3 verifier decision, load an artifact, or launch a
kernel. Its result therefore grants no verifier, load, launch, KFD, HIP, or Qwen
inference authority. The qualification HIP harness remains separate under
`qualification/qwen3-swiglu-v1`.

Retaining a recovered owner does not make its publication permanently current.
The eventual consuming verifier/admission transition must revalidate durable
currentness and protected rollback state at its own authority boundary.
