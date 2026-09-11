# TP1 Ordered Dispatch Batches V1

This is an explicit engineering transport experiment, not a serving or performance
qualification. `--runtime-ordered-batches` uses the core ordered-packet API instead
of the existing synchronous `--dispatch-sequences` API. The default path and the
seven-key `performance_profile` object are unchanged. Enabled runs additionally
emit `Setup.runtime_ordered_batches: true`; existing frozen comparators reject
that extension unless a separately pinned candidate validator admits it.

## Scope

The initial profile requires TP1, `v5-mfma32`, MFMA projections, baseline
attention, an explicitly admitted v8 head image, `device-tp1-v3`, output-head
pruning and the legacy physical KV pool. Both explicit v8 head precision choices
remain available; the initial reference canary uses FP32. Legacy dispatch
sequences, wave attention/projection, large-KV v9, peers, numerical readback
capture and replica controls are rejected. Queue rollover remains separately
opt-in; bounded live ingress still requires it.

All arithmetic, buffer layouts, row ordering and single-dispatch roots are
unchanged. Each layer uses the existing ten-dispatch attention segment and
five-dispatch feed-forward segment. Each segment is flushed before its existing
residual collective. Embedding, the two residual dispatches per layer, and final
norm/head/argmax remain synchronous singleton calls. Metadata and token writes
precede embedding; choice readback follows the completed final singleton.
Diagnostic projection capture is excluded, so the grouped segments contain no
host reads or writes. The kernel command sequence is identical to the nonordered
path, including exact active extents, wide grids and transposed weight bindings.

## Completion And Failure

The worker client uses a distinct pending request type and requires a
`DispatchOrderedBatchCompleted` response with the exact submitted count and no
payload. Legacy sequence receipts cannot satisfy it. Each batch contains 1..16
dispatches and has one aggregate 60-second core deadline, bounded by the existing
120-second parent transport deadline. Every existing metadata, buffer ownership,
extent, scalar and pointer-fixup packing check completes before any publication.

The core retains dependent packet ordering and all signal/currentness/lifetime
checks. The Ferric client does not infer completion from submission or a partial
receipt. Any failed or malformed completion terminates the worker and poisons
the driver batch; it cannot publish a pool completion or continue inference.
Counters advance by completed kernel packets, not by IPC groups. A full forward
uses 72 groups plus singleton calls: 616 packets when the head runs, or 613 when
pruning skips a prefill-only head. Existing whole-forward packet reservation and
checked queue rollover remain in force.

Aggregate completion time is host wall time, not per-kernel or GPU duration.
Host timing records the `dispatch_ordered_batch` IPC operation and
`flush_ordered_batches` span. Enqueue-only attention/feed-forward scopes can end
before the flush; overlapping host scopes must not be presented as GPU timings.

## Validation Boundary

Recording transport tests compare every flattened command and host read/write
against the nonordered path at rows 1/16/17/32, including selected and empty head
rows. Fault tests reject partial/wrong-kind responses and prove terminal failure,
exact packet accounting, pending-close draining and policy freeze. These are CPU
host-body checks, not native ordering or numerical evidence. Native core probes,
exact-reference model runs, and matched performance measurements are separate
root-owned gates. Any preliminary build using an unpublished core source override
must retain that override and must not be labeled a final public-pin gate.
