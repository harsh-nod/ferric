# Default-Mode CPU Retry

The fresh MI350 retry passed all 27 phases in 193.055 seconds: 1,180 runtime
tests with eight ignored, 838 worker tests with four ignored, the ten focused
scopes and ten facade doctests. Every phase exited naturally with its process
reaped and process group absent. Postcheck errors are empty.

This build has the new diagnostic feature disabled. It establishes that the
one-block Rust syntax repair compiles and preserves the old named test
outcomes. It does not test feature-enabled instrumentation or run the GPU.
The [original failed attempt](../default-cpu-failed-v1/README.md) is retained.

The [original receipt](evidence/complete.json) is 2,544,682 bytes, SHA-256
`c2371fe9155ddc2046c1e1cddae12447ec48a77b6c18df17d4ece2071f81ccbe`.
The final source map is 447,045 bytes, SHA-256
`1c61d9240a1bd44c0b7cd9f7799c1e7752497d135b003c1e0036684694061b07`.
The actual worker ELF is 6,546,992 bytes, SHA-256
`486b529e147c99b34c02f61df52627216e174c38ef691231e4ea65b56f860a71`.
Its identity was unchanged across the worker tests.

The original archive is 1,849,926 bytes, SHA-256
`11e08be108e30e08392384a374746762bb191158104b39a38c093ec5a5cef02a`:
192 members, 191 manifest pins and 13,183,006 expanded bytes, including 142
original raw bodies. MI350 export rehashed the full 1,069-source composition,
tools, private cache, dependencies and all eleven selected executable products.
The capsule retains selected source bodies and original product metadata, not
the ELF or external crate bodies.

Local retention checked the archive, manifest closure and original results
as data without executing retained code. This README is added commentary.
Feature-enabled worker, parent, native and numerical qualification remain
separate gates. There is no performance claim.
