# Scoped Full-Request Comparison Qualification

All 42 synthetic data tests passed on `mi350` in 31.152 seconds. The single
owned test process exited naturally, was reaped, and left no process group.
Source, tool and evidence postchecks passed. GPU visibility was empty.

The suite preserves all 34 prior full/scoped admission tests and adds eight
comparison tests. The adapter separately authenticates the original parent
wrapper, retained summary, policy stderr and owned process records. It then
uses the existing exact comparison of all 256 generated token IDs and raw
decoded bytes. No tie exemption, tolerance adjustment or teacher-forced
substitution is introduced.

The added cases cover mismatched independent histories, decoded bytes,
source records, original policy, late forward-chain/feedback records, final
Close, command selection, process retirement, and prompt/model identity.

This qualification tests the comparator with synthetic data. It is not an
actual native/reference model comparison, a GPU run, full-tensor numerical
acceptance, a throughput measurement, or approval to launch a long request.

The capsule contains 24 originals: fourteen executed source bodies, seven
raw records, the original terminal, the data-only collector and its manifest.
The manifest pins 23 bodies. `retention.json` is separate local bookkeeping.

- Terminal: 19,089 bytes, `68e93b8a12e870b5b25a5272120075642e3ac6377b25d1e78d2e6dcd297c2841`.
- Archive: 60,260 bytes, `46e886f2e1ebdcf6f491c5fc50f6116e80edaa74b67f5f1aaee69fe62e2461fb`.

The real BF16 target-only 2,048/256 generated-output and performance gates
remain open.
