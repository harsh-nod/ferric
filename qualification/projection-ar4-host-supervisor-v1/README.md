# Projection AR4 Host Supervisor

The frozen 21-member supervisor package passed all 91 synthetic policy tests
on MI350, with zero failures, errors or skips and unchanged source hashes.
This checkpoint qualifies Python admission, validation and supervision policy
only. The new Rust build, runtime audits and native host-observation run are
not established by this receipt.

| Test Module | Passed |
| --- | ---: |
| Native AR4 structure | 17 |
| Host wrapper and sidecar | 8 |
| Intake | 16 |
| Linked-emission admission | 15 |
| Scoped observer CPU admission | 8 |
| RoPE admission | 7 |
| Owned supervisor | 12 |
| SiLU admission | 8 |
| Total | 91 |

The package preserves the existing AR4 arithmetic validator and checked
RoPE/SiLU/residual image provenance. It admits only an actual successful new
scoped CPU receipt and matching executable artifacts, not predicted hashes.
Fresh runtime reviews remain required for the rebuilt parent and worker.

The new validator binds the diagnostic stdout wrapper to the exact native
summary bytes and a separate bounded sidecar. Tests cover Rust byte-array and
external hexadecimal FilePins, profile/completion/Close joins, seven snapshots,
six monotone counter deltas, forward and serialization interval bounds, and
failure retention with three pre-audits, three post-audits and no retry.
These fixtures do not measure real host latency or independently run a model.

## Retained Evidence

The publication retains the exact package and manifest, plus the four raw pure
records: completion, source-before/source-after maps and test transcript.

- Completion, 20,829 bytes: `14852452f3a5111a1982575090ce4a3c96c4d6b1cfddda0539f64c317bbd8a80`.
- Package manifest: `8ce9861812a27d115e68edb7f2fb0a3f6fc5aaf3ee045bd70313356aa9d333b1`.
- Bounded test wrapper: `f9e38e0793bb4f2f5d285d29ef8df817d83cf67f66207153c353c161a2dfa6e2`.
- Identical source maps: `ab6ed83646ce58918e5416490ee704625f4a4169e54b1697e88f5703e3af3654`.
- Test transcript: `cea4d32e9425efa825b44113537a353c348eb19d13600f6858cb790111a45fea`.

Root executed the CPU-only tests under hidden-GPU, CPU8/9, nice10 and bounded
resource settings. The source package's authored-status prose is preserved
byte-for-byte; this receipt records the subsequent actual policy test result.

No native/GPU execution, Rust qualification, numerical or full-model acceptance,
2048-token prefill/256-token decode, calibrated GPU timing, speedup, throughput
or production authority is claimed by this checkpoint. Future host counters
are inclusive overlapping scopes, not additive GPU durations.
