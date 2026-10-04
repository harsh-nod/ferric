# Independent Prefix Case Matrix

The gfx950 prefix image and independent native adapter completed the six
selected MI350 cases. Baseline-v5 and tiles-v6 were each checked independently
against the unchanged conditional prefix, attention and output references.
Every row below comes from the actual GPU and subsequent CPU receipts.

| Case | Norm / QKV Violations | Attention Exact / Tolerated | Max BF16 Steps | Max O Bound Ratio | Useful Candidate WGs (Ranks 0 / 1) |
| --- | ---: | ---: | ---: | ---: | ---: |
| genuine-pos0 | 0 / 0 | 8192 / 0 | 0 | 0.0150524074 | 64 / 64 |
| genuine-pos4 | 0 / 0 | 8192 / 0 | 0 | 0.020596355 | 64 / 64 |
| patterned-pos15 | 0 / 0 | 8192 / 0 | 0 | 0.00831735947 | 64 / 64 |
| patterned-pos16 | 0 / 0 | 8192 / 0 | 0 | 0.0110380209 | 64 / 64 |
| patterned-pos2047 | 0 / 0 | 8188 / 4 | 1 | 0.0229802254 | 64 / 64 |
| patterned-pos2048 | 0 / 0 | 8188 / 4 | 1 | 0.0229954712 | 64 / 64 |

Each case contains two profiles and two ranks, or 8,192 attention BF16 elements.
The output bound ratio is error divided by the fixed reference bound, not a
speedup. A ratio at or below one passes that bound. Workgroup counts are
observations from the actual candidate terminal state, not a launch-grid
size, fairness guarantee or performance result.

## Unchanged Attention Policy

Position zero must be exact. At a nonzero position, the preregistered policy
accepts an element when either its BF16 distance is at most one step **or**
its absolute error is at most `5e-5 * per-KV-head max(abs(causal V))`. Thus
a larger BF16 step count near cancellation is not itself a failed result.
Exact and tolerated counts are reported separately; no tolerance is enlarged
to accommodate this matrix. Nonfinite query, causal K/V, score and output
values are rejected by the reference checks.

The policy SHA is
`438b10d7cf2bdc7cd693024edcfa7f3f84929e4a852787d72964c43263cd0fc2`.
The prefix policy and output-reference hashes are also fixed in `result.json`.
The local publisher checks the recorded policy identities and authenticated
passing results; it does not recompute the per-element mathematical bounds.

## Execution and Evidence

Each fresh case uses one native parent attempt, baseline-first execution of
two profile children, zero retries, eight naturally completed owned leaves,
three pre-audits and three post-audits. Across the matrix this is 48 leaves,
36 device/process audits, 60 child sidecars and 24 full rank captures.
The total capture extent is 114,180,096 bytes. Captures remain in session
evidence and are not committed to Git.

After each GPU case and its post-audits, a separate bounded CPU leaf on MI350
replayed the actual captures and ran the independent conditional references.
The local publisher rehashes all 366 GPU-case files, checks the exact leaf and
audit joins, and binds every profile/rank numerical row to the actual capture.
It does not launch kernels, rerun references or change the historical receipts.

`result.json` contains the compact per-case metrics, policies and receipt pins.
Each `cases/<case>/` directory contains unchanged GPU and numerical receipts,
numerical inputs and the case-specific arithmetic review. `publisher.py` is
the standalone retained-byte audit and publication gate. The earlier
first-case qualification is not copied into this matrix publication.

## Limits

These are six selected prefix cases, including KV-page and long-context
boundaries. They are not a full-model run or the sustained 2,048/256 decode
benchmark. Matching profiles is not the acceptance criterion.

Checks remain conditional on actual preceding-stage values, authenticated
supplied rotary values and each recorded arithmetic-assumption review.
Universal sqrt, ordinary attention division and OCML-exp error premises and
the emitted runtime requirements are not discharged by this matrix. Runtime
owner counts do not establish universal scheduler progress or fairness.

The receipts retain false blanket numerical/full-prefix/production authority
fields while reporting true conditional operator checks. No performance
parity, full-model correctness, production readiness or 700 tokens/s claim
follows from this publication.
