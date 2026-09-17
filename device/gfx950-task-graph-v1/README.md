# Finite gfx950 Atomic Task Graph

This ordinary fe2o3 Rust source is an engineering scheduler/visibility fixture,
not a Qwen kernel, inference engine, performance result, or production authority.
It requires the normal checked source-to-semantic-to-ranked-to-KIR compiler path.
A host test does not execute or qualify the device entry point.

Current checkpoint: ordinary Rust passes source, semantic/ranked/formal,
translation, and target barrier checks, then links into gfx950 HSACO. The fixed
GPU suite on `mi350-2` passed 20 dispatches: 16 valid epochs and four stale-epoch
negatives. It checked 260 atomic state words, comprising 244 deterministic values
and 16 valid ownership encodings. The valid epochs included 112 exact payload
values. Both workgroups owned tasks and four dependency edges crossed workgroups
in every valid epoch. These are observed executions of this seven-task graph,
not a general tensor-publication or scheduler qualification.

The tested Ferric source/dependency checkpoint is `e3d198ee`; the unchanged kernel
source SHA-256 begins `6d17ed3b`. The clean fe2o3 compiler is `af4f1460`, and the
HSACO SHA-256 is
`8f4904773d92c58b0eb2ebb06e0195eac536b5a28f35a92e3b9fa2bda5ba328f`.
The [GPU evidence and reproduction procedure](../../qualification/gfx950-task-graph-v1/README.md)
retain exact identities, checks, and limitations. All compiler gates remain
enabled; linking and execution use the engineering route, not protected
production authority. Projection GPU results remain separate evidence.

## Graph and Arithmetic

The graph has seven tasks, two workgroup workers, and two wave64 waves per worker:

```text
0 -> {1,2} -> 3 -> {4,5} -> 6
```

Precisely, its edges are `0->1, 0->2, 1->3, 2->3, 3->4, 3->5, 4->6, 5->6`.
Task `t` reduces `inputs[t*128..(t+1)*128]`, then adds its immediate predecessor
payloads. All values and payloads are `u32`, with input values at most 1024.
The largest valid final result is `13*128*1024 = 1,703,936`; no wrapping arithmetic
is needed. Invalid inputs and overflowing LDS/atomic sums set error 4 and do
not publish that task's payload, completion bit, or successors. Invalid inputs
are propagated through an LDS sentinel so all lanes agree before publication.
All 128 lanes contribute to each task. The current reduction deliberately uses
a straightforward LDS read loop, not a tuned tree reduction or MFMA.

## Fixed ABI

Symbol: `ferric_gfx950_task_graph_v1`. Workgroup `[128,1,1]`, AQL grid
`[256,1,1]`, source maximum workgroup grid `[2,1,1]`, static LDS 1024 bytes.
The LDS allocation is `WorkgroupPipeline<u32,2,128,1>`.

| Argument | Physical offsets | Extent |
| --- | --- | --- |
| `inputs: &[u32]` | pointer 0, length 8 | 896 words |
| `config: &[u32]` | pointer 16, length 24 | 1 word: expected epoch |
| `epoch, ready, done, claimed, owners, errors` | pointers 32 through 72 | one atomic u32 each |
| `payload0` through `payload6` | pointers 80 through 128 | one atomic u32 each |

There are 17 explicit physical records occupying 136 bytes, alignment 8. This table
is a source contract; ELF qualifiers that are absent must remain reported absent.
All 15 allocations are distinct, live, and properly aligned. Atomics require
coherent memory and system-scope support. Inputs/config are immutable during a
dispatch. No ordinary shared tensor memory is transported between workgroups.

Initially `epoch=config[0]!=0`, `ready=1`, all other state/payload words are zero.
The epoch and allocation lease cannot be reused until every worker has finished.
Successful final state is ready 0, done 127, claimed 127, errors 0, unchanged epoch.
For task `t`, `(owners >> (2*t)) & 3` is the claiming workgroup index plus 1;
higher bits are zero. Errors are stale epoch 1, duplicate claim/completion 2,
invalid input/ready mask 4. A stale epoch sets only error 1 and performs all uniform
LDS phases without touching the queue or payload values.

## Publication and Progress

Only the leader claims a ready bit with an acquire-release atomic `fetch_and`.
A first unconditional LDS phase broadcasts the task ID to both waves. A second
phase publishes their 128 contributions. The leader writes its atomic payload,
sets its done bit with acquire-release `fetch_or`, and releases successor bits.
Fan-in uses the **old value returned by that exact done RMW**, not a later load;
only the last predecessor publishes the join task. Atomic payload reads acquire.

Every worker follows 16 fixed rounds, including idle/stale rounds, so barriers
are never skipped. A worker retires after 8 empty probes. Every nonempty attempt
either claims a task or observes another worker's claim of a new task, so there
are at most 7 nonempty attempts. The 15 possible probes fit within 16 rounds.
An owner publishes all successor work before probing again. Thus one resident
workgroup can finish the graph without a grid barrier or peer-residency premise.

Two launched workgroups do not prove a cross-workgroup dependency was exercised:
the measurement harness must report the actual owner IDs and crossing graph
edges. Serial ownership remains a correct result but not cross-workgroup evidence.
See [the host model](../../tools/task-graph-model/README.md) for scheduler tests.
