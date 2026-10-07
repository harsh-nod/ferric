# Bank-Scoped Rearm Qualification

This opt-in experiment scopes currentness discovery across the 36 completed
layers of one guarded-MLP bank rearm. It preserves per-entry identity, signal,
queue and generation checks, entry/exit Full currentness checks, and poison-on-
failure behavior. Ordinary rearm and allocation preflights remain unchanged.
The scoped window is not claimed temporally equivalent to ordinary Full checks.

The runtime and worker were qualified together on MI350 and their actual Rust
postimages integrated. The complete suites passed 1,152 runtime tests (eight
existing ignored) and 760 worker tests (four existing ignored). The original
failed build is retained separately, as is the successful repaired attempt.

- [Successful coupled CPU qualification](cpu-attempt-v2/README.md).
- [Original worker test-build failure](cpu-attempt-v1/README.md).
- [82-test admission-checker qualification](checker-cpu-v1/README.md).
- [Integration postcheck](integration-v1/postcheck.json).
- [519-test host integration qualification](parent-cpu-attempt-v2/README.md).
- [Parent integration postcheck](parent-integration-v1/postcheck.json).
- [Nine-test report qualification](matched-timing-report-cpu-v1/README.md).
- [Successful same-binary native pair](matched-timing-gpu-v1/README.md).
- [Measured ablation, plots and reproduction commands](matched-timing-report-v1/README.md).

Host integration passed all 67 phases, and its seven tested Rust files are
integrated with a complete 1,273-body source postcheck. The same-binary scoped-layer
versus bank-plus-layer GPU pair now also passes all 40 prompt records, four
complete captured payloads and clean shutdown. Warm-forward parent wall time
is 45.820% lower in this ordered pair; total parent wall time is 15.613% lower.
First-use forward time is 1.570% higher. Allocation preflights remain unchanged.

This zero-generated-token engineering experiment does not establish independent
numerical acceptance, decode throughput, GPU overlap, Full2303 launch feasibility
or the 700 tokens/s target. Scoped currentness changes observation cadence; it is
not temporally equivalent to repeated full discovery.
