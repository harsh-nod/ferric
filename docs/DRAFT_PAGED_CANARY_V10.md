# Paged Draft06B Canary V10

This is a separate, non-authoritative model canary for the admitted Draft06B v10
paged driver. It does not generate speculative proposals, settle speculative KV,
publish protected output, or qualify performance. The existing contiguous BF16
draft canary and its reference remain unchanged.

## Fixed Invocations

The optional `tp-batch-engineering` binary is
`ferric-qwen3-draft-paged-canary`. Every invocation opens a fresh driver and pool,
encodes the literal `The capital of France is` into exactly five canonical tokens,
and generates exactly two tokens. Required arguments are:

```text
--source MODEL_BUNDLE
--artifact V10_CLOSED_IMAGE
--worker WORKER --worker-sha256 SHA256
--device-unique-id ID
--projection baseline|mfma
--prefill full|tokenwise
--reference REFERENCE_JSON --reference-sha256 SHA256
--allow-unauthenticated-machine-code
```

Only the existing `--runtime-cache-admission` and `--runtime-operational` flags
are optional. Other modes, sequences, ordered batches, rollover, captures, peer
transport, target profiles, and live serving are unavailable. The exact v10 image
and Draft06B identity are admitted before GPU allocations. The wrapper must pin
the controller, worker, artifact, model, reference and complete command line.

| Prefill | Ordered input positions per forward | Forwards | Dispatch packets |
| --- | --- | ---: | ---: |
| `full` | `[0,1,2,3,4]`, `[5]` | 2 | 960 |
| `tokenwise` | `[0]`, `[1]`, `[2]`, `[3]`, `[4]`, `[5]` | 6 | 2880 |

Every forward selects its last row for the FP32 head. Tokenwise prompt choices
before position 4 are checked but not generated outputs. Position 5 consumes the
completed choice from position 4; the second generated choice is not resident.
Each forward is 28 layers times 17 dispatches plus embedding and three final
head dispatches, or 480 packets. Two repetitions of each projection/prefill
profile are **separate processes**, not one long-lived pool.

The TP1 pool has row capacity 32, context limit 32 and two physical pages. KV
payload is 3,670,016 bytes and FP32 workspace is 19,447,808 bytes. MFMA prepares
1,191,968,768 bytes of transposed projections/head; scalar prepares zero. Payload
counts exclude allocator rounding and runtime resources. The authenticated model
loader still retains both target and draft host payloads.

## Independent FP32 Reference

`tools/draft_paged_reference.py` is a new producer and custody checker. It imports
only the exact frozen `tools/draft_reference.py` bytes with SHA-256
`e491b4f243855dd57ce5b7cd6b2cd7b78811b5605622018827bb3932d41cb96e`.
It does not monkeypatch or use the old BF16 producer, policy, or adapter. The
existing pinned image, package/source hashes, checkpoint hashes, tokenizer,
offline settings and Draft06B geometry remain required.

The new producer uses the BF16 Qwen3 model body with SDPA, then applies a separate
FP32 linear head to the final BF16 hidden state converted to FP32 and the tied
embedding/head weights converted to FP32. It freezes both schedules above before
execution and runs two repetitions of each, each with an empty initial cache.
It checks full finite vocabulary logits and exact lowest-ID argmax at every
forward. All 16 little-endian FP32 vectors are retained, 607,744 bytes each and
9,723,904 bytes total. Each vector has an exact name, byte count and SHA-256.

The output directory contains only `raw.json` and those 16 vectors. The distinct
`FerricIndependentDraftPagedFp32ObservationV10` document binds the producer,
helper, image, package/source, model and policy identities. Adaptation reopens all
payloads, rejects symlinks/hardlinks, validates the complete roster and hashes,
recomputes finite logits/top2/argmax, and requires exact repeated vectors and
observations within each schedule. It does not require or presume equality
between full and tokenwise schedules.

Root-only execution contract, with externally frozen source hashes:

```text
python3 draft_paged_reference.py \
  --producer-sha256 PRODUCER_SHA \
  --legacy-helper draft_reference.py --legacy-helper-sha256 FROZEN_HELPER_SHA \
  produce --source CANONICAL_DRAFT_CHECKPOINT --output NEW_RAW_DIRECTORY \
  --image PINNED_IMAGE --image-id PINNED_IMAGE_ID

python3 draft_paged_reference.py \
  --producer-sha256 PRODUCER_SHA \
  --legacy-helper draft_reference.py --legacy-helper-sha256 FROZEN_HELPER_SHA \
  adapt --raw-dir RAW_DIRECTORY --raw-sha256 RAW_JSON_SHA \
  --identity ADMITTED_IDENTITY_JSON --identity-sha256 IDENTITY_SHA \
  --prefill full --output NEW_FULL_REFERENCE_JSON
```

Repeat only the adaptation with `--prefill tokenwise` and a separate output.
The admitted identity uses existing `FerricDraftReferenceIdentityV1`; the new
references use `FerricDraftPagedCanaryReferenceV10`, exact `fp32-v10` precision,
schedule, ordered inputs/positions/selected rows, choices, resident cursors,
generated tokens and decoded UTF-8 bytes. References must be frozen before
Ferric output is examined. Root's lifecycle wrapper must require zero producer
exit, owned-process cleanup and all-device idle receipts; a document alone is
not successful execution evidence.

## Output And Boundaries

Successful Ferric execution emits exactly one Setup, Observation and Closed
record, each with `performance_qualified=false` and `authority=none`. The CLI
checks all reference **choices, token IDs and decoded bytes**. It does not read
back or compare full GPU logits; complete logit checking applies to the
independent producer, not the Ferric canary. Setup records all effective profiles,
artifact and executable identities. Observation records exact consumed rows,
choices, cursors and dispatch counts. Closed binds completed batches, worker PID,
reference disposition and worker teardown. A parity mismatch is emitted but
returns failure after close. No timing or throughput fields are emitted.

Synthetic native v10 fixtures qualify their own output/guard scope only. Host
tests and image admission do not establish this paged model's numerical parity,
automatic proposal generation, speculative serving, or a new Verus proof.
