# Indexed Atomic Storage Fixture

This fixed 256-cell source is the next engineering compiler/runtime vertical
slice after the finite scheduler micrograph. Each invocation Release-stores one
input through a genuine Rust `AtomicU32` slice, Acquire-loads the same cell, and
writes a disjoint output. The intended launch is two 128-thread wave64
workgroups, with two waves per workgroup.

It is not a producer-consumer test: each lane reads its own write. Do not infer
cross-workgroup tensor publication, model correctness, performance, or
production execution authority from this fixture.

Inputs use `DisjointSlice<u32>` with checked `get_mut(ThreadIndex)` access.
This is a genuine exclusive capability separating input storage from the shared
atomic slice, although this kernel only reads input values. Its logical access
contract is RW; the harness independently requires input bytes to remain
unchanged. An ordinary `&[u32]` input alongside the shared atomic slice currently
fails conservative alias analysis. The atomic slice is never marked noalias.

Compiler source controls now pass through checked gfx950 LLVM. The fixture pins
fe2o3 revision `3dfa5b3fdac1832bd7d8902e32f591d81300d1e3`; use a matched
backend, extractor and CLI from that revision. Its nominal atomic host binding
is inert: protected runtime preparation still rejects unjoined atomic memory
contracts. Native build and GPU evidence are recorded separately under
[qualification](../../qualification/gfx950-atomic-channel-v1/README.md).
Do not bypass a compiler rejection with handwritten IR.
