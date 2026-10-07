# Owned Kernel Admission Opportunity

Source-only proposal. No build, test, loader call, benchmark or remote command
has been executed for it. There is no native runtime change, model operation,
new unsafe code, KFD dependency, GPU selection, cache flag or launch authority.
Root owns the eventual serialized MI350 execution and its original evidence.

## Question

The qualified warm layer prepares two Prefix and eight paired-MLP dispatches.
Each preparation currently reconstructs a loader closure from immutable owned
object bytes. Is that repeated pure admission work expensive enough to justify
a separately reviewed private lexical reuse path? This benchmark answers only
that opportunity question, not end-to-end gain or Full2303 feasibility.

The earlier source/timing analysis is
`../model-full-scoped-tail-v1/next-overhead-review.md`, SHA256
`6136f7bcab0b707dc67dcf5a52eb3d1bb999cd53f7836fd5f23b359c9bd49366`.
Its proposed currentness, ownership, dynamic fixup, deadline, numerical and
policy constraints are not changed by this pure program.

## Exact Work

One sample repeats 36 layers. Each has ten ordered preparations: Prefix rank
0, Prefix rank 1, then R1/MLP/guard/R2 for rank 0, then those four for rank 1.
The four original images contain five selected symbols; guard and R2 share an
object but select different kernels. No embedding, tail, device, checkpoint,
tensor or synthetic replacement image is included.

| Image copy name | Bytes | Original SHA256 |
| --- | ---: | --- |
| prefix.hsaco | 54344 | 29fd58e7b09fee003ed31660e6201c20a7eb5da4b873d0b4d954993f7018f2f8 |
| projection.hsaco | 10864 | 25338beac39121cf81adfdc5c82c6f39bcfc82c0f7effaa2166bf8a2a9598a25 |
| mlp.hsaco | 33320 | b0d1766fdb0fda17ab09496d1120ba3cee29100692b48fcc5b05cf0db0a8d589 |
| guarded.hsaco | 28440 | de880dcebf79425ccb555c1a2bdca3ff78b926f7b2062d06b918763da4de0f66 |

The original source of these pins is Ferric's retained
`qualification/guarded-mlp-scoped-tail-v1/matched-timing-gpu-v2/tail-request.json`,
9171 bytes / `430c00268bd29dbd0861e0e75b5e4470f8041bc19849ec1d616f71f3864f0b11`.
`source-manifest.json` retains the exact remote original paths and the source
contracts that fix the five symbols and preparation order. The four image
bodies were not assumed present or loaded here. Root must hash the originals
on MI350 against these exact pins, make fresh ordinary single-link copies with
the four names above, and retain original-to-copy identities. The executable
independently checks exact length/hash and inode/metadata stability before and
after all samples. No caller-supplied hash, symbol, profile or workload knob
can change the closed roster.

Arm Fresh calls the actual safe `validate(bytes, Gfx950XnackOffCov6)` and
`bind_kernel(symbol)` each time. Arm Owned first creates ten distinct owned
closures with `validate_owned(...).bind_kernel(...)`, then calls `validated()`
on the corresponding closure each preparation. These ten rank-role owners
reuse the same admitted image contents, matching the loaded role reuse rather
than constructing 360 new owners per sample.

Before any samples and again afterward, the program directly compares both
arms' load plans, complete selected `InspectedKernel`, selected index,
descriptor binding, resources, original descriptor and entry bytes, closure
identity and relocation evidence. Object identities must equal the fixed
original image hashes. Every owned closure reports one semantic binding pass.
No debug-string or digest-only approximation replaces those direct Eq checks.

Both timed arms call the exact same `consume` function with `black_box` over
the same descriptive fields. Both perform the same bounded digest-byte sum,
deadline checks and closure destruction. Each timed sample must consume 360
preparations and yield the same sum. The sum is only a dead-code-elimination
guard, not an equality proof; exact equality is established separately against
owned immutable bytes. Object authentication and Eq comparisons are outside
the timed arms. Owned cold creation, including input copies, is recorded
separately; it is not an estimate of native startup cost.

## Fixed Limits And Output

- Two warmup pairs and twelve measured pairs; every pair contains both arms.
- Alternating Fresh-first / Owned-first order, exactly six measured pairs of
  each order. No random search, adaptive repetitions, outlier removal or retry.
- Each arm samples 36 x 10 preparations. Per-image preparation counts are
  `[72, 72, 72, 144]`. Four original input bodies total 126968 bytes.
- Internal wall deadline 60 seconds from entry through final serialization;
  checked before and after each preparation. Blocking reads/calls still require
  the outer owner's wall enforcement.
- Root must impose 45 seconds CPU, 256 MiB address space, 128 KiB stdout and
  64 KiB stderr, CPUs 8/9, nice 10 and hidden GPUs before starting the leaf.
  The program explicitly reports that it does not verify external limits.
  Retain the actual imposed limits and natural/reaped/group-absent result.
- No per-sample clock ratio or float result. Original warmup and measured
  elapsed nanoseconds, preparation counts and consumption sums are emitted as
  JSON integers. All raw samples remain, even if Owned is slower.

The output schema is `ferric-owned-kernel-admission-opportunity-v1`. Success
requires all pre/post custody, exact equality and sample checks. Failure emits
an error and nonzero exit, not a success report; root retains original stdout,
stderr and lifecycle evidence without promoting a partial attempt. No outcome
pin, target saving or expected timing is bound in this proposal.

The output has explicit false fields for native/model execution, dynamic
dispatch-validation measurement, end-to-end gain, Full2303 feasibility and
numerical acceptance. `passed` means this pure bounded comparison completed,
not that reuse was faster or an optimization is admitted.

## Standalone Build Layout

There is no new supervisor, transport, retainer, generic harness or canonical
Cargo edit. This proposal owns only `bench/Cargo.toml` and `bench/src` plus
metadata. The standalone crate has its own `[workspace]` boundary.

For a future root-owned build, materialize the exact reviewed dependency source
snapshot as sibling `runtime/`, making
`bench/../runtime/crates/fe2o3-amdhsa-loader` resolve. The source manifest pins
the current loader, HSACO, AMD-target and target-spec crate bodies, the runtime
workspace manifest and original lock. These are descriptive pure libraries;
none depends on KFD. Do not point at an unqualified mutable checkout silently.
Root may materialize the complete qualified runtime snapshot or the pinned
workspace subset, preserving the original inherited package/dependency fields.

Root resolves and retains a fresh benchmark Cargo.lock offline using its
authenticated private cache before qualification. No generated lock or actual
ELF is invented here. Direct registry versions are exact and already present
in the qualified runtime lock. Record the resulting dependency graph/lock and
compiler versions; refuse an unexpected dependency. Compile the benchmark in
release mode with this fixed opt-level 2, debug assertions and overflow checks
enabled. Both arms run in the same ELF and process. Do not alter flags between
arms or silently use an unoptimized test executable for timing.

The ten unit tests cover roster/order/multiplicity, fixed balanced samples,
image extent/digest drift, limits/deadline boundary, integer overflow,
consumption accounting, malformed-object refusal in both actual loader APIs,
and missing-roster refusal. Positive actual-image equivalence is mandatory in
`run` before timing; those images are not fabricated as local unit fixtures.
All tests/builds/runs are pending MI350 execution by root, after the active
Full coupled job. Existing actual library qualification does not qualify this
new crate or substitute for its tests.

## Interpretation

The Owned arm reconstructs an existing borrowed descriptor from its owned
closure; it is not a runtime pointer-fixup benchmark. Fixed deadline-clock and
consumption overhead can be a large fraction of its small duration. Timing
includes those equal per-iteration costs and destruction; it does not time
serialization, file hashing, equality checks, cold construction or printing.
CPU cache state and ordering are part of this one-process experiment. Retain
every sample and do not extrapolate the result into an achieved Full runtime.

Only a material observed opportunity warrants a separately reviewed explicit
lexical runtime experiment. That later route must retain all dynamic checks,
unchanged currentness cadence, finite/argmax/custody/deadlines and truthful new
policy bytes, never reinterpret old `cache_kernel_admission=false`. The
independent 2048/256 correctness gate and unresolved model mismatch remain.
