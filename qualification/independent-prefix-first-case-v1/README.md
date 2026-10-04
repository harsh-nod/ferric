# First Independent Prefix GPU Case

The newly compiled gfx950 prefix image ran on MI350 for `genuine-pos0`, and
both baseline-v5 and tiles-v6 passed the independent, conditional numerical
checks. The GPU observation and subsequent CPU reference comparison are
actual executions, not synthetic policy tests.

| Check | Result across two profiles and two ranks |
| --- | --- |
| Normalization | Zero bound violations |
| QKV projection | Zero bound violations |
| Head normalization, RoPE and current KV append | Accepted by the unchanged conditional reference |
| Attention | 8,192 BF16 elements exact; zero tolerated differences |
| Output projection partials | Maximum error/bound ratio `0.015052407427894559` |
| Candidate useful workgroups | 64 observed on each rank |

The output ratio is about 1.51% of the allowed bound, not a speedup. The maximum
reported output absolute-error upper bound is `6.035406840965153e-8`. Each
profile is checked independently against its actual preceding-stage inputs
and the fixed reference policies. A paired bitwise comparison is not the
acceptance criterion.

## Execution and Custody

The GPU case has one native parent attempt and two ordered profile children,
baseline first. Eight owned leaves completed naturally: inspection, native,
three pre-audits and three post-audits. Both profiles reported Close. Ten
child sidecars and four full captures were retained, totaling 19,030,016
capture bytes. The six audit outputs show the selected physical GPU pair idle
without an unrelated-process exception.

After GPU completion and post-audits, a separate CPU leaf on MI350 replayed the
actual case and ran the independent prefix, attention and output references.
The local publisher does **not** rerun those references. It hashes the complete
61-file GPU case, joins the actual CPU receipt to the GPU observation and
the four profile/rank captures, and preserves the case-scoped arithmetic
review. Captures remain in session evidence; their bytes are not committed.

`result.json` contains the compact qualification and exact artifact pins.
`gpu-observation.json` and `numerical-observation.json` are the unchanged raw
receipts. `numerical-inputs.json` and `arithmetic-review.json` preserve the
supplied input and assumption joins. `publisher.py` records the local byte
audit and publication gate.

## Limits

This is one prefix case at position zero, not a full model or sustained decode
benchmark. The other five selected cases are not covered by this publication.
The observed 64 useful candidate workgroups per rank are runtime observations,
not a proof of fairness, universal progress or a performance improvement.

Checks remain conditional on the recorded arithmetic assumptions, actual
preceding stages and authenticated supplied rotary values. Universal sqrt,
ordinary attention division and OCML-exp error premises, as well as the
emitted runtime requirements, are not discharged by this case. The raw
receipts therefore retain false blanket numerical/full-prefix/production
authority fields while reporting true conditional operator checks.

No performance parity, device-only timing, full-model correctness, production
readiness or 700 tokens/s claim follows from this result.
