# Paired Paged K4 Canary V1

This additive engineering binary runs two genuine private-proposal rounds with
Qwen3-8B target and Qwen3-0.6B draft. It is not a serving endpoint, protected M1
authority, a performance qualification, or a speculative speedup claim. Native
success requires an independent wrapper receipt; adding this source is not that
receipt. Existing controller CLIs and frozen references remain unchanged.

## Fixed Contract

`ferric-qwen3-paired-paged-canary` is gated by `tp-batch-engineering`. Its closed CLI
requires source/model artifacts, the worker path and SHA-256, a nonzero device
unique ID, the original target reference, the adapted reference and its SHA-256,
explicit unauthenticated-machine-code consent, and all of:

- `--runtime-cache-admission`
- `--runtime-operational`
- `--runtime-rollover`

It accepts no candidate tokens, acceptance counts, alternative K, round count,
ordered batches, runtime sequences, wave attention, prefix reuse or concurrency.
Target profile is batch32 MFMA, device-TP1 collective, baseline attention,
output-head pruning and FP32-output v8 head. Draft profile is v10 baseline
projection and attention with its existing FP32-output head. Profiles are fixed
before observations; their kernels do not have identical operand precision.

Both independent row-capacity-32 pools use context 160 and ten 16-token pages.
Each role executes eight no-output prefill forwards, in chunks of at most 16,
for exactly the first 127 frozen prompt inputs. Prompt token 127 is the initial
nonresident anchor, not an already committed KV input.

Exactly two rounds then run:

1. The owner reserves target K+1 and draft K capacity before issuing draft work.
   Four sequential draft forwards consume the anchor followed by each previous
   driver-sealed choice. Every live candidate comes from a real driver result.
2. A target forward consumes the anchor plus all four proposed IDs and produces
   five selected target choices. Only its sealed result authorizes settlement.
3. Greedy prefix settlement emits A accepted draft IDs and one target correction
   or bonus, for A in 0..4. Target commits A+1 inputs; draft commits min(A+1,4).
4. Full acceptance requires a genuine no-output draft catch-up forward consuming
   the fourth proposal at the one missing KV position. This is mandatory even
   after the second/final round. The target correction/bonus remains the next
   nonresident anchor. Final cursors must agree and the owner must be Ready.

The existing owner constructor takes a complete untrusted round index. This
binary supplies zero candidate slots only to bootstrap the owner. The private
`reserve_proposals` operation replaces every live slot before driver work.
Bootstrap placeholders never enter proposal traces or target verification.
There is no new public constructor, mutable choice access or completion minting
API. Added result accessors expose only immutable diagnostic identities, inputs
and choices; sealed results are still consumed by the existing owner.

## Independent Comparison

The original `FerricMatched128ReferenceV1` bytes are pinned to
`cd7f512537e44d677244c606cba919e57747e8c26caf3c0df4a0aca7acc0eb8b`.
The separate seven-field `FerricPairedPagedK4ReferenceV1` adapter retains both
full 128-token arrays unchanged, links that source hash, includes ten independently
decoded UTF-8 prefixes, and records tokenizer and producer source hashes.
The binary requires both files, verifies both byte identities, uses closed typed
schemas, checks full-array equality and vocabulary bounds, and compares its
actual 2..10 emitted tokens plus decoded UTF-8 bytes with the matching independent
prefix. The adapter cannot choose draft outputs or force acceptance.

Accepted-token counts and catch-up coverage are observations. A run with no full
acceptance may pass output parity but does not demonstrate native catch-up.
Host Recording tests deliberately inject synthetic transport outputs to cover
zero/partial/full acceptance and final catch-up; these are not GPU numerical
evidence. The frozen original target reference remains the numerical authority.

## Ownership And Bounds

One authenticated model owner retains target 16,381,470,720 and draft 1,503,264,768
weight bytes. The two drivers borrow its views but own separate workers, GPU
allocations, queue rollover state and KV pools. Target MFMA additionally retains
15,136,194,560 transposed weight bytes; draft baseline retains none. Target KV is
23,592,960 bytes; draft KV is 18,350,080 bytes. Each role's FP32 logits workspace
is 19,447,808 bytes. Runtime allocations, other activations and host transposition
temporaries require additional headroom; these payload counts are not complete
host/VRAM limits. The launch wrapper owns conservative admission, wall timeout,
output bounds, all-eight-GPU idle checks and process/resource cleanup.

Calculated completed packet counts are fixed by the admitted profiles:

| Work | Count Per Forward | Maximum Forwards |
| --- | ---: | ---: |
| Target prefill | 613 | 8 |
| Target verification | 616 | 2 |
| Draft prefill | 477 | 8 |
| Draft proposal | 480 | 8 |
| Draft catch-up | 477 | 2 |

Target total is 6136 packets. Draft total is 7656 + 477 times the observed catch-up
count, at most 8610. The controller verifies every forward delta and final
completed-batch totals. Each complete forward must fit the runtime's advertised
unretired queue budget; rollover is explicitly required for cumulative work.

Prefill batch IDs are 1..8 in both pools. Target verification uses 9 then 10. The
draft aggregate reservation consumes 9 without a driver forward, so first-round
proposal IDs are 10..13 and an optional catch-up is 14. Second-round proposal IDs
are 15..18 plus the first-round catch-up count; the optional final catch-up ID is 19
plus that count. Reservation IDs are not fabricated completion counts.

Errors after submitted work do not permit settlement or reuse: the controller
uses the owner's abort-or-quarantine path and attempts both driver closes even
when execution or the first close fails. Prefill submission errors quarantine
the affected pool. Read-only owner access leaves metadata resident until the
owned drivers close; no mutable pool extraction or premature free is added.
Success requires exact reference parity, both successful closes and a complete
diagnostic output stream. An external timeout or forced process termination
cannot be a successful canary receipt.

## Trace And Verification

A successful JSONL stream contains exactly six records, in order: Setup,
Prefill, Round 0, Round 1, Observation, Closed. Every record declares
`authority: none` and `performance_qualified: false`. Setup binds controller,
worker, artifacts, source/reference, fixed profile and pool identities in an
engineering-plan digest, not a protected plan. Round records report only actual
sealed proposal inputs/choices, selected target rows/choices, settlement and
conditional catch-up. Observation includes exact output parity, both cursors,
packet totals and retained pool state. Closed reports both owned worker outcomes.

Independent validation must reconstruct both proposal chains, the longest
matching target prefix, emitted IDs, cursor changes, epochs, batch IDs, actual
catch-up necessity and packet totals. It must compare the externally pinned
reference and reject partial streams, unknown records, extra fields, wrong
worker rosters, any forced close, leaked children or non-idle postflight. This
source does not alter an existing comparator or normalize actual choices.

Focused host tests cover the closed CLI/reference contract, fixed packet budget,
immutable result diagnostics, cross-page two-round settlement with zero/partial/
full acceptance, final-round catch-up and fail-closed result aggregation. Existing
proposal tests continue covering dropped/sealed tickets, transport failures,
foreign drivers and missing rollover. Compile-fail tests retain private sealing
and immutable choice boundaries. Builds and tests run only on the shared remote
CPU build host after integration review; this work authorizes no GPU launch.
