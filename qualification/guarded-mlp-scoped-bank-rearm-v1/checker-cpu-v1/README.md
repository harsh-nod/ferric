# Bank Rearm Comparison Checks

All 82 synthetic tests passed on `mi350` in 34.057 seconds. The single owned
test process exited naturally, was reaped, and left no process group. Source,
tool and evidence postchecks passed. GPUs were hidden throughout.

All 70 prior admission and timing tests remain unchanged. Twelve new tests
cover the explicit bank-scoped policy, separate bank/layer counters, integer
overflow and framing, original stderr authentication, exact complete captured
payloads, and a same-executable scoped-layer versus bank-plus-layer comparison.
Both sides retain the existing 124-span parent host timing format.

The synthetic comparison requires all 40 semantic records and four complete
selected payloads to agree. It checks identical parent/worker executable and
CPU-qualification metadata; actual executable qualification and owned native
process authentication remain separate requirements of the GPU runner.

This is not a GPU run, numerical model acceptance, measured bank-rearm gain,
GPU-overlap trace or tokens-per-second result. No native data is substituted
by the synthetic fixtures.

The capsule retains 28 originals: eighteen executed sources, seven raw
records, the original terminal, the data-only collector and its manifest.
There are 27 manifest pins; local `retention.json` is separate bookkeeping.

- Terminal: 25,828 bytes, `ecaab2e083039d021095cf6778748bebf978f43b57addea7344f99741f10eace`.
- Archive: 61,834 bytes, `af89e7eaf26a0b7ae5516d4627d5db94e211ac035d862450fb9a0d19b543dffe`.
