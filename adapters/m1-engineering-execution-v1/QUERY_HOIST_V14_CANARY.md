# Query Hoist V14 Canary

Source-only opt-in binary: `ferric-qwen3-query-hoist-v14-canary`, under the existing
`tp-batch-engineering` feature. No default, HTTP route, kernel, worker, reference,
dependency pin or inventory is changed. Formatting, compilation, tests and native
execution are pending separate review and authorization.

## Closed Comparison

The new parser requires `--attention-mode resident-wave|query-hoist-v14` and
`--query-hoist-artifact PATH`. All other flags pass through the unchanged layer
canary contract: explicit `--layer-projection c1-wave`, `--submission ordered`,
`--attention wave`, `--argmax-mode wave-v11`, TP1 and the fixed reference files.
The inherited prompt is exactly 128 tokens, prefill chunk 16, context 256, pages
16 and output length exactly 8 or 128. Cache admission, operational currentness
and rollover remain required; sequences, runtime profiling and shared currentness
remain forbidden. Prefix reuse is disabled. The old `--attention wave` flag
admits the common wave prerequisites; the new explicit mode selects the actual
attention root and is reported directly, without normalization to an old schema.

Both modes open the same separately admitted V14 image and load it after V8/V11
but before driver allocations. Both use
`new_wide32_with_argmax_v11_and_query_hoist_v14`. Resident-wave selects the existing
ordered C1/V11 terminal policy; query-hoist-v14 uses its atomic V14 terminal
selector. All layer projection and V8/V11 head choices are otherwise identical.
The actual driver attention/layer/argmax getters and packet counts are checked
before the workload. The setup records the supplied V14 path and admitted
HSACO/manifest/handoff identity in either mode. A future launch wrapper must bind
the same exact artifact bytes for both arms; this source does not invent pins.

Four new `FerricQueryHoistV14Canary{Setup,Prefill,Observation,Closed}V1` schemas
record `attention_mode`, actual `attention`, C1 layers and ordered submission.
Benchmark and serving admission remain false. Setup is copied exactly into the
host-timing journal. Existing canary schemas, parser behavior and emitted record
fields are retained. Host elapsed spans are not HTTP timings or GPU durations.

The unchanged execution path checks every generated token and UTF8 byte against
the fixed target reference, requires 9,219 or 83,139 packets and 15 or 135 batches,
retires without prefix caching and closes the driver on configuration/workload/
reference/output failure. Artifact-load failures close the worker; constructors
retain their preallocation rejection and close behavior. Failed batches retain
the existing poison/quarantine rules. No frozen numerical oracle is rewritten.

## Source Tests

Five new parser methods cover identical arm inputs/counts, required mode/image,
unsupported policy/runtime changes, option-shaped path values and legacy parser
rejection. Three new shared profile methods cover four exact record schemas,
unchanged legacy record annotation, runtime/image mismatch rejection and no
sidecar effects. One recording test compares unpublished/published 16-row chunks
and one-row decode, with identical admitted image state, allocations, head and
retirement; only GQA root names are normalized. It uses a small synthetic context,
not a full-model or full-128 prompt numerical oracle. Existing V14 all-row,
preallocation, atomicity and failure tests remain unchanged.

One new source-policy method closes preload/constructor/selector/reference order.
The historical `query_hoist_v14_route_adds_no_controller_or_default_selection`
method is renamed to
`query_hoist_v14_route_keeps_defaults_and_all_legacy_entrypoints_closed`; only the
new exact binary is recognized, and all prior exclusions/default checks remain.

The source-derived new binary forecast is 64 passing and 4 ignored invocations:
argmax contract 3/1, shared runtime 14/0, attention contract 5/0, layer contract
5/0, paired contract's nested module 8/0, host timing 2/0, worker plus nested
diagnostics 17/3, submission contract 5/0 and new parser 5/0. The three new
runtime tests also appear in each of the four existing argmax canary binaries.
Together with one new library recording and one new source-policy test, the
relative all-target forecast is +78 passing, +4 ignored, +1 result row. These
are unexecuted source counts, not test results or an absolute post-migration
forecast. Inherited worker diagnostic and paired modules are included explicitly.

Source base is exact `0588f997d5e2325cf071899f5669390906a0f7e4`. Root will integrate
the additive source onto its separately admitted dependency migration. Inherited
manifests/locks retain their original pins here; no guessed lock or inventory
regeneration is included. A future source-fresh host gate must cover all inherited
test-module invocations as well as these additions, strict Clippy, artifact
admission and exact legacy byte comparison before any native qualification.
