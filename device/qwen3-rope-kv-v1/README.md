# Ferric Qwen3 RoPE and Paged-KV Device V1

This standalone package owns the two attributed K3 device roots intended to
supply Ferric's M1 Qwen3 envelope: split-half rotary embedding and P16 paged
key/value-cache writes. It retains the exact finite target/draft machine
profiles, Wave64 ABI, BF16/FP32 storage types, fixed deployment trigonometric
tables, global 16,384-page cache pool, and immutable source dependency closure.

RoPE and Ferric's host profile use the same exact scalar-dependent,
one-dimensional launch. It assigns one wave to each active row and uses
compiler-authenticated row-striped write-only outputs. Paged-KV instead
launches exactly one Wave64 workgroup for each of the 16,384 physical cache
pages. Every page wave scans the bounded admitted row roster and writes only
rows whose page-table entry selects its page. Its cache outputs use the static
`Blocked<Index1D, 64, 256>` mapping: each lane owns all 16 token slots, eight
heads, and two split halves for one physical page. Page-table aliases therefore
remain sequential writes by the same page owner rather than creating
cross-invocation write races. The 16 token cases contain literal component
stores because the pinned compiler closure does not authenticate dynamic
blocked components. This is a parallel ownership boundary. It does not claim
performance; the bounded reverse lookup still requires hardware measurement.
Ferric's retained host profile uses the same fixed physical-page grid. Final
integration still requires exact-compiler extraction and authenticated runtime
admission before this root can be launched through Ferric.

The standalone generated d955 write-only KFD path seeds each device allocation
from the destination and writes it back only after successful dispatch
completion. That preserves cache elements not touched by this source without
granting device read access, but it also stages both fixed 512 MiB cache
buffers in each direction. It is a qualification adapter, not Ferric's
inference lifecycle. Production integration must bind the write-only arguments
to long-lived device-resident KV allocations retained across dispatches and
retire them only after exact queue quiescence.

The package pins immutable reviewed `fe2o3-device` and generated-host source at
revision `2f6da870a31b`. The exact `cargo-fe2o3` compiler used to produce an
artifact remains separately bound by the protected compiler-execution receipt.

Source and host tests establish the exact two-root roster, finite machine
profiles, scalar-dependent RoPE launch, fixed physical-page KV launch, static
blocked ownership and address identity, alias-page serialization, untouched
cache preservation, BF16 rotary source order, bit-exact KV copies, and generated
KFD argument effects.
They do not establish extracted host-plan binding, page-table provenance or
generation, artifact identity, Worker V3 execution, numerical qualification,
hardware behavior, parallel KV performance, or M1 closure.
