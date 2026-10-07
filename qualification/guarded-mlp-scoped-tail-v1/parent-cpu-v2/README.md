# Tail Parent CPU Qualification

The fresh parent retry on `ssh mi350` passed 568 selected tests across 58
existing scopes and all 68 phases in 416.482 seconds. The library inventory
contains 1,032 names; this is not a claim that the full library suite ran.
All seven selected executables were built. Every owned phase exited naturally,
was reaped and left no process group; source/dependency/tool postchecks passed.

This attempt consumes the [actually qualified test-only repair](../cpu-attempt-v2/README.md).
The [original failed parent attempt](../parent-cpu-failed-v1/README.md) remains
unchanged. Its old projected census omitted an imported shared-sequence test;
the existing selector covers that test without a new phase or broader filter.
The actual observed retry is 568 passes and 1,032 inventory names.

All seven actual tested parent Rust postimages are integrated. The
[complete source postcheck](../parent-integration-v2/postcheck.json) covers all
1,290 Ferric source bodies, including 228 unchanged qualified worker bodies
and the original parent Cargo.lock. Only the seven declared parent paths were
formatter targets; the runtime/worker suites were not rerun by this parent job.

The [original terminal](evidence/complete.json) is 4,163,125 bytes, SHA-256
`a59c39cfa2be0ec11d0ffa6a352576814917243627ce9a73aca1d4f6501266e9`.
The final source map is 538,263 bytes, SHA-256
`f8b92a7b220c9733e75aa7745498d915217241369ea2a003edfbdda55f52d4b2`.
The original evidence archive is 3,223,863 bytes, SHA-256
`757ab42ddb3fb6601df05be72a42e33f92958655b0e76dc69c25b3f0bff22091`.
All 508 original members are retained, including 344 raw bodies and 131
lineage inputs. Export rehashed all 1,293 sources, 121 dependency roots,
247 private-cache files, eight tools and seven products. Local retention
authenticates archived bytes and metadata, not remote-only executable bodies.

The Readiness40 parent executable is 14,246,864 bytes, SHA-256
`9cf24b52ffcb6b4d9cdeb6b58c3f97668f9ce20f550463d40451c21483a4e0b6`.
It is a CPU-qualified product, not a native GPU outcome. The separately retained
[matched Census V3/Tail V4 comparison](../matched-timing-gpu-v2/README.md) has
since passed. No native correctness, numerical acceptance,
Full2303 feasibility, performance gain or production admission follows from
this result. Original deadlines, CPU affinity and storage floors are unchanged.
