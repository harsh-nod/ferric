# Fixed Ordinary Publication

Engineering source fixture for V31 `publish_once_128`: WG0 writes 128 ordinary
f32 payload cells and releases READY; WG1 releases REQUEST, acquires once, and
reads only after observing the current producer's READY. Initial atomic bits
need not be zero. The fixed launch is grid256/WG128/wave64.

The five separate allocations are payload f32[128], flags AtomicU32[128], input
f32[128], status u32[256], and result f32[256]. The input is consumed into the
V30 readonly capability. Both result fields are retained; zero Ready results
cannot qualify cross-workgroup publication. The protocol is not a wait loop.

Device and host dependencies pin reviewed compiler revision
`cb8f51ec0b52894a1fb510bc3f523005dda8bfaf`. A pin alone establishes no GPU
execution, native-memory eligibility, protected runtime admission, performance,
or model-inference result.

The [Asrock engineering suite](../../qualification/gfx950-static-publication-v1/README.md#asrock-results)
now records a fresh native Rust build and all 44 predeclared GPU attempts.
Its observed numerical pass does not widen the execution boundary above.
