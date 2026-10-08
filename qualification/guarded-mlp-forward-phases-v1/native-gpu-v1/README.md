# Forward-Phase Host Attribution

One retained TP2 BF16 Readiness40 diagnostic: 40 prompt forwards, zero generated tokens.
This report reads authenticated originals only; it does not rerun a model or GPU.

## Worker Phases

First use is positions 0-1; warm is positions 2-39.
Nanosecond reductions are exact integers. Seconds and percentages use Decimal half-even display.

| Phase | First Use (s) | Warm (s) | Warm Body Share |
| --- | ---: | ---: | ---: |
| input | 0.000010 | 0.000181 | 0.000265% |
| metadata | 0.162739 | 3.093112 | 4.545794% |
| embedding | 0.117168 | 2.221773 | 3.265229% |
| bank | 0.027434 | 0.875156 | 1.286174% |
| layers | 25.020339 | 59.301174 | 87.151988% |
| tail | 0.288349 | 1.485821 | 2.183638% |
| frame | 0.001876 | 0.035425 | 0.052062% |
| fence | 0.027229 | 0.515442 | 0.757519% |
| commit | 0.027236 | 0.515314 | 0.757331% |
| Total worker body | 25.672379 | 68.043397 | 100.000000% |

![Warm host phase attribution](host-phases.svg)

## Nested Callbacks

These intervals are already inside the worker phases; they are not additional time.
The first two callback rows are unmeasured, although their forward phases are measured.

| Warm Nested Interval | Seconds |
| --- | ---: |
| Bank callbacks | 0.853449 |
| Bank guarded body, including its callbacks | 0.874770 |
| Layer callbacks | 19.614708 |
| Other layer interval | 39.686466 |
| Tail callbacks | 0.180369 |

The other layer interval includes arithmetic, polling, checks and other work not classified
as these callbacks. It is neither measured GPU time nor a removable fraction.

## Parent Timeline

The following separate table sums the 124 contiguous parent spans.
Forward subsets and worker timings overlap this timeline; do not add them to it.

| Parent Group | Seconds |
| --- | ---: |
| source_preparation | 92.330228 |
| spawn_to_setup_seal | 221.657885 |
| prepare_write | 0.006007 |
| flush_to_frame_read | 93.775164 |
| validate_retain_commit | 0.021795 |
| close_and_retirement | 29.558526 |
| postcheck_and_ordinary_publication | 0.433487 |
| Total parent timeline | 437.783090 |

## Signed Boundary Comparisons

| Subset | Parent Flush-to-Read Minus Worker Body (ns) | Whole Parent Forward Minus Worker Body (ns) |
| --- | ---: | ---: |
| first_use | 17286651 | 20430093 |
| warm | 42101024 | 66759054 |
| all | 59387675 | 87189147 |

Position 14's flush-to-read difference is -2425341 ns.
Differences remain signed and are not clamped. The parent marks Flushed after flush returns;
the worker can already be processing the request. These different process/boundary intervals
are not exclusive unmeasured timers, and neither comparison establishes an optimization ceiling.

## Next Measurement

Prioritize the layer interval: 87.151988% of the warm worker body.
Its 39.686465506 s noncallback remainder is mixed work, not measured GPU compute.
The next bounded measurement should separate the closed layer engine's prefix, MLP and hidden-state work,
then the paired MLP preflight/consume/reserve/publish/poll/retire/terminal path in
engineering_gfx950_peer_combined_mlp_paired_v1.rs:123. Preserve all polling, currentness checks and durability.
This is a measurement priority, not a proposed removal or a predicted speedup.

## Source Boundaries

- Worker guarded_mlp_long_sequence_v2.rs timestamps at lines 114/125/129/133/137/144/148/172/176/185 delimit the nine phases.
- native_guarded_mlp_readiness_v1.rs:361 constructs Driver outside the timed body. Final row acceptance and wire publication are also outside.
- finite_guarded_mlp_long_wire_v2.rs:380 frame construction/hash work is inside frame; write_frame's recomputation at line 529 is outside.
- Parent readiness.rs:631 reads before Read at line 638; append at line 644 precedes Committed at line 658.
- finite_guarded_mlp_long_wire_v2.rs:557-598 capture validation is inside parent read. readiness_evidence.rs:144-189 capture retention/sync/rehash and per-frame sync_data are inside retain/commit.

These are qualified source boundaries, not inferred GPU kernel boundaries.

## Original Evidence

- [Original 169-member archive](../native-gpu-v1.tar.gz)
- [Original native terminal](native-complete.json)
- [Original three-record stderr](child-stderr.bin)
- [Original parent timing](host-timing.json)
- [Original retention result](retention-result.json)
- [Data-only reporter source](report_forward_phases.py)
- [Exact integer report and provenance](report.json)

Archive SHA256: d22c87f34e686653caa3cdd5584e8b37569845455eb2d398f19d81902209403e.
Native terminal SHA256: aa5d9b023c8ca9b7cb916bc60d41282804769479b294a01ab619787ed0a4dc56.

All 169 archive members and 168 manifest pins are authenticated. The original LF-inclusive
policy/currentness/phase records, cross-hashes, all 40 phase sums, callback containment
and parent span sums are rechecked. Full ordinary admission and historical payload parity
were performed by the separately retained strict retainer; this reducer is not a replacement validator.

The reporter is a fixed-input data-only MI350 recipe; it never imports the archived helpers.
The JSON retains each signed row and all twelve callback categories for independent recomputation.

No GPU timing, overlap, speedup, generated-token correctness, Full2303 acceptance or 700 tokens/s claim is made.
The independent position-5 numerical diagnostic is not changed by this instrumentation.
