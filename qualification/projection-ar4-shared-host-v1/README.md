# Projection AR4 Shared-Full Host Route

An explicit, opt-in shared-full currentness route for the existing four-forward
projection-residual observer. Built and tested on ASROCK (`mi350-2`) on
2026-10-05 UTC, then integrated from the exact formatted source used by the run.
This checkpoint qualifies the Rust adapter and regression tests, not GPU execution
of the new route, numerical acceptance, or a measured speedup.

## Implementation

The [previous MI350 observation](../projection-ar4-host-native-v1/README.md)
measured thousands of full-currentness checks per forward. This route uses the
existing runtime `configure_performance_v2(false, false, true)` before observer
enable: admission caching and operational currentness remain disabled; shared-full
group checking is enabled. Raw timestamps independently remain disabled.

Each shared group fence still performs fresh topology discovery, all-rank mutable
device checks, generation rechecking and queue checks. No topology snapshot is
cached across operations. This explicitly changes group-fence scheduling; it is
not a claim that the default policy is unchanged within the shared route.
The ordinary/default routes, their public decoders and regression tests remain.
There are no fe2o3, compiler, provider, driver, kernel or HSACO changes here.

The new `tp-batch-engineering` parent binary is:

```text
ferric-qwen3-finite-projection-residual-decode-shared-host-engineering
  --request /absolute/request.json
  --allow-unauthenticated-machine-code
  --observe-projection-shared-host
```

Arguments are passed together in that order. The existing request must select
autoregressive mode. The parent selects the distinct worker flag
`--engineering-native-projection-residual-decode-shared-host-v1` and publishes
`FerricFiniteProjectionResidualDecodeSharedHostDiagnosticV1` only after healthy
Close, EOF and child reap. The unchanged worker loop retains own-output recurrence,
bootstrap/image bindings, control/payload identities and the exclusive sidecar.

The entire sidecar is pinned under the existing 64 KiB cap. Its closed
`FerricProjectionResidualDecodeSharedHostEnvelopeV1` contains policy `shared-full`,
an unsigned `configuration_host_ns`, and a distinct shared observation schema.
All seven snapshots require shared-full mode; the old V1 decoder rejects them.
Configuration time precedes observer enable and is outside the zero-baseline
snapshots and six intervals. It is not hidden in, or subtracted from, a forward.

## Actual Qualification

| Scope | Result |
| --- | --- |
| Worker library | 519 passed, 4 historical ignores |
| Worker `shared_wire` | 13 passed |
| Selected parent library and binaries | 351 passed |
| Rust total | **883 passed, 4 historical ignores** |
| Bounded build/test phases | 63 completed naturally, all exit 0 |
| Selected production executables | 4 built and pinned |
| Source, dependencies and old-target postchecks | Passed |
| Controller policy tests | 15 passed |
| Separate cache-cleanup scope tests | 12 passed |

Twenty new test methods yield 28 added executions because eight data tests compile
in both crates. All 855 baseline executions and seventeen previous parent binary
harnesses remain selected. The historical runtime's 208 tests are not rerun or
included in the total. This is not a fresh compiler qualification.

Tests cover fixed CLI routes, configuration-before-enable and failure ordering,
legacy/shared schema cross-refusal, zero baselines, rank/epoch/counter validation,
own-output recurrence, worker/profile/projection binding, actual-file payload
drift, missing/oversized sidecars and Close/reap failures. The setup test double
does not establish native sysfs/DRM behavior or GPU fault coverage.

The controller copied the actual CPU855 paired source, checked ten replacement
preimages and four additions, and formatted only thirteen Rust bodies. The
7,003-file source roster, Cargo metadata and dependencies were checked. Bounds
remained CPU8/9, nice10, hidden GPUs, 12 GiB address space per leaf, 6 GiB aggregate
target, 40/38 GiB free-space floors and the original command deadlines.

## Evidence

- [Actual completion](raw/complete.json): `deb2aeacded08c336d1fb5a1638cd2accc2b65ec65ffa3e90177bd5e61146d74` (466,118 bytes).
- [Executed controller](controller/run.py): `1f9fc61f35cc2d8367592ecf29ff7ea48e6d6250556a72940abe3acb028a41c5`.
- [Primary preparatory test observation](raw/preparation-tests-primary.json): `d349e4e8f6ca28771c1ea0b7a74cfbecf38e61013697d9aa6a802938bdf407c2`.
- [Source proposal](proposal/source-manifest.json): `faaece5ab1afea85b7b6f03b02d772847889565a0ae70922180d6f62a1770461`.
- Actual formatted source map: `ed98abb445b2dc3209fcd575bfb25e817b86b7bce0dbdb5f299984067149f3b4`.

`raw/` contains the completion and all 315 five-file phase records. Seven large
source/configuration/old-target maps remain outside Git; their original pins are
in the completion. All 322 raw records and fourteen tested source bodies were
downloaded and verified. The retained 337-member, 2,800,326-byte archive has SHA-256
`387e0fabea38c433096799a4b155a8a66ec817d00a8c2a15710b0dbfb172b0a6`.
`source-overlay/` is byte-identical to the integrated files. Original authored-only
wording in copied proposal/controller documents is preserved; actual results here
supersede it. The four executable bodies remain at their recorded remote paths.

| Executable | Bytes | SHA-256 |
| --- | ---: | --- |
| Shared observer parent | 13,948,728 | `8214d2f3c2237d243c5ad97c109fdda3558d22caff3ed17fa0805b54eae27da5` |
| Default observer parent | 13,947,904 | `a5f8323b87f475f28dea91120742204e9e84b231257a42699967c08b35ea4ace` |
| Plain AR4 parent | 13,925,384 | `afab676b199de3bb89c49e66862316609389eea50e0df2870841b1ca97ac1345` |
| Worker | 5,176,608 | `3d36a6a53a2ab952a57b7e4784c8b2607348c45420b77b9a34e9483bef9218a8` |

Before building, a separate [cache cleanup](storage/complete.json) removed 4,008
direct-dependency `.rlib`/`.rmeta` cache files, reclaiming 21,333,028,864 allocated
bytes. All 34,854 protected bodies were unchanged. No executable, shared library,
GPU image, source or non-target file was removed. The full reviewed plan and
per-file removal journal remain outside Git at the completion's exact pins.

## Next Comparison

Fresh MI350 audits must bind both new observer parents and the same new worker.
Then run default and shared modes with identical kernel/model/prompt inputs and
compare the four actual input histories and all 152 tensor slices. Byte equality
would establish route repeatability, not independent model accuracy.

Compare rank-full, group-full and publication-full currentness scopes together;
do not add nested command/read/write/dispatch timers or call them GPU time.
Report configuration, setup, forwards, serialization and Close separately.
No measured shared-route speedup, sustained 2,048/256 result, 700 tokens/s,
production authority or issue #42 milestone closure follows from this checkpoint.
