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

## Build Prerequisites

The finite worker uses `../../../fe2o3/crates/fe2o3-kfd`, a sibling checkout
relative to this repository. Its matching engineering runtime is published at
[fe2o3 `6964f6129`](https://github.com/harsh-nod/fe2o3/commit/6964f6129c4c42d343117f61cdbcfe6166392535),
also available in the [mirror](https://github.com/powderluv/fe2o3/commit/6964f6129c4c42d343117f61cdbcfe6166392535).
An arbitrary fe2o3 checkout, or the parent adapter's pinned fe2o3 Git revision,
is not an equivalent dependency. The parent dependency graph remains unchanged.

The original Ferric snapshot `9eca257697069f10c87ee9f624016391f6a2e479` and
this fe2o3 commit were freshly extracted from Git archives on `mi350-2` into
sibling `ferric/` and `fe2o3/` directories, with an empty Cargo target.
All 387 selected worker tests passed, four were ignored, and the worker binary
built. All 6,921 extracted files remained unchanged. Cargo metadata selected
exactly nine local runtime crates from the fresh fe2o3 tree, the fresh Ferric
worker, and 29 registry packages; no original development source path was used
as a local package. The existing registry cache and toolchain were reused.
See the [fresh-build record](../../docs/assets/finite-prefix-v228/clean-worker-build.json).
This is a worker source-build result, not a parent build, cold toolchain setup,
Rust-to-HSACO reproduction, or GPU qualification of the newly built binary.

To select the same source pair in a new working directory:

```sh
git clone --branch codex/p228-finite-runtime-integration-v1 https://github.com/harsh-nod/fe2o3.git fe2o3
git -C fe2o3 checkout --detach 6964f6129c4c42d343117f61cdbcfe6166392535
git clone --branch codex/p227-finite-prefix-integration-v1 https://github.com/harsh-nod/ferric.git ferric
git -C ferric checkout --detach 9eca257697069f10c87ee9f624016391f6a2e479
cd ferric
```

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
three runs do not establish a reliable speedup. The shared-currentness arm
and V2 autoregressive diagnostic remain pending. The earlier
uninstrumented GPU generation passed four teacher-forced and four
autoregressive forwards with bitwise native-baseline parity. The V1
host-instrumented (CPU605) generation has separately completed its teacher-forced
run with all 152 native tensor rows bitwise equal, six passing device audits,
and clean Close/reap. Its instrumented autoregressive run is still pending.

Host counters are inclusive and nested. Do not sum them as disjoint costs or
label them GPU time, kernel overlap, or tokens/s. Admission caching, shared
currentness, operational currentness and raw timestamp queues stay off in the
baseline diagnostic. Full tensor acceptance, the 2,048/256 target workload,
performance qualification and production proof remain open.
