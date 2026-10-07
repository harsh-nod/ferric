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

This worker serializes rank dispatches. Explicit parent runtime options can
configure immutable admission caching and operational currentness exactly once
before loading artifacts. Same-rank sequences contain 1..16 prevalidated
dispatches with a checked aggregate timeout of at most 600000 ms. Full all-group
checks bracket every sequence; operational currentness and observed idle checks
cover every participant before and after each synchronous kernel. Mapping,
host access and teardown retain full checks. Sequence failures poison all ranks
without returning partial success. Queue rollover remains unsupported and must
be rejected, not silently ignored. No rank overlap or speedup is claimed.

The producer qualification executable accepts optional final
`--cached-sequences`. This enables cache/currentness options and repeats each
fixture operation as a two-command sequence before checking exact GPU-produced
BF16 peer copies, FP32 peer reductions, active extents and guards. Repetition is
an explicit native API qualification fixture, not a performance benchmark.
An independent optional final `--wide32` selects the exact v5 projection/v6
peer symbols and rows 17, 31 and 32 with 32-row guarded capacities. It requires
separately admitted/digest-bound wide images; it does not relax the v4 image's
16-row limits. Both options can be combined, and duplicate options reject.

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
