# Finite Prefix Host Policies

This opt-in engineering diagnostic isolates two host-overhead optimizations
for the four-forward gfx950 prefix route. It does not change production
admission or the V1 diagnostic. The V2 baseline and admission-cache arm have
completed on `mi350`. The repeated baseline and shared-currentness arm remain
pending; no speedup is reported here.

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
remain gaps. The previous V1 GPU result does not qualify these new binaries;
their separate V2 baseline result follows.

## GPU Baseline

The freshly built CPU633 parent and worker completed one four-forward,
teacher-forced baseline diagnostic on `mi350`. All 152 retained tensor rows
matched the prior native route bit-for-bit. Three pre-run and three post-run
device-state audits passed; all owned processes exited and were reaped without
forced cleanup. All 57 final output files were independently rehashed after
local retention. The [baseline record](assets/finite-prefix-v228/host-policy-baseline.json)
binds these results to the actual binaries, deployment, controller and sidecar.

| Position | Forward Wall (ms) | Full Currentness R0 (ms) | Full Currentness R1 (ms) | Admission R0 (ms) | Admission R1 (ms) |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0 | 18,266.096 | 8,529.099 | 8,492.171 | 14.202 | 10.533 |
| 1 | 18,304.417 | 8,544.323 | 8,511.074 | 14.190 | 10.485 |
| 2 | 20,174.308 | 9,481.453 | 9,443.423 | 14.096 | 10.484 |
| 3 | 20,171.139 | 9,476.559 | 9,445.684 | 14.115 | 10.574 |

Setup's snapshot interval was 174.817 seconds; Close took 26.475 seconds.
The parent diagnostic took 421.828 seconds, while the surrounding controller
took 439.514 seconds. These are different timing boundaries. The table records
inclusive, nested host scopes, not additive costs or GPU time. Both optional
optimizations were disabled. This single baseline is not an ablation, a speedup claim,
independent full-model acceptance or the sustained 2,048/256 workload.
The V2 autoregressive diagnostic remains unrun.

## Admission Cache Observation

The next single-run diagnostic enabled only `immutable-admission-cache`,
using the same CPU633 binaries, device images, model, prompt and numerical
checks. All 152 native tensor rows again matched bit-for-bit. All six device
audits passed, and all owned processes exited and were reaped without forced
cleanup. All 57 final outputs were independently rehashed locally. The
[cache record](assets/finite-prefix-v228/host-policy-cache.json) retains its
own receipt, policy snapshots and host counters.

The intended counter change occurred: per-forward repeated kernel admissions
fell from 148 / 145 on ranks 0 / 1 to zero on both ranks, with zero time recorded
in that repeated-admission scope. Setup still performed the required 9 / 6
initial image admissions. Dispatch counts remained 148 / 145 per forward;
full-currentness counts, shared publication counts, and I/O counts and byte
extents were unchanged. Operational-currentness shortcuts, shared group
currentness and raw timestamps remained off in all seven snapshots.

| Position | Baseline Forward Wall (ms) | Cache Forward Wall (ms) | Cache Minus Baseline (ms) |
| --- | ---: | ---: | ---: |
| 0 | 18,266.096 | 18,368.441 | +102.345 |
| 1 | 18,304.417 | 18,268.495 | -35.921 |
| 2 | 20,174.308 | 20,133.830 | -40.479 |
| 3 | 20,171.139 | 20,162.169 | -8.971 |

This removes the intended repeated work, but these two diagnostic runs do
not establish an overall latency improvement. The four forward durations sum
to 76.916 seconds for baseline and 76.933 seconds for cache. Their positions
are different steps, not independent repetitions of one workload; no
confidence interval or speedup claim is justified. The largest measured host
scopes remain full-currentness checks. The cache setup interval was 167.157
seconds, Close 26.697 seconds, parent 414.917 seconds and controller 432.616
seconds. Do not attribute the cross-run setup difference to caching when
initial admission and setup operation counts did not change.

The next comparison is a fresh baseline repeat, then the separate
`shared-full-currentness` arm. This is pre-qualification host-overhead
diagnosis, not completion of issue #42's M6 performance gate.

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
