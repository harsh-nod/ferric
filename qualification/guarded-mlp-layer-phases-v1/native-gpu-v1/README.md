# Closed-Layer Host Attribution

One retained TP2 BF16 Readiness40 diagnostic: 40 prompt forwards, zero generated tokens.
This report reads authenticated originals only; it does not rerun a model or GPU.

## Worker Phases

First use is positions 0-1; warm is positions 2-39.
Nanosecond reductions are exact integers. Seconds and percentages use Decimal half-even display.

| Phase | First Use (s) | Warm (s) | Warm Body Share |
| --- | ---: | ---: | ---: |
| input | 0.000010 | 0.000183 | 0.000269% |
| metadata | 0.163800 | 3.099929 | 4.549280% |
| embedding | 0.117971 | 2.227751 | 3.269322% |
| bank | 0.027663 | 0.878917 | 1.289848% |
| layers | 25.154831 | 59.363788 | 87.118926% |
| tail | 0.291271 | 1.499645 | 2.200794% |
| frame | 0.001969 | 0.036210 | 0.053140% |
| fence | 0.027148 | 0.517788 | 0.759876% |
| commit | 0.027109 | 0.516880 | 0.758544% |
| Total worker body | 25.811771 | 68.141092 | 100.000000% |

![Warm host phase attribution](host-phases.svg)

## Closed-Layer Stages

Only positions 2-39 have layer metrics: 38 forwards, 36 ordered returns each, 1,368 total.
Cold positions 0-1 have no nested layer observations; absent is not measured zero.

| Closed Stage | Warm Seconds | Closed Body Share |
| --- | ---: | ---: |
| enter_pre_census | 2.828972 | 4.766325% |
| prefix | 18.863720 | 31.782088% |
| mlp_retired_seal | 33.703503 | 56.784540% |
| hidden_post_census | 1.500054 | 2.527330% |
| full_exit | 2.455390 | 4.136906% |
| commit_prepare | 0.001668 | 0.002810% |
| Closed body | 59.353307 | 100.000000% |

## Paired-MLP Stages

Already inside mlp_retired_seal above; do not add this table to the closed body.
| Paired Stage | Warm Seconds | Paired Body Share |
| --- | ---: | ---: |
| preflight | 1.516199 | 4.528169% |
| consume | 0.145614 | 0.434879% |
| reserve | 0.000507 | 0.001515% |
| publish | 0.292547 | 0.873700% |
| poll | 30.941304 | 92.407059% |
| retire | 0.000813 | 0.002427% |
| terminal | 0.586719 | 1.752251% |
| Paired body | 33.483702 | 100.000000% |

Forward layers outside measured closed bodies: 0.010480666 s.
Closed MLP outside measured paired bodies: 0.219800759 s.
These same-process nested differences include uninstrumented boundary/acceptance work, not a GPU estimate.
The wire retains per-forward aggregates; individual return checks come from qualified Rust, not this report.

## Nested Callbacks

These intervals are already inside the worker phases; they are not additional time.
The first two callback rows are unmeasured, although their forward phases are measured.

| Warm Nested Interval | Seconds |
| --- | ---: |
| Bank callbacks | 0.857497 |
| Bank guarded body, including its callbacks | 0.878550 |
| Layer callbacks | 19.761554 |
| Other layer interval | 39.602234 |
| Tail callbacks | 0.180948 |

The other layer interval includes arithmetic, polling, checks and other work not classified
as these callbacks. It is neither measured GPU time nor a removable fraction.

## Parent Timeline

The following separate table sums the 124 contiguous parent spans.
Forward subsets and worker timings overlap this timeline; do not add them to it.

| Parent Group | Seconds |
| --- | ---: |
| source_preparation | 92.296543 |
| spawn_to_setup_seal | 224.278177 |
| prepare_write | 0.013522 |
| flush_to_frame_read | 94.004455 |
| validate_retain_commit | 0.023559 |
| close_and_retirement | 29.710357 |
| postcheck_and_ordinary_publication | 0.416445 |
| Total parent timeline | 440.743058 |

## Signed Boundary Comparisons

| Subset | Parent Flush-to-Read Minus Worker Body (ns) | Whole Parent Forward Minus Worker Body (ns) |
| --- | ---: | ---: |
| first_use | 17382531 | 20688143 |
| warm | 34209496 | 67984961 |
| all | 51592027 | 88673104 |

Position 14's flush-to-read difference is 1070910 ns.
Differences remain signed and are not clamped. The parent marks Flushed after flush returns;
the worker can already be processing the request. These different process/boundary intervals
are not exclusive unmeasured timers, and neither comparison establishes an optimization ceiling.

## Interpretation And Source Boundaries

Six closed stages and seven paired stages are disjoint only within their own level.
Callbacks remain nested across those stages; no callback subtotal is assigned to a single stage.
Poll time mixes device waiting, host currentness checks, fences, pauses and scheduling.
A large bucket is not a removable fraction. Preserve checks, deadlines, poison, polling and durability.

- engineering_gfx950_peer_scoped_layer_v1.rs::closed_layer_recorded measures entry/pre-census, prefix, MLP/retired seal, hidden/post-census, full exit and provisional commit preparation.
- Its body ends before diagnostic validation, the inherited final deadline and guard disarm; those remain inside the enclosing worker layers phase.
- engineering_gfx950_peer_combined_mlp_paired_v1.rs::coordinate_recorded begins after the original initial deadline setup and ends after terminal checks/readbacks, before the original final timestamp/deadline and Completion construction.
- The excluded paired boundary work and retired-arena sealing remain inside the enclosing closed MLP stage.
- guarded_mlp_long_sequence_v2.rs delimits nine forward phases; Driver construction, final row acceptance and wire publication remain outside the worker forward body.
- finite_guarded_mlp_long_wire_v2.rs frame construction/hashing is inside frame; write_frame recomputation is outside.
- Parent readiness.rs reads the frame before its Read mark and appends evidence before Committed. Capture validation is in read; readiness_evidence.rs retention, rehash and sync_data are in retain/commit.

These are qualified source boundaries, not GPU kernel boundaries or new execution authority.

## Original Evidence

- [Original 170-member archive](../native-gpu-v1.tar.gz)
- [Original native terminal](native-complete.json)
- [Original three-record stderr](child-stderr.bin)
- [Original parent timing](host-timing.json)
- [Original retention result](retention-result.json)
- [Data-only reporter source](report_layer.py)
- [Exact integer report and provenance](report.json)

Archive SHA256: a5d0ab5018ebc7e23e08c4d30402941fe054bbdfa810b90835bf9fa65839103e.
Native terminal SHA256: 068721f3857cc962c52fe3f35d2d02490c9ba46e6b470c696fd7ad8e17664105.

All 170 archive members and 169 manifest pins are authenticated. The original LF-inclusive
policy/currentness/phase records, cross-hashes, all 40 phase sums, callback containment,
all six/seven stage sums, aggregate nesting and parent span sums are rechecked. Full ordinary admission and historical payload parity
were performed by the separately retained strict retainer; this reducer is not a replacement validator.

The reporter is a fixed-input data-only MI350 recipe; it never imports the archived helpers.
The JSON retains each signed row and all twelve callback categories for independent recomputation.

No GPU timing, overlap, speedup, generated-token correctness, Full2303 acceptance or 700 tokens/s claim is made.
The independent position-5 numerical diagnostic is not changed by this instrumentation.
