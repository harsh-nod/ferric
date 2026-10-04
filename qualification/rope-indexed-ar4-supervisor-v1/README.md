# Linked RoPE AR4 Supervisor Qualification

All 74 synthetic policy tests passed on MI350 in 1.829 seconds, with no
failures, errors or skips and unchanged source snapshots. This qualifies
the tested admission and lifecycle policies only. It is not GPU execution,
numerical acceptance or a decode benchmark.

The supervisor admits the exact newly emitted gfx950 prefix image while
retaining the checked producer and indexed consumer as separate generations.
It preserves the existing CPU1037 runtime binaries, SiLU and projection
images, four-step own-output recurrence, one-attempt limit, resource bounds,
process ownership, six idle/topology audits and tensor/transcript validation.
The historical failed compiler aggregate is never relabeled as a success.

| Test Group | Passed |
| --- | ---: |
| Supervisor lifecycle | 11 |
| Autoregressive validation | 17 |
| Base intake | 16 |
| SiLU lineage | 8 |
| Retained RoPE admission | 7 |
| Linked producer/consumer admission | 15 |
| Total | 74 |

The preceding supervisor had 65 tests. Six cases specific to a successful
nine-fresh-stage compiler result were replaced by fifteen tests for the
actual split-generation emission. The other 59 cases remain. The package
documentation names the replaced cases; the result retains the complete
executed before/after test census and transcript.

The new image still needs an actual autoregressive GPU run and independent
tensor-level comparison. No sustained 2,048/256 result, 700 tokens/s result,
production admission or full-model numerical acceptance is claimed.
