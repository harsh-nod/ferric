# Matched-Input Layer-0 MLP Diagnostic

Source-only proposal. No framework execution, GPU result, numerical acceptance,
performance improvement, or production authority is claimed here.

## Question

The current guarded capture's two native post-normalized inputs are byte-identical
to each other. They differ from the genuine framework layer-0 input at two of
4096 BF16 words. This job distinguishes input propagation from differences in the
MLP implementations by calling the real framework `model.model.layers[0].mlp`
with the actual native input. It does not replace inputs in a full-model run.

Exactly four module calls are permitted, in this order:

1. Original framework post-normalized input, control 1.
2. Actual native post-normalized input, matched call 1.
3. Original framework post-normalized input, control 2.
4. Actual native post-normalized input, matched call 2.

Each call uses contiguous BF16 `[1, 1, 4096]` input and the actual loaded Qwen3 MLP.
Hooks immediately materialize immutable bytes for input, gate/up inputs and
outputs, SiLU input/output, product at the Down pre-hook, Down output and module
output. All handles are removed in `finally`; inputs are checked before and after
the call. There are no full-model forward calls, KV updates, native kernel calls,
alternate reduction implementations, autocast, or compiler changes.

## Mandatory Control

Both control calls must reproduce all ten corresponding historical stage bodies,
and both native-input calls must repeat exactly. Otherwise the job retains its
actual stage files and a failed report; matched-input comparisons are `null`.
The 31 original input bodies retain their original logical pins. A separate flat
physical-location map supports read-only container mounts without rewriting any
historical receipt.

The historical framework ran on ASRock. The proposed MI350 environment uses the
immutable image `sha256:c5f9efa2623d9c8b2e483d2a139202e496baf5a04900a4f32907149f41a42dba`
and a separately pinned private package overlay. Current Python, Torch, NumPy,
distribution versions and implementation sources are recorded explicitly. They
are not described as the historical environment. The current observed package
contract must match before imports/model work; the four implementation sources
are checked against the actual loaded callables. `modeling_qwen3.py` remains the
qualified Transformers 4.51 source. Historical byte controls remain mandatory
even when implementation source files match.

The loader remains the qualified full-checkpoint `AutoModelForCausalLM` loader:
BF16, local files only, no remote code, safetensors, no loading discrepancies,
SDPA configuration, exact model geometry and source hashes. The qualified helper
moves devices without narrowing the FP32 RoPE buffer and enters eval mode. Only
the layer-0 MLP is then invoked, under inference mode and the original
deterministic/highest-precision policies with BF16 reduced-precision reduction
disabled. A new layer-only checkpoint loader is deliberately outside this scope.

## Comparisons

When the control gate passes, six rows compare gate, up and product for each TP
rank. Each rank occupies exactly 6144 BF16 elements, or 12288 bytes, of the full
12288-element framework intermediate. Every row includes the old genuine-chain
comparison, the matched-MLP-input comparison and the framework's input-change
effect, using the unchanged qualified BF16 diagnostic arithmetic.

Gate/up have identical operator inputs in the matched comparison. Product is a
downstream stage with the same MLP-boundary input; its difference can still include
gate/up propagation. Native `activation` means SiLU(gate) times up, not standalone
SiLU. There is no captured native standalone SiLU to compare. Individual FP32
native Down partials are never compared directly to the full BF16 framework Down.
The two framework Down outputs may be compared to describe the input-change
effect only. No threshold, causal attribution, speedup, or full-model acceptance
is inferred from any of these rows.

## Root Execution Contract

Root alone stages, reviews, runs and retains this proposal. No execution occurred
during authoring.

Fresh root:
`E/guarded-mlp-model-matched-input-mlp-v228-v1`, where E is the existing finite
resident integration evidence directory. `source/` contains exactly the manifest
file roster plus `source-manifest.json`. `inputs/` contains the 31 files named by
`inputs.json.locations`, plus observed `environment.json`. `local-input-locations.json`
is a local packaging aid, not a runtime source/input and not receipt authority.

`environment.json` has schema
`ferric-guarded-mlp-matched-input-environment-v1`, immutable image identity,
`model_root: /model`, `framework_gpu_execution: false`, observed Python version
and executable, exact nine package versions, HIP version, `expected_arch: gfx950`,
and four `implementation_sources` pins using container paths. The full actual
`gcnArchName` is recorded only by the GPU job, not invented by CPU preparation.

`launch-plan.json` has schema `ferric-guarded-mlp-matched-input-launch-v1`, `image`,
fresh `topology` (`host`, `boot`, `unique_id`, `pci_path`), `source_manifest` compact
pin, `overlay_root`, `overlay_manifest` full pin and `environment` compact pin.
The overlay manifest contains `files` keyed by paths relative to its package root.
The exact plan SHA is the sole launcher argument:

```text
/usr/bin/python3 -B ROOT/source/launch.py SHA256_OF_LAUNCH_PLAN
```

The launcher first runs 20 named pure tests with GPU visibility empty. It then
checks the immutable image, exact task name, physical renderD128 identity and
three all-eight-GPU idle samples before the sole container start. The container
has no network, a read-only root filesystem and read-only source/input/model/
package mounts, no extra capabilities, uid/gid 9661, 64 GiB cgroup memory with no
extra swap, two CPUs and 256 PIDs. Explicit USER/LOGNAME/HOME and private cache
paths avoid the image's missing passwd entry for uid 9661.

The GPU leaf has a 900-second inner timeout plus 15-second kill grace and a
930-second outer attach bound. The owner has a 1200-second operational cutoff and
a separate 600-second retirement/postflight reserve. Docker CLI leaves reuse the
qualified pidfd/subreaper ownership helper; their 130-second retirement reserve
covers its primary and emergency cleanup. Whole-owner signal handling remains
active between leaves and defers repeated signals during retirement. Failed
workload storage predicates do not prevent stop/remove. Any observed signal,
forced cleanup, failed control, drift or incomplete retirement prevents success.

Only the exact task name, container ID, owner label and image may be retired.
After exit, inspect/stop if needed/remove/name-absence and three idle samples are
required. Source, input, overlay, plan and topology postchecks remain mandatory.
Original raw CLI output, lifecycle records, full stage bodies and implementation
source snapshots are retained. Container process exit/quiescence belongs to the
outer owner, not the inner report. Failure artifacts must never be promoted.

## Focused Tests

The 20 pure tests cover four-call order, exact stage/argument joins, control and
repeat gating, rank slicing and strict integer ranks, product/SiLU distinction,
partial/authority exclusions, nonfinite/extent refusal, duplicate and missing
hooks, reverse cleanup on exceptions, pin/link refusal, explicit user/cache/GPU
environment, immutable container ownership/mounts, separate retirement budget,
failed-storage cleanup and signal deferral. The census parser accepts both known
unittest verbose spellings but requires the same 20 unique successful names.

No test has been run locally by the proposal author. Actual CPU and GPU outcomes
remain pending and must be retained separately from these source statements.
