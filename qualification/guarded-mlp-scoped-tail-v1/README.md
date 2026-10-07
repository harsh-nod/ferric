# Scoped Tail Currentness

This opt-in engineering path applies one closed currentness window to the
final normalization, vocabulary head, argmax and three readbacks. It preserves
the original serial dispatch order, local rank/group checks, deadlines, queue
retirement and output validation. Entry and exit still perform full discovery;
intermediate checks retain generation probes and local predicates. This is not
temporally equivalent to repeated full discovery.

The runtime and worker passed [fresh MI350 CPU qualification](cpu-v1/README.md).
All six runtime and thirteen worker source postimages were integrated exactly
as formatted and tested. The [complete source postcheck](integration-v1/postcheck.json)
covers 825 canonical runtime files and 1,288 Ferric files. The reduced fixture
Cargo manifests are not integrated. The [strict checker](checker-cpu-v1/README.md)
also passed all 114 synthetic tests on MI350.

The explicit worker selector is
`--engineering-native-guarded-mlp-readiness40-position5-bank-scoped-census-tail-v4`.
It is limited to the existing 40-position prompt diagnostic. The first two
forwards remain ordinary; the following 38 use the new tail window after
the existing bank/layer/census path. Tail counters are separate from layer
and allocation counters. Errors and unwind are terminal, without fallback.

The first [parent qualification failed at test compilation](parent-cpu-failed-v1/README.md)
because a shared test referenced a worker-only module. Its original evidence
is retained. The [test-only repair passed a fresh coupled qualification](cpu-attempt-v2/README.md)
and is integrated with a complete canonical source postcheck. The
[fresh parent retry](parent-cpu-v2/README.md) passed all 68 phases and 568
selected tests; its seven actual postimages are integrated with a complete
1,290-file source postcheck. The [final bound report tool](matched-timing-report-cpu-v1/README.md)
also passed all 13 synthetic tests on MI350, with independently checked
original evidence. The [same-binary Census V3/Tail V4 GPU pair](matched-timing-gpu-v2/README.md)
now passes in one attempt per mode, with all 40 records and four complete
payloads matching. The [report with tables and plots](matched-timing-report-v2/README.md)
shows 5.410224% lower warm-forward parent wall time but only 0.234021% lower
complete-parent time; first-use time rose 1.119270%. This is one ordered
host-wall pair, not repeated throughput or GPU timing. This change does not
add a Full2303 tail route, relax the one-hour bound, or resolve the independent
position-5 numerical discrepancy. Exact independent 256 output IDs/raw decoded
bytes, repeated equal-work performance, GPU overlap and 700 tokens/s remain
open. All issue #42 M0-M7 gates remain open.
