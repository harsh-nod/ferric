# Ferric Qwen3 RoPE and Paged-KV Device V1

This non-authoritative compatibility package exposes the two attributed K3
device roots whose canonical source is owned by `qwen3-all-kernels-v1`:
split-half rotary embedding and P16 paged key/value-cache writes. It is
retained for focused tests and is not a production selected-package or
publication root. It retains the exact finite target/draft machine
profiles, Wave64 ABI, BF16/FP32 storage types, fixed deployment trigonometric
tables, global 16,384-page cache pool, and immutable source dependency closure.

RoPE and Ferric's host profile use the same exact scalar-dependent,
one-dimensional launch. It assigns one wave to each active row and uses
compiler-authenticated row-striped write-only outputs. Paged-KV instead
launches one Wave64 workgroup. Its unique grid leader serially traverses only
the bounded active row roster, follows each row's exact page-table entry, and
writes all 1,024 BF16 key/value components through compiler-authenticated
`GridExclusive` cache custody. Arbitrary physical-page order is preserved
without a fixed 16,384-workgroup reverse scan or cross-invocation write race.
This initial work-proportional schedule deliberately leaves the other 63 lanes
idle because the current typed API has no authenticated indirect-permutation
index space for parallel arbitrary page-table writes. It does not claim
performance; hardware measurement remains required. Ferric's retained host
profile binds the same single-workgroup geometry. Final integration still
requires exact-compiler extraction and authenticated runtime admission before
this root can be launched through Ferric.

The pinned generated write-only KFD path seeds each device allocation
from the destination and writes it back only after successful dispatch
completion. That preserves cache elements not touched by this source without
granting device read access, but it also stages both fixed 512 MiB cache
buffers in each direction. It is a qualification adapter, not Ferric's
inference lifecycle. Production integration must bind the write-only arguments
to long-lived device-resident KV allocations retained across dispatches and
retire them only after exact queue quiescence.

The package pins immutable reviewed `fe2o3-device` and generated-host source at
revision `2d275684d7a2`. The exact `cargo-fe2o3` compiler used to produce an
artifact remains separately bound by the protected compiler-execution receipt.

Source and host tests establish the exact two-root roster, finite machine
profiles, scalar-dependent RoPE launch, single-workgroup KV launch, static
grid-exclusive ownership and address identity, bounded active-row traversal,
untouched cache preservation, BF16 rotary source order, bit-exact KV copies,
and generated KFD argument effects.
They do not establish extracted host-plan binding, page-table provenance or
generation, artifact identity, Worker V3 execution, numerical qualification,
hardware behavior, parallel KV performance, or M1 closure.
