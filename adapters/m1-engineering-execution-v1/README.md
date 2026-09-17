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

`ferric-m1-engineering-speculative-smoke` accepts an explicit leading
`--mfma13` to admit an `AttributedMfma13` engineering artifact. Omitting it
preserves the `LegacyScalar12` route. Both one-step and resident reports bind
`program_strategy` and `program_count` to the admitted artifact, not just the
command-line flag. This remains gfx942-only, authority-free engineering
execution, not protected publication or performance qualification.

```sh
ferric-m1-engineering-speculative-smoke [--mfma13] \
  PREPACKED-SNAPSHOT ENGINEERING-OBSERVATION-DIRECTORY \
  GPU-UNIQUE-ID RAW-PROMPT [RESIDENT-MAX-NEW-TOKENS:1..32]
```

For engineering startup diagnosis only, set
`FERRIC_M1_ENGINEERING_STARTUP_PHASE_DIAGNOSTICS_V1=1` when invoking either
smoke binary. The opt-in writes fixed, cumulative `std::time::Instant` phase
completion records to stderr. They carry authority `none`, are not evidence or
benchmark-comparable timing, contain no prompt/model data, and do not change
the stdout observation schema. Unset or any value other than the exact literal
`1` disables them.

For a direct authority-free smoke, the identity-closure argument may be the
reserved literal `@derive-engineering-identities-v1`. That mode derives inert,
nonzero preliminary runner inputs from the exact admitted engineering
observation, model admission, prepacked manifests, deployment bundle, and plan
catalog. Its V3 report records
`identity_input_mode=derived-engineering-observation-model-plan-v1`. These
derived identities authenticate no source, compiler, proof, ABI, validator,
TCB, or protocol and cannot replace a protected receipt or qualification
closure. A file path still selects the existing external-closure mode and its
unchanged V2 report shape.

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
The coordinator exposes a terminal proof only after all 20 successful ordered
measurement responses and the normal stop response have been digest-acknowledged.
An exact replay may recover a lost normal-stop acknowledgement, but cleanup-stop
or fault-stop replay cannot create that proof.

This foundation grants no compiler authentication, publication, load, queue,
allocation, or launch authority. Those capabilities must be supplied by a
separate protected owner. It also does not provide HTTP, unrestricted
continuous batching, or late arrival.

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

This ownership join is intentionally not serving-complete. The legacy
`new_with_s1_t128_prefill_bootstrap` constructor binds one exact canonical
S1/T128 row, consumes the active authenticated runner, initialized model/KV
pool, fresh Engine, pretokenized prompt, and paired workspace plans, and reaches
authenticated prepublication. It retains the queue-ready batch plus successor
KV pages and rollover intent, then returns
`authenticated-window-execution-unavailable` and transitions the exact instance
to `Faulted`. The default constructor still returns
`authenticated-window-bootstrap-unavailable`.

The opt-in `new_with_s1_t128_target_window` constructor additionally requires
all target-only workspace plans and queue bounds up front. It executes exactly
one bound row through the authenticated queue, derives tokens only from checked
device completions, settles Engine, registry, KV-page, and queue custody, and
constructs one report from `CLOCK_MONOTONIC_RAW` offsets measured from the
accepted `measure` boundary. Success retains terminal execution custody and
enters `Faulted`; a second window is rejected. It therefore cannot satisfy the
required 20-window R33 run and is not a qualification result.

`new_with_s1_k4_resident_windows` consumes the complete ordered twenty-window
roster and retains the authenticated instance across completed windows. The
finite-plan constructor `new_with_s1_finite_resident_windows` additionally
accepts S1/K8 or S1/K16, with one exact successor selection shared by every
window. Each window starts with S1/T128 paired prefill. Its bootstrap and
resident input constructors must explicitly select the same successor; the
legacy constructors remain K4-only. Queue slots, packet lowering, readback and
KV tail storage are prepared for that selected plan before measurement.

The resident path currently fails closed if another round would use a common
anchor while the draft KV cursor trails the target after full acceptance.
An authenticated draft catch-up dispatch is still required for that case;
changing the host cursor would not populate the missing KV token. A terminal
full-accept round does not need another proposal round. This finite source
extension is not S8 support, continuous admission, physical qualification or
a measured speculative speedup.

None of these authenticated constructors routes through the structural
physical runner. The legacy prepublication-only constructor observes no clock
or token and never constructs a measurement report.

## Eight-GPU Engineering Execution

The optional `tp-engineering` feature builds `ferric-qwen3-tp-engineering`.
It is a separate engineering command, not an HTTP server or protected M1
admission. Ferric authenticates canonical model bytes, inspects the closed
fourteen-kernel TP artifact, and sends owned bytes to one isolated fe2o3 KFD
worker per distinct physical gfx950 device. No unsafe Rust is added to Ferric.
The operator must explicitly trust the machine code through
`--allow-unauthenticated-machine-code`; structural inspection alone is not a
source-to-device correctness proof.

```sh
ferric-qwen3-tp-engineering \
  --source /private/models \
  --artifact /private/output/fe2o3-engineering-v1/CONTENT_ID \
  --worker /private/fe2o3-gfx950-engineering-worker \
  --devices ID0,ID1,ID2,ID3,ID4,ID5,ID6,ID7 \
  --allow-unauthenticated-machine-code \
  --prompt 'The capital of France is' \
  --new-tokens 32 --repetitions 3 --warmup 1 --capacity 128
```

Supported world sizes are exactly 1, 2, and 8. Q/K/V, attention, and MLP
projections run on rank-local GPUs. O/down outputs are FP32 partials reduced
in rank order on the host, with one residual addition and BF16 rounding.
Embedding and final logits run on rank zero. This changes floating-point
grouping and is Contracted, not bitwise-equivalence or numerical qualification.
Prompt priming is token-at-a-time m=1 execution, not optimized batched prefill.
The current sequence owns contiguous rank-local KV, not a radix prefix cache.

JSONL setup records separate model intake and upload/setup from generation.
Each run records the raw prompt/output IDs, decoded output, controller-clock
TTFT, every decode interval, mean TPOT, and exact completed dispatch counts
for every rank. Warmup runs are explicitly marked. Fixed-length greedy output
does not stop early at EOS; raw decoded bytes are retained even if the final
token ends mid-UTF8. The command preflights all runs against the conservative
131,072-packet per-rank ring limit; queue rollover is not implemented here.
Timings include IPC and host collectives and must
not be presented as a controlled vLLM/SGLang comparison. A final close record
is emitted only after every worker reports successful teardown and exits.
Timeouts or malformed responses poison the instance; they cannot count as a
successful measurement. Binary/artifact identities are observations, not
publication or launch authority. Actual hardware results are tracked in
`docs/M1_TEAM_PROGRESS.md`.
