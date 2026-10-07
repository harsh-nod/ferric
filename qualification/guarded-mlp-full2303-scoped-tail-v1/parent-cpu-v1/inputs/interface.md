# Full2303 Bank/Census/Tail Worker Interface

Source-only proposal. No execution, qualification, performance, numerical acceptance,
or Full launch authority is conveyed by this interface. Runtime API is unchanged.

## Explicit Identity

- Pure module: `finite_guarded_mlp_full2303_bank_scoped_census_tail_v1`.
- Worker flag: `--engineering-native-guarded-mlp-full2303-bank-scoped-census-tail-v1`.
- Policy schema: `FerricFull2303BankScopedCensusTailPolicyV1`.
- Execution profile: `Full2303BankScopedCensusTailCurrentnessV1`.
- Parent entry: `run_bank_scoped_census_tail`.
- Parent flag: `--observe-guarded-full2303-bank-scoped-census-tail-v1`.
- Parent wrapper: `FerricFull2303BankScopedCensusTailObservationV1`, fields in order
  `schema, observation, currentness_policy`.
- Policy bound remains 4,096 bytes including its sole terminal newline.

## Policy API and Ordered Fields

`PolicyRecord::new(b: &full::Bootstrap, transcript: [u8;32], generated_tokens:
&[u32], worker: [u8;32], counts: Counts) -> io::Result<Self>`.
`validate(&self,b,transcript,generated_tokens,worker)` and
`decode(raw:&[u8],b,transcript,generated_tokens,worker)` retain the existing Full
argument types/order. `encode` and `write_to` retain canonical single-record framing.
The parent independently supplies all 256 committed own-output IDs, not a worker
assertion. Their SHA256 is over ordered u32 little-endian bytes.

Reuse the exact existing Full Bank/Census PolicyRecord ordered fields, inserting
`scoped_tail, full_entry_exit_per_scoped_tail,
scope_includes_tail_dispatch_and_readback` (all true) immediately after
`census_counters_are_layer_subset` and before `temporal_equivalent_to_full`.
All existing fields and booleans retain their original order and values.

`Counts` has ordered fields `layers, banks, census, tails`. Public LayerCounts,
BankCounts and CensusCounts are aliases of the unchanged Full Bank/Census types.
The first three sets validate through the existing Full Counts validator, including
checked subtraction and revalidation of the census subset. Tail counts are
independent and are never included in/subtracted from the layer or bank counters.

`TailCounts` ordered fields/types:

```text
ordinary_tails: u32
scoped_tails: u32
dispatches: u32
readbacks: u32
readback_bytes: u64
full_discoveries: u64
local_checkpoints: u64
before_calls: u64
after_calls: u64
generation_probes: u64
```

Closed tail assertions: ordinary=2; scoped=2,301; dispatches=readbacks=6,903;
readback_bytes=718,068,468; full_discoveries=4,602. With L=local_checkpoints,
L>=62,127; before_calls=after_calls=L+36,816; generation_probes=2L+6,903.
Every operation is checked for overflow. These are structural relations, not
measured whole-call counts: each scoped tail has twelve two-rank group and fifteen
rank-local checkpoints, plus variable periodic currentness checks. A periodic
check is not a raw poll iteration. All inherited layer/bank/census totals remain
unchanged (82,836 warm layers, 2,301 bank rearms and 1,325,376 census checkpoints).

## Ownership and Refusal

The Full wire/Sequence/Bootstrap/Closed protocol is unchanged: 2,048 authentic
prompt positions, then 255 own-choice feedback forwards, producing 256 IDs;
2,303 committed forwards; captures [0,2047,2048,2302]; existing one-hour maximum.
Positions 0 and 1 retain ordinary bank/layer/tail execution. Positions 2..2302
use existing scoped bank/census layers followed by the existing scoped tail
runtime facade. Each tail follows all 36 committed layers and precedes the next
forward. A failed runtime return, numerical validation, count join or unwind
makes the route terminal; there is no retry or ordinary fallback.

The runtime owns the three tail dispatches and three returned readbacks
(choice 4, normalized 8,192, logits 303,872 bytes). Original numerical checks
run once after its full exit, on those exact returned bytes. The worker validates
observed tail counters before committing the route state. The runtime API,
allocation/lifetime rules, hidden reads, model arithmetic and deadline checks
are unchanged. The outer Full owner validates its own 256 outputs against the
parent transcript before healthy Close and policy publication.

Old Full, Full Scoped, Full Bank/Census and all Readiness selectors remain exact.
No Full feasibility follows from a Readiness timing pair. Temporal equivalence,
numerical acceptance, performance claims and production authority remain false.

## Source Base

Canonical Ferric source read at 1cffbfa0faf7950a5a5b9948ae778ae933c1861b.
Worker base is actual Tail coupled V2: terminal
3c4fe3a6c4942b20ec61fd7ea5c3409526e2167c43d24a62fb8660c34b0e713e,
source map 5de4929b2832eda55a53f99945fc4b64a9fbda28b4d4f8bef07f2e8d5e000d6a.
Unchanged runtime tail interface:
749fd470a27cf7630f292b04c5464567e4e7d398ee7855b1fbd152824421a8e4.
Exact source preimages/postimages and test names will be closed by the worker
source manifest; this interface is not a future qualification receipt.

