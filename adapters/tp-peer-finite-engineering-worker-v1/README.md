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

## Build Prerequisites

The finite worker uses `../../../fe2o3/crates/fe2o3-kfd`, a sibling checkout
relative to this repository. It requires the separately retained fe2o3
engineering generation that supplies the finite gfx950 and host-observation
APIs. An arbitrary fe2o3 checkout, or the parent adapter's pinned fe2o3 Git
revision, is not an equivalent dependency. Publishing this Ferric source
snapshot does not yet provide a self-contained clean-clone reproducer.

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
checks. CPU tests do not launch the GPU experiment.

## Evidence Limits

The current CPU cohort passed 605 tests with four ignored. The earlier
uninstrumented GPU generation passed four teacher-forced and four
autoregressive forwards with bitwise native-baseline parity. The newer
host-instrumented generation is separately qualified; it cannot inherit that
GPU result merely because its CPU tests pass.

Host counters are inclusive and nested. Do not sum them as disjoint costs or
label them GPU time, kernel overlap, or tokens/s. Admission caching, shared
currentness, operational currentness and raw timestamp queues stay off in the
baseline diagnostic. Full tensor acceptance, the 2,048/256 target workload,
performance qualification and production proof remain open.
