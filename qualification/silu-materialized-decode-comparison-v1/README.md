# SiLU Four-Step Framework Comparison

The [MI350 four-step run](../silu-materialized-decode-native-v1/README.md) was
compared on ASROCK with the retained independent framework execution. All 152
tensor slices were compared and all four output tokens match. None of the 152
complete tensor slices is bitwise identical. These are diagnostics, not a
numerical acceptance decision or performance result.

## Measured Results

Both runs use BF16 Qwen3-8B and teacher-forced inputs `[9112, 2190, 3772, 220]`
at positions 0-3. The baseline already has corrected projection-residual
materialization; the candidate additionally materializes SiLU in BF16 before
the up product. Both are compared with the same genuine reference tensor hashes.

| Position | Reference / Native Token | Baseline Logit Relative L2 | Candidate Logit Relative L2 | Baseline Max Abs | Candidate Max Abs |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0 | 67 | 0.0033234414 | 0.0030448043 | 0.0625 | 0.0625 |
| 1 | 198 | 0.0075731107 | 0.0056276866 | 0.1875 | 0.1875 |
| 2 | 25 | 0.0041078125 | 0.0061773318 | 0.125 | 0.1875 |
| 3 | 16 | 0.0085935474 | 0.0079468052 | 0.25 | 0.1875 |

The result is mixed: relative-L2 error decreases at positions 0, 1 and 3,
but increases at position 2. Position-2 logit maximum absolute error also
increases. The stronger layer-zero agreement does not establish uniform
improvement across the model or subsequent KV histories. No acceptance threshold
was selected from these measurements, and token agreement alone is insufficient.

[table.md](table.md) retains all 152 baseline/candidate tensor rows and the four
token comparisons. [table.json](table.json) preserves the reported precision,
exact-word counts, first differences, identities and per-layer trajectories.
Baseline metrics are copied from the authenticated earlier comparison, not
recomputed; the adapter checks the identical reference identity for every row.
The trajectories describe changes, including decreasing error, without asserting
causal attribution or monotonic growth.

## Validation Evidence

The new comparator passed all 17 synthetic tests on ASROCK, with no skips or
source changes. [tests](tests) retains the transcript and source snapshots.
The actual comparison rehashes all eight genuine framework payloads from two
fresh-cache reference passes, checks all four native payloads, and replays the
native structural validator, Close/reap records and six historical audit leaves.
It does not launch another GPU run or treat historical MI350 audits as current
ASROCK device state.

[complete.json](complete.json), SHA-256
`58152fbcc0be9a0894ab6f7530811a968aa9688a7e18e90a91e9ef42ec995dac`,
records the actual comparison. [result.json](result.json) binds the copied
source, test evidence and tables. Publication validates metadata and copies
metrics; it does not recompute tensor arithmetic or reread every consumed buffer.

This checkpoint does not cover autoregressive generation, a 2,048-token prompt,
256 generated tokens, a predeclared full-model numerical acceptance policy, or
sustained throughput. Remaining differences need investigation before claiming
full-model correctness. All issue #42 milestones and the 700 tokens/s goal
remain open.
