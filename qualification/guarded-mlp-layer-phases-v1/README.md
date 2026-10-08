# Readiness40 Closed-Layer Phase Diagnostic

This default-off diagnostic extends the existing host forward-phase record
with closed warm-layer and paired-MLP intervals. It changes neither kernel
arithmetic nor currentness policy. Source review and CPU qualification do not
establish a new GPU measurement, numerical correctness, overlap, speedup,
Full2303 feasibility or the 700 tokens/s target.

## Measurement Boundaries

The existing `engineering-currentness-duration-diagnostics` feature is
required. The worker and parent retain the explicit Readiness40 Tail route;
there is no new selector, environment switch or execution facade. Feature-off
builds add no diagnostic clocks or allocations.

The original nine forward intervals remain in this exact order:
`input`, `metadata`, `embedding`, `bank`, `layers`, `tail`, `frame`,
`fence`, `commit`. All 40 forward bodies remain measured. Positions 0 and 1
use the ordinary first-use route: their callback and nested layer measurements
are absent, not zero-valued observations. Positions 2 through 39 each aggregate
exactly 36 ordered warm scoped-layer returns: 38 forwards and 1,368 layers.

The feature-only runtime observation field `layer_durations` has type
`Gfx950EngineeringPeerScopedLayerDurationsV1`. It contains
`phase_ns: [u64; 6]`, `layer_body_ns: u64`,
`paired_mlp_phase_ns: [u64; 7]` and `paired_mlp_body_ns: u64`.
The worker's `LayerMetrics` adds the checked `layers` count and aggregates
these arrays into each warm forward row.

| Closed-Layer Stage | Included Work |
| --- | --- |
| `enter_pre_census` | Existing scoped-layer entry and pre-census |
| `prefix` | Existing paired Prefix operation and its deadline check |
| `mlp_retired_seal` | Paired MLP coordinator, final checks and genuine retired-arena sealing |
| `hidden_post_census` | Both hidden readbacks, validation and post-census comparison |
| `full_exit` | Mandatory full-currentness exit and deadline check |
| `commit_prepare` | Provisional completion/result assembly before diagnostic acceptance |

Within `mlp_retired_seal`, seven paired intervals retain this exact order:

| Paired-MLP Stage | Included Work |
| --- | --- |
| `preflight` | Existing policy, owner, input and reusable-arena preparation |
| `consume` | Both owner consumptions and original deadline checks |
| `reserve` | Both software-ring reservations and original deadline checks |
| `publish` | Both batch publications, fences and original deadline checks |
| `poll` | Complete polling loop, including checks, fences and pauses |
| `retire` | Both logical retirements and original deadline checks |
| `terminal` | Original terminal validation, completion readbacks and finish checks |

The paired body's first mark follows the original initial deadline setup.
Its final mark precedes the original final timestamp/deadline check and
Completion construction. Those excluded operations remain inside the enclosing
MLP interval. The closed-layer body ends after provisional result assembly;
metric validation, the inherited final deadline and guard disarm follow it
inside the enclosing worker layer interval.

Checked integer sums must satisfy all of the following:

- Six closed stages equal `layer_body_ns`.
- Seven paired stages equal `paired_mlp_body_ns`, which is contained in the
  closed layer's MLP stage.
- The 36-layer aggregate is contained in the forward's `layers` interval.
- Each returned layer's callback subtotal is contained in its closed body;
  the aggregate callback subtotal is checked again during record admission.
- Nine forward stages equal `forward_body_ns`; all 40 bodies together remain
  within the existing one-hour measurement bound.

These are nested host elapsed intervals, not additive totals across levels.
Callback subtotals must not be added to the six-stage or nine-stage totals, or
subtracted from an individual stage without a matching measurement boundary.
Polling time can include device waiting, host checks and scheduling. None of
these fields measures GPU kernel time. Parent intervals remain separate;
cross-process differences are signed comparisons, not exclusive residuals or
optimization ceilings.

## Original Records And Failure

The original policy and currentness records/codecs are unchanged. The third
record changes from `FerricReadiness40ForwardPhaseDurationsV1` to the distinct
`FerricReadiness40ForwardLayerDurationsV2` schema. Both codecs remain present;
the new consumer rejects the old third schema instead of relabeling it, and
the old consumer rejects the new schema.

The V2 third record retains the original forward rows and adds fixed stage
orders, nested layer metrics and explicit nesting flags. It authenticates the
LF-inclusive original currentness record and the original policy, session,
worker and transcript. The parent still authenticates the entire original
stderr file, never a substituted prefix. The whole-stderr cap remains 69,632
bytes and the third-record cap remains 32,768 bytes.

The existing Sequence, arithmetic, first-use behavior, policy counters,
currentness checks, fences, polls, pauses and deadlines are preserved. The
closed runtime guards remain armed through diagnostic validation and the
original final deadline. Invalid sums, backward clocks, overflow, containment
errors or unwind keep failure terminal and quarantine the owner. The worker
independently validates each return before committing its diagnostic
accumulators; later collection failures prevent successful publication.
Already completed effects are not rolled back or represented as unexecuted.

All three records are validated and encoded only after healthy Close, before
the first stderr write. Write failures propagate; partial external writes are
not claimed to be atomic. GPU timing, numerical acceptance, performance and
execution-authority claims remain false.

## CPU Qualification

All six root-owned CPU configurations passed on `mi350`, with GPUs hidden,
two Cargo jobs, CPU cores 8/9 and nice level 10. Each used the same sealed
project sources; the feature-off/on outcomes are separate original runs.

| Configuration | Observed Result | Status |
| --- | --- | --- |
| Runtime, diagnostic | Main all-target suite: 1,209 passed, 0 failed, 8 ignored; 10 facade rustdocs; 131 focused reruns | Passed, 20 naturally completed supervised steps |
| Runtime, default | Main all-target suite: 1,180 passed, 0 failed, 8 ignored; 10 facade rustdocs; 113 focused reruns | Passed, 20 naturally completed supervised steps |
| Worker, diagnostic | 911 passed, 0 failed, 4 ignored | Passed, 8 naturally completed supervised steps |
| Worker, default | 838 passed, 0 failed, 4 ignored | Passed, 8 naturally completed supervised steps |
| Parent, diagnostic | 641 passing test instances across 61 selected scopes; 0 failed or ignored | Passed, 70 naturally completed supervised steps |
| Parent, default | 584 passing test instances across 58 selected scopes; 0 failed or ignored | Passed, 67 naturally completed supervised steps |

The runtime diagnostic retry took approximately 74.811 seconds. Its original terminal
is 3,301,993 bytes, SHA256
`5d1d0d6ac7af9cf94a5883486d41f61812238ac1ae593f26830e3113c1673d13`.
The 1,350 passing invocations summed across runtime suite, rustdocs and focused
reruns are not 1,350 distinct tests. Parent instances are identified by scope
and name, not globally unique bare names. The final source/product/archive
bindings belong to the [checkpoint manifest](manifest.json).

| Original CPU Evidence | Feature On | Feature Off |
| --- | --- | --- |
| Runtime | [Diagnostic](runtime-diagnostic-cpu-v3.tar.gz) | [Default](runtime-default-cpu-v1.tar.gz) |
| Worker | [Diagnostic](worker-diagnostic-cpu-v1.tar.gz) | [Default](worker-default-cpu-v1.tar.gz) |
| Parent | [Diagnostic](parent-diagnostic-cpu-v1.tar.gz) | [Default](parent-default-cpu-v1.tar.gz) |

The capsules retain the original commands, raw streams, source/ELF/tool pins,
named outcomes and natural-exit/reap/process-group postchecks. They contain
no ELF or dependency-cache bodies. The worker executable remains byte-identical
before and after its CLI probes. These are CPU tests, not inference benchmarks.

The tested runtime is published on both forks at
[`bf8b33dcb0`](https://github.com/harsh-nod/fe2o3/commit/bf8b33dcb0f40a4781f7f3ac5e76f6fe6d4c5fd5).
The [source capsule](source-v2.tar.gz) contains the 30 cumulative Rust
postimages and their earlier-base reconstruction patches. Integration into the
previous forward-phase checkpoint changes only 24 paths: 12 runtime and 12
Ferric worker/parent paths. Do not blindly apply cumulative patches over the
already integrated checkpoint. Cargo manifests, dependency locks and kernel
arithmetic are unchanged from the preceding qualified source snapshot.

These CPU runs use the inherited reduced fe2o3 qualification workspace, not a
repository-wide test selection. Its 833 runtime capsule entries include the
reduced root `Cargo.toml`/`Cargo.lock` and the retained
`Cargo.toml.original`/`Cargo.lock.input`. The other 829 runtime bodies match
the published source; the retained originals match the live repository root
manifest and lock. The reduced root files are not applied to the repository.

Two earlier failed attempts remain separate evidence:

- The [first runtime attempt](runtime-diagnostic-failed-v1.tar.gz) failed a new source-bound test because rustfmt
  wrapped a census call and the assertion required its original whitespace.
  The fresh source descendant normalizes whitespace while retaining the same
  three call/presence assertions; production behavior is unchanged.
- The [next attempt](runtime-diagnostic-failed-v2.tar.gz)'s raw runtime children passed, but its controller refused
  with `RuntimeError('malformed worker libtest result')`. Two known nested
  `queue_linux` process-terminal tests emit multiline result text. The generic
  controller had omitted the predecessor's exact two-name normalizer; the
  fresh retry restores it only for runtime parsing. The failed qualification
  is not relabeled as success.

## V2 Checker Qualification

The [strict V2 checker](checker-cpu-v1/README.md) passes **178 tests** on MI350:
32 new aggregate-layer cases and all 146 unchanged predecessor cases.
The three supervised processes exited naturally in 78.007586 seconds, with
unchanged sources, no cleanup and no failed, skipped or errored tests.
The [original archive](checker-cpu-v1.tar.gz) retains all 30 source bodies,
17 raw evidence files, input and terminal, plus the authoring seal and stager.
These are synthetic CPU admission tests, not native inference measurements.
Individual layer-return containment remains checked by the separately
qualified Rust runtime and worker; the wire exposes only per-forward aggregates.

No layer-duration native result is established by either CPU checkpoint.
Source/ELF admission, preparation, retention workflow qualification and a retained
native run are still required. Full2303 launch admission, independent generated
outputs, sustained throughput and issue #42 M0-M7 remain open.
