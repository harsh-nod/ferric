# Qwen Tensor Parallel Planning V1

## Executable Boundary

`crates/ferric-engine/src/tensor_parallel.rs` is the production implementation,
not a separate executable model. Its `verus!` bodies compile into the ordinary
Ferric engine. The constructor consumes the existing `ModelConfig` admission
contract, and tensor slicing consumes `Qwen3TensorMetadata::validate`.

Only the admitted Qwen3-8B and Qwen3-0.6B shapes and world sizes 1, 2, and 8 are
supported. All query and KV heads are divided without KV-head replication.
Query/key/value and gate/up projection weights divide the output dimension in
the original BF16 `[output, input]` matrix. Output/down projection weights
divide the input dimension and require a sum after local projection. Embedding,
normalization, and language-model-head weights remain replicated.

The immutable plan exposes rank-local query/KV head and channel ranges plus
feed-forward ranges. The tensor rectangle exposes original row strides and
checked byte intervals; `copy_bf16_row_into` copies an actual compact shard row
from a full dense BF16 payload into caller-owned storage, without allocation.

## Direct Obligations

- A successful uniform partition is nonempty, has width `extent / world`,
  starts at `rank * width`, and ends within the original extent. Its widths
  exactly cover the original dimension without truncation.
- Accepted model and tensor metadata satisfy the existing exact admission
  contracts. A tensor rectangle partitions the correct projection axis and
  leaves the other axis intact; replicated tensors retain both full axes.
- Successful rank head/channel ranges preserve their 128-channel head mapping.
- Each successful source row interval lies within the exact full BF16 payload.
  Each copied destination byte equals the corresponding original source byte.
  Every rejected copy preserves the whole destination.
- A readiness arrival preserves group, model role, epoch, and ordinal. It sets
  exactly the requested rank's previously unset bit and rejects a wrong group,
  model, epoch, layer, operation, rank, or duplicate without changing state.
- Advancement requires all world-size rank bits. It clears readiness and moves
  from attention output to feed-forward down, then to the next layer. The final
  layer advances the epoch. Missing ranks and epoch overflow preserve all state.

The tests reconstruct every sharded projection for both models at TP1/2/8,
check every GQA head mapping, cover invalid metadata and sizes, and exercise
complete, incomplete, reordered, duplicate, stale, cross-group, and overflowing
collective readiness traces.

## Non-Claims

This module does not create device allocations, bind rank IDs to physical
devices, dispatch kernels, authenticate rank ownership, enqueue a collective,
sum floating-point values, or establish device completion. `group_id` uniqueness
within the live coordinator is caller-owned; its value is not an authenticated
communicator identity. Readiness state is clonable because it is a planning
contract, not a linear resource or transport authority.

The plan does not establish a sharded kernel's numerical semantics, distributed
KV ownership, request publication, failure recovery, tensor-parallel Qwen
execution, performance, or MI350 support. No compiler, artifact, hardware,
benchmark, serving, or M1/M3 qualification authority is granted by these APIs.
Payload lengths and metadata are checked here; authenticating actual weight
contents against the admitted model identity remains the caller's obligation.

Verus covers the executable host arithmetic/copy/readiness bodies and consumes
the existing model/tensor validation contracts. It does not prove LLVM, HSACO,
firmware, hardware, or an external transport. Positive transcripts and actual
body mutation rejections must be bound to their exact checked source; shared
source inventory and proof-policy admission remain the integration lead's work.
