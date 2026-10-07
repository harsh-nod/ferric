# Successful CPU Retry

The fresh `ssh mi350` attempt passed all ten phases in 8.456 seconds,
including all ten library tests with zero failures, ignores or filtered tests.
It then completed two warmup pairs and twelve fixed alternating measured pairs.
All children exited naturally, were reaped and left no process group.

Compilation used the unchanged safe Rust benchmark, four pure local loader
crates, twenty authenticated registry dependencies and a fresh private offline
Cargo cache. Release opt-level 2, debug assertions and overflow checks remained
enabled. Only three benchmark Rust files were formatted; runtime sources and
the qualified reduced workspace were unchanged. Source, lock, eight tools,
73 cache inputs, dependencies and both selected executable postchecks passed.

The benchmark leaf retained its 60-second wall, 45-second CPU and 256 MiB
address-space limits. The whole controller retained 900 seconds, CPUs 8/9,
nice 10, hidden GPUs and 40/38 GiB initial/live storage floors. No model or GPU
work occurred.

The [original terminal](evidence/complete.json) is 1,905,350 bytes, SHA-256
`33f13e8bbe3bafefc51611e4367994ec1542b1a2d5e080793cb819108d802a60`.
The [raw benchmark report](evidence/benchmark.stdout) is 9,738 bytes, SHA-256
`df3e09b004746f20888fd536e58e0272c6deb0afa8ffe68081d3bee8a9105500`.
The archive is 2,857,470 bytes, SHA-256
`8260cc77a586e13a7099f97fcf437da7f85dd2c9d1c68ff937e70716f610494c`:
903 members, 902 manifest pins, 17,200,231 expanded bytes.

The capsule retains all 843 tested source bodies, four original images, the
generated Cargo.lock, 56 raw files and original stage ledger. The exporter
rehashed live sources, tools, cache, twenty dependency roots and two products;
ELF and cache-archive bodies are not packed. Local retention authenticated
archive bytes but did not repeat those remote observations. This README and
retention.json are not archive originals.

See the [result and interpretation](../README.md). This is a pure-loader
opportunity measurement, not native dispatch or inference performance.
