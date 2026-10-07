# Native Decode Diagnostic

This is the byte-identical, remotely tested driver for the separate a004
binary-only diagnostic controller. It is an engineering checkpoint, not a
general-purpose serving launcher or a qualified performance optimization.

- `decode_capture.py`: pure bounded replay of the three stderr records and
  registered decode window, including exact counts, endpoint identities,
  metadata sizes, chronology, durations and independently recomputed deltas.
- `run_decode.py`: two fresh one-request processes, sequential A then B. Uses
  the immutable a004 campaign's isolation/lifecycle helpers and image provenance,
  but independently requires the new controller's CPU qualification.
- `prepare_capture.py`: create-only private tmpfs staging from pinned artifacts.
- `qualify_capture.py` and `test_decode_capture.py`: the exact 15-test remote CPU
  cohort, including bad-token, malformed-stream and failed-teardown paths.

Builds and tests run on `mi300x-2`; native capture runs on `mi350`. Do not run the
tests locally. The scripts intentionally require the retained private stages,
physical device, ELF, source and evidence identities. They are not portable
merely by replacing a pathname. Further changes require a new qualification
and fresh output directory; failed records are never overwritten.

The qualified Rust controller and formatted source are retained in
`gate-up-decode-cpu-a004-retained/evidence.tar.gz` under the performance workspace.
Its SHA256 is
`b95c39aebf61b9bdeb4cf5cc7c904a0599012f63f6270723268276b26313dfdc`.
The source is deliberately separate from the working adapter and unchanged
historical controllers. This tool projection is not an adapter integration,
new build, or source-to-binary proof.

Both native requests completed with exact 128-token parity and clean shutdown.
All 493 files, 32,260,311 logical bytes, are retained locally and match the remote
content roster. The [checkpoint](../../docs/performance/gate-up-decode-diagnostic-a004-checkpoint.json)
records qualification and custody.

All timing is host wall time. Worker counters overlap, wait includes polling,
sleeping, currentness and retirement, and registered-span/127 is not TPOT.
Never add overlapping counters, call these GPU/shader timings, promote the
candidate from these two requests, or compare them to old HTTP vendor samples.

Remaining fixture gap: the successful preceding-A raw-tamper validation and
endpoint-retention `finally` branches were source-reviewed but lack direct
dedicated fixtures in this 15-test cohort.
