# Ordered Wave Attention And V11 Driver Profile

Experimental driver-only composition, based on af84c7483d88dd35dd90483ebec3fc52b1756c54.
No current CLI, native canary, protocol/checker, kernel, image, compiler, runtime,
default or numerical operation changes. No GPU or performance qualification.

The new terminal selector is configure_ordered_wave_attention_fp32_argmax_v11.
It checks the existing preallocation-admitted exact v11 binding and fresh
target TP1/capacity32 state, FP32-v8 workspace, MFMA, pruning, wave attention,
device TP1 reduction and ordered transport capability before changing either
selector field. Large KV, peers, capture, sequences and existing ordered/v11
selection remain rejected. Both fields are published only after validation
and creation of the bounded host command vector. No GPU allocation or worker
re-admission occurs here. Existing selectors and policy guards are unchanged.

Execution reuses the existing ten-command attention and five-command FFN
groups at36 layer barriers. Each group is flushed before its residual.
Embedding and all three selected output-head calls remain synchronous:
dispatch_zero flushes pending groups before submit/wait. V11 is never added
to a dependent group. Output pruning, row permutation, arguments, grid,
buffer identities and packet totals (613 unpublished,616 published) remain
unchanged. No change to the pending-request or ordered acknowledgment rules.

Any forward error poisons the driver without minting a successful completion.
Ordered flush clears pending commands and advances dispatch counts only
after a complete acknowledgment. Submitted pool work remains the caller's
quarantine responsibility. Completed ownership may be retired independently
of model parity; the unchanged fixed-reference checker remains a separate
admission requirement and is not modified by this experiment.

Six focused recording tests cover all rows1/16/17/32 and empty/last/sparse/all
selections, flattened byte arguments and IO, unchanged allocations and packet
reservations, exact ordered groups and singleton head/residual barriers,
25 invalid admission states including capability, legacy rejection and
terminal policy immutability, submitted failures/quarantine, invalid row
selection before publication and transparent host timing. Synthetic MFMA
weights/outputs and modeled ordered completion are not device math evidence.
Existing constructor-image and worker wire/rollover tests remain applicable.

Seven attention operation spans measure command preparation in ordered mode;
actual waits move to collective_attention/collective_feed_forward flush
scopes. They are not GPU durations. A later separately reviewed canary and
closed checker must describe this honestly rather than accepting the frozen
synchronous timing roster. No CLI or model launch is enabled by this change.
