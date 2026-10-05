# Explicit Shared-Full Projection AR4

Source-only proposal. No formatting, compiler invocation, tests, runtime audit
or native execution has been performed on these candidate bytes. Root owns
qualification, integration and execution. Live Ferric and fe2o3 are untouched.

## Scope

The 14-file overlay contains ten exact-preimage replacements and four new
files. All ten preimages match the actual observer CPU855 source snapshot
`1d173129c684afe5bcdc009ec939f8e4f0371bad8a4edecdf62b4548cb04f920`,
not merely a live Git assumption. `source-manifest.json` records every
before/after byte extent and SHA256 plus the actual CPU7d8c predecessor.

The existing runtime implementation is reused without modification:
`Gfx950EngineeringPeerGroupV1::configure_performance_v2(false, false, true)`.
This is an explicit group-fence scheduling change, not unchanged default
policy. Each group fence still performs a fresh complete topology discovery,
all-rank before/after mutable checks, final generation recheck and every
queue/frontier/exception check. No topology snapshot escapes the call.
Operational currentness, admission caching, legacy profiling and raw timestamp
queues remain off. There is no new topology-ABA or reset-proof claim.

The ordinary decode route, V1 selector, V1 report schema and V1 false-only
snapshot validator retain their behavior. Internal helpers take a closed
`DefaultFull`/`SharedFull` policy; old public entries pass `DefaultFull`.
The existing Recorder, setup, four-forward serve loop, own-output recurrence,
projection bootstrap, image bindings, completion chain, Close, EOF/reap and
exclusive sidecar writer are shared. No kernel/provider/compiler/fe2o3 source
or image is changed, and no whole observer implementation is duplicated.

## Explicit Route

New parent binary, using `tp-batch-engineering`:

```sh
ferric-qwen3-finite-projection-residual-decode-shared-host-engineering \
  --request /absolute/request.json --allow-unauthenticated-machine-code \
  --observe-projection-shared-host
```

The request remains the existing projection AR4 request. The new parent selects
worker flag `--engineering-native-projection-residual-decode-shared-host-v1`
with the same devices/timeout/mode arguments and `--host-sidecar PATH`.
Teacher-forced mode, policy combinations and extra flags are refused.
Parent Diagnostic schema is
`FerricFiniteProjectionResidualDecodeSharedHostDiagnosticV1`.

The new sidecar path suffix is `-projection-shared-host-observation.json`.
Its entire bytes are bound by the parent `host_sidecar` FilePin, under the
unchanged 64 KiB limit and aggregate 8 MiB evidence limit. It is a closed
envelope with these fields:

- `schema`: `FerricProjectionResidualDecodeSharedHostEnvelopeV1`.
- `policy`: `shared-full` only.
- `configuration_host_ns`: inclusive host wall duration of the successful
  runtime configuration call, as an unsigned 64-bit integer.
- `observation`: the existing report fields, with distinct inner schema
  `FerricProjectionResidualDecodeSharedHostObservationV1` and all seven
  snapshots requiring `shared_full_currentness=true`.

Neither the new envelope nor its inner shared report is accepted by the old
V1 decoder. Shared decoding does not rewrite schemas or booleans to impersonate
old evidence. The parent retains exact worker/PID/profile/bootstrap/source/
projection/completion/control/payload/chain joins and actual Close/EOF/reap
requirements before publishing any successful diagnostic.

## Timing Scope

The runtime is configured before observer enable and before user setup. Any
configuration or enable failure propagates immediately; no setup/forward is
started. The runtime retains its own fresh-only and terminal-poison checks.
Observer activation starts the zero counter baseline only afterward.

`configuration_host_ns` is outside all seven snapshots and six intervals. It
excludes group open, observer enable and subsequent setup. It is not constrained
against a post-enable interval, added to nested counters, or called GPU time.
No timer/counter is reset to hide configuration. Existing interval meanings
remain inclusive; forward, currentness, read/write and wait counters overlap.

Static successful TP2 group-fence topology discoveries change four to one;
before/after device pairs change four to two, with two queue checks retained.
A group read/write has two group fences and two unchanged owner checks, hence
ten to four discoveries. Publication fences already share in both modes;
single-context lifecycle, dispatch preparation and wait checks remain full.
These are source counts, not a measured speedup or a whole-run exact census.

## Qualification Work

Twenty new test methods are authored: six worker-only, eight shared data
methods compiled in both crates, five parent-library and one new binary.
This is 14 added worker executions and 14 added parent executions. Existing
test bodies/names are preserved; two existing test files only gain an include
or child-module declaration. The manifest gives exact full names.

Focused cases cover fixed CLI selection, configuration-before-enable ordering
and refusal, unchanged default setup, schema/policy cross-refusal, zero baseline,
rank/epoch/counter checks, configuration time outside snapshots, AR recurrence,
projection/profile/worker bindings, all four serializations then Close,
actual-file payload drift, missing/oversized sidecars and Close/reap refusal.
The small setup double checks orchestration, not native GPU construction or
the runtime's sysfs/DRM behavior. Existing runtime fence-order/fault/poison and
publication tests remain relevant; no fresh execution is claimed here.

Root's bounded CPU successor must format copies, retain exact resulting source
bodies, preserve all prior selected parent/worker tests and add the new binary
list/test. Expected additions come from names, not fabricated outcome counts.
Fresh parent/worker ELFs and runtime audits are required before deployment.
A separate frozen GPU intake/report validator must select this new route and
the exact new envelope; the old false-only validator must remain unchanged.

The first hardware comparison must retain identical kernel/model/prompt inputs,
four own outputs, all 152 tensors and full lifecycle evidence. Same-image
output equality is a useful route regression, not independent model accuracy.
Report measured configuration, setup, forward and currentness scopes separately.
No result here qualifies numerical acceptance, performance, production or the
2048-input/256-output workload.
