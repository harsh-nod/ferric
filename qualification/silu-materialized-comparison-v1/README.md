# BF16 SiLU Materialization: Independent Comparison

The new [MI350 layer-zero capture](../silu-materialized-native-capture-v1/README.md)
was compared on ASROCK with the retained independent framework reference.
The comparison covers token 9112 at position zero, twelve BF16 stages on each
TP rank. It retains all differences without choosing an acceptance threshold.
This is a numerical diagnostic, not full-model acceptance or a performance test.

## Measured Change

Only the selected MLP image changes from the corrected-residual baseline. The
kernel now rounds SiLU to BF16 before multiplying by up; exponential arithmetic,
Down2 scheduling and the runtime are unchanged. All 22 pre-SwiGLU arrays are
byte-identical between native captures. The same framework reference is used for
both columns below.

| Stage | Rank | Baseline Exact Words | New Exact Words | Total Words | Baseline Relative L2 | New Relative L2 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Activation product | 0 | 4,235 | 5,766 | 6,144 | 0.0022693397 | 0.00043476921 |
| Activation product | 1 | 4,318 | 5,777 | 6,144 | 0.0017507864 | 0.00022103495 |
| Final hidden | 0 | 2,728 | 3,807 | 4,096 | 0.0017142513 | 0.00059154807 |
| Final hidden | 1 | 2,728 | 3,807 | 4,096 | 0.0017142513 | 0.00059154807 |

Final-hidden maximum absolute error falls from 0.0078125 to 0.00390625.
The two ranks have identical final-hidden outputs, so these are not two
independent samples. [table.md](table.md) contains all 24 baseline/candidate
rows, conditional checks and native FP32 Down-partial changes;
[table.json](table.json) preserves the reported precision and identities.

## Controls And Limits

The independent conditional residual calculation matches all 16,384 words
across the two residual stages and both ranks. This conditions on native
projection partials; it does not validate those partials against a full
framework projection.

The framework product control reproduces all 12,288 words from captured BF16
SiLU and up. Where native gate words equal the framework gate, the new native
products match the conditional materialized prediction on all 5,911 rank-0 and
5,958 rank-1 words. Previously, 1,594 and 1,542 words differed. These controls
use the actual native up values, including cases where up differs from the
framework. They do not predict products for different gate values, observe the
native SiLU intermediate, or prove universal OCML/framework equivalence.

The first observable framework difference remains QKV on rank zero: one of
3,072 BF16 words differs. Gate and up already contain differences before SiLU.
Observation ordering is not causal attribution. No tolerance was fitted to
these measurements; full-layer and full-model numerical acceptance remain open.

## Reproduction Evidence

[complete.json](complete.json) is the actual bounded comparison receipt,
SHA-256 `828f8fdb4fc4194b5d7d5c65135222b472d2ec76080667fae8f0b8febba169aa`.
It replays the candidate's seven owned process records and six historical
device audits, checks 21 framework process results, and records the source and
consumed-data identities. It performs no new GPU launch or current-device audit.
The twelve synthetic comparator tests passed separately with no skips;
their actual transcript and source snapshots are in [pure](pure).

[result.json](result.json) records publication identities and limitations.
The publisher copies the measured results and source closure; it does not
recompute numerical metrics or reread every consumed tensor buffer. Tensor
payloads remain outside Git, identified by exact pins. Source READMEs retain
their author-time status; the actual receipts above establish execution status.

The subsequent [four-step, 36-layer comparison](../silu-materialized-decode-comparison-v1/README.md)
records the same image's mixed full-model diagnostic results. This layer-zero
checkpoint does not cover the 2,048-token prompt / 256-token target, production
admission, or the 700 tokens/s objective.
