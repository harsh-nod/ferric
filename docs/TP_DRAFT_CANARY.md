# Standalone Draft Numerical Canary

`ferric-qwen3-draft-canary` is a separately opted-in engineering executable for
the authenticated canonical Qwen3-0.6B draft. The target CLI, target-only default
intake, existing kernel bodies, scheduler and fast profiles are unchanged.

## Fixed Envelope

- Exactly one explicitly named physical GPU, the existing closed 13-root TP-v1
  image, and the baseline BF16 single-sequence driver.
- Five tokenized prompt inputs and two greedy outputs. Five prompt steps and
  one generated-token step execute exactly six forwards, 424 packets each,
  totaling 2,544 completed dispatches. The last generated output is not fed back.
- Capacity 32, hidden 1024, intermediate 3072, 28 layers, 16 query heads, eight
  KV heads, head width 128, vocabulary 151936 and tied embedding/head weights.
- No batched prefill, MFMA, FP32 head, wave attention, operational/cache runtime
  flags, sequences, ordered batches, peer transport, prefix cache, replication,
  serving or speculative verification/settlement.
- No TTFT, TPOT or throughput fields. Every record states `authority: none`
  and `performance_qualified: false`.

The caller uses `EngineeringQwenModelV1::open_with_draft`. The borrowed draft
configuration, weight bytes and authenticated layout feed the unchanged
`EngineeringTpExecutionV1::new`. Before a worker is spawned, the canary validates
the exact draft role/geometry/payload length/section count and checks both the
authenticated digests and complete bytes of the tied embedding/head sections.
The driver then independently checks every role-local tensor digest and shard
range before uploading any weights.

The intake still retains both target and draft CPU payloads: 16,381,470,720 and
1,503,264,768 bytes respectively, about 17.9 GB combined payload, excluding
allocator/tokenizer/staging overhead. Reserve at least 24 GiB of available host
RAM for an initial isolated run. Only draft weights are uploaded; capacity-32
draft KV payload is 3,670,016 bytes. Model weights and ordinary workspaces are
additional GPU allocations. No memory or numerical qualification is inferred
from this arithmetic.

## Build And Run

All build/test work belongs on mi300x in a private owned stage. The binary uses
the existing `tp-batch-engineering` build feature because the standalone
`tp-engineering` feature has an independently recorded, preexisting optional
numerical-module import problem; this canary does not enable a batched runtime
profile or change that unrelated feature boundary.

```bash
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
export CARGO_PROFILE_RELEASE_DEBUG=0
export RUSTC_BOOTSTRAP=fe2o3_macros,fe2o3_device
cargo +1.97.1 build --locked --release \
  --manifest-path adapters/m1-engineering-execution-v1/Cargo.toml \
  --features tp-batch-engineering --bin ferric-qwen3-draft-canary
```

Only the integration lead schedules native execution. Before launch, freeze
the controller/worker, exact image/manifest/handoff, canonical source, prompt,
optional reference, idle roster and available host/GPU memory. Use the existing
owned-process timeout/reap and before/after all-eight-GPU idle wrapper. Example
arguments, with externally established values, are:

```bash
ferric-qwen3-draft-canary \
  --source "$CANONICAL_SOURCE" --artifact "$TP_V1_IMAGE_DIRECTORY" \
  --worker "$EXACT_WORKER" --worker-sha256 "$EXACT_WORKER_SHA256" \
  --device-unique-id "$PHYSICAL_UNIQUE_ID" \
  --allow-unauthenticated-machine-code \
  --prompt 'The capital of France is'
```

The prompt must tokenize to exactly five IDs under the authenticated shared
tokenizer. Unknown/duplicate options, multiple-device syntax, zero device IDs,
noncanonical hashes, a different token count and broader execution flags are
rejected. The worker's running `/proc/PID/exe` digest must match the externally
required file digest. Runtime/image admission remains independent; an old
emitted image never acquires new compiler provenance through a new controller.

## Independent Reference

Without a reference, successful execution reports `reference_passed: null`.
That is **not** a numerical pass. Expected draft tokens must not be copied from
the target model or manufactured from this candidate's output.

An independent reference can be supplied with both `--reference FILE` and
`--reference-sha256 HEX`. The exact file is bounded to 64 KiB, read once, and
SHA-256 checked before parsing or model setup. Its closed JSON schema is:

```text
schema: "FerricDraftCanaryReferenceV1"
model: canonical DRAFT_REPOSITORY
model_revision: canonical DRAFT_REVISION
identity:
  model_bundle_id: 64 lowercase hexadecimal characters
  draft_model_id: 64 lowercase hexadecimal characters
  draft_config_id: 64 lowercase hexadecimal characters
  draft_weights_sha256: SHA-256 of the full canonical header-free draft payload
prompt_tokens: exactly five u32 token IDs
generated_tokens: exactly two u32 token IDs
generated_utf8_bytes: exact decoded output bytes
producer: nonempty description of the independently frozen reference producer
```

All identities and prompt IDs must match this authenticated run before worker
spawn. Every token must be in the canonical vocabulary. Unknown fields, wrong
roles/revisions, changed reference bytes, output token order or decoded bytes
are rejected without relaxing numerical comparisons. Synthetic host-test token
IDs are protocol fixtures only, never a canonical reference.

## Receipt And Failure Rules

The raw stream has distinct `FerricDraftCanarySetupV1`,
`FerricDraftCanaryObservationV1` and `FerricDraftCanaryClosedV1` schemas. Setup
records exact model/payload/executable/image identities and the fixed execution
policy. Observation records all six input/choice/position/cumulative-packet
steps, both output IDs and bytes, and the independent-reference status.

The caller must require exit zero, the successful final Closed record, exact
2,544 completed packets, position six, expected PID, clean process reaping and
external idle/ownership receipts. Observation is emitted before teardown so a
reference mismatch remains inspectable; it cannot alone establish completion.
A mismatch or failed close returns nonzero and emits no successful Closed.
Driver failures retain existing poison/no-retry behavior; the canary always
attempts close after entering the resident execution scope. Worker Drop remains
the existing last-resort owned-child cleanup on earlier failures.

## Kernel And Reference Availability

The retained engineering archive `ferric-tp-kernels-evidence-8cdde149` contains
the original gfx950 TP-v1 image:

- Source: `8cdde149643446b20730bc60206669c5bab1ca8d`.
- Compiler and SDK: `3546d54d2c4a913f5d079701aed557d0a378bba8`.
- Image: `7c0b1934a27569a97cf535c96a8a56dd57babb63a6d1becad1c7be3a3d26edec`.
- Observation: `f6b652b203ce8f01062e4709d41029c4b20b845dcdc07218ca12497d11acf98a`.
- Handoff: `220091f4a7cd2f31758592a06efb1683eb9a63fa65675e146811ab2d813fd6ef`.

Its TP-v1 bodies and four imported helper modules have no source diff through
the canary's base `2ce566d`. An explicit CPU-only image admission test takes
`FERRIC_DRAFT_TP_V1_ARTIFACT`; native qualification still belongs to root. No
current compiler emission or old-image relabel is implied. The inspected local
reference archives did not establish a standalone canonical draft five-input,
two-output numerical reference. The initial actual-model run and independent
reference gate therefore remain external prerequisites, not host-test results.

Host tests cover fixed scope, role/geometry/length/tied-byte rejections,
reference hash/schema/identity/output checks, actual driver six-step routing,
poison/close paths and the existing CPU fake-worker protocol tests. Driver tests
use synthetic outputs and small placeholder weights; they are neither a GPU
emulator nor a successful full-model intake test. No M1, Verus, serving,
speculative acceptance or performance qualification is added.
