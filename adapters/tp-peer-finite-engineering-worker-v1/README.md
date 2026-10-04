# Finite TP2 Engineering Worker

This standalone adapter implements bounded parent/worker experiments for
Qwen3 on gfx950. It is not a production execution path or a sustained decode
performance result. See the [validation checkpoint](../../docs/GFX950_FINITE_PREFIX_PROGRESS_V1.md).

## Layout

- `finite_*_wire_v1.rs`: typed requests, generations, bounded payloads and Close
  messages shared with the parent adapter.
- `resident_layer/`: checked image, allocation, dispatch and readback ownership
  for finite layer experiments.
- `native_prefix_tiles_decode_v6.rs`: the four-forward prefix/MLP composition.
- `prefix_decode_host_observation_v1.rs`: the bounded host-counter record.
- `native_prefix_decode_host_v1.rs`: explicit diagnostic opt-in, snapshots and
  separate sidecar publication after normal shutdown.

The parent entry point is
`adapters/m1-engineering-execution-v1/src/bin/ferric-qwen3-finite-prefix-decode-host-engineering.rs`.
It requires `--allow-unauthenticated-machine-code` and `--observe-host` for the
diagnostic route. This opt-in does not constitute production admission.
Other finite experiments remain distinct typed routes, not silent fallbacks.

The separately versioned V2 route adds three explicit host policies: baseline,
immutable-admission caching, and shared full-currentness fences. See the
[policy guide](../../docs/GFX950_HOST_POLICY_V2.md) for the new parent command,
request schema, retained mutable checks, and measurement limits. V1 keeps its
all-optimizations-off behavior.

## Raw Device Diagnostic

The worker's `--engineering-native-prefix-decode-device-v1` selector uses
fresh timestamp-enabled queues and requires `--device-sidecar` with a new
absolute output path. It retains the same Four wire, model roots, kernel
images, launch geometry and native state checks. It records prefix, MLP,
both residual pairs and all five dependent embedding/copy/tail operations:
293 records per forward, 1,172 for the four-forward diagnostic.

Each record comes from a native completion-signal observation and the selected
loaded kernel. Packet order, queue/device identity and input image hashes are
checked; all host intervals must match the ordinary control payload. The report
becomes available only after consuming native Close and successfully writing
the Closed response. The output is bounded to 2 MiB. Existing ordinary and host
diagnostic selectors do not enable timestamp queues.

The [matching parent diagnostic](../m1-engineering-execution-v1/README.md#finite-prefix-device-tick-diagnostic)
is now implemented and separately CPU-qualified. The pair's
[MI350 GPU capture](../../qualification/native-device-observation-v1/README.md)
passed: all 1,172 raw rows, four unchanged payloads and 152 unchanged tensors,
six audits, consuming Close and natural reap.
Both transported binaries pass [MI350 runtime dependency audits](../../qualification/native-device-runtime-v1/README.md);
the first controller admission stopped on a legacy-import defect before native launch.
The corrected controller passed 47 tests before this successful attempt.
Raw ticks are not nanoseconds; cross-device alignment and
overlap graphs require separate clock calibration. Single-command raw rounds
also add a publication fence, so their host timings are not a like-for-like
optimization of the ordinary single-command path.

The [CPU qualification](../../qualification/native-device-routing-v1/README.md)
retains the fresh `mi350-2` build: 609 Rust tests passed, four existing tests
were ignored, and 13 controller tests passed. Its source and artifact records
cover this worker route, not the separate parent qualification or a GPU run.

## Clock Recorder V2

The separate `--engineering-native-prefix-decode-device-clock-v2` selector
retains the complete V1 raw dispatch report and adds sixteen native clock
samples: before and after each of four forwards, on both ranks. Each sample
binds the generation, packet boundary, device identity, queue epoch and a
monotonic host sampling bracket. GPU, CPU and system counters remain raw;
the system counter frequency is not treated as the GPU frequency.

The recorder consumes the existing native Close result before publication.
Sampling, wire, capture and control failures poison both the recorder and
the real owner, including failures discovered after a forward has committed.
Existing plain, host-only and raw-device V1 routes remain separate.

The [clock-recorder CPU qualification](../../qualification/gfx950-clock-recorder-v1/README.md)
passed 669 selected Rust tests with four existing ignores and fifteen
controller-policy tests on `mi350-2`. This qualifies the worker build and
synthetic failure boundaries, not a native clock sample. The
[matching parent](../../qualification/gfx950-clock-parent-v1/README.md) now has
its separate passing CPU qualification. GPU capture and calibration remain
separate gates; no duration,
cross-device alignment, overlap or throughput claim follows from this result.

## Build Prerequisites

The finite worker uses `../../../fe2o3/crates/fe2o3-kfd`, a sibling checkout
relative to this repository. Its matching engineering runtime is published at
[fe2o3 `27b53d2b7`](https://github.com/harsh-nod/fe2o3/commit/27b53d2b74c1f239988a891a4aed39e089b05663),
also available in the [mirror](https://github.com/powderluv/fe2o3/commit/27b53d2b74c1f239988a891a4aed39e089b05663).
An arbitrary fe2o3 checkout, or the parent adapter's pinned fe2o3 Git revision,
is not an equivalent dependency. The parent dependency graph remains unchanged.

The original Ferric snapshot `9eca257697069f10c87ee9f624016391f6a2e479` and
the earlier fe2o3 `6964f6129c4c42d343117f61cdbcfe6166392535` were freshly
extracted from Git archives on `mi350-2` into
sibling `ferric/` and `fe2o3/` directories, with an empty Cargo target.
All 387 selected worker tests passed, four were ignored, and the worker binary
built. All 6,921 extracted files remained unchanged. Cargo metadata selected
exactly nine local runtime crates from the fresh fe2o3 tree, the fresh Ferric
worker, and 29 registry packages; no original development source path was used
as a local package. The existing registry cache and toolchain were reused.
See the [fresh-build record](../../docs/assets/finite-prefix-v228/clean-worker-build.json).
This is a worker source-build result, not a parent build, cold toolchain setup,
Rust-to-HSACO reproduction, or GPU qualification of the newly built binary.

To select the current integration branch and its required runtime:

```sh
git clone --branch codex/p228-finite-runtime-integration-v1 https://github.com/harsh-nod/fe2o3.git fe2o3
git -C fe2o3 checkout --detach 27b53d2b74c1f239988a891a4aed39e089b05663
git clone --branch codex/p227-finite-prefix-integration-v1 https://github.com/harsh-nod/ferric.git ferric
cd ferric
```

The branch tip is mutable. Each qualification record identifies the exact
source archives and overlay used for its test results; the historical build
above does not qualify later changes.

The tested worker toolchain is `nightly-2026-04-03`; the parent suite used
Rust `1.97.1` and `RUSTC_BOOTSTRAP=fe2o3_device,fe2o3_macros`. Both used two
build/test threads, disabled incremental compilation and optimized test
profiles with debug assertions and overflow checks retained. After provisioning
the exact dependencies and Cargo cache, the core CPU selections are:

```sh
CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_TEST_OPT_LEVEL=2 \
  cargo +nightly-2026-04-03 test --offline --locked --jobs 2 \
  --manifest-path adapters/tp-peer-finite-engineering-worker-v1/Cargo.toml \
  --lib --test shared_wire -- --test-threads=2

CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_TEST_OPT_LEVEL=2 \
  RUSTC_BOOTSTRAP=fe2o3_device,fe2o3_macros \
  cargo +1.97.1 test --offline --locked --jobs 2 \
  --manifest-path adapters/m1-engineering-execution-v1/Cargo.toml \
  --features tp-batch-engineering --lib tp_finite_client:: -- --test-threads=2
```

These two commands are only part of the recorded 34-command suite. They do
not reproduce its complete source, compiler, resource and process-ownership
checks. The fresh build ran the worker selection above, not the parent selection.
It then built the worker with `cargo build --offline --locked --jobs 2` using
the same manifest and `CARGO_PROFILE_DEV_OPT_LEVEL=2`, with debug information
disabled. The test suite retained debug assertions and overflow checks.
CPU tests do not launch the GPU experiment.

## Evidence Limits

The original CPU cohort passed 605 tests with four ignored. The newer V2
host-policy cohort passed 633 with four ignored from fresh source archives
and its exact overlay. All 12 binaries built and the parent default-library
check passed. Its separate [record](../../docs/assets/finite-prefix-v228/host-policy-cpu.json)
is CPU-only. Those new binaries have separately completed the V2 teacher-forced
baseline: all 152 retained native tensor rows matched, six device audits passed,
and Close/reap completed without forced cleanup. See the
[GPU record](../../docs/assets/finite-prefix-v228/host-policy-baseline.json).
The separate V2 admission-cache arm also passed those parity, audit and teardown
checks. Repeated admissions fell to zero, but no overall latency improvement
is established by these single diagnostic runs; see the
[comparison](../../docs/GFX950_HOST_POLICY_V2.md#admission-cache-observation).
The [baseline repeat](../../docs/GFX950_HOST_POLICY_V2.md#baseline-repeat)
also passed parity, six audits and clean teardown. Its total forward duration
exposes baseline drift larger than the initial aggregate cache delta; these
three runs do not establish a reliable cache speedup. The separate
[shared-currentness arm](../../docs/GFX950_HOST_POLICY_V2.md#shared-currentness-observation)
passed the same parity, audit and cleanup gates, with a 61.458-second forward
total versus the baselines' 76.916 and 77.471 seconds. This unisolated diagnostic
observation is not a qualified decode speedup; the linked plot and table retain
the counter changes and limits. V2 autoregressive validation remains pending. The earlier
uninstrumented GPU generation passed four teacher-forced and four
autoregressive forwards with bitwise native-baseline parity. The V1
host-instrumented (CPU605) generation has separately completed its teacher-forced
run with all 152 native tensor rows bitwise equal, six passing device audits,
and clean Close/reap. Its instrumented autoregressive run is still pending.

The later [consolidation candidate](../../docs/GFX950_HOST_POLICY_V2.md#consolidation-candidate)
uses runtime commit `725ecc6a500ff49e7dfaefb38b027f6bcc223ebf`. It passed 77
selected runtime tests and 398 worker/shared-wire tests with four ignored,
and built a distinct worker binary. All retained raw records and the binary
were rehashed locally. The original A/B/A/C graph still uses the older worker.
The new worker's [separate GPU comparison](../../docs/GFX950_HOST_POLICY_V2.md#consolidation-gpu-observation)
passed all 152 native rows, six audits and clean Close/reap. Four-forward host
time was 30.394 seconds versus 61.458 seconds for the old shared worker, with
18,720 duplicate full checks removed and unchanged dispatch/transfer counts.
This single unisolated diagnostic is not a sustained decode speedup claim.

Host counters are inclusive and nested. Do not sum them as disjoint costs or
label them GPU time, kernel overlap, or tokens/s. Admission caching, shared
currentness, operational currentness and raw timestamp queues stay off in the
baseline diagnostic. Full tensor acceptance, the 2,048/256 target workload,
performance qualification and production proof remain open.
