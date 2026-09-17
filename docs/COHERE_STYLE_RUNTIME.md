# Cohere-Style Decode Runtime On gfx950

This is the implementation direction for [issue #42](https://github.com/harsh-nod/ferric/issues/42),
not a performance result or production qualification. The model order is
Qwen3-0.6B, Qwen3-8B, then North Mini Code. The initial operating envelope remains
one gfx950 GPU, batches 1/2/4/8, contexts 1024/4096/8192, BF16 storage and a
declared FP32 accumulation and rounding policy.

## Current Evidence

The finite decoder fixture has passed four numerical cases on `mi350-2`,
checking 40,960 intermediate/final values across two 128-thread workgroups.
It is a small F32 layer, not the complete Qwen model. Rebuilding it after
integrating fe2o3 main produced byte-identical LLVM and HSACO and passed a
fresh GPU regression with the extended probe.

The [real-weight Qwen3 key projection](../qualification/gfx950-qwen3-kproj-v1/README.md)
uses the pinned Qwen3-0.6B layer-zero BF16 tensor `[1024, 1024]`, ordinary
Rust source, and eight 128-thread workgroups. Four GPU cases checked 4,096
FP32 outputs against an independent FP64 reference under a policy frozen
before execution. Zero, basis, and cancellation cases had zero maximum
absolute error; the mixed case had maximum absolute error `1.7881393432617188e-7`.
Input immutability, all allocation guards, and orderly worker cleanup passed.
This is a scalar correctness baseline, not an optimized GEMV or inference.

A separately compiled [coalesced wave64 candidate](../qualification/gfx950-qwen3-kproj-wave64-v1/README.md)
uses one wave per row, 16 FMAs per lane, and six shuffle/add reduction stages.
It passed the same 4096-value GPU workload under a separately derived gamma22
bound, with mixed-case maximum absolute error `1.1920928955078125e-7`.
No timing or speedup is inferred from the coalesced layout alone.

The [real-weight projection-to-K-RMSNorm chain](../qualification/gfx950-qwen3-knorm-v1/README.md)
now passes five GPU cases and ten dispatches. It validates 5120 projection
values against the independent bound, all 5120 intermediate BF16 keys, and
all 5120 normalized BF16 outputs exactly against the staged sqrt/div reference.
The same projection allocation is reused across exact completion without a
host rewrite. All input/guard/lifecycle checks pass. This establishes a
two-dispatch correctness baseline with real layer-0 projection and norm weights,
not a single-launch megakernel, eager-framework bitwise parity, or full Qwen
generation. RoPE, attention/KV, remaining layers, and token-loop checks remain.

The [finite atomic task graph](../qualification/gfx950-task-graph-v1/README.md)
also compiles and runs on `mi350-2`: all 20 dispatches passed, with 16 valid
epochs and four stale-epoch negatives. All 16 valid epochs used both workgroups
and observed four cross-workgroup dependency edges. Its seven-task integer DAG
checked 260 state words, including 112 exact valid-epoch payload values; the
ownership encodings are validated without assuming a deterministic schedule.

The separate [indexed atomic slice fixture](../qualification/gfx950-atomic-channel-v1/README.md)
now compiles ordinary Rust `&[AtomicU32]` through the checked source pipeline
to gfx950 HSACO and passes four GPU cases: 2048 exact integer comparisons,
unchanged inputs/guards, and complete worker cleanup. It uses two 128-thread
workgroups with two wave64 waves each. Each invocation reads its own write;
this validates the indexed storage path, not inter-workgroup tensor visibility.
The original potentially aliasing shared-input source remains rejected, and
protected runtime preparation stays closed without its memory-contract join.

The compiler now also admits [consuming read-only allocations](https://github.com/harsh-nod/fe2o3/blob/a99c9148b4c07f410902e3ff1fbee38e677b9c7b/docs/semantic-read-only-allocation-v30.md)
alongside indexed atomic state. The original exclusive allocation lease is
retained while arbitrary-index reads permit many invocations to share immutable
BF16 weight storage or FP32 inputs. Whole-kernel checks reject writes and escapes,
including uses before conversion. Both actual Rust element-type fixtures reach
checked AMDGPU LLVM; source rejection cases and the original atomic controls
pass, as do 696 compiler tests. This is compiler evidence, not an additional GPU
result, writable tensor publication, or permission to enter the protected runtime.
The projection-to-normalization GPU baseline above remains bound to its earlier
frozen compiler and has not been silently rebuilt against this new source.

This scheduler evidence is separate from the projection and decoder numerical
fixtures. Atomic integer publication does not establish ordinary cross-task
tensor visibility, KV correctness, or a production token loop. Full-model
Qwen3-0.6B/8B generation and matched ROCm performance measurements remain open;
North Mini Code follows those milestones.

## Architecture

[Cohere's runtime](https://github.com/cohere-ai/cohere-megakernel) provides the
architectural reference: a host-built tile schedule, resident GPU workers for
one decode pass, dependency counters, and stage-specific work queues for
variable attention and MoE work. Its host decode loop owns consecutive steps;
the GPU kernel is not an indefinitely resident service across all tokens.

Ferric owns the exact model graph, authenticated weights, request and KV
lifetimes, plan binding, and host step loop. fe2o3 owns reusable device
primitives, checked lowering, scheduling support, and runtime resource custody.
This is an execution backend inside Ferric, not a second inference project.

A step will reserve KV/workspace, bind the current request epoch to a finite
task plan, launch the worker grid, await exact device completion, validate its
outputs, then atomically commit request/KV state. Prefill stays a separate
checked path initially. Batch mutation and allocation reuse require quiescence;
a host timeout alone does not establish that the GPU stopped using memory.

## First Cooperative Slice

The new task-graph fixture is a scheduler test, not an ML benchmark. Its DAG is
`0 -> {1,2} -> 3 -> {4,5} -> 6`, with seven tasks and two worker workgroups.
Each 128-thread workgroup has two wave64 waves. A bounded ready-bit protocol
avoids assigning a resident consumer to wait for an unscheduled producer.

The unique task owner publishes its payload before its completion bit.
Join readiness is derived from the old value of the completion RMW: a later
load could let both producers publish the same join task. Each task is claimed
once, and every completed task must have exactly one recorded owner. The first
payload representation is atomic integer state. This does not establish
ordinary floating-point tensor publication across workgroups; that requires
its own memory-order and compiler-analysis evidence.

The recorded GPU suite checks the exact DAG outputs, completion and ownership,
stale epochs, successive launches, buffer guards, unchanged inputs, and resource
release. The separate bounded host model covers competing claims, last-arriver
fan-in, and one-resident-worker progress; the two-workgroup GPU observations are
not proof of every residency/interleaving case. Known deadlocking mutants remain
host-model tests, not experiments on a shared GPU.

The tested source is Ferric `e3d198ee`, compiled by clean fe2o3 `af4f1460` to
HSACO `8f490477...5ba328f`. The evidence package records the full digests.
No device-only timing, speedup, or full-model claim follows from this fixture.

## AMD-Specific Implementation

Do not translate Cohere's twelve CUDA warps, TMA, WGMMA, or named-barrier
protocol literally. Start with uniform, checked workgroup cooperation and
explicit atomic publication. Later tune wave roles, gfx950 MFMA versus GEMV,
LDS staging, prefetch distance, multi-buffering, and task order from profiles.
Any independently running controller wave needs a synchronization design that
does not accidentally wait for it at a worker-only barrier.

Qwen3's attention and MLP are sequentially dependent. North Mini Code's
parallel attention/MoE opportunities are not legal scheduling transformations
of the Qwen graph. Scheduling must preserve the chosen model, numerical policy,
and all scratch lifetimes.

## Performance Target And Measurement

The goal is comparable utilization and relative gains to Cohere's published
H100 results, not an assertion that a different model on MI350 must produce the
same tokens/second. Cohere reports 1.58x batch-one decode improvement and
1.25x-1.41x end-to-end improvement over its stated vLLM baseline for North Mini
Code. These are external reference targets, not Ferric measurements, and do not
establish a Qwen expectation. See [their benchmark conditions](https://github.com/cohere-ai/cohere-megakernel#performance).

For Qwen, compare against a tuned applicable ROCm vLLM/SGLang baseline and a
Ferric separate-dispatch implementation on the same allocated MI350, using
identical weights, precision, token counts, live KV lengths, batch, and numerical
policy. Count reset, embedding, logits/sampling, host scheduling, and completion
costs when they lie inside the declared whole-step boundary. Keep device-only
timing, worker submit-through-observed-completion, RPC time, and whole-token
latency separately labeled. The engineering worker's elapsed_ns is a host
clock interval including kernarg setup and completion polling, not a GPU event.

Required ablations are dispatch consolidation, ready-task scheduling, tile and
worker counts, prefetch, software pipelining, and LDS buffering. Each measured
variant needs its own executable identity and fresh numerical validation.
Report negative results and interactions; gains are not assumed additive.

Estimate an optimistic step floor as
`max(required_bytes / sustained_bandwidth, precision_flops / sustained_compute,
dependency_critical_path_floor)`. State cache/reuse assumptions, active weight
and KV traffic, and how device ceilings were measured. Report floor/measured
efficiency alongside latency and paired confidence intervals, following
[PERFORMANCE.md](PERFORMANCE.md). Do not borrow H100 bandwidth or model traffic
to manufacture an MI350 bound.

Full-model generation, same-hardware comparisons, and independent release
qualification remain required before a performance or production claim.
