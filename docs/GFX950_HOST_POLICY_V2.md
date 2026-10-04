# Finite Prefix Host Policies

This opt-in engineering diagnostic isolates two host-overhead optimizations
for the four-forward gfx950 prefix route. It does not change production
admission or the V1 diagnostic. No V2 GPU run or speedup is reported here.

## Policies

| Request policy | Immutable admission cache | Shared full-currentness fence |
| --- | --- | --- |
| `baseline` | Off | Off |
| `immutable-admission-cache` | On | Off |
| `shared-full-currentness` | Off | On |

All three explicitly configure a fresh peer group before enabling observation
or allocating/uploading data. Operational-currentness shortcuts, raw GPU
timestamps and legacy profiling remain disabled. There is no combined policy,
automatic selection, retry or silent fallback. Unknown policies and late or
repeated runtime configuration are rejected.

The admission cache avoids repeating immutable kernel-object validation and
binding during dispatch preparation. Initial image admission still runs.
Current pointer ownership and extent, aliasing, ABI, dispatch geometry,
workgroup/grid limits, idle/completion frontier and queue checks remain.

Shared full-currentness performs one fresh topology discovery at each group
fence, bracketed by each rank's mutable checks. Every retained rank snapshot
and generation is rechecked; a failure poisons the group. It is not a topology
cache across calls. Individual I/O, wait and lifecycle checks remain. Round
publication already shares a fresh observation in the baseline, so that
existing behavior is not a contribution of this optimization.

These mechanisms live in the published
[fe2o3 runtime](https://github.com/harsh-nod/fe2o3/commit/6964f6129c4c42d343117f61cdbcfe6166392535).
Ferric selects and observes the policy; it does not replace the runtime checks.

## Request And Execution

Wrap a complete existing finite-prefix V1 decode configuration in a JSON
object with these fields:

- `schema`: `FerricFinitePrefixDecodeHostPolicyRequestV2`
- `policy`: one of the three names in the table
- `decode`: the complete, unchanged V1 configuration object

The new parent entry point is separate from the V1 diagnostic:

```sh
ferric-qwen3-finite-prefix-decode-host-policy-engineering \
  --request /absolute/policy-request.json \
  --allow-unauthenticated-machine-code --observe-host-policy
```

The request is limited to 64 KiB. The sidecar path is derived from the decode
evidence directory as `<evidence-directory>-host-policy-v2.json`; it must be
absent before launch. The parent binds the requested policy to the actual
worker binary, bootstrap, transcript and all seven recorded snapshots. It
validates the sidecar only after four completions, Close, EOF, successful
child exit and owned-process reap, before publishing the summary. A missing,
stale, mismatched or oversized sidecar fails the diagnostic.

This remains an explicitly unauthenticated-machine-code engineering route,
not authority to launch arbitrary artifacts through the production engine.

## CPU Validation

On `mi350-2`, fresh Ferric `9eca2576` and fe2o3 `6964f6129` Git archives plus
the exact 13-file V2 source overlay passed 633 tests with four ignored. The
[validation record](assets/finite-prefix-v228/host-policy-cpu.json) and
[source overlay](assets/finite-prefix-v228/host-policy-source.json) identify
the tested bytes and selected binary hashes.

| Selection | Passed | Ignored |
| --- | ---: | ---: |
| Worker library | 385 | 4 |
| Shared wire integration tests | 13 | 0 |
| Disjoint parent library selections | 224 | 0 |
| Eleven parent CLI tests | 11 | 0 |
| Total | 633 | 4 |

All 12 binaries built, and the parent's default-library check passed. The
runner checked actual test inventories against named outcomes, rather than
adding overlapping filters. The new V2 tests cover policy selection, strict
request parsing, policy/flag mismatches, stale identities, malformed sidecars,
and publication ordering. Shared data tests execute in both parent and worker.

The run used an initially empty target, two CPU cores and two Cargo/test
threads. Worker compilation used `nightly-2026-04-03`; parent compilation used
Rust `1.97.1` with `RUSTC_BOOTSTRAP=fe2o3_device,fe2o3_macros`. Dependencies and
toolchains were cached. Both test and binary builds used optimized test
profiles with debug assertions and overflow checks retained. All 6,928
post-overlay source files remained unchanged. Cargo metadata checked 39 worker
packages and 209 parent packages against their separate dependency graphs.

This is a selected CPU cohort, not a workspace-wide regression, formatting,
Clippy, compiler/HSACO reproduction or GPU qualification. Direct tests of
real-group recorder initialization and post-Close sidecar-write failures
remain gaps. The previous V1 GPU result does not qualify these new binaries.

## GPU Comparison Gates

Run each arm in a fresh worker with identical devices, images, inputs, token
history and numerical checks. Retain a baseline repeat between candidates
to expose drift, and preserve pre/post device audits and Close/reap evidence.
Only compare an arm after its own tensor checks and shutdown checks pass.

Report admission-cache and shared-currentness effects separately. Admission
counts include required load-time work, and shared-group counters are distinct
from existing publication counters. Individual full-currentness counts may
remain nonzero in the shared arm.

Recorded scopes are inclusive, nested host timings. Do not sum them as
disjoint contributions, subtract them into GPU time, infer kernel overlap or
convert four forwards into the 2,048/256 throughput target. Independent
floating-point premises, the sustained workload, equal-work baselines and
production qualification remain open; see the
[overall checkpoint](GFX950_FINITE_PREFIX_PROGRESS_V1.md).
