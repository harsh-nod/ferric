# Wave/V8/V11 Live Profile V1

Source-only implementation pending host and native validation. This is a
separate opt-in engineering entrypoint, not a protected serving grant, a
default change, or a performance result. No kernel or fe2o3 source changes.

## Closed Interface

`ferric-qwen3-wave-argmax-live` requires `tp-batch-engineering`. It uses the
existing `FerricQwen3TpLiveCommandV1/FerricQwen3TpLiveEventV1` stdin/stdout
protocol and existing setup/close schemas understood by `tools/serve_ferric.py`.
The Python transport, original batch CLI, and all fixed canaries are unchanged.

Required value options:

- `--source DIR`
- `--target-artifact DIR` for the exact admitted v5 MFMA image
- `--target-head-artifact DIR` for the exact admitted v8 image
- `--argmax-artifact DIR` for the separately admitted one-root v11 image
- `--worker FILE` and `--worker-sha256 SHA256`
- `--device-unique-id ID`, one nonzero physical device
- `--submission synchronous|ordered`
- `--context N`, 1 through 8192 logical positions per request
- `--pages N`, 1 through 512 aggregate physical pages
- `--max-batches N`, 1 through 1000000 completed live batches

Required switches: `--live-stdin`, `--allow-unauthenticated-machine-code`,
`--runtime-cache-admission`, `--runtime-operational`, `--queue-rollover`,
`--disable-prefix-cache`, and `--prune-output-head`.
Optional `--host-timing FILE` creates an exclusive diagnostic sidecar. Omit it
for the matched HTTP performance path. Runtime profiling is not accepted.

TP1, capacity32, chunk16, MFMA, wave attention, FP32-v8, wave-v11 argmax,
device-TP1 residuals, pruning and no prefix cache are fixed by this binary.
Alternative attention/projection/head/argmax settings, batch/chunk overrides,
large-KV, peers, draft/speculative execution, sequences, shared currentness,
static workloads, numerical capture and replica controls are rejected before
timing-file creation, model opening or worker spawn. No policy is silently
normalized at runtime. Duplicate flags and unknown options fail closed.

## Integration And Ownership

All three images are inspected before model/worker work. The worker hash is
checked before spawn and against its running executable. The v8 and v11 images
load before any device buffer allocation, and the driver uses
`new_wide32_with_argmax_v11` to retain the exact private binding.

Configuration order is pruning, device-TP1 residuals, MFMA, wave attention,
FP32-v8, then exactly one terminal selector. Synchronous uses the existing
wave/v11 selector. Ordered uses the existing atomic wave/v11/ordered selector;
no independent ordered setter follows it. The worker's runtime mode is fixed
before spawn. Both modes retain the same images and selected-head arithmetic.

The existing live ingress, scheduler and batch runtime own continuous batching,
request cancellation, private page reservation, selected-row commit and
retirement. The new entrypoint does not reproduce these algorithms. Image
admission and policy errors explicitly close the owned worker/driver. Live-loop
or output errors still close the runtime. A successful drain additionally
requires no retained requests/sequences, every page free, and no cached,
retained or quarantined pages; it does not confuse retirement with virgin
pool identity. Close records report actual execution and worker-close results.

Setup retains the existing live schema and adds `live_profile`, `submission`,
`argmax_mode`, exact argmax image identities, and the actual ordered Boolean.
Authority remains none; performance/serving qualification remains false.

## Context And Measurement Boundary

The matched 128-input/128-output cell is representable with `--context 8192`
and `--pages 512`: each request reserves 16 pages. The unchanged v5 wave root
admits 8192 logical positions and 512 physical pages. Its loop bound is the
configured context, so a canary measured at context256 is not a timing proxy
for context8192. Full fixed-oracle qualification at context8192 is required.

The 512-page limit is 8192 aggregate retained token slots, not 8192 slots for
each of 32 active requests. A 4096-input/256-output request reserves 272 pages;
this profile can admit only one such request concurrently. Larger pools and
their wave/v11/ordered composition are separate, unimplemented work.

HTTP configuration only needs a newly pinned literal argv pointing at this
binary. Existing HTTP client timing, SSE bytes, usage, warmups and aggregation
must remain unchanged. The new profile needs fresh source/binary/image/config
pins and before/after exact token/byte diagnostics. Do not reuse a canary
workload digest for a live request stream or compare preparation-only spans
against synchronous submit-and-wait spans as GPU timings.

## Source Gate Plan

Six parser/contract tests cover both modes, the matched envelope, mandatory
and duplicate flags, unsupported profiles, context/page boundaries, runtime
identity drift, and option-shaped paths. Four entrypoint tests cover exact
profile metadata, pre-effect rejection, unconditional close on success/failure,
and cancellation/retirement without false virgin-pool assumptions. A source
policy test binds real admission/configuration order, the terminal selectors,
shared live runtime and unchanged frozen entrypoints. These fixtures do not
execute kernels or establish native model parity.

Reuse existing recording tests for rows1/16/17/32 with empty/last/sparse/all
selections, ordered [10,5] groups, head barriers, capability rejection, poisoned
completion and quarantine. Host-gate plans must also retain the old CLI/source
policy and HTTP adapter suites. All builds/tests/formatting are remote-only
and require their own reviewed bounded stage plan.

Native follow-up, separately authorized: exact context8192 model diagnostics
before/after the matched HTTP workload, page and context boundary fixtures,
mixed request selections, cancellation/re-admission, and queue rollover. Keep
all failed attempts. No native dispatch or performance claim is made here.
