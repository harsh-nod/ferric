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
is retained. A test-only repair and fresh coupled/parent qualifications are
required before the same-binary Census V3/Tail V4 GPU comparison. No tail native
result or speedup is claimed yet. This change does not
add a Full2303 tail route, relax the one-hour bound, or resolve the independent
position-5 numerical discrepancy. Exact independent 256 output IDs/raw decoded
bytes, repeated equal-work performance, GPU overlap and 700 tokens/s remain
open. All issue #42 M0-M7 gates remain open.
