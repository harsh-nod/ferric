# Combined State Vendor Preparation

Status: **offline preparation passes on MI350** for combined-state CPU
attempt v3. This prepares the exact dependency source tree for a separate
checked compile; it is not compiler, HSACO or GPU acceptance.

The [actual receipt](attempt-v3/evidence/complete.json) records one bounded
`cargo vendor --offline --locked --versioned-dirs` phase. It exits naturally
with status zero, is reaped and leaves no process group. Qualified CPU sources,
the Rust source tree and consumed inputs remain unchanged; postchecks are
clean. Whole preparation time is 2.569767 seconds, not kernel latency.

The [retained capsule](attempt-v3/retention-manifest.json) contains 17 members,
16 content pins and all ten raw records. Four selected CPU parent files bind
the preparation to the actual 39-test qualification, its controller, input
manifest and final source map. The dependency/vendor file maps are retained,
not all source bodies. No public Cargo configuration or compiler authority is
created by this preparation.

- Receipt: 16,533 bytes, SHA-256
  `30b139d4d35084ab8ba37dd32bf07b2d63d4341ef449cf9da00b5d446ef8916d`.
- Archive: 1,869,403 bytes, SHA-256
  `119e50313966a8eec05ce729655d7bd385db4e2707297c9af36585eba3846d8e`.

All issue #42 milestones, GPU/model acceptance and the 700 tokens/s target
remain open.
