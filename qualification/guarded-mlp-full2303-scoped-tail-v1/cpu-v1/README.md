# Full-Request Scoped Tail Worker Qualification

A fresh CPU run on `ssh mi350` passed all 27 phases in 186.074 seconds.
The runtime passed 1,180 tests with eight existing ignores; the worker passed
838 tests with four existing ignores. All earlier named outcomes are preserved,
with exactly seventeen new library tests and one real-executable CLI test.
The fifteen scoped-tail tests, ten selected facade doctests and eight parser
regressions also passed. This is CPU qualification, not GPU execution.

All phases exited naturally, were reaped and left no owned process group.
Source, dependency, tool and product postchecks passed. Eleven Cargo-selected
executables and 142 original raw files were authenticated. The worker ELF
tested by the CLI targets is the final built executable: 6,545,296 bytes,
SHA-256 `11699f6c0dabf0935aa6c09e5c43a20a9339999f408d3959ba7fce2c97d25935`.

The twelve worker Rust paths add an explicit full-workload bank/census/tail
selector and closed ordering/counter validation. All 827 runtime fixture
bodies are unchanged. Only eight of the twelve allowed worker paths changed
during formatting; the final 1,063-source map remained stable throughout tests.
No lockfile, dependency, default route, numerical operation or deadline changed.

Independent original-data review authenticated all archive members, named
outcomes, phase retirement and product identities, and reviewed every formatter
diff. The twelve actual tested postimages are integrated; a separate full
[canonical postcheck](../worker-integration-v1/postcheck.json) verifies 1,294
Ferric bodies and 825 unchanged canonical runtime bodies. The original workspace
Cargo files are preserved, not replaced with the CPU harness fixtures.

The [original terminal](evidence/complete.json) is 2,498,847 bytes, SHA-256
`f25bd6d9f582a4e632c05e385961c260c0c95d2a119bac5af4927af6dc3b61ed`.
The [source map](evidence/sources-after.json) is 435,848 bytes, SHA-256
`eda7f5f696a5b93d048923cd5f8bd83b51a5ca9cb77efab8322da97f4286e928`.
The exported archive is 1,764,612 bytes, SHA-256
`4993f3cdd744f2f280987d4197630454e69f578e1e9136b026bff3e75eedc3fc`.
All 182 original members are retained, with 181 bodies pinned by the manifest.
The exporter rehashed live sources, tools, dependencies, private cache and
products. Local retention checks originals; it does not reobserve remote ELF
bodies or processes. This README is separate commentary, not an archive member.

This result does not establish Full2303 launch feasibility, independent model
acceptance, generated-token throughput or 700 tokens/s. The one-hour bound,
resource limits and exact independent 256-ID/raw-decoded-byte gate remain.
