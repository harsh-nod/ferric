# Serial Peer Engineering Worker V4

This standalone crate is a disposable, explicitly unauthenticated engineering
entry. It is not protected M1 execution, a Verus proof, or a production serving
authority. The main adapter retains `unsafe_code = "forbid"`; only this separate
process invokes the unsafe fe2o3 engineering peer owner.

The exact CLI is:

```text
ferric-tp-peer-engineering-worker-v4 --allow-unauthenticated-machine-code --device-unique-ids UID,UID[,six more]
```

Only two or eight distinct physical gfx950 devices are accepted. One
single-threaded child owns every KFD context and allocation. The controller
uses bounded framed IO threads with deadlines; the child itself has no threads
or independent GPU client. Every error is terminal, and every rank proxy shares
the same child failure state. A successful group close is acknowledged only
after all peer mappings are unmapped before their owners are freed.

Commands are monotonically numbered and rank-tagged. Buffer IDs are globally
unique in this child, never raw GPU pointers. Host access remains owner-only.
Only explicitly shared buffers permit peer read-only kernel bindings; writable
peer arguments, foreign groups, stale IDs, invalid extents, unadmitted kernels,
reordered responses and partial completion reject. Host or lifecycle operations
cannot overlap outstanding rank dispatch requests.

The Ferric parent loads two separately admitted artifacts: the existing base
projection/attention image and the exact two-root v4 collective image. Parent
ABI packing and metadata comparison reuse the independent worker's unchanged
checks. The driver explicitly selects `device-peer-serial-v4`; no host reduction
fallback is possible. It shares fresh FP32 partial buffers and BF16 hidden
ping-pong buffers before the first dispatch. After rank zero's embedding has
completed, a GPU copy initializes every other rank's hidden state. Each layer
then performs two rank-ordered FP32 reductions, adds its local residual once,
and rounds to BF16 once. Hidden IDs swap only after all ranks complete.

This initial worker serializes individual rank dispatches and retains full
group currentness checks. Dispatch sequences, queue rollover, operational
currentness and cached admission are unsupported here and must be explicitly
rejected by the CLI, not silently ignored. It makes no overlap or speedup claim.

Evidence must bind both artifacts, the child executable, source and environment
identities, the exact ordered device roster, and the shared child PID. The
rank-sized PID list intentionally repeats exactly one PID only for this label.
Expected increments are 612 transformer/reduction dispatches per rank, one
embedding or peer-copy dispatch on every rank, plus zero or three final-head
dispatches on rank zero depending on pruning. All tensor arithmetic and native
visibility remain Contracted until separately qualified on the hardware.

Host tests exercise framing, timeouts, process reaping, rank demultiplexing,
global identities, owner-only host access, all-rank completion, exact arithmetic
ordering and active-row tails. Their arithmetic uses an independent CPU oracle;
it does not establish GPU kernel correctness or cross-device cache coherence.
