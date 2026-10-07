# Timing-Record Checker Retry

The fresh MI350 retry passed all 126 synthetic tests in 72.089 seconds of
controller wall time. The original 114 tests and twelve new diagnostic cases
all passed, with no failures, errors or skips. The sole child exited naturally,
was reaped and left no process group; postcheck errors are empty.

The repair is fixture-only: the generic Tail fixture's bank counts are
replaced by this diagnostic's fixed per-forward counts. The positive full
fixture must validate before mutation tests run. The production validator and
all inherited test bodies are unchanged. The [original failed
attempt](../checker-cpu-failed-v1/README.md) remains retained.

The checks cover original two-record stderr, policy identity and counts,
canonical framing, ordered forwards, bounded integers, callback/body overlap,
all 124 parent timing spans, original file pins and same-side payload parity.
Synthetic tests do not establish measured host costs or model correctness.

The [original receipt](evidence/complete.json) is 35,853 bytes, SHA-256
`92dc097186d0ff256018612f0e759ef0a5a81f232556463596d653948819ffad`.
The original archive is 78,412 bytes, SHA-256
`a77decc27f0759bb7dd25fd68f606b63872f288724c303cdf83302792da99fdb`:
36 members, 35 manifest pins and 425,886 expanded bytes. All 26 deployed
sources, seven raw files, terminal, exporter and manifest are retained.

After a transport interruption, a fresh read-only connection confirmed the
original successful terminal; the original SSH session also exited zero.
No duplicate run was submitted. Local retention checked the original bytes
as data without executing retained code. This README is added commentary.
