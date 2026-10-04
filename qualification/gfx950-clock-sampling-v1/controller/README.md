# Gfx950 Clock Sampler CPU Qualification

Reconstruct the exact qualified CPU609 sources from its original clean archive
pair and frozen worker overlay. Join the complete reconstructed source map to
CPU609, then apply only ten formatted runtime Rust files. No Ferric source or
manifest changes are part of this qualification.

Keep all 169 previous runtime and 440 passing worker tests. Add 14 clock tests
and 25 existing currentness, device-profile and queue tests. Cargo substring
selectors are checked against the compiled inventory and must be disjoint.
Expected: 648 passes, four existing worker ignores, 33 bounded phases.

The fresh target uses the pinned nightly, offline locked Cargo, two build/test
threads, CPU8/9, niceness10, empty GPU visibility and existing resource bounds.
All sources, inputs, dependency manifests, tools and the built worker are
rechecked. The 14 controller-policy tests are synthetic and reported separately.

CPU-only qualification does not establish successful native clock ioctls,
timestamp calibration, GPU correctness, numerical acceptance or performance.
