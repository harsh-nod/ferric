# gfx942 And gfx950 Megakernel Model Sequence

Engineering feasibility checkpoint, 2026-10-08 UTC. This is a model-execution
handoff, not a serving implementation, benchmark result or release acceptance.

Every model must work end to end with the megakernel approach on both gfx942
and gfx950. A single-target success, CPU test, simulated result or component
kernel is not dual-target model acceptance.

Implementation order is **Qwen first, GPT-OSS-120B second,
DeepSeek-V4.1-Flash with Engram third**. North Mini Code is no longer in this
sequence. Later-model metadata research does not displace Qwen implementation.
The megakernel owner is responsible for correct end-to-end model execution;
the performance owner separately owns speculation and matched vLLM comparisons.

## Locked Qwen Target

The direct performance target remains single-request Qwen3-8B, BF16,
target-only decoding, a 2,048-token prompt and 256 generated tokens, with
700 output tokens/s as the goal. Quantization or speculative throughput cannot
satisfy that target. Target/draft source admission is not proof of speculation.

| Role | Repository | Pinned revision |
| --- | --- | --- |
| Target | `Qwen/Qwen3-8B` | `b968826d9c46dd6066d109eabc6255188de91218` |
| Separate draft source | `Qwen/Qwen3-0.6B` | `c1899de289a04d12100db370d81485cdf75e47ca` |

These are the existing [compiled model pins](../crates/ferric-build/src/model.rs)
and [source shard identities](../crates/ferric-build/src/fixtures/safetensors/README.md),
not newly downloaded weights. Target shards total 16,381,516,776 bytes; the
draft artifact is 1,503,300,328 bytes. Both tokenizers must satisfy the existing
exact-byte compatibility contract.

The current finite megakernel lane has TP2 gfx950, concurrency one, 40 prompt
forwards through 36 layers and zero generated tokens. Its four retained payload
captures compare with historical Ferric, not an independent full-model framework.
That does not establish the requested 2,048/256 correctness or throughput.
The performance lane's historical TP1 observations are a separate cohort.

Next: resolve the [callback-lowering boundary](../qualification/guarded-mlp-force-inline-worker-v1/README.md),
produce checked artifacts, qualify generation/rearm component behavior, then
integrate full-model generation and independently verify all 256 token IDs and
decoded bytes plus numerical and lifecycle gates. Component correctness is
necessary but cannot substitute for full-model acceptance.

## Later Checkpoint Availability

The following official metadata was inspected on October 8. The fixed-revision
tree sizes sum root safetensors files only. No later-model weights were
downloaded, loaded or executed, and local model-cache availability is unverified.

| Model | Immutable revision | Root weight files | Native format |
| --- | --- | ---: | --- |
| `openai/gpt-oss-120b` | `b5c939de8f754692c1647ca79fbf85e8c1e70f8a` | 15 files, 65,248,893,184 bytes | MXFP4 experts; nonexpert tensors include BF16 |
| `deepseek-ai/DeepSeek-V4.1-Flash` | `2cba9e42aa026125f3ed06c6d98c1db82f7ca027` | 48 files, 510,296,708,312 bytes | FP8/block-scaled configuration with FP4 experts; not all-BF16 |

Sources: [GPT-OSS fixed tree](https://huggingface.co/api/models/openai/gpt-oss-120b/tree/b5c939de8f754692c1647ca79fbf85e8c1e70f8a)
and [DeepSeek fixed tree](https://huggingface.co/api/models/deepseek-ai/DeepSeek-V4.1-Flash/tree/2cba9e42aa026125f3ed06c6d98c1db82f7ca027).
Published file sizes are planning inputs, not authenticated downloaded weights
or proof of runtime memory fit.

### GPT-OSS-120B

The [pinned config](https://huggingface.co/openai/gpt-oss-120b/blob/b5c939de8f754692c1647ca79fbf85e8c1e70f8a/config.json)
has 36 layers, hidden/intermediate 2880, 128 experts with top-4 routing,
64 attention heads, eight KV heads, head dimension 64 and vocabulary 201088.
It alternates full and 128-token sliding attention; its context limit is
131072. Config identity: 2,089 bytes, SHA-256
`933aeb666a3fd851133ddd7686414f369bc564c4185fb5704416550879f10566`.

Required megakernel work includes exact MXFP4 block/scale interpretation,
MoE routing/combine, the model's SwiGLU behavior, attention sinks, sliding/full
KV semantics and [Harmony encoding](https://github.com/openai/gpt-oss).
A GPT-OSS layer-tile demonstration does not provide those full-model contracts.

Single-gfx950, TP1, concurrency-one is a feasibility starting point for native
weights, not a qualified device allocation. Actual packed weights, workspace,
KV and routing memory must pass measured capacity checks first. The original
OpenAI checkpoint must not silently become the
[MI355X recipe's AMD derivative](https://github.com/vllm-project/recipes/blob/main/OpenAI/GPT-OSS.md),
which uses a different weight/activation representation.

### DeepSeek-V4.1-Flash With Engram

The [pinned config](https://huggingface.co/deepseek-ai/DeepSeek-V4.1-Flash/blob/2cba9e42aa026125f3ed06c6d98c1db82f7ca027/config.json)
has 40 layers, hidden 5120, MoE intermediate 2304, 384 routed plus one shared
expert and top-6 routing. Engram is present at layers 1 and 14 with
384006168 and 384016682 embedding rows, head dimension 256 and maximum n-gram 4.
Config identity: 3,311 bytes, SHA-256
`8be45ce0476004a3f529fd896115a4a2e800a129ad2d3ec05b16050f52e21879`.

Required work is full Engram history/hash/gather integration, actual
sparse/compressed-attention and MoE semantics, authenticated native scales
and explicit CPU/GPU memory placement. An isolated N-gram gather is not model
acceptance. TP/EP count, host-memory/offload layout and device topology remain
unlocked; source-shard totals alone do not establish multi-GPU fit.

The [upstream ROCm recipe](https://recipes.vllm.ai/deepseek-ai/DeepSeek-V4.1-Flash)
is a baseline-research input, not proof that Ferric or the installed image
supports this checkpoint. The performance owner must pin a compatible actual
baseline. Synthetic speculative acceptance is not an end-to-end correctness
or performance result. Any offload must remain inside measured model timing.

## Execution Handoff

| Target | Hardware lane | Current candidate evidence | Missing acceptance |
| --- | --- | --- | --- |
| gfx950 | `mi350` / `mi350-2` | CPU-qualified source; V11 checked compilation fails at callback enum ABI | Checked target artifact, eligible device, native component and complete independent model outputs |
| gfx942 | `mi300x` / `mi300x-2` | No current dual-target port or native qualification in this lane | Target-specific code/runtime support, compilation, native numerical/lifecycle results and full-model outputs |

Architecture-specific implementations are allowed, but the corresponding
feature and model semantics must work on both targets. gfx950 instructions
cannot be relabeled as gfx942 support. Native low-precision representation,
memory/TP/placement feasibility, proof applicability and matched baseline
performance each require target-specific evidence.

For each model, the megakernel owner supplies exact checkpoint/config/tokenizer
and weight identities, compiler/runtime/source pins, checked images, input IDs,
complete output IDs/decoded bytes, numerical-reference evidence and clean
resource/lifecycle retirement. The performance owner preregisters the matched
baseline, prompt set, TP/concurrency, cache/speculation policy, timing boundary,
repetitions and uncertainty before comparisons. A missing or losing cell stays
visible; no model or precision substitution can manufacture a win. Each target
must join real Rust source and compiler to its own artifact, matching hardware,
completion/readback and independent oracle. Private candidate results remain
candidate evidence until integrated public source is rerun.

For later models, low-batch concurrency-one 2,048/256 is only a proposed initial
workload. Model-specific prompt encoding, EOS policy, reference tolerances,
hardware/TP and sustained-load cells must be fixed before measurements.
No later-model benchmark cell is accepted by this metadata note.

All implementation and qualification remain on the authorized SSH host.
This lane's stricter direct instruction keeps authoring and CPU/cross-compile
work on `mi350`. gfx942 native execution needs a separately authorized owner
on its matching host or an explicit host-rule change; it cannot be silently
run on a different machine or inferred from gfx950.
No new GPU lease, large download, resource-limit increase or main merge is
created by this plan. One device used by the historical TP2 harness currently
has a coordinator-reported unresolved completion-retirement hold. Reusing that
request requires actual recovery evidence or a separately qualified device set;
absence of a process is not recovery.

Metadata feasibility is complete. Qwen's checked callback lowering and full
independent output/lifecycle acceptance are still blocking dependencies.
The October 21 showcase is a target, not a justified full-model delivery promise.
No 700 tokens/s result or multi-model vLLM win has been established here.
