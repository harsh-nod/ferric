# Engineering Tensor-Parallel Execution

This change adds an isolated engineering driver, not a protected M1 runner.
It executes one token at a time using rank-local GPU projection, normalization,
RoPE, KV append, attention, and final logits. O/down projections return FP32
partials. The host adds them in ascending rank order, adds the residual once,
rounds once to BF16, and actually broadcasts those bytes to every rank.

## Direct Proof Boundary

`crates/ferric-engine/src/tensor_parallel_execution.rs` is the executable
sequence cursor consumed by the driver. Its same-source contracts establish:

- capacity is in 1..=8192 and every accepted token is in vocabulary;
- a successful begin has exactly one current position below capacity;
- another begin/reset cannot reuse an in-flight position;
- completion advances exactly once, without exceeding capacity;
- poison permanently prevents further begin/completion/reset;
- successful reset increments the sequence epoch and starts at position zero;
- epoch overflow and every returned error preserve the entire cursor.

The prior proved shard rectangles and collective readiness state are reused
without modification. No transport response is treated as a Verus theorem.
The cursor contracts do not prove device-memory initialization or completion.

## Contracted Execution Boundary

`adapters/m1-engineering-execution-v1/src/tp_execution.rs` and its numerical
collective are Contracted. Their floating-point arithmetic, authenticated
section byte hashing, dispatch construction, kernel results, IPC child
isolation, distinct physical devices, loader, firmware and hardware are not
proved by the sequence cursor. In particular:

- a transport owns exactly one independent device and one pending request;
- wait succeeds only after its submitted request completed;
- retained kernel descriptors match every ordered argument/access/launch;
- buffer IDs are child-local, and all byte extents are checked in that child;
- source-controlled kernel code is explicitly opted into as engineering code;
- no pointers cross the IPC boundary and Ferric contains no unsafe block;
- CPU transport tests are control-flow tests, not GPU kernel emulation;
- no TP Qwen result, device numerical correctness, speedup, serving capability,
  M1 qualification, or protected execution authority is implied by host tests.

Setup verifies every actual prepacked section against the digest retained by
the authenticated weight layout, then uploads only compact rank-local shards.
Only rank zero stores embedding/head weights and computes final greedy choice.
Prompt priming is sequential m=1 execution and must be reported as such.

For the 36-layer target each token completes 544 dispatches on rank zero and
540 on each other rank. These counters describe observed transport completions,
not authenticated proof receipts. Rank submissions happen before waits so
independent devices can overlap. On partial submission/completion failure the
driver drains submitted peers, poisons the group and permits only close.

## Checks

The initial remote closure passed eight adapter tests, three engine cursor
tests, strict adapter Clippy, and strict same-source Verus verification with
eight verified obligations and zero errors. Seven explicit executable cursor
bodies are proved; the additional verified function is derived `Clone` for
the error enum. These are host checks, not GPU execution results.

Run `check.sh OWNED_REMOTE_STAGE tests`, `clippy`, `proof`, and `identity` only
on mi300x. The stage contains `source/`; the strict proof target must be fresh
and never used for non-strict verification. `run-negative.py` changes actual
cursor bodies in an exclusively owned stage, requires intended Verus failures,
and restores the original bytes even on failure. It grants no qualification
or provenance authority. Retain source/tool hashes and positive/negative
transcripts, then remove the exact owned remote build stage.
